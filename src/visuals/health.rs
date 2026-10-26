//! Screen-space health and effects from successful simulation healing events.
use super::*;
#[derive(Component)]
struct HealthBar(usize);
#[derive(Component)]
struct Fill;
#[derive(Component)]
struct Caption(bool);
#[derive(Default)]
struct Feedback {
    run: Option<u32>,
    last_tick: Option<u32>,
    pulses: Vec<(phalanx::HealEvent, f32)>,
    captured: bool,
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup_bars)
        .add_systems(Update, update.after(super::super::camera_motion));
}
fn setup_bars(mut commands: Commands) {
    for id in 0..20 {
        commands
            .spawn((
                HealthBar(id),
                GlobalZIndex(4),
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    width: px(64),
                    height: px(24),
                    ..default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    BackgroundColor(Color::srgba(0.015, 0.02, 0.03, 0.9)),
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(17),
                        width: px(64),
                        height: px(7),
                        padding: UiRect::all(px(1)),
                        ..default()
                    },
                ))
                .with_children(|p| {
                    p.spawn((
                        Fill,
                        BackgroundColor(Color::srgb(0.2, 0.85, 0.3)),
                        Node {
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },
                    ));
                });
                p.spawn((
                    Caption(false),
                    Text::new(""),
                    super::super::font(11.),
                    TextColor(Color::WHITE),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                ));
                p.spawn((
                    Caption(true),
                    Text::new(""),
                    super::super::font(13.),
                    TextColor(Color::srgb(0.2, 1.0, 0.45)),
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(-18),
                        ..default()
                    },
                ));
            });
    }
}
#[allow(clippy::too_many_arguments)]
fn update(
    session: Res<Session>,
    time: Res<Time>,
    camera: Single<(&Camera, &GlobalTransform), super::super::MainCameraFilter>,
    mut bars: Query<(&HealthBar, &mut Node, &Children), Without<Fill>>,
    mut fills: Query<(&mut Node, &mut BackgroundColor), (With<Fill>, Without<HealthBar>)>,
    mut captions: Query<(&Caption, &mut Text)>,
    children: Query<&Children>,
    mut feedback: Local<Feedback>,
    mut gizmos: Gizmos,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    let battle = &session.battle;
    if feedback.run != Some(session.run_id) {
        *feedback = Feedback {
            run: Some(session.run_id),
            ..default()
        };
    }
    for event in &battle.heals {
        if feedback.last_tick.is_none_or(|tick| event.tick > tick) {
            feedback.pulses.push((event.clone(), 0.0));
        }
    }
    feedback.last_tick = battle.heals.last().map(|e| e.tick);
    for (_, age) in &mut feedback.pulses {
        *age += time.delta_secs();
    }
    feedback.pulses.retain(|(_, age)| *age < 1.0);
    let at = |id: usize| {
        let p = session.presented(id);
        Vec3::new(p[0], battle.arena.elevation(p) + 1.1, p[1])
    };
    for (event, age) in &feedback.pulses {
        if session.menu {
            continue;
        }
        let source = at(event.healer);
        let target = at(event.target);
        let green = Color::srgba(0.15, 1.0, 0.45, 1.0 - age);
        if *age < 0.45 {
            gizmos.line(source, target, green);
        }
        let p = session.presented(event.target);
        gizmos.circle(
            Isometry3d::new(
                Vec3::new(p[0], battle.arena.elevation(p) + 0.12, p[1]),
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ),
            0.35 + age * 0.4,
            green,
        );
        for i in 0..4 {
            let phase = *age * 5.0 + i as f32 * std::f32::consts::FRAC_PI_2;
            let p = target + Vec3::new(phase.cos() * 0.32, *age * 0.8, phase.sin() * 0.32);
            gizmos.line(p - Vec3::Y * 0.08, p + Vec3::Y * 0.08, green);
            gizmos.line(p - Vec3::X * 0.06, p + Vec3::X * 0.06, green);
        }
    }
    if std::env::args().any(|a| a == "--healing-witness")
        && time.elapsed_secs()
            > if std::env::args().any(|a| a == "--hybrid-witness") {
                1.5
            } else {
                3.0
            }
    {
        if !feedback.captured
            && feedback
                .pulses
                .iter()
                .any(|(_, age)| *age > 0.15 && *age < 0.4)
        {
            feedback.captured = true;
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(
                    if std::env::args().any(|a| a == "--hybrid-witness") {
                        "outputs/hybrid-witness.png"
                    } else {
                        "outputs/healing-witness.png"
                    },
                ));
            println!(
                "HEALING_WITNESS {}",
                serde_json::json!({"fighter_class":battle.units[session.selected].class_name(), "title":battle.units[session.selected].command_title, "mastery":battle.units[session.selected].mastery, "shots":battle.units[session.selected].rocks_thrown, "heals":battle.heals,"equipment":battle.units[session.selected].equipment})
            );
        }
        if time.elapsed_secs() > 7.0 {
            exit.write(AppExit::Success);
        }
    }
    let mut occupied: Vec<Vec2> = Vec::new();
    for (bar, mut node, descendants) in &mut bars {
        node.display = Display::None;
        let Some(unit) = battle.units.get(bar.0) else {
            continue;
        };
        if session.menu || unit.health <= 0.0 || session.cinematic_playing() {
            continue;
        }
        let Ok(screen) = camera.0.world_to_viewport(
            camera.1,
            at(bar.0) + Vec3::Y * if unit.displays_standard() { 1.6 } else { 0.55 },
        ) else {
            continue;
        };
        let size = camera
            .0
            .logical_viewport_size()
            .unwrap_or(Vec2::new(1440.0, 900.0));
        if screen.x < 35.0
            || screen.x > size.x - if session.player.details { 370.0 } else { 35.0 }
            || screen.y < 90.0
            || screen.y > size.y - 120.0
        {
            continue;
        }
        let mut location = Vec2::new(screen.x - 32.0, screen.y - 24.0);
        for _ in 0..20 {
            if !occupied
                .iter()
                .any(|p| (p.x - location.x).abs() < 68.0 && (p.y - location.y).abs() < 36.0)
            {
                break;
            }
            location.y -= 36.0;
        }
        occupied.push(location);
        node.display = Display::Flex;
        node.left = px(location.x);
        node.top = px(location.y);
        let ratio = unit.health_fraction().clamp(0.0, 1.0);
        for entity in descendants.iter() {
            if let Ok((kind, mut caption)) = captions.get_mut(entity) {
                let heal = feedback
                    .pulses
                    .iter()
                    .filter(|(e, _)| e.target == unit.id)
                    .map(|(e, _)| e.amount)
                    .sum::<f32>();
                **caption = if kind.0 {
                    if heal > 0.0 {
                        format!("+{heal:.0} HP")
                    } else {
                        String::new()
                    }
                } else {
                    format!("{:.0}/{:.0}", unit.health, unit.max_health())
                };
            }
            if let Ok(parts) = children.get(entity) {
                for part in parts.iter() {
                    if let Ok((mut fill, mut color)) = fills.get_mut(part) {
                        fill.width = percent(ratio * 100.0);
                        color.0 = if ratio < 0.3 {
                            Color::srgb(0.95, 0.15, 0.12)
                        } else if ratio < 0.6 {
                            Color::srgb(1.0, 0.65, 0.1)
                        } else {
                            Color::srgb(0.2, 0.85, 0.3)
                        };
                    }
                }
            }
        }
    }
}
