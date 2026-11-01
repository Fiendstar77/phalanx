//! Jointed physical death reactions. Battle remains the authority for outcomes.
use super::*;
use avian3d::prelude::*;
use std::collections::BTreeMap;

#[derive(Component)]
struct Rig {
    id: usize,
    female: bool,
    bones: BTreeMap<String, Entity>,
    alive: bool,
}
#[derive(Clone)]
struct Binding {
    bone: Entity,
    body: Entity,
    origin: Vec3,
    rotation: Quat,
}
struct Corpse {
    model: Entity,
    bindings: Vec<Binding>,
}
#[derive(Resource, Default)]
pub(super) struct Audit {
    run: Option<u32>,
    arena: Option<Arena>,
    entities: Vec<Entity>,
    terrain: Vec<Entity>,
    corpses: Vec<Corpse>,
    pub bodies: usize,
    pub joints: usize,
    pub pose_updates: u64,
    pub invalid_poses: u32,
    pub max_pose_offset: f32,
}

pub(super) fn install(app: &mut App) {
    app.add_plugins(PhysicsPlugins::default())
        .insert_resource(Time::<Fixed>::from_hz(120.))
        .insert_resource(SubstepCount(8))
        .insert_resource(Gravity(Vec3::NEG_Y * 9.81))
        .init_resource::<Audit>()
        .add_systems(Update, clock.after(super::super::simulate))
        .add_systems(
            PostUpdate,
            begin
                .after(face::Pose)
                .before(bevy::transform::TransformSystems::Propagate),
        )
        .add_systems(
            PostUpdate,
            pose.after(begin)
                .before(bevy::transform::TransformSystems::Propagate)
                .before(super::follow_crowns),
        );
}
pub(super) fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    models: Query<&Model>,
    children: Query<&Children>,
    names: Query<&Name>,
    session: Res<Session>,
) {
    let Ok(model) = models.get(event.entity) else {
        return;
    };
    let bones = children
        .iter_descendants(event.entity)
        .filter_map(|e| names.get(e).ok().map(|n| (n.to_string(), e)))
        .collect();
    commands.entity(event.entity).insert(Rig {
        id: model.id,
        female: model.female,
        bones,
        alive: session.visual_unit(model.id).is_some_and(|u| u.health > 0.),
    });
}
fn clock(session: Res<Session>, mut time: ResMut<Time<Physics>>) {
    if session.running || session.battle.finished {
        time.unpause();
        time.set_relative_speed(if session.battle.finished {
            1.
        } else {
            session.speed
        });
    } else {
        time.pause();
        time.advance_by(std::time::Duration::ZERO);
    }
}
fn ground(arena: Arena) -> Collider {
    let half = arena.half_size();
    let nx = (half[0] * 4.).ceil() as usize;
    let nz = (half[1] * 4.).ceil() as usize;
    let mut points = Vec::new();
    let mut triangles = Vec::new();
    for z in 0..=nz {
        for x in 0..=nx {
            let p = [
                (x as f32 / nx as f32 * 2. - 1.) * half[0],
                (z as f32 / nz as f32 * 2. - 1.) * half[1],
            ];
            points.push(Vec3::new(p[0], arena.elevation(p), p[1]));
            if x < nx && z < nz {
                let a = (z * (nx + 1) + x) as u32;
                let b = a + nx as u32 + 1;
                triangles.extend([[a, b, a + 1], [a + 1, b, b + 1]]);
            }
        }
    }
    Collider::trimesh(points, triangles)
}
// Pelvis outward: writeback can use already corrected parents.
type Segment = (&'static str, &'static str, Option<usize>, u8, f32);
const SEGMENTS: [Segment; 13] = [
    ("Hips", "Spine2", None, 0, 0.17),
    ("Spine2", "Neck", Some(0), 0, 0.18),
    ("Head", "", Some(1), 0, 0.22),
    ("LeftArm", "LeftForeArm", Some(1), 1, 0.065),
    ("LeftForeArm", "LeftHand", Some(3), 1, 0.055),
    ("RightArm", "RightForeArm", Some(1), 2, 0.065),
    ("RightForeArm", "RightHand", Some(5), 2, 0.055),
    ("LeftUpLeg", "LeftLeg", Some(0), 4, 0.095),
    ("LeftLeg", "LeftFoot", Some(7), 4, 0.075),
    ("LeftFoot", "", Some(8), 4, 0.07),
    ("RightUpLeg", "RightLeg", Some(0), 8, 0.095),
    ("RightLeg", "RightFoot", Some(10), 8, 0.075),
    ("RightFoot", "", Some(11), 8, 0.07),
];
fn begin(
    mut commands: Commands,
    session: Res<Session>,
    mut audit: ResMut<Audit>,
    mut rigs: Query<(Entity, &mut Rig)>,
    parents: Query<&ChildOf, Without<CrownAnchor>>,
    locals: Query<&Transform, Without<CrownAnchor>>,
) {
    let reset = audit.run != Some(session.run_id);
    if reset {
        for e in audit.entities.drain(..) {
            commands.entity(e).despawn();
        }
        audit.corpses.clear();
        audit.bodies = 0;
        audit.joints = 0;
        audit.pose_updates = 0;
        audit.invalid_poses = 0;
        audit.max_pose_offset = 0.;
        audit.run = Some(session.run_id);
        for (_, mut rig) in &mut rigs {
            rig.alive = session.visual_unit(rig.id).is_some_and(|u| u.health > 0.);
        }
    }
    if reset || audit.arena != Some(session.battle.arena) {
        for e in audit.terrain.drain(..) {
            commands.entity(e).despawn();
        }
        audit.arena = Some(session.battle.arena);
        let floor = commands
            .spawn((
                RigidBody::Static,
                ground(session.battle.arena),
                CollisionLayers::from_bits(1, u32::MAX),
                Friction::new(0.8),
                Transform::default(),
            ))
            .id();
        audit.terrain.push(floor);
        for (wall, (center, size)) in session.battle.arena.walls().iter().enumerate() {
            let timber = matches!(session.battle.arena, Arena::Pressure(f) if f.layout > 0 && wall < 4 - (f.destroyed & 15).count_ones() as usize);
            let h = if timber { 1.7 } else { 2.8 };
            let e = commands
                .spawn((
                    RigidBody::Static,
                    Collider::cuboid(size[0], h, size[1]),
                    CollisionLayers::from_bits(1, u32::MAX),
                    Transform::from_xyz(
                        center[0],
                        session.battle.arena.elevation(*center) + h / 2.,
                        center[1],
                    ),
                ))
                .id();
            audit.terrain.push(e);
        }
    }
    for (model, mut rig) in &mut rigs {
        let Some(unit) = session.battle.units.get(rig.id) else {
            continue;
        };
        if rig.female != female(&session, rig.id) {
            continue;
        }
        if unit.health > 0. {
            rig.alive = true;
            continue;
        }
        if audit.corpses.iter().any(|c| c.model == model) {
            continue;
        }
        let hit = session
            .battle
            .damage_events
            .iter()
            .rev()
            .find(|e| e.defender == unit.id);
        if !rig.alive && hit.is_none() {
            continue;
        }
        let impact = hit.map_or(0., |h| (h.amount / unit.max_health()).clamp(0., 1.));
        let velocity = hit
            .map_or(Vec3::ZERO, |h| Vec3::new(h.velocity[0], 0., h.velocity[1]))
            .normalize_or(Vec3::Z)
            * (1.2 + 3. * impact)
            + Vec3::Y * (0.4 + impact);
        let mut bindings = Vec::new();
        let mut pieces: Vec<Option<(Entity, Vec3, Quat)>> = vec![None; SEGMENTS.len()];
        for (i, (name, end, parent, limb, radius)) in SEGMENTS.iter().enumerate() {
            if unit.lost_limbs & !unit.prosthetics & limb != 0 {
                continue;
            }
            let Some(&bone) = rig.bones.get(&format!("mixamorig{name}")) else {
                continue;
            };
            let world = chain_global(bone, &parents, &locals);
            let start = world.translation();
            let scale = world.to_scale_rotation_translation().0.abs().max_element();
            let end = rig
                .bones
                .get(&format!("mixamorig{end}"))
                .map(|e| chain_global(*e, &parents, &locals).translation())
                .unwrap_or(
                    start
                        + world.rotation()
                            * Vec3::Y
                            * if *name == "Head" {
                                0.10 * scale
                            } else {
                                0.12 * scale
                            },
                );
            let center = (start + end) * 0.5;
            let rotation = Quat::from_rotation_arc(Vec3::Y, (end - start).normalize_or(Vec3::Y));
            let radius = radius * scale;
            let collider = if *name == "Head" {
                Collider::sphere(radius)
            } else {
                Collider::capsule(radius, ((end - start).length() - 2. * radius).max(0.01))
            };
            let bit = 1u32 << (unit.id + 1).min(30);
            let body = commands
                .spawn((
                    RigidBody::Dynamic,
                    collider,
                    Transform::from_translation(center).with_rotation(rotation),
                    LinearVelocity(velocity),
                    AngularVelocity(Vec3::new(0.6, 0.2, -0.4)),
                    LinearDamping(0.5),
                    AngularDamping(1.8),
                    Friction::new(0.8),
                    Restitution::new(0.05),
                    CollisionLayers::from_bits(bit, !bit),
                    ColliderDensity(600.),
                ))
                .id();
            audit.entities.push(body);
            audit.bodies += 1;
            bindings.push(Binding {
                bone,
                body,
                origin: rotation.inverse() * (start - center),
                rotation: rotation.inverse() * world.rotation(),
            });
            pieces[i] = Some((body, center, rotation));
            if let Some((previous, pc, pr)) = parent.and_then(|p| pieces[p]) {
                let a1 = pr.inverse() * (start - pc);
                let a2 = rotation.inverse() * (start - center);
                let joint =
                    if name.ends_with("ForeArm") || *name == "LeftLeg" || *name == "RightLeg" {
                        let hinge = (pr * Vec3::Y)
                            .cross(rotation * Vec3::Y)
                            .normalize_or(world.rotation() * Vec3::X);
                        let basis = Quat::from_rotation_arc(Vec3::X, hinge);
                        commands
                            .spawn(
                                RevoluteJoint::new(previous, body)
                                    .with_local_anchor1(a1)
                                    .with_local_anchor2(a2)
                                    .with_local_basis1(pr.inverse() * basis)
                                    .with_local_basis2(rotation.inverse() * basis)
                                    .with_hinge_axis(Vec3::X)
                                    .with_angle_limits(-0.10, 2.2)
                                    .with_point_compliance(0.000_000_1),
                            )
                            .id()
                    } else {
                        commands
                            .spawn(
                                SphericalJoint::new(previous, body)
                                    .with_local_anchor1(a1)
                                    .with_local_anchor2(a2)
                                    .with_local_basis1(pr.inverse())
                                    .with_local_basis2(rotation.inverse())
                                    .with_swing_limits(-1.2, 1.2)
                                    .with_twist_limits(-0.5, 0.5)
                                    .with_point_compliance(0.000_000_1),
                            )
                            .id()
                    };
                audit.entities.push(joint);
                audit.joints += 1;
            }
        }
        rig.alive = false;
        audit.corpses.push(Corpse { model, bindings });
    }
}
fn world_transform(world: &World, entity: Entity) -> GlobalTransform {
    let mut stack = Vec::new();
    let mut cursor = entity;
    for _ in 0..48 {
        if let Some(t) = world.get::<Transform>(cursor) {
            stack.push(*t);
        }
        let Some(parent) = world.get::<ChildOf>(cursor) else {
            break;
        };
        cursor = parent.parent();
    }
    stack
        .iter()
        .rev()
        .fold(GlobalTransform::IDENTITY, |g, t| g.mul_transform(*t))
}
fn pose(world: &mut World) {
    let bindings: Vec<_> = world
        .resource::<Audit>()
        .corpses
        .iter()
        .flat_map(|c| c.bindings.iter().cloned().map(|b| (c.model, b)))
        .collect();
    for (model, b) in bindings {
        let Some(body) = world.get::<Transform>(b.body).copied() else {
            continue;
        };
        if !body.translation.is_finite() || !body.rotation.is_finite() {
            world.resource_mut::<Audit>().invalid_poses += 1;
            continue;
        }
        let offset = body
            .translation
            .distance(world_transform(world, model).translation());
        let mut audit = world.resource_mut::<Audit>();
        audit.max_pose_offset = audit.max_pose_offset.max(offset);
        let Some(parent) = world.get::<ChildOf>(b.bone).map(ChildOf::parent) else {
            continue;
        };
        let parent = world_transform(world, parent);
        if let Some(mut bone) = world.get_mut::<Transform>(b.bone) {
            bone.translation = parent
                .affine()
                .inverse()
                .transform_point3(body.translation + body.rotation * b.origin);
            bone.rotation = (parent.rotation().inverse() * body.rotation * b.rotation).normalize();
        }
        world.resource_mut::<Audit>().pose_updates += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    #[test]
    fn terrain_changes_preserve_corpses_and_reset_clears_their_entities() {
        let session = Session::new(phalanx::Battle::new(6, false));
        let mut app = App::new();
        app.insert_resource(session)
            .init_resource::<Audit>()
            .add_systems(Update, begin);
        app.update();
        let body = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<Audit>().entities.push(body);
        let floor = app.world().resource::<Audit>().terrain[0];
        app.world_mut().resource_mut::<Session>().battle.arena = Arena::Choke;
        app.update();
        assert!(app.world().get_entity(body).is_ok());
        assert!(app.world().get_entity(floor).is_err());
        app.world_mut().resource_mut::<Session>().run_id += 1;
        app.update();
        assert!(app.world().get_entity(body).is_err());
        assert!(app.world().resource::<Audit>().corpses.is_empty());
    }

    #[test]
    fn jointed_bodies_land_stay_connected_and_freeze_when_paused() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, PhysicsPlugins::new(Update)))
            .insert_resource(SubstepCount(8))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                1. / 60.,
            )));
        app.finish();
        app.cleanup();
        app.world_mut().spawn((
            RigidBody::Static,
            ground(Arena::OpenField),
            Transform::default(),
        ));
        let a = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::capsule(0.1, 0.3),
                Transform::from_xyz(0., 2., 0.),
                CollisionLayers::from_bits(2, 1),
            ))
            .id();
        let b = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::capsule(0.08, 0.3),
                Transform::from_xyz(0., 1.5, 0.),
                CollisionLayers::from_bits(2, 1),
            ))
            .id();
        app.world_mut().spawn(
            SphericalJoint::new(a, b)
                .with_local_anchor1(Vec3::NEG_Y * 0.25)
                .with_local_anchor2(Vec3::Y * 0.25)
                .with_swing_limits(-1.2, 1.2),
        );
        for _ in 0..240 {
            app.update();
        }
        let upper = app.world().get::<Position>(a).unwrap().0;
        let lower = app.world().get::<Position>(b).unwrap().0;
        assert!(upper.is_finite() && lower.is_finite());
        assert!(upper.y < 1.5 && upper.y > 0.05 && lower.y > 0.05);
        assert!(
            upper.distance(lower) < 0.56,
            "joint separated: {upper:?} {lower:?}"
        );
        app.world_mut().resource_mut::<Time<Physics>>().pause();
        app.world_mut()
            .resource_mut::<Time<Physics>>()
            .advance_by(Duration::ZERO);
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(app.world().get::<Position>(a).unwrap().0, upper);
        assert_eq!(app.world().get::<Position>(b).unwrap().0, lower);
    }

    #[test]
    fn skin_writeback_respects_parent_translation_rotation_and_scale() {
        let mut world = World::new();
        world.init_resource::<Audit>();
        let parent = world
            .spawn(
                Transform::from_xyz(2., 1., -3.)
                    .with_rotation(Quat::from_rotation_y(0.8))
                    .with_scale(Vec3::splat(0.7)),
            )
            .id();
        let bone = world.spawn((Transform::default(), ChildOf(parent))).id();
        let pose = Transform::from_xyz(4., 2., -1.).with_rotation(Quat::from_rotation_z(0.5));
        let body = world.spawn(pose).id();
        world.resource_mut::<Audit>().corpses.push(Corpse {
            model: parent,
            bindings: vec![Binding {
                bone,
                body,
                origin: Vec3::Y * 0.2,
                rotation: Quat::IDENTITY,
            }],
        });
        super::pose(&mut world);
        let actual = world_transform(&world, bone);
        assert!(
            actual
                .translation()
                .distance(pose.translation + pose.rotation * Vec3::Y * 0.2)
                < 0.0001
        );
        assert!(actual.rotation().angle_between(pose.rotation) < 0.0001);
    }
}
