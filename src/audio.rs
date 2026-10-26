//! Local procedural score and bounded, event-driven synthetic combat foley.
mod director;
mod synth;
use super::Session;
use bevy::{
    audio::{AudioSinkPlayback, Volume},
    prelude::*,
};
use phalanx::FighterClass;
use serde::{Deserialize, Serialize};

pub struct GameAudio;
#[derive(Resource, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    music: bool,
    effects: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            music: true,
            effects: true,
        }
    }
}
#[derive(Component)]
struct Music {
    layer: usize,
    gain: f32,
}
#[derive(Message)]
pub(crate) enum Cue {
    Xp,
    Level,
}
#[derive(Component)]
struct Effect;
#[derive(Component)]
struct UiEffect;
#[derive(Component, Clone, Copy)]
enum Toggle {
    Music,
    Effects,
}
#[derive(Resource)]
struct Bank {
    effects: [Handle<AudioSource>; 27],
}
#[derive(Clone)]
struct Previous {
    health: f32,
    limbs: u8,
    cooldown: f32,
    hits: u32,
    shots: u32,
    healing_cooldown: f32,
    action: usize,
}
#[derive(Resource, Default)]
struct Events {
    run: Option<u32>,
    units: Vec<Previous>,
    last_tick: u32,
    finished: bool,
}
#[derive(Resource, Default)]
pub(crate) struct Audit {
    pub emitted: [usize; 27],
    pub music_peak: [f32; 6],
}

impl Plugin for GameAudio {
    fn build(&self, app: &mut App) {
        let settings = std::fs::read("outputs/audio-settings.json")
            .ok()
            .and_then(|data| serde_json::from_slice::<Settings>(&data).ok())
            .unwrap_or_default();
        app.insert_resource(settings)
            .init_resource::<Events>()
            .init_resource::<Audit>()
            .add_message::<Cue>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (controls, combat, cues, playback)
                    .chain()
                    .after(super::simulate),
            );
    }
}
fn silent_witness() -> bool {
    std::env::args().any(|a| a == "--visual-witness")
        && !std::env::args().any(|a| a == "--audible-witness")
}
fn setup(
    mut commands: Commands,
    session: Res<Session>,
    settings: Res<Settings>,
    mut assets: ResMut<Assets<AudioSource>>,
) {
    for layer in 0..6 {
        let score = assets.add(AudioSource {
            bytes: synth::stem(session.battle.seed, layer).into(),
        });
        commands.spawn((
            Music {
                layer,
                gain: if layer == 0 { 0.5 } else { 0. },
            },
            AudioPlayer::new(score),
            PlaybackSettings {
                volume: Volume::Linear(0.),
                muted: !settings.music || silent_witness(),
                ..PlaybackSettings::LOOP
            },
        ));
    }
    let effects = std::array::from_fn(|i| {
        assets.add(AudioSource {
            bytes: synth::effect(i).into(),
        })
    });
    commands.insert_resource(Bank { effects });
}

