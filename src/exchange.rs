//! In-game file exchange with a reversible, isolated challenge session.
use super::*;
use phalanx::challenges::{Challenge, ChallengeResult, TeamSnapshot};
use std::path::{Path, PathBuf};

pub struct Exchange;
impl Plugin for Exchange {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, dropped_files.after(controls));
    }
}
pub struct Live {
    challenge: Challenge,
    result: Option<ChallengeResult>,
    original_campaign: Campaign,
    original_battle: Battle,
    original_id: Option<u64>,
    original_rewarded: bool,
}
pub struct State {
    pub pending: Option<TeamSnapshot>,
    pub files: Vec<PathBuf>,
    pub seed: u64,
    pub live: Option<Live>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            pending: None,
            files: Vec::new(),
            seed: 42,
            live: None,
        }
    }
}
fn available(session: &Session) -> Result<(), String> {
    if session.exchange.live.is_some() {
        Err("Return to your campaign before importing or exporting another team".into())
    } else if !session.battle.founders || (session.battle.tick > 0 && !session.battle.finished) {
        Err("Finish the current battle before exchanging teams".into())
    } else {
        Ok(())
    }
}
pub fn refresh_files(session: &mut Session) {
    let path = Path::new("outputs/challenges/inbox");
    let result = std::fs::create_dir_all(path).and_then(|()| std::fs::read_dir(path));
    match result {
        Ok(entries) => {
            let mut files = entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "json") && p.is_file())
                .collect::<Vec<_>>();
            files.sort();
            files.truncate(12);
            session.exchange.files = files;
        }
        Err(e) => session.save_notice = format!("Cannot read snapshot inbox: {e}"),
    }
}
pub fn import_file(session: &mut Session, path: &Path) {
    let result = available(session).and_then(|()| {
        match TeamSnapshot::load(path) {
            Ok(snapshot) => {
                if snapshot.team.fighters.len() != session.campaign.teams[0].fighters.len() {
                    return Err(format!("League mismatch: imported team has {} fighters; your squad has {}", snapshot.team.fighters.len(), session.campaign.teams[0].fighters.len()));
                }
                session.exchange.pending = Some(snapshot);
                Ok(format!("Imported {}. Review the team, then choose Fight imported Phalanx.", path.display()))
            }
            Err(snapshot_error) => match ChallengeResult::load(path) {
                Ok(result) => {
                    result.verify()?;
                    Ok(format!("Verified result: {} versus {}, seed {}. The complete replay matches. Campaign unchanged.", result.challenge.teams[0].team.name, result.challenge.teams[1].team.name, result.challenge.seed))
                }
                Err(_) => Err(snapshot_error),
            },
        }
    });
    session.save_notice = result.unwrap_or_else(|e| format!("Import rejected: {e}"));
}
fn output_path(kind: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    PathBuf::from(format!("outputs/challenges/outbox/{kind}-{stamp}.json"))
}
pub fn export_team(session: &mut Session) {
    let result = available(session).and_then(|()| {
        let snapshot = TeamSnapshot::from_campaign(&session.campaign, 0)?;
        let path = output_path("phalanx");
        snapshot.save(&path)?;
        Ok(format!(
            "Exported {} to {}",
            snapshot.team.name,
            path.display()
        ))
    });
    session.save_notice = result.unwrap_or_else(|e| format!("Export failed: {e}"));
}
pub fn start(session: &mut Session) {
    let result = available(session).and_then(|()| {
        let opponent = session
            .exchange
            .pending
            .clone()
            .ok_or("Import an opponent snapshot first")?;
        let home = TeamSnapshot::from_campaign(&session.campaign, 0)?;
        let challenge = Challenge::new(home, opponent, session.exchange.seed)?;
        let (campaign, battle) = challenge.start()?;
        let original_campaign = std::mem::replace(&mut session.campaign, campaign);
        let original_battle = std::mem::replace(&mut session.battle, battle);
        session.exchange.live = Some(Live {
            challenge,
            result: None,
            original_campaign,
            original_battle,
            original_id: session.active_battle_id,
            original_rewarded: session.rewarded,
        });
        session.active_battle_id = None;
        session.rewarded = false;
        reset_view(session);
        session.menu = false;
        session.camp_page = None;
        Ok("Frozen challenge ready. Press Run to fight; your campaign is preserved.".into())
    });
    session.save_notice = result.unwrap_or_else(|e| format!("Challenge rejected: {e}"));
}
pub fn reset(session: &mut Session) {
    let Some(live) = &session.exchange.live else {
        return;
    };
    match live.challenge.start() {
        Ok((campaign, battle)) => {
            session.campaign = campaign;
            session.battle = battle;
            session.exchange.live.as_mut().unwrap().result = None;
            session.rewarded = false;
            reset_view(session);
            session.save_notice = "Challenge restarted from the same frozen teams and seed".into();
        }
        Err(e) => session.save_notice = e,
    }
}
pub fn finish(session: &mut Session) {
    if !session.battle.finished || session.rewarded {
        return;
    }
    let Some(live) = &mut session.exchange.live else {
        return;
    };
    match ChallengeResult::capture(live.challenge.clone(), &session.battle) {
        Ok(result) => {
            live.result = Some(result);
            session.save_notice = "Challenge finished. Export its result or return to your campaign. No progression was applied.".into();
        }
        Err(e) => session.save_notice = format!("Challenge result rejected: {e}"),
    }
    session.rewarded = true;
    session.running = false;
    session.menu = true;
    session.camp_page = Some(camp::Page::Challenges);
}
pub fn export_result(session: &mut Session) {
    let result = (|| {
        let packet = session
            .exchange
            .live
            .as_ref()
            .and_then(|l| l.result.as_ref())
            .ok_or("Finish a snapshot challenge first")?;
        let path = output_path("result");
        packet.save(&path)?;
        Ok::<_, String>(format!(
            "Exported reproducible battle result to {}",
            path.display()
        ))
    })();
    session.save_notice = result.unwrap_or_else(|e| format!("Export failed: {e}"));
}
pub fn leave(session: &mut Session) {
    let Some(live) = session.exchange.live.take() else {
        session.save_notice = "Your local campaign is already active".into();
        return;
    };
    session.campaign = live.original_campaign;
    session.battle = live.original_battle;
    session.active_battle_id = live.original_id;
    session.rewarded = live.original_rewarded;
    reset_view(session);
    session.menu = true;
    session.camp_page = Some(camp::Page::Challenges);
    session.save_notice = "Returned to your campaign. Fighters, rivals, resources and the previous report are preserved.".into();
}
fn reset_view(session: &mut Session) {
    session.running = false;
    session.accumulator = 0.;
    session.selected = 0;
    session.previous_positions = session.battle.units.iter().map(|u| u.position).collect();
    session.disagreement_pair = None;
    session.camera_follow = false;
    session.camera_target = Vec3::new(3.5, 0., 0.);
    session.pitch = 0.72;
    session.zoom = frame_zoom(session.battle.arena);
    session.logged_disagreements = 0;
    session.run_id += 1;
    session.refresh_preview();
}
pub fn summary(session: &Session) -> String {
    if let Some(live) = &session.exchange.live {
        let outcome = session
            .battle
            .winner()
            .map(|team| format!("{} wins", live.challenge.teams[team].team.name))
            .unwrap_or_else(|| "Draw".into());
        let mut text = format!(
            "FROZEN EXHIBITION | {} vs {} | Seed {}\n{}\n\nCampaign progression is preserved. Both teams use their published loadouts.\n\n",
            live.challenge.teams[0].team.name,
            live.challenge.teams[1].team.name,
            live.challenge.seed,
            if session.battle.finished {
                outcome.as_str()
            } else {
                "Return to arena, then press Run. Reset replays the same challenge."
            }
        );
        if session.battle.finished {
            for unit in &session.battle.units {
                let fighter = &live.challenge.teams[unit.team].team.fighters
                    [unit.id % session.battle.team_size()];
                text.push_str(&format!(
                    "{} / {} | HP {:.0}/{:.0} | damage {:.0} | healing {:.0}\n",
                    live.challenge.teams[unit.team].team.name,
                    fighter.name,
                    unit.health,
                    unit.max_health(),
                    unit.damage_dealt,
                    unit.healing_done
                ));
            }
            text.push_str("\nExport battle result to share this match. Its recipient can replay and verify it.\n");
        }
        return text;
    }
    format!(
        "YOUR PHALANX | {} | League {}\nExport your team, then exchange the file with another player.\nChallenge seed: {}\n\n{}",
        session.campaign.teams[0].name,
        session.campaign.teams[0].fighters.len(),
        session.exchange.seed,
        session
            .exchange
            .pending
            .as_ref()
            .map(|s| format!("IMPORTED OPPONENT\n{}", s.summary()))
            .unwrap_or_else(|| {
                "No imported opponent yet. Drop a team file here or choose one from the inbox."
                    .into()
            })
    )
}
fn dropped_files(mut files: MessageReader<FileDragAndDrop>, mut session: ResMut<Session>) {
    for event in files.read() {
        if session.camp_page == Some(camp::Page::Challenges)
            && let FileDragAndDrop::DroppedFile { path_buf, .. } = event
        {
            import_file(&mut session, path_buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        let mut session = Session::new(Battle::new(42, false));
        session.autosave = false;
        session.campaign = Campaign::new();
        session.restart_founders(42, Arena::OpenField, Controller::Utility);
        session.exchange.pending = Some(TeamSnapshot::from_campaign(&session.campaign, 1).unwrap());
        session
    }
    #[test]
    fn frozen_game_controls_and_workshop_cannot_mutate_either_snapshot_or_save() {
        let mut session = session();
        let original_campaign = serde_json::to_string(&session.campaign).unwrap();
        let original_battle = serde_json::to_string(&session.battle).unwrap();
        apply(Control::StartChallenge, &mut session);
        assert!(session.exchange.live.is_some());
        let frozen_campaign = serde_json::to_string(&session.campaign).unwrap();
        let frozen_battle = serde_json::to_string(&session.battle).unwrap();
        for control in [
            Control::Class(FighterClass::Healer),
            Control::Title(CommandTitle::Champion),
            Control::Trait(0, 0.1),
            Control::Swap,
            Control::Arena,
            Control::Setup,
            Control::Load,
            Control::Save,
            Control::Recruit,
            Control::NextBattle,
            Control::ChooseRival(0),
            Control::Controller,
            Control::Appearance(0),
            Control::TeamColor(2),
        ] {
            apply(control, &mut session);
        }
        rpg::apply(
            rpg::Action::Craft(phalanx::equipment::Gear::BalancedWeapon),
            &mut session,
        );
        assert!(!session.save_campaign());
        assert_eq!(
            serde_json::to_string(&session.campaign).unwrap(),
            frozen_campaign
        );
        assert_eq!(
            serde_json::to_string(&session.battle).unwrap(),
            frozen_battle
        );
        apply(Control::Step, &mut session);
        assert_eq!(session.battle.tick, 1);
        apply(Control::Reset, &mut session);
        assert_eq!(
            serde_json::to_string(&session.battle).unwrap(),
            frozen_battle
        );
        while !session.battle.finished {
            session.battle.step();
        }
        session.settle_finished();
        session
            .exchange
            .live
            .as_ref()
            .unwrap()
            .result
            .as_ref()
            .unwrap()
            .verify()
            .unwrap();
        assert_eq!(
            serde_json::to_string(&session.campaign).unwrap(),
            frozen_campaign
        );
        assert_eq!(session.camp_page, Some(camp::Page::Challenges));
        apply(Control::LeaveChallenge, &mut session);
        assert!(session.exchange.live.is_none());
        assert_eq!(
            serde_json::to_string(&session.campaign).unwrap(),
            original_campaign
        );
        assert_eq!(
            serde_json::to_string(&session.battle).unwrap(),
            original_battle
        );
    }
    #[test]
    fn mid_battle_challenge_entry_is_rejected_without_discarding_the_local_match() {
        let mut session = session();
        session.battle.step();
        let original = serde_json::to_string(&session.battle).unwrap();
        apply(Control::StartChallenge, &mut session);
        assert!(session.exchange.live.is_none());
        assert_eq!(serde_json::to_string(&session.battle).unwrap(), original);
    }

    #[test]
    fn game_button_dispatcher_imports_a_file_and_starts_the_frozen_match() {
        let mut session = session();
        let snapshot = session.exchange.pending.take().unwrap();
        let path = std::env::temp_dir().join(format!(
            "phalanx-ui-snapshot-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        snapshot.save(&path).unwrap();
        session.exchange.files = vec![path.clone()];
        let mut app = App::new();
        app.insert_resource(session).add_systems(Update, controls);
        app.world_mut().spawn((
            Control::ImportSnapshot(0),
            Interaction::Pressed,
            BackgroundColor::default(),
            PressLatch::default(),
        ));
        app.update();
        assert_eq!(
            app.world()
                .resource::<Session>()
                .exchange
                .pending
                .as_ref()
                .unwrap()
                .team
                .name,
            snapshot.team.name
        );
        app.world_mut().spawn((
            Control::StartChallenge,
            Interaction::Pressed,
            BackgroundColor::default(),
            PressLatch::default(),
        ));
        app.update();
        assert!(app.world().resource::<Session>().exchange.live.is_some());
        app.world_mut().spawn((
            Control::LeaveChallenge,
            Interaction::Pressed,
            BackgroundColor::default(),
            PressLatch::default(),
        ));
        app.update();
        assert!(app.world().resource::<Session>().exchange.live.is_none());
        std::fs::remove_file(path).unwrap();
    }
}
