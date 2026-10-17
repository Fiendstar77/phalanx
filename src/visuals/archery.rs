//! Two-handed bow presentation. Simulation defines grip/origin; the rig follows it, never the reverse.
use super::*;
use bevy::math::Affine3A;
use phalanx::weapons::ProjectileWeapon as W;
#[derive(Component)]
struct Rig {
    id: usize,
    female: bool,
    arms: [[Entity; 3]; 2],
}
#[derive(Clone, Copy)]
enum Kind {
    Bow(Entity),
    Quiver,
}
#[derive(Component)]
struct Root {
    model: Entity,
    kind: Kind,
}
#[derive(Clone, Copy)]
enum Shape {
    Limb(usize),
    String(bool),
    Loaded,
    Arrow(u8),
}
#[derive(Component)]
struct Piece {
    model: Entity,
    shape: Shape,
}
#[derive(Resource)]
pub(super) struct Kit {
    cube: Handle<Mesh>,
    tip: Handle<Mesh>,
    wood: Handle<StandardMaterial>,
    string: Handle<StandardMaterial>,
    steel: Handle<StandardMaterial>,
    feather: Handle<StandardMaterial>,
}
#[derive(Resource, Default)]
pub(super) struct Audit {
    pub poses: u32,
    pub max_grip_error: f32,
    pub max_string_error: f32,
    pub max_draw: f32,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Audit>()
        .add_systems(Startup, setup)
        .add_systems(
            PostUpdate,
            pose.after(bevy::app::AnimationSystems)
                .before(bevy::transform::TransformSystems::Propagate),
        );
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(Kit {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        tip: meshes.add(Cone::new(0.035, 0.10)),
        wood: materials.add(Color::srgb(0.34, 0.14, 0.045)),
        string: materials.add(Color::srgb(0.90, 0.83, 0.59)),
        steel: materials.add(Color::srgb(0.70, 0.77, 0.79)),
        feather: materials.add(Color::srgb(0.18, 0.65, 0.25)),
    });
}
fn block(
    p: &mut ChildSpawnerCommands,
    kit: &Kit,
    mat: &Handle<StandardMaterial>,
    at: Vec3,
    scale: Vec3,
) {
    p.spawn((
        Mesh3d(kit.cube.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(at).with_scale(scale),
    ));
}
fn arrow(p: &mut ChildSpawnerCommands, kit: &Kit) {
    block(p, kit, &kit.wood, Vec3::ZERO, Vec3::new(0.018, 0.018, 1.0));
    p.spawn((
        Mesh3d(kit.tip.clone()),
        MeshMaterial3d(kit.steel.clone()),
        Transform::from_xyz(0., 0., 0.53)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    ));
    block(
        p,
        kit,
        &kit.feather,
        -Vec3::Z * 0.40,
        Vec3::new(0.09, 0.010, 0.15),
    );
    block(
        p,
        kit,
        &kit.feather,
        -Vec3::Z * 0.40,
        Vec3::new(0.010, 0.09, 0.15),
    );
}
pub(super) fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    models: Query<&Model>,
    names: Query<&Name>,
    kit: Res<Kit>,
) {
    let Ok(model) = models.get(event.entity) else {
        return;
    };
    let find = |name: &str| {
        children
            .iter_descendants(event.entity)
            .find(|e| names.get(*e).is_ok_and(|n| n.as_str() == name))
    };
    let (Some(la), Some(lf), Some(lh), Some(ra), Some(rf), Some(rh), Some(hip)) = (
        find("mixamorigLeftArm"),
        find("mixamorigLeftForeArm"),
        find("mixamorigLeftHand"),
        find("mixamorigRightArm"),
        find("mixamorigRightForeArm"),
        find("mixamorigRightHand"),
        find("mixamorigHips"),
    ) else {
        return;
    };
    commands.entity(event.entity).insert(Rig {
        id: model.id,
        female: model.female,
        arms: [[la, lf, lh], [ra, rf, rh]],
    });
    commands.entity(lh).with_children(|p| {
        p.spawn((
            Root {
                model: event.entity,
                kind: Kind::Bow(lh),
            },
            Transform::default(),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            for i in 0..12 {
                p.spawn((
                    Piece {
                        model: event.entity,
                        shape: Shape::Limb(i),
                    },
                    Mesh3d(kit.cube.clone()),
                    MeshMaterial3d(kit.wood.clone()),
                    Transform::default(),
                ));
            }
            for upper in [false, true] {
                p.spawn((
                    Piece {
                        model: event.entity,
                        shape: Shape::String(upper),
                    },
                    Mesh3d(kit.cube.clone()),
                    MeshMaterial3d(kit.string.clone()),
                    Transform::default(),
                ));
            }
            block(
                p,
                &kit,
                &kit.steel,
                Vec3::ZERO,
                Vec3::new(0.065, 0.15, 0.055),
            );
            p.spawn((
                Piece {
                    model: event.entity,
                    shape: Shape::Loaded,
                },
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|p| arrow(p, &kit));
        });
    });
    commands.entity(hip).with_children(|p| {
        p.spawn((
            Root {
                model: event.entity,
                kind: Kind::Quiver,
            },
            Transform::from_xyz(-0.32, -0.03, -0.04).with_rotation(Quat::from_rotation_z(-0.18)),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            block(
                p,
                &kit,
                &kit.wood,
                Vec3::new(0., 0.05, 0.),
                Vec3::new(0.18, 0.42, 0.16),
            );
            block(
                p,
                &kit,
                &kit.string,
                Vec3::new(0., 0.22, 0.),
                Vec3::new(0.20, 0.045, 0.18),
            );
            for i in 0..12u8 {
                p.spawn((
                    Piece {
                        model: event.entity,
                        shape: Shape::Arrow(i),
                    },
                    Transform::from_xyz(
                        (i % 3) as f32 * 0.045 - 0.045,
                        0.32 + (i % 2) as f32 * 0.04,
                        (i / 3) as f32 * 0.035 - 0.05,
                    )
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::splat(0.75)),
                    Visibility::Inherited,
                ))
                .with_children(|p| arrow(p, &kit));
            }
        });
    });
}
type Nodes<'w, 's> = Query<'w, 's, (&'static Transform, Option<&'static ChildOf>)>;
fn world(entity: Entity, nodes: &Nodes) -> Option<Affine3A> {
    let mut chain = Vec::new();
    let mut e = entity;
    for _ in 0..48 {
        let (t, parent) = nodes.get(e).ok()?;
        chain.push(t.compute_affine());
        if let Some(p) = parent {
            e = p.parent()
        } else {
            break;
        }
    }
    Some(
        chain
            .into_iter()
            .rev()
            .fold(Affine3A::IDENTITY, |a, b| a * b),
    )
}
fn solve(
    arm: [Entity; 3],
    goal: Vec3,
    hint: Vec3,
    nodes: &mut ParamSet<(Nodes, Query<&mut Transform>)>,
) -> Option<Vec3> {
    let (parent, shoulder, upper, lower, fore, hand) = {
        let q = nodes.p0();
        let (_, parent) = q.get(arm[0]).ok()?;
        let parent = world(parent?.parent(), &q)?;
        let upper = world(arm[0], &q)?;
        let lower = world(arm[1], &q)?;
        (
            parent,
            Vec3::from(upper.translation),
            upper,
            lower,
            *q.get(arm[1]).ok()?.0,
            *q.get(arm[2]).ok()?.0,
        )
    };
    let a = upper.transform_vector3(fore.translation).length();
    let b = lower.transform_vector3(hand.translation).length();
    let delta = goal - shoulder;
    let distance = delta.length().clamp((a - b).abs() + 0.001, (a + b) * 0.995);
    let axis = delta.normalize_or_zero();
    let end = shoulder + axis * distance;
    let x = (a * a - b * b + distance * distance) / (2. * distance);
    let bend = (hint - axis * hint.dot(axis)).normalize_or_zero();
    let elbow = shoulder + axis * x + bend * (a * a - x * x).max(0.).sqrt();
    let local = parent
        .inverse()
        .transform_vector3(elbow - shoulder)
        .normalize_or_zero();
    nodes.p1().get_mut(arm[0]).ok()?.rotation =
        Quat::from_rotation_arc(fore.translation.normalize(), local);
    let upper = {
        let q = nodes.p0();
        world(arm[0], &q)?
    };
    let local = upper
        .inverse()
        .transform_vector3(end - elbow)
        .normalize_or_zero();
    nodes.p1().get_mut(arm[1]).ok()?.rotation =
        Quat::from_rotation_arc(hand.translation.normalize(), local);
    let q = nodes.p0();
    Some(world(arm[2], &q)?.translation.into())
}
fn segment(t: &mut Transform, a: Vec3, b: Vec3, width: f32) {
    let d = b - a;
    t.translation = (a + b) * 0.5;
    t.rotation = Quat::from_rotation_arc(Vec3::Y, d.normalize_or_zero());
    t.scale = Vec3::new(width, d.length(), width);
}
fn draw(session: &Session, u: &phalanx::Unit) -> f32 {
    u.windup
        .as_ref()
        .filter(|d| d.weapon == W::Bow)
        .map(|d| (1. - (d.remaining - session.accumulator) / W::Bow.windup()).clamp(0., 1.))
        .unwrap_or(0.)
}
fn pose(
    session: Res<Session>,
    rigs: Query<(Entity, &Rig)>,
    roots: Query<(Entity, &Root)>,
    pieces: Query<(Entity, &Piece)>,
    mut nodes: ParamSet<(Nodes, Query<&mut Transform>)>,
    mut visibility: Query<&mut Visibility>,
    mut audit: ResMut<Audit>,
) {
    for (_, rig) in &rigs {
        let Some(u) = session.visual_unit(rig.id) else {
            continue;
        };
        if rig.female != female(&session, rig.id)
            || u.health <= 0.
            || u.projectile_weapon() != Some(W::Bow)
            || (!u.is_ranged() && !u.bow_follow_through())
        {
            continue;
        }
        let release = u.bow_follow_through();
        if u.windup.as_ref().is_none_or(|w| w.weapon != W::Bow) && !release {
            continue;
        }
        let p = session.presented(u.id);
        let aim = u.bow_aim();
        let m = phalanx::archery::muzzle(p, aim);
        let grip = Vec3::new(
            m[0],
            session.battle.arena.elevation(p) + phalanx::archery::GRIP_HEIGHT,
            m[1],
        );
        let f = Vec3::new(aim[0] - p[0], 0., aim[1] - p[1]).normalize_or_zero();
        let side = Vec3::new(-f.z, 0., f.x);
        let amount = draw(&session, u);
        audit.max_draw = audit.max_draw.max(amount);
        if let Some(hand) = solve(rig.arms[0], grip, -side - Vec3::Y * 0.2, &mut nodes) {
            audit.poses += 1;
            audit.max_grip_error = audit.max_grip_error.max(hand.distance(grip));
        }
        let nock_goal = grip - f * (0.098 + 0.147 * amount);
        if let Some(hand) = solve(
            rig.arms[1],
            nock_goal,
            side - f * 0.4 + Vec3::Y * 0.15,
            &mut nodes,
        ) {
            audit.max_string_error = audit.max_string_error.max(hand.distance(nock_goal));
        }
    }
    for (entity, root) in &roots {
        let Ok((_, rig)) = rigs.get(root.model) else {
            continue;
        };
        let Some(u) = session.visual_unit(rig.id) else {
            continue;
        };
        let shown = rig.female == female(&session, rig.id)
            && u.health > 0.
            && u.projectile_weapon() == Some(W::Bow);
        if let Ok(mut v) = visibility.get_mut(entity) {
            *v = if shown
                && (matches!(root.kind, Kind::Quiver) || u.is_ranged() || u.bow_follow_through())
            {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if !shown {
            continue;
        }
        if let Kind::Bow(hand) = root.kind {
            let p = session.presented(u.id);
            let aim = u.bow_aim();
            let yaw = (aim[0] - p[0]).atan2(aim[1] - p[1]);
            let rotation = {
                let q = nodes.p0();
                world(hand, &q).map(|w| w.to_scale_rotation_translation().1)
            };
            if let Some(rotation) = rotation
                && let Ok(mut t) = nodes.p1().get_mut(entity)
            {
                t.rotation = rotation.inverse() * Quat::from_rotation_y(yaw);
            }
        }
    }
    for (entity, piece) in &pieces {
        let Ok((_, rig)) = rigs.get(piece.model) else {
            continue;
        };
        let Some(u) = session.visual_unit(rig.id) else {
            continue;
        };
        let amount = draw(&session, u);
        let tip =
            |upper: bool| Vec3::new(0., if upper { 0.62 } else { -0.62 }, -0.15 - 0.025 * amount);
        let nock = Vec3::new(0., 0., -0.14 - 0.21 * amount);
        let mut q = nodes.p1();
        let Ok(mut t) = q.get_mut(entity) else {
            continue;
        };
        match piece.shape {
            Shape::Limb(i) => {
                let point = |i: usize| {
                    let h = i as f32 / 12. * 2. - 1.;
                    Vec3::new(0., h * 0.62, -(0.15 + 0.025 * amount) * h * h)
                };
                segment(&mut t, point(i), point(i + 1), 0.035);
            }
            Shape::String(upper) => segment(&mut t, tip(upper), nock, 0.008),
            Shape::Loaded => {
                t.translation = nock + Vec3::Z * 0.5;
                if let Ok(mut v) = visibility.get_mut(entity) {
                    *v = if u.windup.is_some() && u.is_ranged() {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                }
            }
            Shape::Arrow(i) => {
                if let Ok(mut v) = visibility.get_mut(entity) {
                    *v = if i < u.ammunition {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                }
            }
        }
    }
}
