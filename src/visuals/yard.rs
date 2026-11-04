//! A theatrical Yard around the current battlefield. All geometry is decorative:
//! monumental spectators sit outside the legal ground; reeds occupy cover only.
//! The presentation clock, attention and impact pulses never enter Battle or its RNG.
use super::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

#[derive(Clone, Copy)]
enum Kind {
    Watcher,
    House,
    Reed,
    Smoke,
    Bird,
    Panel,
    Rig,
    Hand,
    Orrery,
}
#[derive(Component)]
struct Piece(Kind, usize);
#[derive(Component)]
struct Wing(f32, f32);
#[derive(Component)]
struct Iris(Vec3);
#[derive(Component)]
struct Halo;
#[derive(Component)]
struct Finger(f32, f32);
#[derive(Component)]
pub(crate) struct StageKey;
#[derive(Resource, Default)]
struct Atmosphere {
    seconds: f32,
    impact: f32,
    tick: u32,
    run: Option<u32>,
    reveal: f32,
    tension: f32,
}
#[derive(Resource)]
struct StageGlow(Handle<StandardMaterial>);

pub(super) fn install(app: &mut App) {
    app.init_resource::<Atmosphere>()
        .add_systems(Startup, build.after(setup))
        .add_systems(Update, (clock, animate.after(clock)));
}

