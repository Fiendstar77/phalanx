use bevy::{
    asset::RenderAssetUsages,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    mesh::PrimitiveTopology,
    prelude::*,
};
use phalanx::{
    ACTIONS, Battle, DT, Mind, Role,
    learned::Controller,
    navigation::threat_at_known,
    perception::{self, Source},
    terrain::Arena,
};

#[derive(Resource)]
struct Session {
    battle: Battle,
    running: bool,
    selected: usize,
    speed: f32,
    accumulator: f32,
    paths: bool,
    overlay: usize,
    orbit: f32,
    zoom: f32,
    previous_positions: [[f32; 2]; 10],
    logged_disagreements: usize,
    run_id: u32,
    disagreement_log: Option<std::io::BufWriter<std::fs::File>>,
}
impl Session {
    fn new(battle: Battle) -> Self {
        let previous_positions = std::array::from_fn(|i| battle.units[i].position);
        Self {
            battle,
            running: false,
            selected: 0,
            speed: 1.0,
            accumulator: 0.0,
            paths: true,
            overlay: 0,
            orbit: 0.0,
            zoom: 24.0,
            previous_positions,
            logged_disagreements: 0,
            run_id: 0,
            disagreement_log: None,
        }
    }
    fn remember_positions(&mut self) {
        for unit in &self.battle.units {
            self.previous_positions[unit.id] = unit.position;
        }
    }
    fn presented(&self, id: usize) -> [f32; 2] {
        let current = self.battle.units[id].position;
        if !self.running {
            return current;
        }
        let old = self.previous_positions[id];
        let alpha = (self.accumulator / DT).clamp(0.0, 1.0);
        [
            old[0] + (current[0] - old[0]) * alpha,
            old[1] + (current[1] - old[1]) * alpha,
        ]
    }
}
#[derive(Component)]
struct Pawn(usize);
#[derive(Component)]
struct RockVisual(usize);
#[derive(Component)]
struct GroundArena(Arena);
#[derive(Component)]
struct ChokeWall;
#[derive(Component, Default)]
struct PressLatch(bool);
#[derive(Component)]
struct Inspector;
#[derive(Component)]
struct TraitValue(usize);
#[derive(Component)]
struct Status;
#[derive(Component)]
struct DivergenceSummary;
#[derive(Component, Clone, Copy, Debug)]
enum Control {
    Run,
    Pause,
    Step,
    Reset,
    Swap,
    Speed,
    Paths,
    Arena,
    Overlay,
    Controller,
    Aggressive,
    Disciplined,
    Trait(usize, f32),
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arena = args
        .windows(2)
        .find(|w| w[0] == "--arena")
        .map(|w| Arena::parse(&w[1]).expect("arena must be ridge, choke, or open"))
        .unwrap_or_default();
    let seed = args
        .windows(2)
        .find(|w| w[0] == "--seed")
        .map(|w| {
            w[1].parse::<u64>()
                .expect("seed must be an unsigned integer")
        })
        .unwrap_or(42);
    if args.iter().any(|a| a == "--headless") {
        let mut battle = Battle::with_arena(seed, args.iter().any(|a| a == "--swap"), arena);
        while !battle.finished {
            battle.step();
        }
        let output = serde_json::to_string_pretty(&battle).unwrap();
        if let Some(path) = args.windows(2).find(|w| w[0] == "--output").map(|w| &w[1]) {
            std::fs::write(path, output).expect("cannot write telemetry");
        } else {
            println!("{output}");
        }
        return;
    }
    let mut battle = Battle::with_arena(seed, args.iter().any(|a| a == "--swap"), arena);
    battle.controller = if args.iter().any(|a| a == "--learned") {
        Controller::Learned
    } else {
        Controller::Utility
    };
    let mut session = Session::new(battle);
    std::fs::create_dir_all("outputs").expect("create log directory");
    let log_path = format!(
        "outputs/disagreements-{}.jsonl",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    session.disagreement_log = Some(std::io::BufWriter::new(
        std::fs::File::create(&log_path).expect("create disagreement log"),
    ));
    if let Some(log) = &mut session.disagreement_log {
        use std::io::Write;
        writeln!(log,"{}",serde_json::json!({"event":"session","schema_version":1,"simulation_version":env!("CARGO_PKG_VERSION"),"actions":ACTIONS,"policy_source_sha256":session.battle.policy.as_ref().map(|p|&p.source_sha256)})).expect("write log header");
    }
    info!("Live controller disagreement log: {log_path}");
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "PHALANX | Minds x Terrain".into(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.055)))
    .insert_resource(session)
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            controls,
            keyboard,
            simulate,
            log_disagreements,
            sync_terrain,
            sync_rocks,
            sync_pawns,
            pick,
            camera_motion,
            overlays,
            update_text,
            update_minds,
            update_status,
            update_divergence,
        )
            .chain(),
    );
    if args.iter().any(|a| a == "--profile") {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin {
                filter: Some(
                    [
                        FrameTimeDiagnosticsPlugin::FPS,
                        FrameTimeDiagnosticsPlugin::FRAME_TIME,
                    ]
                    .into_iter()
                    .collect(),
                ),
                ..default()
            },
        ));
    }
    app.run();
}
fn font(size: f32) -> TextFont {
    TextFont {
        font_size: FontSize::Px(size),
        ..default()
    }
}
fn button(parent: &mut ChildSpawnerCommands, title: &str, control: Control) {
    parent
        .spawn((
            Button,
            PressLatch::default(),
            control,
            Node {
                padding: UiRect::axes(px(12), px(6)),
                margin: UiRect::all(px(3)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.18, 0.25)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(title),
                font(15.0),
                TextColor(Color::srgb(0.85, 0.91, 0.96)),
            ));
        });
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    session: Res<Session>,
) {
    commands.spawn((
        GroundArena(session.battle.arena),
        Mesh3d(meshes.add(terrain_mesh(session.battle.arena))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::default(),
    ));
    for z in [-5.0, 5.0] {
        commands.spawn((
            ChokeWall,
            Mesh3d(meshes.add(Cuboid::new(2.0, 2.4, 6.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.3, 0.35, 0.4))),
            Transform::from_xyz(0.0, 1.2, z),
            Visibility::Hidden,
        ));
    }
    let rock_mesh = meshes.add(Sphere::new(0.16));
    let rock_material = materials.add(Color::srgb(0.85, 0.8, 0.65));
    for i in 0..20 {
        commands.spawn((
            RockVisual(i),
            Mesh3d(rock_mesh.clone()),
            MeshMaterial3d(rock_material.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(22.4, 0.5, 16.4))),
        MeshMaterial3d(materials.add(Color::srgb(0.07, 0.11, 0.15))),
        Transform::from_xyz(0.0, -0.3, 0.0),
    ));
    let capsule = meshes.add(Capsule3d::new(0.28, 0.65));
    let thrower = meshes.add(Cone::new(0.4, 1.1));
    let head = meshes.add(Sphere::new(0.2));
    let base = meshes.add(Cylinder::new(0.43, 0.09));
    let red = materials.add(Color::srgb(0.92, 0.28, 0.25));
    let blue = materials.add(Color::srgb(0.2, 0.58, 0.94));
    for id in 0..10 {
        let material = if id < 5 { red.clone() } else { blue.clone() };
        commands
            .spawn((
                Pawn(id),
                Mesh3d(if id % 5 >= 3 {
                    thrower.clone()
                } else {
                    capsule.clone()
                }),
                MeshMaterial3d(material.clone()),
                Transform::default(),
            ))
            .with_children(|p| {
                if id % 5 >= 3 {
                    p.spawn((
                        Mesh3d(head.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_xyz(0.0, 0.45, 0.0),
                    ));
                }
                p.spawn((
                    Mesh3d(base.clone()),
                    MeshMaterial3d(material),
                    Transform::from_xyz(0.0, -0.57, 0.0),
                ));
            });
    }
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_xyz(-6.0, 12.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 21.0, 24.0).looking_at(Vec3::new(3.5, 0.0, 0.0), Vec3::Y),
    ));
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: px(22),
            left: px(26),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|p| {
            p.spawn((
                Text::new("PHALANX"),
                font(32.0),
                TextColor(Color::srgb(0.91, 0.94, 0.96)),
            ));
            p.spawn((
                Text::new("AUTONOMOUS COMBAT / MINDS x TERRAIN"),
                font(13.0),
                TextColor(Color::srgb(0.45, 0.65, 0.71)),
            ));
            p.spawn((
                Status,
                Text::new(""),
                font(17.0),
                Node {
                    margin: UiRect::top(px(12)),
                    ..default()
                },
            ));
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16),
                top: px(16),
                bottom: px(90),
                width: px(330),
                padding: UiRect::all(px(18)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.055, 0.085, 0.97)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("AGENT INSPECTOR"),
                font(20.0),
                TextColor(Color::srgb(0.65, 0.85, 0.88)),
            ));
            p.spawn((
                Inspector,
                Text::new(""),
                font(12.0),
                Node {
                    margin: UiRect::top(px(14)),
                    ..default()
                },
            ));
            p.spawn((
                Text::new("Edit mind | used at next decision"),
                font(12.0),
                Node {
                    margin: UiRect::top(px(10)),
                    ..default()
                },
            ));
            p.spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|row| {
                button(row, "Aggressive", Control::Aggressive);
                button(row, "Disciplined", Control::Disciplined);
            });
            for (i, name) in [
                "Aggression",
                "Caution",
                "Courage",
                "Teamwork",
                "Patience",
                "Self-preservation",
                "Discipline",
            ]
            .iter()
            .enumerate()
            {
                p.spawn(Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        TraitValue(i),
                        Text::new(*name),
                        font(13.0),
                        Node {
                            width: px(165),
                            ..default()
                        },
                    ));
                    button(row, "-", Control::Trait(i, -0.1));
                    button(row, "+", Control::Trait(i, 0.1));
                });
            }
        });
    commands.spawn((
        DivergenceSummary,
        Text::new(""),
        font(13.0),
        TextColor(Color::srgb(0.6, 0.78, 0.82)),
        Node {
            position_type: PositionType::Absolute,
            left: px(26),
            top: px(205),
            ..default()
        },
    ));
    commands.spawn(Node { position_type: PositionType::Absolute, left: px(20), bottom: px(18), flex_direction: FlexDirection::Column, ..default() }).with_children(|p| {
        p.spawn((Text::new("Click a fighter to inspect | Space pause/run | R reset | N step | Q/E orbit | W/S zoom | M arena | O overlay | C controller"),font(13.0),TextColor(Color::srgb(0.6,0.72,0.78))));
        p.spawn(Node { margin: UiRect::top(px(8)), ..default() }).with_children(|row| {
            button(row,"Run",Control::Run); button(row,"Pause",Control::Pause); button(row,"Step",Control::Step); button(row,"Reset",Control::Reset); button(row,"Swap profiles",Control::Swap); button(row,"Speed",Control::Speed); button(row,"Paths",Control::Paths); button(row,"Arena",Control::Arena); button(row,"Overlay",Control::Overlay); button(row,"Utility/Learned",Control::Controller);
        });
    });
}
fn apply(control: Control, session: &mut Session) {
    debug!(?control, "Battle control");
    let controller = session.battle.controller;
    match control {
        Control::Run => session.running = !session.battle.finished,
        Control::Pause => session.running = false,
        Control::Step => {
            session.running = false;
            session.remember_positions();
            session.battle.step();
        }
        Control::Reset => {
            session.battle = Battle::with_arena(
                session.battle.seed,
                session.battle.swapped,
                session.battle.arena,
            );
            session.running = false;
            session.accumulator = 0.0;
        }
        Control::Swap => {
            session.battle = Battle::with_arena(
                session.battle.seed,
                !session.battle.swapped,
                session.battle.arena,
            );
            session.running = false;
            session.accumulator = 0.0;
        }
        Control::Speed => {
            session.speed = if session.speed < 10.0 {
                if session.speed == 4.0 {
                    10.0
                } else {
                    session.speed * 2.0
                }
            } else {
                1.0
            }
        }
        Control::Controller => {
            session.battle.controller = if session.battle.controller == Controller::Utility {
                Controller::Learned
            } else {
                Controller::Utility
            };
            session.battle.learned_team = None;
        }
        Control::Paths => session.paths = !session.paths,
        Control::Overlay => session.overlay = (session.overlay + 1) % 5,
        Control::Arena => {
            session.battle = Battle::with_arena(
                session.battle.seed,
                session.battle.swapped,
                session.battle.arena.next(),
            );
            session.running = false;
            session.accumulator = 0.0;
        }
        Control::Aggressive => session.battle.units[session.selected].mind = Mind::aggressive(),
        Control::Disciplined => session.battle.units[session.selected].mind = Mind::disciplined(),
        Control::Trait(i, change) => {
            let m = &mut session.battle.units[session.selected].mind;
            let field = match i {
                0 => &mut m.aggression,
                1 => &mut m.caution,
                2 => &mut m.courage,
                3 => &mut m.teamwork,
                4 => &mut m.patience,
                5 => &mut m.self_preservation,
                _ => &mut m.discipline,
            };
            *field = (*field + change).clamp(0.0, 1.0);
        }
    }
    if matches!(control, Control::Reset | Control::Swap | Control::Arena) {
        session.battle.controller = controller;
        session.logged_disagreements = 0;
        session.run_id += 1;
        session.remember_positions();
    }
}
fn controls(
    mut session: ResMut<Session>,
    mut buttons: Query<(
        &Interaction,
        &Control,
        &mut BackgroundColor,
        &mut PressLatch,
    )>,
) {
    for (interaction, control, mut color, mut latch) in &mut buttons {
        color.0 = match interaction {
            Interaction::Pressed => Color::srgb(0.2, 0.43, 0.47),
            Interaction::Hovered => Color::srgb(0.2, 0.29, 0.36),
            _ => Color::srgb(0.12, 0.18, 0.25),
        };
        if *interaction == Interaction::Pressed && !latch.0 {
            apply(*control, &mut session);
        }
        latch.0 = *interaction == Interaction::Pressed;
    }
}
fn keyboard(keys: Res<ButtonInput<KeyCode>>, mut session: ResMut<Session>) {
    if keys.just_pressed(KeyCode::Space) {
        let c = if session.running {
            Control::Pause
        } else {
            Control::Run
        };
        apply(c, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyR) {
        apply(Control::Reset, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyN) {
        apply(Control::Step, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyC) {
        apply(Control::Controller, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyM) {
        apply(Control::Arena, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyO) {
        apply(Control::Overlay, &mut session);
    }
    if keys.just_pressed(KeyCode::Tab) {
        session.selected = (session.selected + 1) % 10;
    }
}
fn simulate(time: Res<Time>, mut session: ResMut<Session>) {
    if !session.running || session.battle.finished {
        return;
    }
    session.accumulator += time.delta_secs().min(0.25) * session.speed;
    while session.accumulator >= DT {
        session.accumulator -= DT;
        session.remember_positions();
        session.battle.step();
        if session.battle.finished {
            session.running = false;
            break;
        }
    }
}
fn sync_pawns(session: Res<Session>, mut pawns: Query<(&Pawn, &mut Transform, &mut Visibility)>) {
    for (pawn, mut transform, mut visible) in &mut pawns {
        let unit = &session.battle.units[pawn.0];
        let p = session.presented(unit.id);
        let next = Vec3::new(p[0], session.battle.arena.elevation(p) + 0.64, p[1]);
        if transform.translation != next {
            transform.translation = next;
        }
        *visible = if unit.health > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
fn pick(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut session: ResMut<Session>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if cursor.x > window.width() - 365.0 || cursor.y > window.height() - 100.0 {
        return;
    }
    let (camera, transform) = *camera;
    let mut best = (35.0, session.selected);
    for unit in &session.battle.units {
        if unit.health <= 0.0 {
            continue;
        }
        let p = session.presented(unit.id);
        if let Ok(screen) = camera.world_to_viewport(
            transform,
            Vec3::new(p[0], session.battle.arena.elevation(p) + 0.6, p[1]),
        ) {
            let d = screen.distance(cursor);
            if d < best.0 {
                best = (d, unit.id);
            }
        }
    }
    session.selected = best.1;
    debug!(?cursor, selected = best.1, "Agent selection");
}
fn camera_motion(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<Session>,
    mut camera: Single<&mut Transform, With<Camera3d>>,
) {
    let dt = time.delta_secs();
    if keys.pressed(KeyCode::KeyQ) {
        session.orbit -= dt * 0.7;
    }
    if keys.pressed(KeyCode::KeyE) {
        session.orbit += dt * 0.7;
    }
    if keys.pressed(KeyCode::KeyW) {
        session.zoom -= dt * 8.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        session.zoom += dt * 8.0;
    }
    session.zoom = session.zoom.clamp(13.0, 38.0);
    let next = Transform::from_xyz(
        session.orbit.sin() * session.zoom,
        session.zoom * 0.88,
        session.orbit.cos() * session.zoom,
    )
    .looking_at(Vec3::new(3.5, 0.0, 0.0), Vec3::Y);
    if camera.translation != next.translation || camera.rotation != next.rotation {
        **camera = next;
    }
}
fn overlays(session: Res<Session>, mut gizmos: Gizmos) {
    let arena = session.battle.arena;
    let u = &session.battle.units[session.selected];
    let rendered = session.presented(u.id);
    if u.health > 0.0 {
        gizmos.circle(
            Isometry3d::new(
                Vec3::new(rendered[0], arena.elevation(rendered) + 0.08, rendered[1]),
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ),
            0.65,
            Color::srgb(1.0, 0.85, 0.35),
        );
        if u.role == Role::RockThrower {
            gizmos.circle(
                Isometry3d::new(
                    Vec3::new(rendered[0], arena.elevation(rendered) + 0.12, rendered[1]),
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                ),
                u.role.range(),
                Color::srgba(1.0, 0.8, 0.35, 0.4),
            );
        }
    }
    if session.paths {
        for unit in &session.battle.units {
            if unit.health <= 0.0 {
                continue;
            }
            let color = if unit.team == 0 {
                Color::srgb(1.0, 0.4, 0.3)
            } else {
                Color::srgb(0.3, 0.7, 1.0)
            };
            let mut previous = session.presented(unit.id);
            for waypoint in unit.route.points.iter().skip(unit.path_cursor) {
                // Sample each actual route edge so its overlay follows the ground.
                for i in 0..4 {
                    let t = i as f32 / 4.0;
                    let v = (i + 1) as f32 / 4.0;
                    let p = [
                        previous[0] + (waypoint[0] - previous[0]) * t,
                        previous[1] + (waypoint[1] - previous[1]) * t,
                    ];
                    let q = [
                        previous[0] + (waypoint[0] - previous[0]) * v,
                        previous[1] + (waypoint[1] - previous[1]) * v,
                    ];
                    gizmos.line(
                        Vec3::new(p[0], arena.elevation(p) + 0.12, p[1]),
                        Vec3::new(q[0], arena.elevation(q) + 0.12, q[1]),
                        color,
                    );
                }
                previous = *waypoint;
            }
        }
    }
    if session.overlay == 4 {
        for contact in &u.contacts {
            let color = if contact.visible {
                Color::srgb(0.2, 1.0, 0.6)
            } else if matches!(contact.source, Source::Ally(_)) {
                Color::srgb(0.8, 0.4, 1.0)
            } else {
                Color::srgb(1.0, 0.75, 0.25)
            };
            let p = contact.position;
            let center = Vec3::new(p[0], arena.elevation(p) + 0.2, p[1]);
            gizmos.circle(
                Isometry3d::new(center, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                0.35 + (1.0 - contact.confidence(session.battle.tick)) * 0.55,
                color,
            );
            gizmos.line(center, center + Vec3::Y * 1.8, color);
        }
        if u.action == 8 || u.action == 3 {
            let a = Vec3::new(
                u.position[0],
                arena.elevation(u.position) + 0.25,
                u.position[1],
            );
            let p = u.target;
            let b = Vec3::new(p[0], arena.elevation(p) + 0.25, p[1]);
            gizmos.line(a, b, Color::srgb(1.0, 0.85, 0.3));
        }
    }
    if session.overlay > 0 && session.overlay < 4 {
        let known = perception::threats(u, session.battle.tick);
        for x in -10..=10 {
            for z in -7..=7 {
                let p = [x as f32 + 0.5, z as f32 + 0.5];
                if arena.blocked(p) {
                    continue;
                }
                let color = match session.overlay {
                    1 => {
                        if arena.cover(p) > 0.0 {
                            Color::srgb(0.2, 0.95, 0.5)
                        } else if arena.roughness(p) > 0.0 {
                            Color::srgb(0.9, 0.55, 0.2)
                        } else {
                            Color::srgb(0.25, 0.4, 0.5)
                        }
                    }
                    2 => {
                        let t = (threat_at_known(arena, p, &known) / 2.0).min(1.0);
                        Color::srgb(0.15 + t * 0.85, 0.55 * (1.0 - t), 0.25 * (1.0 - t))
                    }
                    _ => {
                        let h = arena.elevation(p) / 1.7;
                        Color::srgb(0.2 + h * 0.65, 0.3 + h * 0.65, 0.8)
                    }
                };
                gizmos.rect(
                    Isometry3d::new(
                        Vec3::new(p[0], arena.elevation(p) + 0.16, p[1]),
                        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                    ),
                    Vec2::splat(0.85),
                    color,
                );
            }
        }
    }
}
fn update_text(session: Res<Session>, mut text: Query<&mut Text, With<Inspector>>) {
    let u = &session.battle.units[session.selected];
    let scores: String = if u.decision_counts.iter().sum::<u32>() == 0 {
        "Awaiting first decision.\n".into()
    } else {
        ACTIONS
            .iter()
            .zip(u.scores)
            .map(|(name, score)| {
                if score < 0.0 {
                    format!("{name:<13} --\n")
                } else {
                    format!("{name:<13} {score:.2}\n")
                }
            })
            .collect()
    };
    let focus = u
        .enemy
        .and_then(|id| u.contacts.iter().find(|c| c.enemy == id))
        .map(|c| {
            let source = if c.visible {
                "own sight".into()
            } else {
                match c.source {
                    Source::OwnSight => "own memory".into(),
                    Source::Ally(id) => format!("ally {:02} report", id % 5 + 1),
                }
            };
            format!(
                "Contact {:02}: {} ({:.1}s)",
                c.enemy % 5 + 1,
                source,
                session.battle.tick.saturating_sub(c.seen_tick) as f32 * DT
            )
        })
        .unwrap_or_else(|| "No contact: search deployment".into());
    let support = u
        .support_target
        .map(|id| format!("Supporting ally {:02}", id % 5 + 1))
        .unwrap_or_else(|| "".into());
    let comparison = u
        .comparison
        .as_ref()
        .map(|c| {
            format!(
                "UTILITY: {}\nPYTORCH: {}\nAGREEMENT: {}\nEXECUTED [{}]: {}",
                ACTIONS[c.utility],
                c.learned.map(|i| ACTIONS[i]).unwrap_or("UNAVAILABLE"),
                if c.disagreed {
                    "NO"
                } else if c.learned.is_some() {
                    "YES"
                } else {
                    "UNKNOWN"
                },
                c.controller.name(),
                ACTIONS[c.executed]
            )
        })
        .unwrap_or_else(|| "Comparison awaits first decision".into());
    for mut t in &mut text {
        let content = format!(
            "{}-{:02}   HEALTH {:.0}%\n{} | {}\nSurface: cover {:.0}% / rough {:.0}%\nVisible {} / known {} / reports {}\n{}\n{}\n{}\n\nLAST DECISION SCORES\n{}\nRoute cost {:.1} | {} waypoints\nExposure {:.1} / threat {:.1}\nRocks thrown {} / hits {}\nWhy (last decision):\n{}",
            if u.team == 0 { "RED" } else { "BLUE" },
            u.id % 5 + 1,
            u.health,
            u.role.name(),
            if u.health <= 0.0 {
                "ELIMINATED"
            } else {
                ACTIONS[u.action]
            },
            session.battle.arena.cover(u.position) * 100.0,
            session.battle.arena.roughness(u.position) * 100.0,
            u.visible_enemies,
            u.contacts.len(),
            u.contacts
                .iter()
                .filter(|c| matches!(c.source, Source::Ally(_)))
                .count(),
            focus,
            support,
            comparison,
            scores,
            u.route.cost.total(),
            u.route.points.len().saturating_sub(u.path_cursor),
            u.route.cost.exposure,
            u.route.cost.threat,
            u.rocks_thrown,
            u.rocks_hit,
            u.decision_reason
        );
        if t.0 != content {
            t.0 = content;
        }
    }
}

fn update_status(session: Res<Session>, mut text: Query<&mut Text, With<Status>>) {
    for mut t in &mut text {
        let content = format!(
            "{} | {:.1}s | x{:.0} | Seed {}\nRED {} / 5     BLUE {} / 5\n{} | {}\nController: {} | disagreements {}\nOverlay: {} | {}",
            if session.running { "RUNNING" } else { "PAUSED" },
            session.battle.tick as f32 * DT,
            session.speed,
            session.battle.seed,
            session.battle.survivors(0),
            session.battle.survivors(1),
            session.battle.result,
            session.battle.arena.name(),
            session.battle.controller.name(),
            session.battle.disagreements.len(),
            [
                "OFF",
                "TERRAIN",
                "THREAT (known contacts)",
                "ELEVATION",
                "KNOWLEDGE"
            ][session.overlay],
            match session.overlay {
                2 => "green=low / red=high",
                3 => "blue=low / pale=high",
                4 => "green=seen / gold=memory / purple=report",
                _ => "green=cover / brown=rough",
            }
        );
        if t.0 != content {
            t.0 = content;
        }
    }
}

fn update_divergence(session: Res<Session>, mut text: Query<&mut Text, With<DivergenceSummary>>) {
    let summary = &session.battle.comparison_summary;
    let agreement = summary
        .agreement_rate()
        .map(|r| format!("{:.1}%", r * 100.0))
        .unwrap_or_else(|| "N/A".into());
    let mut content = format!(
        "ALL DECISIONS | Agreement: {agreement} / {} compared / {} unavailable",
        summary.compared, summary.unavailable
    );
    let pairs = summary.top_disagreements(3);
    if pairs.is_empty() {
        content.push_str("\nTop disagreements: none yet");
    } else {
        content.push_str("\nTop disagreements (UTILITY -> LEARNED):");
        for pair in pairs {
            content.push_str(&format!(
                "\n{} -> {}  {}",
                pair.utility, pair.learned, pair.count
            ));
        }
    }
    for mut t in &mut text {
        if t.0 != content {
            t.0 = content.clone();
        }
    }
}

fn terrain_mesh(arena: Arena) -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    for x in -22..22 {
        for z in -16..16 {
            let a = [x as f32 * 0.5, z as f32 * 0.5];
            let b = [a[0] + 0.5, a[1]];
            let c = [a[0], a[1] + 0.5];
            let d = [a[0] + 0.5, a[1] + 0.5];
            let middle = [a[0] + 0.25, a[1] + 0.25];
            let color = if arena.cover(middle) > 0.0 {
                Color::srgb(0.18, 0.4, 0.24)
            } else if arena.roughness(middle) > 0.0 {
                Color::srgb(0.4, 0.28, 0.17)
            } else {
                Color::srgb(0.17, 0.23, 0.24)
            };
            for p in [a, c, b, b, c, d] {
                positions.push([p[0], arena.elevation(p), p[1]]);
                colors.push(color.to_linear().to_f32_array());
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.compute_normals();
    mesh
}
fn sync_terrain(
    session: Res<Session>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut ground: Single<(&Mesh3d, &mut GroundArena)>,
    mut walls: Query<&mut Visibility, With<ChokeWall>>,
) {
    let (mesh, rendered) = &mut *ground;
    if rendered.0 != session.battle.arena {
        if let Some(mut asset) = meshes.get_mut(&mesh.0) {
            *asset = terrain_mesh(session.battle.arena);
        }
        rendered.0 = session.battle.arena;
    }
    for mut wall in &mut walls {
        *wall = if session.battle.arena == Arena::Choke {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
fn sync_rocks(
    session: Res<Session>,
    mut visuals: Query<(&RockVisual, &mut Transform, &mut Visibility)>,
) {
    let arena = session.battle.arena;
    for (visual, mut transform, mut visibility) in &mut visuals {
        if let Some(rock) = session.battle.rocks.get(visual.0) {
            *visibility = Visibility::Inherited;
            let fraction = if session.running {
                session.accumulator / DT
            } else {
                0.0
            };
            let t = ((rock.age as f32 + fraction) / rock.flight_ticks as f32).min(1.0);
            let x = rock.from[0] + (rock.to[0] - rock.from[0]) * t;
            let z = rock.from[1] + (rock.to[1] - rock.from[1]) * t;
            let y = arena.elevation(rock.from)
                + (arena.elevation(rock.to) - arena.elevation(rock.from)) * t
                + 0.9
                + (t * std::f32::consts::PI).sin() * 1.4;
            transform.translation = Vec3::new(x, y, z);
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn update_minds(session: Res<Session>, mut values: Query<(&TraitValue, &mut Text)>) {
    let m = session.battle.units[session.selected].mind;
    let traits = [
        m.aggression,
        m.caution,
        m.courage,
        m.teamwork,
        m.patience,
        m.self_preservation,
        m.discipline,
    ];
    let names = [
        "Aggression",
        "Caution",
        "Courage",
        "Teamwork",
        "Patience",
        "Self-preservation",
        "Discipline",
    ];
    for (value, mut text) in &mut values {
        let label = format!("{} {:.2}", names[value.0], traits[value.0]);
        if text.0 != label {
            text.0 = label;
        }
    }
}
fn log_disagreements(mut session: ResMut<Session>) {
    use std::io::Write;
    if session.logged_disagreements >= session.battle.disagreements.len() {
        return;
    }
    let fresh = session.battle.disagreements[session.logged_disagreements..].to_vec();
    session.logged_disagreements = session.battle.disagreements.len();
    let run_id = session.run_id;
    let seed = session.battle.seed;
    let arena = session.battle.arena;
    if let Some(log) = &mut session.disagreement_log {
        for comparison in fresh {
            let event = serde_json::json!({"event":"disagreement","run_id":run_id,"seed":seed,"arena":arena,"comparison":comparison});
            if let Err(e) = writeln!(log, "{event}") {
                error!("Disagreement log write failed: {e}");
            }
        }
        if let Err(e) = log.flush() {
            error!("Disagreement log flush failed: {e}");
        }
    }
}