pub(crate) fn menu(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(10),
            ..default()
        })
        .with_children(|row| {
            for (toggle, title) in [
                (Toggle::Music, "Music ON [F8]"),
                (Toggle::Effects, "Effects ON [F9]"),
            ] {
                row.spawn((
                    Button,
                    toggle,
                    Node {
                        padding: UiRect::axes(px(12), px(7)),
                        border_radius: BorderRadius::all(px(5)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.18, 0.23)),
                ))
                .with_children(|p| {
                    p.spawn((Text::new(title), super::font(14.0)));
                });
            }
        });
}
fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<Settings>,
    buttons: Query<(&Interaction, &Toggle), Changed<Interaction>>,
    labels: Query<(&Toggle, &Children)>,
    mut texts: Query<&mut Text>,
) {
    let mut music = keys.just_pressed(KeyCode::F8);
    let mut effects = keys.just_pressed(KeyCode::F9);
    for (interaction, toggle) in &buttons {
        if *interaction == Interaction::Pressed {
            match toggle {
                Toggle::Music => music = true,
                Toggle::Effects => effects = true,
            }
        }
    }
    if music {
        settings.music = !settings.music;
    }
    if effects {
        settings.effects = !settings.effects;
    }
    if music || effects {
        match serde_json::to_vec_pretty(&*settings)
            .map_err(|e| e.to_string())
            .and_then(|data| {
                std::fs::write("outputs/audio-settings.json", data).map_err(|e| e.to_string())
            }) {
            Ok(()) => {}
            Err(error) => warn!("Could not save audio preferences: {error}"),
        }
    }
    for (toggle, children) in &labels {
        let (name, on, key) = match toggle {
            Toggle::Music => ("Music", settings.music, "F8"),
            Toggle::Effects => ("Effects", settings.effects, "F9"),
        };
        let label = format!("{name} {} [{key}]", if on { "ON" } else { "OFF" });
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child)
                && text.0 != label
            {
                text.0 = label.clone();
            }
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn combat(
    mut commands: Commands,
    session: Res<Session>,
    bank: Res<Bank>,
    settings: Res<Settings>,
    mut events: ResMut<Events>,
    active: Query<(), With<Effect>>,
    mut audit: ResMut<Audit>,
) {
    let snapshot = || {
        session
            .battle
            .units
            .iter()
            .map(|u| Previous {
                health: u.health,
                limbs: u.lost_limbs,
                cooldown: u.cooldown,
                hits: u.rocks_hit,
                shots: u.rocks_thrown,
                healing_cooldown: u.heal_cooldown,
                action: u.action,
            })
            .collect()
    };
    if events.run != Some(session.run_id) || events.units.len() != session.battle.units.len() {
        events.run = Some(session.run_id);
        events.units = snapshot();
        events.last_tick = session.battle.tick;
        events.finished = session.battle.finished;
        return;
    }
    if events.last_tick == session.battle.tick {
        return;
    }
    events.last_tick = session.battle.tick;
    let mut pending: Vec<(usize, usize)> = Vec::new();
    for unit in &session.battle.units {
        let before = &events.units[unit.id];
        let voice = usize::from(super::visuals::female(&session, unit.id));
        if session.battle.finished
            && !events.finished
            && unit.health > 0.0
            && session.battle.winner() == Some(unit.team)
        {
            pending.push((0, 20 + voice));
        }
        if unit.health > 0.0 && unit.action == 5 && before.action != 5 {
            pending.push((1, 18 + voice));
        }
        if unit.health <= 0.0 && before.health > 0.0 {
            pending.push((0, 22 + voice));
            pending.push((1, 8));
            pending.push((1, 14 + unit.id % 2));
        } else if unit.lost_limbs != before.limbs {
            pending.push((0, 22 + voice));
            pending.push((0, 3));
            pending.push((1, 12 + (session.battle.tick as usize + unit.id) % 4));
        } else if unit.health < before.health {
            let damage = before.health - unit.health;
            pending.push((
                1,
                if damage > 25.0 {
                    22 + voice
                } else {
                    16 + voice
                },
            ));
            pending.push((2, 12 + (session.battle.tick as usize + unit.id) % 4));
        }
        if unit.rocks_hit > before.hits {
            pending.push((2, 10));
        }
        if unit.health > 0.0 {
            if unit.heal_cooldown > before.healing_cooldown {
                pending.push((3, 11));
            }
            if unit.rocks_thrown > before.shots {
                pending.push((3, 9));
            } else if unit.cooldown > before.cooldown
                && !unit.is_ranged()
                && unit.action == 1
                && unit.heal_cooldown <= before.healing_cooldown
            {
                pending.push((
                    3,
                    if unit.class == Some(FighterClass::Berserker) {
                        1
                    } else {
                        0
                    },
                ));
            }
        }
    }

    events.units = snapshot();
    events.finished = session.battle.finished;
    if !settings.effects || silent_witness() || session.menu {
        return;
    }
    pending.sort_by_key(|p| p.0);
    // Bound the full mixer, not just one event category. No catch-up sound storm.
    let budget = 16_usize.saturating_sub(active.iter().count()).min(4);
    for (_, kind) in pending.into_iter().take(budget) {
        let n = audit.emitted[kind];
        commands.spawn((
            Effect,
            AudioPlayer::new(bank.effects[kind].clone()),
            PlaybackSettings {
                volume: Volume::Linear(0.30),
                speed: 0.94 + (n % 5) as f32 * 0.03,
                ..PlaybackSettings::DESPAWN
            },
        ));
        audit.emitted[kind] += 1;
    }
}
fn cues(
    mut commands: Commands,
    mut cues: MessageReader<Cue>,
    bank: Res<Bank>,
    settings: Res<Settings>,
    session: Res<Session>,
    mut previous: Local<[u32; 3]>,
    mut audit: ResMut<Audit>,
    active: Query<(), With<Effect>>,
) {
    let mut pending = Vec::new();
    for i in 0..3 {
        if previous[i] != session.founding.stamps[i] {
            previous[i] = session.founding.stamps[i];
            if previous[i] > 0 {
                pending.push(24);
            }
        }
    }
    for cue in cues.read() {
        pending.push(match cue {
            Cue::Xp => 25,
            Cue::Level => 26,
        });
    }
    if !settings.effects || silent_witness() {
        return;
    }
    for kind in pending
        .into_iter()
        .take(16_usize.saturating_sub(active.iter().count()).min(2))
    {
        commands.spawn((
            Effect,
            UiEffect,
            AudioPlayer::new(bank.effects[kind].clone()),
            PlaybackSettings {
                volume: Volume::Linear(0.28),
                ..PlaybackSettings::DESPAWN
            },
        ));
        audit.emitted[kind] += 1;
    }
}
fn playback(
    session: Res<Session>,
    settings: Res<Settings>,
    time: Res<Time>,
    mut audit: ResMut<Audit>,
    mut sinks: Query<(
        &mut AudioSink,
        Option<&mut Music>,
        Option<&Effect>,
        Option<&UiEffect>,
    )>,
) {
    let target = director::target(&session.battle, session.menu);
    for (mut sink, music, effect, ui_effect) in &mut sinks {
        if let Some(mut music) = music {
            let mut mix = [0.; 6];
            mix[music.layer] = music.gain;
            director::fade(&mut mix, target, time.delta_secs().min(0.1));
            music.gain = mix[music.layer];
            audit.music_peak[music.layer] = audit.music_peak[music.layer].max(music.gain);
            sink.set_volume(Volume::Linear(music.gain * 0.22));
            if settings.music && !silent_witness() {
                sink.unmute();
            } else {
                sink.mute();
            }
        }
        if effect.is_some() {
            if settings.effects {
                sink.unmute();
            } else {
                sink.mute();
            }
            if ui_effect.is_none()
                && (session.menu || (!session.running && !session.battle.finished))
            {
                sink.pause();
            } else {
                sink.play();
            }
        }
    }
}
pub(crate) fn export_witness(seed: u64) -> std::io::Result<()> {
    let directory = std::path::Path::new("outputs/audio-witness");
    std::fs::create_dir_all(directory)?;
    std::fs::write(directory.join("music.wav"), synth::music(seed))?;
    for (i, name) in director::NAMES.iter().enumerate() {
        std::fs::write(
            directory.join(format!("stem-{name}.wav")),
            synth::stem(seed, i),
        )?;
    }
    for (kind, name) in synth::NAMES.iter().enumerate() {
        std::fs::write(directory.join(format!("{name}.wav")), synth::effect(kind))?;
    }
    println!(
        "Exported action score, six stems and 27 synthesized effects to {}",
        directory.display()
    );
    Ok(())
}