fn build(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sphere = meshes.add(Sphere::new(1.).mesh().uv(20, 12));
    let cube = meshes.add(Cuboid::new(1., 1., 1.));
    let cone = meshes.add(Cone::new(1., 1.));
    let ring = meshes.add(Torus::new(1.38, 1.52));
    let porcelain = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.84, 0.69),
        perceptual_roughness: 0.58,
        ..default()
    });
    let dark = materials.add(Color::srgb(0.035, 0.055, 0.065));
    let lips = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.055, 0.075),
        perceptual_roughness: 0.3,
        ..default()
    });
    let gold = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.53, 0.13),
        metallic: 0.65,
        perceptual_roughness: 0.32,
        ..default()
    });
    let eye = materials.add(StandardMaterial {
        base_color: Color::srgb(0.94, 0.62, 0.16),
        emissive: LinearRgba::rgb(0.45, 0.11, 0.015),
        ..default()
    });
    let brick = materials.add(Color::srgb(0.20, 0.29, 0.30));
    let roof = materials.add(Color::srgb(0.36, 0.10, 0.13));
    let smoke = materials.add(StandardMaterial {
        base_color: Color::srgba(0.32, 0.44, 0.43, 0.14),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let reeds = materials.add(Color::srgb(0.39, 0.45, 0.18));
    let crimson = materials.add(Color::srgb(0.66, 0.025, 0.045));
    let electric = materials.add(StandardMaterial {
        base_color: Color::srgb(0.025, 0.75, 0.80),
        emissive: LinearRgba::rgb(0.03, 1.3, 1.6),
        metallic: 0.4,
        perceptual_roughness: 0.25,
        ..default()
    });
    // A vertical celestial machine behind the whole stage. Its scale comes
    // from actual geometry and parallax, never enlarging the simulated fighters.
    commands
        .spawn((
            Piece(Kind::Orrery, 0),
            Transform::default(),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(sphere.clone()),
                MeshMaterial3d(dark.clone()),
                Transform::default().with_scale(Vec3::new(19., 19., 0.24)),
            ));
            for (radius, material) in [
                (14., crimson.clone()),
                (18., gold.clone()),
                (21., electric.clone()),
            ] {
                p.spawn((
                    Mesh3d(ring.clone()),
                    MeshMaterial3d(material),
                    Transform::from_xyz(0., 0., 0.5)
                        .with_scale(Vec3::splat(radius / 1.5))
                        .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                ));
            }
            for index in 0..32 {
                let a = index as f32 / 32. * TAU;
                p.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(if index % 4 == 0 {
                        crimson.clone()
                    } else {
                        gold.clone()
                    }),
                    Transform::from_xyz(a.cos() * 22., a.sin() * 22., 0.3)
                        .with_rotation(Quat::from_rotation_z(a))
                        .with_scale(Vec3::new(if index % 4 == 0 { 6. } else { 2. }, 0.10, 0.12)),
                ));
            }
        });
    for id in 0..2 {
        let side = if id == 0 { -1. } else { 1. };
        commands
            .spawn((
                Piece(Kind::Hand, id),
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                p.spawn((
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(porcelain.clone()),
                    Transform::from_xyz(0., 2., 0.).with_scale(Vec3::new(3.2, 4.4, 1.35)),
                ));
                p.spawn((
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(dark.clone()),
                    Transform::from_xyz(0., -5., -0.2).with_scale(Vec3::new(2.3, 5., 1.5)),
                ));
                for index in 0..5 {
                    let spread = (index as f32 - 2.) * 1.38;
                    p.spawn((
                        Finger(spread, side),
                        Transform::from_xyz(spread, 4.2, 0.),
                        Visibility::Inherited,
                    ))
                    .with_children(|f| {
                        let length = if index == 0 || index == 4 { 5.8 } else { 8. };
                        for joint in 0..3 {
                            f.spawn((
                                Mesh3d(sphere.clone()),
                                MeshMaterial3d(porcelain.clone()),
                                Transform::from_xyz(
                                    0.,
                                    length * (joint as f32 + 0.5) / 3.,
                                    0.18 * joint as f32,
                                )
                                .with_scale(Vec3::new(
                                    0.56,
                                    length / 6.,
                                    0.62,
                                )),
                            ));
                            f.spawn((
                                Mesh3d(ring.clone()),
                                MeshMaterial3d(crimson.clone()),
                                Transform::from_xyz(
                                    0.,
                                    length * (joint as f32 + 0.12) / 3.,
                                    0.18 * joint as f32,
                                )
                                .with_scale(Vec3::splat(0.40)),
                            ));
                        }
                        f.spawn((
                            Mesh3d(cone.clone()),
                            MeshMaterial3d(gold.clone()),
                            Transform::from_xyz(0., length + 0.8, 0.5)
                                .with_scale(Vec3::new(0.48, 2.2, 0.36)),
                        ));
                    });
                }
                for cable in 0..5 {
                    p.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(electric.clone()),
                        Transform::from_xyz((cable as f32 - 2.) * 1.1, 15., -0.8)
                            .with_scale(Vec3::new(0.035, 35., 0.035)),
                    ));
                }
            });
    }
    commands.insert_resource(StageGlow(electric.clone()));
    // Lacquered scenic wings pivot out for each bout. Nothing sweeps across
    // legal ground; these are theatrical reveals of the actual battlefield.
    for id in 0..6 {
        commands
            .spawn((
                Piece(Kind::Panel, id),
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                for (at, scale, material) in [
                    (
                        Vec3::new(0., 4.8, 0.),
                        Vec3::new(2.1, 9.6, 0.22),
                        dark.clone(),
                    ),
                    (
                        Vec3::new(0., 4.8, 0.14),
                        Vec3::new(1.8, 9.1, 0.07),
                        crimson.clone(),
                    ),
                    (
                        Vec3::new(-0.91, 4.8, 0.23),
                        Vec3::new(0.055, 9.6, 0.045),
                        electric.clone(),
                    ),
                    (
                        Vec3::new(0.91, 4.8, 0.23),
                        Vec3::new(0.055, 9.6, 0.045),
                        electric.clone(),
                    ),
                    (
                        Vec3::new(0., 9.4, 0.23),
                        Vec3::new(1.9, 0.055, 0.045),
                        electric.clone(),
                    ),
                    (
                        Vec3::new(0., 2.9, 0.21),
                        Vec3::new(1.85, 0.10, 0.07),
                        gold.clone(),
                    ),
                    (
                        Vec3::new(0., 6.8, 0.21),
                        Vec3::new(1.85, 0.10, 0.07),
                        gold.clone(),
                    ),
                ] {
                    p.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(material),
                        Transform::from_translation(at).with_scale(scale),
                    ));
                }
                p.spawn((
                    Mesh3d(ring.clone()),
                    MeshMaterial3d(electric.clone()),
                    Transform::from_xyz(0., 4.8, 0.26)
                        .with_scale(Vec3::new(0.40, 0.40, 0.40))
                        .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                ));
            });
    }
    commands
        .spawn((
            Piece(Kind::Rig, 0),
            Transform::default(),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            for y in [0., 0.7] {
                p.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(dark.clone()),
                    Transform::from_xyz(0., y, 0.).with_scale(Vec3::new(40., 0.12, 0.16)),
                ));
                p.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(electric.clone()),
                    Transform::from_xyz(0., y, 0.10).with_scale(Vec3::new(40., 0.025, 0.035)),
                ));
            }
            for index in 0..13 {
                let x = (index as f32 - 6.) * 3.;
                p.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(gold.clone()),
                    Transform::from_xyz(x, 0.35, 0.)
                        .with_scale(Vec3::new(0.055, 1., 0.055))
                        .with_rotation(Quat::from_rotation_z(0.7)),
                ));
                if index % 2 == 0 {
                    p.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(dark.clone()),
                        Transform::from_xyz(x, -1.8, 0.).with_scale(Vec3::new(0.035, 3.6, 0.035)),
                    ));
                    p.spawn((
                        Mesh3d(sphere.clone()),
                        MeshMaterial3d(electric.clone()),
                        Transform::from_xyz(x, -3.5, 0.).with_scale(Vec3::new(0.24, 0.42, 0.24)),
                    ));
                }
            }
        });
    for id in 0..3 {
        commands
            .spawn((
                Piece(Kind::Watcher, id),
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                // Three immense porcelain masks: a little beautiful, much too
                // interested. Their plinths remain beyond the arena boundary.
                for (at, scale, material) in [
                    (
                        Vec3::new(0., -3.5, -0.3),
                        Vec3::new(1.25, 5., 1.6),
                        brick.clone(),
                    ),
                    (Vec3::ZERO, Vec3::new(1.65, 2.15, 0.9), porcelain.clone()),
                    (
                        Vec3::new(0., -0.85, 0.88),
                        Vec3::new(0.95, 0.56, 0.12),
                        lips.clone(),
                    ),
                    (
                        Vec3::new(0., -0.85, 1.0),
                        Vec3::new(0.79, 0.37, 0.09),
                        dark.clone(),
                    ),
                    (
                        Vec3::new(0., -0.99, 1.09),
                        Vec3::new(0.40, 0.10, 0.07),
                        lips.clone(),
                    ),
                    (
                        Vec3::new(0., 0.12, 0.94),
                        Vec3::new(0.20, 0.47, 0.32),
                        porcelain.clone(),
                    ),
                ] {
                    p.spawn((
                        Mesh3d(sphere.clone()),
                        MeshMaterial3d(material),
                        Transform::from_translation(at).with_scale(scale),
                    ));
                }
                for side in [-1., 1.] {
                    for (x, y, angle) in
                        [(0.80, 1.11, -0.55), (1.05, 0.26, 0.45), (0.75, -0.18, 0.65)]
                    {
                        p.spawn((
                            Mesh3d(cube.clone()),
                            MeshMaterial3d(crimson.clone()),
                            Transform::from_xyz(side * x, y, 0.91)
                                .with_rotation(Quat::from_rotation_z(side * angle))
                                .with_scale(Vec3::new(0.60, 0.12, 0.055)),
                        ));
                    }
                    p.spawn((
                        Mesh3d(sphere.clone()),
                        MeshMaterial3d(dark.clone()),
                        Transform::from_xyz(side * 0.69, 0.59, 0.81)
                            .with_scale(Vec3::new(0.54, 0.35, 0.24)),
                    ));
                    p.spawn((
                        Mesh3d(sphere.clone()),
                        MeshMaterial3d(eye.clone()),
                        Transform::from_xyz(side * 0.69, 0.59, 1.0)
                            .with_scale(Vec3::new(0.38, 0.25, 0.12)),
                    ));
                    let at = Vec3::new(side * 0.69, 0.59, 1.13);
                    p.spawn((
                        Iris(at),
                        Mesh3d(sphere.clone()),
                        MeshMaterial3d(dark.clone()),
                        Transform::from_translation(at).with_scale(Vec3::new(0.055, 0.19, 0.035)),
                    ));
                    p.spawn((
                        Mesh3d(cone.clone()),
                        MeshMaterial3d(gold.clone()),
                        Transform::from_xyz(side * 1.2, 2., 0.)
                            .with_scale(Vec3::new(0.34, 1.65, 0.34))
                            .with_rotation(Quat::from_rotation_z(-side * 0.46)),
                    ));
                }
                for tooth in 0..7 {
                    let x = (tooth as f32 - 3.) * 0.19;
                    p.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(porcelain.clone()),
                        Transform::from_xyz(x, -0.61 + x.abs() * 0.06, 1.10)
                            .with_scale(Vec3::new(0.16, 0.19, 0.10)),
                    ));
                }
                p.spawn((
                    Halo,
                    Mesh3d(ring.clone()),
                    MeshMaterial3d(electric.clone()),
                    Transform::from_xyz(0., 0.8, -0.85)
                        .with_scale(Vec3::splat(1.72))
                        .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                ));
            });
    }
    for id in 0..8 {
        commands
            .spawn((
                Piece(Kind::House, id),
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                p.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(brick.clone()),
                    Transform::from_xyz(0., 1.2, 0.).with_scale(Vec3::new(1.9, 2.4, 1.8)),
                ));
                p.spawn((
                    Mesh3d(cone.clone()),
                    MeshMaterial3d(roof.clone()),
                    Transform::from_xyz(0., 3.1, 0.).with_scale(Vec3::new(1.8, 1.8, 1.8)),
                ));
                for x in [-0.45, 0.45] {
                    p.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(eye.clone()),
                        Transform::from_xyz(x, 1.3, 0.91).with_scale(Vec3::new(0.28, 0.65, 0.035)),
                    ));
                }
            });
    }
    for id in 0..56 {
        commands
            .spawn((
                Piece(Kind::Reed, id),
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                for blade in 0..3 {
                    p.spawn((
                        Mesh3d(cone.clone()),
                        MeshMaterial3d(reeds.clone()),
                        Transform::from_xyz((blade as f32 - 1.) * 0.07, 0.23, 0.)
                            .with_scale(Vec3::new(0.035, 0.47, 0.025))
                            .with_rotation(Quat::from_rotation_z((blade as f32 - 1.) * 0.3)),
                    ));
                }
            });
    }
    for id in 0..24 {
        commands.spawn((
            Piece(Kind::Smoke, id),
            Mesh3d(sphere.clone()),
            MeshMaterial3d(smoke.clone()),
            Transform::default(),
            Visibility::Hidden,
            bevy::light::NotShadowCaster,
        ));
    }
    for id in 0..7 {
        commands
            .spawn((
                Piece(Kind::Bird, id),
                Transform::default(),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                for side in [-1., 1.] {
                    p.spawn((
                        Wing(side, id as f32),
                        Mesh3d(cone.clone()),
                        MeshMaterial3d(dark.clone()),
                        Transform::default(),
                    ));
                }
            });
    }
}

