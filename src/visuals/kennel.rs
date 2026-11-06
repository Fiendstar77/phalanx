//! Authored, skinned war hounds. All behavior and consequences come from Battle.
use super::*;
const PATH: &str = "characters/war-hound-rigged.glb";
const CLIPS: [&str; 8] = [
    "Idle",
    "Walk",
    "Run",
    "Recall",
    "Bite",
    "Sniff",
    "Down",
    "LookAround",
];
const DURATIONS: [f32; 8] = [2.4, 0.8, 0.5, 0.55, 0.7, 2., 1.1, 2.8];
#[derive(Component)]
struct Hound(usize);
#[derive(Component)]
struct Animated {
    hound: usize,
    clip: usize,
    clock: f32,
    old: Option<(AnimationNodeIndex, f32, f32)>,
    position: Vec2,
    tick: u32,
    speed: f32,
}
#[derive(Resource)]
struct Kit {
    graph: Handle<AnimationGraph>,
    clips: Vec<AnimationNodeIndex>,
}
#[derive(Resource, Default)]
pub(super) struct Audit {
    pub loaded: usize,
    pub eye_bones: usize,
    pub skins: usize,
    pub poses: u64,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Audit>()
        .add_systems(Startup, build)
        .add_systems(Update, animate.after(super::super::sync_pawns));
    if std::env::args().any(|a| a == "--hound-witness") {
        app.init_resource::<Review>()
            .add_systems(Update, review.before(super::super::camera_motion));
    }
}
fn build(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let (graph, clips) = AnimationGraph::from_clips(
        (0..CLIPS.len()).map(|i| server.load(GltfAssetLabel::Animation(i).from_asset(PATH))),
    );
    commands.insert_resource(Kit {
        graph: graphs.add(graph),
        clips,
    });
    for id in 0..8 {
        commands
            .spawn((
                Hound(id),
                WorldAssetRoot(server.load(GltfAssetLabel::Scene(0).from_asset(PATH))),
                Transform::default(),
                Visibility::Hidden,
            ))
            .observe(ready);
    }
}
#[allow(clippy::too_many_arguments)]
fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    models: Query<&Hound>,
    children: Query<&Children>,
    names: Query<&Name>,
    skins: Query<&bevy::mesh::skinning::SkinnedMesh>,
    mut players: Query<&mut AnimationPlayer>,
    kit: Res<Kit>,
    mut audit: ResMut<Audit>,
) {
    let Ok(hound) = models.get(event.entity) else {
        return;
    };
    audit.loaded += 1;
    for e in children.iter_descendants(event.entity) {
        if names
            .get(e)
            .is_ok_and(|n| matches!(n.as_str(), "Eye.L" | "Eye.R"))
        {
            audit.eye_bones += 1;
        }
        if skins.contains(e) {
            audit.skins += 1;
        }
        if let Ok(mut player) = players.get_mut(e) {
            player.play(kit.clips[0]).repeat().pause();
            commands.entity(e).insert((
                AnimationGraphHandle(kit.graph.clone()),
                Animated {
                    hound: hound.0,
                    clip: 0,
                    clock: 0.,
                    old: None,
                    position: Vec2::ZERO,
                    tick: u32::MAX,
                    speed: 0.,
                },
            ));
        }
    }
}
#[derive(Clone, Default)]
struct Presented {
    previous: Vec2,
    current: Vec2,
    tick: u32,
    heading: Quat,
}
#[allow(clippy::too_many_arguments)]
fn animate(
    session: Res<Session>,
    time: Res<Time>,
    kit: Res<Kit>,
    mut roots: Query<(&Hound, &mut Transform, &mut Visibility)>,
    mut actors: Query<(&mut Animated, &mut AnimationPlayer)>,
    mut presented: Local<Vec<Presented>>,
    mut audit: ResMut<Audit>,
    review: Option<Res<Review>>,
) {
    presented.resize_with(8, Presented::default);
    let dt = if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else if session.battle.finished {
        time.delta_secs().min(0.1)
    } else {
        0.
    };
    for (hound, mut transform, mut visible) in &mut roots {
        let Some(a) = session.battle.kennel.actors.get(hound.0) else {
            *visible = Visibility::Hidden;
            continue;
        };
        *visible = Visibility::Inherited;
        let state = &mut presented[hound.0];
        let p = Vec2::from_array(a.position);
        if state.tick != session.battle.tick {
            state.previous = state.current;
            state.current = p;
            state.tick = session.battle.tick;
        }
        if state.previous.distance(p) > 2. {
            state.previous = p;
            state.current = p;
        }
        let p = if session.running {
            state
                .previous
                .lerp(p, (session.accumulator / phalanx::DT).clamp(0., 1.))
        } else {
            p
        };
        let direction = Vec3::new(a.goal[0] - p.x, 0., a.goal[1] - p.y);
        if direction.length_squared() > 0.01 && a.health > 0. {
            state.heading = state.heading.slerp(
                Quat::from_rotation_y(direction.x.atan2(direction.z)),
                1. - (-10. * dt).exp(),
            );
        }
        *transform = Transform::from_xyz(p.x, session.battle.arena.elevation(p.to_array()), p.y)
            .with_rotation(state.heading);
    }
    for (mut state, mut player) in &mut actors {
        let Some(a) = session.battle.kennel.actors.get(state.hound) else {
            continue;
        };
        let p = Vec2::from_array(a.position);
        if state.tick != session.battle.tick {
            state.speed = if state.tick == u32::MAX
                || state.tick > session.battle.tick
                || p.distance(state.position) > 2.
            {
                0.
            } else {
                p.distance(state.position)
                    / (session.battle.tick - state.tick).max(1) as f32
                    / phalanx::DT
            };
            state.position = p;
            state.tick = session.battle.tick;
        }
        let clip = if let Some(review) = &review {
            review.clip
        } else if a.health <= 0. {
            6
        } else if a.winding.is_some() {
            4
        } else if state.speed > 0.15 {
            if a.order == phalanx::kennel::Order::Recall {
                3
            } else if state.speed > 0.75 {
                2
            } else {
                1
            }
        } else if a.order == phalanx::kennel::Order::Scout {
            5
        } else if (session.battle.tick / 30 + state.hound as u32) % 4 == 0 {
            7
        } else {
            0
        };
        if state.clip != clip {
            let old_node = kit.clips[state.clip];
            let old_clock = state.clock;
            state.clock = if matches!(clip, 1..=3) && matches!(state.clip, 1..=3) {
                state.clock % DURATIONS[state.clip] / DURATIONS[state.clip] * DURATIONS[clip]
            } else {
                0.
            };
            state.old = Some((old_node, old_clock, 0.12));
            state.clip = clip;
            player.stop_all();
            player.play(old_node).repeat().pause();
            player.play(kit.clips[clip]).repeat().pause();
        } else {
            state.clock += dt
                * if matches!(clip, 1..=3) {
                    // Stance displacement / stance fraction gives distance per cycle.
                    // Distances match the baked paw trajectories at quarter scale.
                    (state.speed * DURATIONS[clip]
                        / [
                            0.,
                            0.60 * 0.25 / 0.65,
                            0.70 * 0.25 / 0.48,
                            0.80 * 0.25 / 0.40,
                        ][clip])
                        .clamp(0.25, 3.5)
                } else {
                    1.
                };
        }
        if let Some((_, end)) = a.winding {
            state.clock = ((session.battle.tick as f32 + session.accumulator / phalanx::DT
                - end.saturating_sub(6) as f32)
                * phalanx::DT)
                .clamp(0., DURATIONS[4] - 0.001);
        }
        if let Some(review) = &review {
            state.clock = review.clock;
            if let Some((node, _, _)) = state.old.take() {
                player.stop(node);
            }
        }
        let clock = if matches!(clip, 4 | 6) {
            state.clock.min(DURATIONS[clip] - 0.001)
        } else {
            state.clock % DURATIONS[clip]
        };
        let mut weight = 1.;
        if let Some((node, old_clock, left)) = &mut state.old {
            *left = (*left - dt).max(0.);
            let amount = *left / 0.12;
            weight = 1. - amount;
            if let Some(old) = player.animation_mut(*node) {
                old.set_weight(amount).set_seek_time(*old_clock);
            }
        }
        if let Some(active) = player.animation_mut(kit.clips[clip]) {
            active.set_weight(weight).set_seek_time(clock);
            audit.poses += 1;
        }
        if state.old.as_ref().is_some_and(|(_, _, left)| *left <= 0.) {
            if let Some((node, _, _)) = state.old.take() {
                player.stop(node);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_export_has_named_clips_weights_and_independent_eye_joints() {
        let raw = include_bytes!("../../assets/characters/war-hound-rigged.glb");
        let n = u32::from_le_bytes(raw[12..16].try_into().unwrap()) as usize;
        let doc: serde_json::Value = serde_json::from_slice(&raw[20..20 + n]).unwrap();
        let names: Vec<_> = doc["animations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, CLIPS);
        assert!(doc["skins"].as_array().is_some_and(|s| !s.is_empty()));
        for eye in ["Eye.L", "Eye.R"] {
            assert_eq!(
                doc["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|n| n["name"] == eye)
                    .count(),
                1
            );
        }
        assert!(doc["meshes"].as_array().unwrap().iter().all(|m| {
            m["primitives"].as_array().unwrap().iter().all(|p| {
                p["attributes"].get("WEIGHTS_0").is_some()
                    && p["attributes"].get("JOINTS_0").is_some()
            })
        }));
    }
}

#[derive(Resource, Default)]
struct Review {
    clip: usize,
    clock: f32,
    seconds: f32,
    stage: u8,
    battle: Option<String>,
}
#[allow(clippy::too_many_arguments)]
fn review(
    time: Res<Time>,
    mut review: ResMut<Review>,
    mut session: ResMut<Session>,
    mut commands: Commands,
    audit: Res<Audit>,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    if audit.loaded < 8 {
        review.seconds = 0.;
        return;
    }
    review.seconds += time.delta_secs().min(0.1);
    review.clock = if review.stage == 0 {
        review.seconds
    } else {
        (review.seconds - (4. + f32::from(review.stage - 1) * 3.)).max(0.)
    };
    review.clip = [0, 1, 4, 7, 7, 6, 6][usize::from(review.stage.min(6))];
    if review.stage == 2 {
        review.clock = 0.50;
    }
    if matches!(review.stage, 3 | 4) {
        review.clock = 0.70;
    }
    session.running = false;
    session.camera_follow = false;
    session.cinema_grabbed = true;
    session.cinema_on = false;
    if let Some(a) = session.battle.kennel.actors.first() {
        session.camera_target = Vec3::new(
            a.position[0],
            session.battle.arena.elevation(a.position) + 0.27,
            a.position[1],
        );
        session.zoom = if review.stage >= 4 { 0.68 } else { 1.35 };
        session.pitch = 0.12;
        session.orbit = if review.stage >= 4 { 0.05 } else { 0.50 };
    }
    if review.battle.is_none() {
        review.battle = Some(serde_json::to_string(&session.battle).unwrap());
    }
    if review.seconds < 4. + f32::from(review.stage) * 3. {
        return;
    }
    let directory = "outputs/war-hound";
    std::fs::create_dir_all(directory).unwrap();
    match review.stage {
        0 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!("{directory}/game-idle.png")));
        }
        1 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!("{directory}/game-walk.png")));
        }
        2 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!("{directory}/game-bite.png")));
        }
        3 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!("{directory}/game-gaze.png")));
        }
        4 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!("{directory}/game-eyes.png")));
        }
        5 => {
            std::fs::write(format!("{directory}/game-validation.json"),serde_json::to_vec_pretty(&serde_json::json!({"fixture":"authored asset pose review; animation clocks staged; no combat claims","loaded_models":audit.loaded,"eye_bones":audit.eye_bones,"skinned_meshes":audit.skins,"pose_frames":audit.poses,"clips":CLIPS,"battle_unchanged":review.battle.as_deref()==Some(serde_json::to_string(&session.battle).unwrap().as_str())})).unwrap()).unwrap();
        }
        _ => {
            assert_eq!(
                review.battle.as_deref(),
                Some(serde_json::to_string(&session.battle).unwrap().as_str())
            );
            exit.write(AppExit::Success);
        }
    }
    review.stage += 1;
}
