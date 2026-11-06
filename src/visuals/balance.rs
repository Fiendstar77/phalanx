//! Bounded pose corrections on top of authored clips; never move Battle actors.
use super::*;
use std::collections::BTreeMap;
#[derive(Component)]
struct Base {
    transform: Transform,
    applied: bool,
}
#[derive(Component)]
struct Rig {
    id: usize,
    female: bool,
    bones: BTreeMap<String, Entity>,
    previous: Vec3,
    velocity: Vec3,
    lean: Vec2,
    feet: [Option<Vec3>; 2],
    run: u32,
}
#[derive(Resource, Default)]
pub(super) struct Audit {
    pub plants: u64,
    pub max_correction: f32,
    pub invalid: u32,
}
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Pose;
pub(super) fn install(app: &mut App) {
    app.init_resource::<Audit>()
        .add_systems(PreUpdate, restore)
        .add_systems(
            PostUpdate,
            pose.in_set(Pose)
                .after(face::Pose)
                .before(bevy::transform::TransformSystems::Propagate),
        );
}
pub(super) fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    models: Query<&Model>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&Transform>,
) {
    let Ok(model) = models.get(event.entity) else {
        return;
    };
    let mut bones = BTreeMap::new();
    for e in children.iter_descendants(event.entity) {
        let Ok(name) = names.get(e) else {
            continue;
        };
        let n = name
            .as_str()
            .trim_start_matches("mixamorig")
            .trim_start_matches(':');
        if matches!(
            n,
            "Hips"
                | "Spine1"
                | "Spine2"
                | "Head"
                | "LeftUpLeg"
                | "LeftLeg"
                | "LeftFoot"
                | "RightUpLeg"
                | "RightLeg"
                | "RightFoot"
        ) {
            bones.insert(n.to_owned(), e);
            if let Ok(t) = transforms.get(e) {
                commands.entity(e).insert(Base {
                    transform: *t,
                    applied: false,
                });
            }
        }
    }
    commands.entity(event.entity).insert(Rig {
        id: model.id,
        female: model.female,
        bones,
        previous: Vec3::ZERO,
        velocity: Vec3::ZERO,
        lean: Vec2::ZERO,
        feet: [None; 2],
        run: u32::MAX,
    });
}
fn restore(mut bones: Query<(&mut Base, &mut Transform)>) {
    for (mut base, mut t) in &mut bones {
        if base.applied {
            *t = base.transform;
            base.applied = false;
        }
    }
}
fn matrix(e: Entity, transforms: &Query<&mut Transform>, parents: &Query<&ChildOf>) -> Mat4 {
    let mut chain = Vec::with_capacity(16);
    let mut at = e;
    for _ in 0..32 {
        if let Ok(t) = transforms.get(at) {
            chain.push(t.to_matrix());
        }
        let Ok(parent) = parents.get(at) else {
            break;
        };
        at = parent.parent();
    }
    chain.iter().rev().fold(Mat4::IDENTITY, |m, t| m * (*t))
}
fn rotate_world(
    e: Entity,
    rotation: Quat,
    transforms: &mut Query<&mut Transform>,
    parents: &Query<&ChildOf>,
) {
    let parent = parents.get(e).map_or(Quat::IDENTITY, |p| {
        matrix(p.parent(), transforms, parents)
            .to_scale_rotation_translation()
            .1
    });
    if let Ok(mut t) = transforms.get_mut(e) {
        t.rotation = parent.inverse() * rotation * parent * t.rotation;
    }
}
fn plant(
    chain: [Entity; 3],
    goal: Vec3,
    transforms: &mut Query<&mut Transform>,
    parents: &Query<&ChildOf>,
) -> bool {
    let [upper, lower, foot] = chain;
    let h = matrix(upper, transforms, parents).transform_point3(Vec3::ZERO);
    let k = matrix(lower, transforms, parents).transform_point3(Vec3::ZERO);
    let f = matrix(foot, transforms, parents).transform_point3(Vec3::ZERO);
    let (a, b) = (h.distance(k), k.distance(f));
    let delta = goal - h;
    if a < 0.01 || b < 0.01 || delta.length() < 0.01 {
        return false;
    }
    let d = delta.length().clamp((a - b).abs() + 0.005, a + b - 0.005);
    let direction = delta.normalize();
    let along = (a * a - b * b + d * d) / (2. * d);
    let knee_plane = (k - h - direction * (k - h).dot(direction))
        .try_normalize()
        .unwrap_or(Vec3::Z);
    let desired = h + direction * along + knee_plane * (a * a - along * along).max(0.).sqrt();
    rotate_world(
        upper,
        Quat::from_rotation_arc((k - h).normalize(), (desired - h).normalize()),
        transforms,
        parents,
    );
    let k = matrix(lower, transforms, parents).transform_point3(Vec3::ZERO);
    let f2 = matrix(foot, transforms, parents).transform_point3(Vec3::ZERO);
    let Some(towards) = (goal - k).try_normalize() else {
        return false;
    };
    rotate_world(
        lower,
        Quat::from_rotation_arc((f2 - k).normalize(), towards),
        transforms,
        parents,
    );
    f.is_finite() && matrix(foot, transforms, parents).is_finite()
}
#[allow(clippy::too_many_arguments)]
fn pose(
    session: Res<Session>,
    time: Res<Time>,
    mut rigs: Query<(Entity, &mut Rig)>,
    actors: Query<&Actor>,
    parents: Query<&ChildOf>,
    mut transforms: Query<&mut Transform>,
    mut bases: Query<&mut Base>,
    mut audit: ResMut<Audit>,
) {
    let dt = if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else {
        0.
    };
    for (root, mut rig) in &mut rigs {
        let Some(u) = session.visual_unit(rig.id) else {
            continue;
        };
        if u.health <= 0. || rig.female != female(&session, rig.id) {
            continue;
        }
        // Capture the clip and facial result, then restore before the next animation
        // evaluation. This also prevents cumulative offsets on held or paused clips.
        for e in rig.bones.values() {
            if let (Ok(t), Ok(mut base)) = (transforms.get(*e), bases.get_mut(*e)) {
                base.transform = *t;
                base.applied = true;
            }
        }
        let root_matrix = matrix(root, &transforms, &parents);
        let (_, rotation, pos) = root_matrix.to_scale_rotation_translation();
        if rig.run != session.run_id || pos.distance(rig.previous) > 2. {
            rig.run = session.run_id;
            rig.previous = pos;
            rig.velocity = Vec3::ZERO;
            rig.lean = Vec2::ZERO;
            rig.feet = [None; 2];
        }
        let actor = actors
            .iter()
            .find(|a| a.id == rig.id && a.female == female(&session, rig.id));
        let walking = actor.is_some_and(|a| matches!(a.clip, 1 | 2 | 25..=30 | 76 | 77));
        if dt > 0. {
            let velocity = (pos - rig.previous) / dt;
            let local = rotation.inverse() * (velocity - rig.velocity) / dt;
            let target = if walking {
                Vec2::new(local.z, -local.x) * 0.009
            } else {
                Vec2::ZERO
            };
            let target = target.clamp_length_max(0.10);
            rig.lean = rig.lean.lerp(target, 1. - (-8. * dt).exp());
            rig.velocity = velocity;
            rig.previous = pos;
        }
        let personal = super::individual::signature(u);
        if walking {
            let phase = actor.map_or(0., |a| a.clock / super::CLIP_DURATIONS[a.clip])
                * std::f32::consts::TAU
                + personal.phase;
            if let Some(e) = rig.bones.get("Hips")
                && let Ok(mut t) = transforms.get_mut(*e)
            {
                t.rotation *= Quat::from_euler(
                    EulerRot::XYZ,
                    0.,
                    phase.sin() * personal.turn * 0.045,
                    phase.sin() * personal.sway * 0.07,
                );
                t.translation.y += (phase * 2.).cos() * personal.bounce * 0.012;
            }
            if let Some(e) = rig.bones.get("Spine2")
                && let Ok(mut t) = transforms.get_mut(*e)
            {
                t.rotation *= Quat::from_euler(
                    EulerRot::XYZ,
                    personal.lean,
                    phase.sin() * personal.turn * -0.035,
                    phase.sin() * personal.sway * -0.04,
                );
            }
        }
        if let Some(e) = rig.bones.get("Spine1")
            && let Ok(mut t) = transforms.get_mut(*e)
        {
            t.rotation *= Quat::from_euler(EulerRot::XYZ, rig.lean.x, 0., rig.lean.y);
        }
        // Look with the neck before turning the whole body, using remembered contacts.
        if walking
            && let Some(contact) = u
                .contacts
                .iter()
                .filter(|c| c.confidence(session.battle.tick) > 0.3)
                .min_by(|a, b| {
                    phalanx::distance(u.position, a.position)
                        .total_cmp(&phalanx::distance(u.position, b.position))
                })
        {
            let local = rotation.inverse()
                * Vec3::new(
                    contact.position[0] - u.position[0],
                    0.,
                    contact.position[1] - u.position[1],
                );
            let yaw = local.x.atan2(local.z).clamp(-0.45, 0.45);
            if let Some(e) = rig.bones.get("Head")
                && let Ok(mut t) = transforms.get_mut(*e)
            {
                t.rotation *= Quat::from_rotation_y(yaw * 0.6);
            }
        }
        if !walking
            || rig.velocity.length() < 0.12
            || u.theus
                .as_ref()
                .is_some_and(|a| a.airborne_until > session.battle.tick)
        {
            rig.feet = [None; 2];
            continue;
        }
        for (side, prefix) in ["Left", "Right"].into_iter().enumerate() {
            if u.lost_limbs & (4 << side) != 0 {
                rig.feet[side] = None;
                continue;
            }
            let Some(chain) = ["UpLeg", "Leg", "Foot"]
                .map(|suffix| rig.bones.get(&format!("{prefix}{suffix}")).copied())
                .into_iter()
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let foot = matrix(chain[2], &transforms, &parents).transform_point3(Vec3::ZERO);
            let ground = session.battle.arena.elevation([foot.x, foot.z]);
            let low = foot.y - ground < 0.20;
            if !low {
                rig.feet[side] = None;
                continue;
            }
            let goal = *rig.feet[side].get_or_insert(Vec3::new(foot.x, ground + 0.07, foot.z));
            if goal.distance(foot) > 0.27 {
                rig.feet[side] = None;
                continue;
            }
            let correction = goal.distance(foot);
            audit.max_correction = audit.max_correction.max(correction);
            if plant(
                [chain[0], chain[1], chain[2]],
                goal,
                &mut transforms,
                &parents,
            ) {
                audit.plants += 1;
            } else {
                audit.invalid += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;
    #[test]
    fn planted_ankle_reaches_world_target_under_a_rotated_scaled_character() {
        let mut world = World::new();
        let root = world
            .spawn(
                Transform::from_rotation(Quat::from_rotation_y(0.7)).with_scale(Vec3::splat(0.7)),
            )
            .id();
        let hip = world
            .spawn((Transform::from_xyz(0., 1.4, 0.), ChildOf(root)))
            .id();
        let knee = world
            .spawn((Transform::from_xyz(0., -0.6, 0.14), ChildOf(hip)))
            .id();
        let foot = world
            .spawn((Transform::from_xyz(0., -0.6, -0.14), ChildOf(knee)))
            .id();
        let mut state: SystemState<(Query<&mut Transform>, Query<&ChildOf>)> =
            SystemState::new(&mut world);
        let (mut transforms, parents) = state.get_mut(&mut world).unwrap();
        let before = matrix(foot, &transforms, &parents).transform_point3(Vec3::ZERO);
        let goal = before + Vec3::new(0.12, 0.06, -0.05);
        assert!(plant([hip, knee, foot], goal, &mut transforms, &parents));
        let after = matrix(foot, &transforms, &parents).transform_point3(Vec3::ZERO);
        assert!(after.distance(goal) < 0.0001, "{after:?} != {goal:?}");
    }
    #[test]
    fn held_clip_corrections_restore_the_original_transform_before_reapplying() {
        let mut app = App::new();
        app.add_systems(Update, restore);
        let original = Transform::from_xyz(0., 1., 0.).with_rotation(Quat::from_rotation_x(0.3));
        let e = app
            .world_mut()
            .spawn((
                Base {
                    transform: original,
                    applied: true,
                },
                Transform::from_xyz(9., 9., 9.),
            ))
            .id();
        app.update();
        assert_eq!(*app.world().get::<Transform>(e).unwrap(), original);
        app.update();
        assert_eq!(*app.world().get::<Transform>(e).unwrap(), original);
    }
}