fn perimeter(kind: Kind, id: usize, half: [f32; 2]) -> Vec3 {
    match kind {
        Kind::Watcher => Vec3::new(
            (id as f32 - 1.) * (half[0] + 1.) * 0.87,
            14. + if id == 1 { 3. } else { 0. },
            -half[1] - 16.,
        ),
        Kind::Hand => Vec3::new(
            if id == 0 {
                -half[0] - 12.
            } else {
                half[0] + 12.
            },
            5.,
            -half[1] - 14.,
        ),
        Kind::Orrery => Vec3::new(0., 24., -half[1] - 32.),
        Kind::Panel => Vec3::new(
            if id.is_multiple_of(2) {
                -half[0] - 4.
            } else {
                half[0] + 4.
            },
            -0.2,
            (id / 2) as f32 * half[1] * 0.85 - half[1] * 0.85,
        ),
        Kind::Rig => Vec3::new(0., 13., -half[1] - 6.),
        _ => {
            let angle = PI + (id as f32 + 0.5) / 8. * PI;
            let edge = angle.cos().abs().max(angle.sin().abs());
            Vec3::new(
                angle.cos() / edge * (half[0] + 7.),
                -0.4,
                angle.sin() / edge * (half[1] + 9.),
            )
        }
    }
}

fn reed_position(arena: Arena, id: usize) -> Option<Vec3> {
    let half = arena.half_size();
    (0..48).find_map(|attempt| {
        let k = id * 97 + attempt * 43;
        let p = [
            ((k * 37 % 997) as f32 / 997. * 2. - 1.) * (half[0] - 0.7),
            ((k * 67 % 991) as f32 / 991. * 2. - 1.) * (half[1] - 0.7),
        ];
        (arena.cover(p) > 0. && !arena.blocked(p) && arena.bridge_height(p) == 0.)
            .then(|| Vec3::new(p[0], arena.ground_elevation(p), p[1]))
    })
}

