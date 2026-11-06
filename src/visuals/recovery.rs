//! Serial art witness of an actual seeded encounter. Only the starting profiles
//! and camera are authored; tactical phases, hits, healing and result resolve normally.
use super::*;
use phalanx::{
    FighterClass as C, learned::Controller, progression::Campaign, styles::BattleStyle,
    tactics::Kind,
};

pub(super) fn install(app: &mut App) {
    if std::env::args().any(|a| a == "--recovery-demo") {
        app.add_systems(Startup, prepare.after(load_visuals))
            .add_systems(
                Update,
                review
                    .after(super::super::settle_progression)
                    .after(animate),
            );
    }
}
fn prepare(mut session: ResMut<Session>) {
    session.autosave = false;
    session.campaign = Campaign::new();
    for (team, squad) in session.campaign.teams.iter_mut().enumerate() {
        for (slot, f) in squad.fighters.iter_mut().enumerate() {
            f.class = [C::Warrior, C::Scout, C::Healer][slot];
            f.mind = BattleStyle::ALL[(slot + team) % 3].mind();
        }
    }
    let seed = session.battle.seed;
    let arena = phalanx::pressure::generate(&session.campaign.teams, seed);
    session.restart_founders(seed, arena, Controller::Utility);
    session.founding.open = false;
    session.menu = false;
    session.map_briefing = false;
    session.running = false;
    session.speed = 1.;
    session.cinema_on = false;
    session.cinema_grabbed = true;
    session.camera_follow = false;
    session.camera_target = Vec3::new(0., 0.8, 0.);
    session.zoom = 23.;
    session.pitch = 0.72;
    session.orbit = 0.2;
    session.player.details = false;
    session.player.research = false;
}
#[derive(Default)]
struct Review {
    seconds: f32,
    started: bool,
    next: f32,
    frame: u32,
    sequence: Option<u64>,
    shots: [bool; 3],
    ended: Option<f32>,
    report_captured: bool,
    rendered_through: u32,
    rendered_events: Vec<phalanx::tactics::Event>,
}
fn screenshot(commands: &mut Commands, path: String) -> Entity {
    commands
        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(path))
        .id()
}
fn review(
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut state: Local<Review>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    state.seconds += time.delta_secs();
    if state.seconds > 5. && !state.started {
        state.started = true;
        state.next = state.seconds;
        session.running = true;
        std::fs::create_dir_all("outputs/recovery-demo").unwrap();
    }
    if state.started && state.ended.is_none() {
        let mut capture = None;
        if state.seconds >= state.next {
            capture = Some(screenshot(
                &mut commands,
                format!("outputs/recovery-demo/{:05}.png", state.frame),
            ));
            state.frame += 1;
            state.next += 1. / 12.;
        }
        for event in &session.battle.tactical_events {
            if state.sequence.is_some_and(|s| event.sequence <= s) {
                continue;
            }
            state.sequence = Some(event.sequence);
            let shot = match event.kind {
                Kind::ScreenPositionReached => Some((0, "screen")),
                Kind::TreatmentReceived => Some((1, "treatment")),
                Kind::ReapproachChosen => Some((2, "reapproach")),
                _ => None,
            };
            if let Some((index, name)) = shot
                && !state.shots[index]
            {
                state.shots[index] = true;
                let path = format!("outputs/recovery-demo/{name}.png");
                if let Some(entity) = capture {
                    commands
                        .entity(entity)
                        .observe(bevy::render::view::screenshot::save_to_disk(path));
                } else {
                    capture = Some(screenshot(&mut commands, path));
                }
            }
        }
        if session.battle.tick >= 350 || session.battle.finished {
            state.rendered_through = session.battle.tick;
            state.rendered_events = session.battle.tactical_events.clone();
            session.running = false;
            // Finish the remaining bout through the same simulation/recorder,
            // then capture its real settlement report. No outcome is assigned.
            while !session.battle.finished {
                session.study_step();
            }
            session.settle_finished();
            session.camp_page = Some(super::super::camp::Page::Report);
            session.menu = true;
            state.ended = Some(state.seconds);
        }
    }
    if let Some(ended) = state.ended {
        if state.seconds - ended > 1.5 && !state.report_captured {
            state.report_captured = true;
            screenshot(&mut commands, "outputs/recovery-demo/report.png".into());
        }
        if state.seconds - ended > 3. {
            let audit = serde_json::json!({"scripted_starting_profiles":true,"human_playtest":false,"real_combat":true,
                "autosave":session.autosave,"seed":session.battle.seed,"frames":state.frame,"requested_capture_fps":12,
                "rendered_through_tick":state.rendered_through,"final_tick":session.battle.tick,
                "remaining_combat_simulated_after_capture":true,"key_shots":state.shots,
                "rendered_tactical_events":state.rendered_events,"report":session.campaign.last_report});
            std::fs::write(
                "outputs/recovery-demo/audit.json",
                serde_json::to_vec_pretty(&audit).unwrap(),
            )
            .unwrap();
            println!(
                "RECOVERY_REVIEW frames={} rendered_through={} final_tick={} shots={:?}",
                state.frame, state.rendered_through, session.battle.tick, state.shots
            );
            exit.write(AppExit::Success);
        }
    }
}