fn clock(time: Res<Time>, session: Res<Session>, mut state: ResMut<Atmosphere>) {
    let dt = time.delta_secs().min(0.1);
    state.seconds += dt;
    state.impact = (state.impact - dt * 1.6).max(0.);
    state.reveal = (state.reveal - dt * 0.45).max(0.);
    if state.run != Some(session.run_id) {
        state.impact = 0.;
        state.reveal = 1.;
        state.run = Some(session.run_id);
        state.tick = session.battle.tick;
    }
    let target = if session.battle.finished {
        0.
    } else {
        session
            .battle
            .units
            .iter()
            .map(|u| 1. - u.health_fraction())
            .sum::<f32>()
            / session.battle.units.len().max(1) as f32
    };
    // Sustained tension, smoothed over seconds. No strobe or damage flash.
    state.tension += (target - state.tension) * (1. - (-dt * 0.65).exp());
    if session.battle.tick != state.tick {
        if session
            .battle
            .damage_events
            .iter()
            .any(|e| e.tick > state.tick)
        {
            state.impact = 1.;
        }
        state.tick = session.battle.tick;
    }
}

// Explicit disjoint Bevy queries keep the four transform writers conflict-free.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn animate(
    session: Res<Session>,
    state: Res<Atmosphere>,
    mut pieces: Query<
        (&Piece, &mut Transform, &mut Visibility),
        (Without<Wing>, Without<Iris>, Without<Halo>, Without<Finger>),
    >,
    mut wings: Query<
        (&Wing, &mut Transform),
        (
            Without<Piece>,
            Without<Iris>,
            Without<Halo>,
            Without<Finger>,
        ),
    >,
    mut irises: Query<
        (&Iris, &mut Transform),
        (
            Without<Piece>,
            Without<Wing>,
            Without<Halo>,
            Without<Finger>,
        ),
    >,
    mut halos: Query<
        &mut Transform,
        (
            With<Halo>,
            Without<Piece>,
            Without<Wing>,
            Without<Iris>,
            Without<Finger>,
        ),
    >,
    mut fingers: Query<
        (&Finger, &mut Transform),
        (Without<Piece>, Without<Wing>, Without<Iris>, Without<Halo>),
    >,
    mut reeds: Local<(Option<Arena>, Vec<Option<Vec3>>)>,
    glow: Res<StageGlow>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lights: Query<&mut DirectionalLight, With<StageKey>>,
) {
    let arena = session.battle.arena;
    if reeds.0 != Some(arena) {
        *reeds = (
            Some(arena),
            (0..56).map(|id| reed_position(arena, id)).collect(),
        );
    }
    let t = state.seconds;
    let half = arena.half_size();
    for (piece, mut transform, mut visible) in &mut pieces {
        *visible = Visibility::Inherited;
        let phase = piece.1 as f32 * 2.399_963;
        *transform = match piece.0 {
            Kind::Watcher => {
                let p = perimeter(piece.0, piece.1, half);
                Transform::from_translation(p + Vec3::Y * (t * 0.63 + phase).sin() * 0.12)
                    .with_rotation(Quat::from_rotation_y((-p.x).atan2(-p.z)))
                    .with_scale(Vec3::new(2.8, 2.8 + (t * 0.8 + phase).sin() * 0.025, 2.8))
            }
            Kind::Hand => Transform::from_translation(perimeter(piece.0, piece.1, half))
                .with_rotation(
                    Quat::from_rotation_z(if piece.1 == 0 { 0.20 } else { -0.20 })
                        * Quat::from_rotation_x(-0.12 + (t * 0.22 + phase).sin() * 0.025),
                ),
            Kind::Orrery => Transform::from_translation(perimeter(piece.0, piece.1, half))
                .with_rotation(Quat::from_rotation_z(t * 0.025)),
            Kind::House => Transform::from_translation(perimeter(piece.0, piece.1, half))
                .with_rotation(Quat::from_rotation_y(phase)),
            Kind::Reed => {
                let Some(p) = reeds.1[piece.1] else {
                    *visible = Visibility::Hidden;
                    continue;
                };
                Transform::from_translation(p).with_rotation(
                    Quat::from_rotation_z((t * 1.8 + phase).sin() * 0.20)
                        * Quat::from_rotation_y(phase),
                )
            }
            Kind::Smoke => {
                let age = (t * 0.12 + piece.1 as f32 / 3.).fract();
                let base = perimeter(Kind::House, piece.1 / 3, half);
                Transform::from_translation(base + Vec3::new(age * 2., 4. + age * 5., age * 0.7))
                    .with_scale(Vec3::splat((0.3 + age * 0.8) * (PI * age).sin().max(0.01)))
            }
            Kind::Bird => {
                let angle = t * 0.12 + phase;
                Transform::from_xyz(
                    angle.cos() * (half[0] + 4.),
                    7.5 + (angle * 2.).sin(),
                    angle.sin() * (half[1] + 3.),
                )
                .with_rotation(Quat::from_rotation_y(-angle))
                .with_scale(Vec3::splat(0.7))
            }
            Kind::Panel => {
                let side = if piece.1.is_multiple_of(2) { -1. } else { 1. };
                Transform::from_translation(
                    perimeter(piece.0, piece.1, half) + Vec3::X * side * (1. - state.reveal) * 1.5,
                )
                .with_rotation(Quat::from_rotation_y(
                    side * (0.25 + (1. - state.reveal) * 0.85),
                ))
            }
            Kind::Rig => Transform::from_translation(perimeter(piece.0, piece.1, half))
                .with_scale(Vec3::new((half[0] + 9.) / 20., 1., 1.)),
        };
    }
    for (finger, mut transform) in &mut fingers {
        *transform = Transform::from_xyz(finger.0, 4.2, 0.).with_rotation(
            Quat::from_rotation_z(-finger.0 * 0.065)
                * Quat::from_rotation_x(
                    0.10 + (t * 0.32 + finger.0).sin() * 0.07 + state.impact * 0.18 * finger.1,
                ),
        );
    }
    for (wing, mut transform) in &mut wings {
        *transform = Transform::from_xyz(wing.0 * 0.17, 0., 0.)
            .with_rotation(Quat::from_rotation_z(
                wing.0 * (1.1 + (t * 7. + wing.1).sin() * 0.45),
            ))
            .with_scale(Vec3::new(0.055, 0.5, 0.13));
    }
    // The ornaments look toward the real fight. Their gaze conveys interest,
    // not the perception or decisions of a simulated citizen.
    let attention = session
        .battle
        .units
        .iter()
        .filter(|u| u.health > 0.)
        .map(|u| u.position[0])
        .sum::<f32>()
        / session.battle.units.len().max(1) as f32;
    for (iris, mut transform) in &mut irises {
        transform.translation = iris.0 + Vec3::new((attention * 0.025).clamp(-0.12, 0.12), 0., 0.);
        transform.scale.y = 0.19 + state.impact * 0.055;
    }
    for mut transform in &mut halos {
        transform.rotation =
            Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_y((t * 0.07).rem_euclid(TAU));
    }
    if let Some(mut material) = materials.get_mut(&glow.0) {
        material.emissive = LinearRgba::rgb(
            0.03 + state.tension * 0.15,
            1.3 - state.tension * 0.45,
            1.6 + state.tension * 0.3,
        );
    }
    for mut light in &mut lights {
        light.color = Color::srgb(1., 0.90 - state.tension * 0.14, 0.79 - state.tension * 0.15);
        light.illuminance = 10000. + state.tension * 1000.;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_scenery_stays_outside_legal_ground_and_reeds_stay_in_cover() {
        for arena in [
            Arena::OpenField,
            Arena::Citadel,
            Arena::Mire,
            phalanx::pressure::generate(&phalanx::progression::Campaign::new().teams, 42),
        ] {
            let half = arena.half_size();
            for (kind, count) in [
                (Kind::Watcher, 3),
                (Kind::House, 8),
                (Kind::Panel, 6),
                (Kind::Rig, 1),
                (Kind::Hand, 2),
                (Kind::Orrery, 1),
            ] {
                for id in 0..count {
                    let p = perimeter(kind, id, half);
                    assert!(p.x.abs() - 2. > half[0] || p.z.abs() - 2. > half[1]);
                }
            }
            for id in 0..56 {
                if let Some(p) = reed_position(arena, id) {
                    assert!(arena.cover([p.x, p.z]) > 0.);
                    assert!(!arena.blocked([p.x, p.z]));
                    assert_eq!(p.y, arena.ground_elevation([p.x, p.z]));
                }
            }
        }
    }
}
