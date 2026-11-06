use phalanx::{
    Battle, FighterClass as C,
    learned::Controller,
    progression::Campaign,
    styles::BattleStyle,
    tactics::{self, Cause, Kind, Phase},
};

fn encounter(seed: u64, four: bool) -> (Campaign, u64, Battle) {
    let mut c = Campaign::new();
    if four {
        c.teams[0].gold = 250;
        c.unlock_fourth().unwrap();
    }
    for (team, squad) in c.teams.iter_mut().enumerate() {
        for (slot, f) in squad.fighters.iter_mut().enumerate() {
            f.class = [C::Warrior, C::Scout, C::Healer, C::Guardian][slot];
            f.mind = BattleStyle::ALL[(slot + team) % 3].mind();
        }
    }
    let arena = phalanx::pressure::generate(&c.teams, seed);
    let (id, b) = c.begin_battle(seed, arena, Controller::Utility).unwrap();
    (c, id, b)
}

#[test]
fn real_battle_screens_treats_and_changes_approach_with_receipts_and_exact_replay() {
    let (mut campaign, id, mut b) = encounter(42, false);
    let mut replay = b.clone();
    let mut screen = false;
    let mut treatment = false;
    let mut resumed = false;
    while !b.finished {
        let tick = b.tick;
        b.step();
        for e in b.tactical_events.iter().filter(|e| e.tick == tick) {
            assert!(e.valid(b.units.len()));
            match e.kind {
                Kind::RallyReached => assert!(phalanx::distance(e.position, e.goal) < 1.4),
                Kind::ScreenPositionReached => {
                    let p = &b.units[e.subject.unwrap()];
                    assert!(phalanx::distance(e.position, e.goal) < 1.2);
                    assert!(
                        phalanx::distance(e.position, p.position)
                            < phalanx::perception::COMMUNICATION_RANGE + 0.5
                    );
                    screen = true;
                }
                Kind::TreatmentReceived => {
                    assert!(b.heals.iter().any(|h| h.tick == e.tick
                        && h.target == e.unit
                        && Some(h.healer) == e.subject
                        && h.amount == e.amount));
                    assert!(e.amount > 0.);
                    treatment = true;
                }
                Kind::Reengaged if e.cause == Cause::RouteReached => {
                    assert!(phalanx::distance(e.position, e.goal) < 1.2);
                    resumed = true;
                }
                _ => {}
            }
        }
    }
    assert!(screen && treatment && resumed);
    assert!(
        b.tactical_events
            .iter()
            .any(|e| e.kind == Kind::ApproachAbandoned)
    );
    let failed = b
        .tactical_events
        .iter()
        .find(|e| e.kind == Kind::ApproachAbandoned)
        .unwrap();
    let changed = b
        .tactical_events
        .iter()
        .find(|e| e.unit == failed.unit && e.kind == Kind::ReapproachChosen)
        .unwrap();
    assert!(phalanx::distance(failed.position, changed.goal) > 3.);
    while !replay.finished {
        replay.step();
    }
    assert_eq!(
        serde_json::to_value(&b).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
    campaign.settle(id, &b).unwrap();
    let restored: Campaign =
        serde_json::from_str(&serde_json::to_string(&campaign).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(
        restored.last_report.unwrap().tactical_events,
        b.tactical_events
    );
}

#[test]
fn unknown_opponents_do_not_change_a_recovery_plan_and_distant_allies_do_not_receive_it() {
    let (_, _, mut a) = encounter(42, false);
    while !a.finished && !a.units.iter().any(|u| tactics::recovering(&u.tactics)) {
        a.step();
    }
    assert!(!a.finished);
    let patient = a
        .units
        .iter()
        .find(|u| tactics::recovering(&u.tactics) && u.tactics.patient == Some(u.id))
        .unwrap()
        .id;
    // Observe the same known local situation in both worlds. Hidden enemy state differs.
    let mut b = a.clone();
    for world in [&mut a, &mut b] {
        let team_size = world.team_size();
        for u in world
            .units
            .iter_mut()
            .filter(|u| u.team != patient / team_size)
        {
            u.position = [
                world.arena.half_size()[0] - 1.,
                world.arena.half_size()[1] - 1.,
            ];
        }
    }
    for u in b.units.iter_mut().filter(|u| u.team != patient / 3) {
        u.health *= 0.8;
        u.mind.courage = 0.01;
        u.position[1] -= 1.;
    }
    a.step();
    b.step();
    assert!(a.units[patient].contacts.iter().all(|c| !c.visible));
    assert_eq!(a.units[patient].target, b.units[patient].target);
    assert_eq!(
        a.units[patient].decision_reason,
        b.units[patient].decision_reason
    );
    assert_eq!(
        a.units[patient].tactics.phase,
        b.units[patient].tactics.phase
    );
    let (_, _, mut far) = encounter(42, false);
    far.units[0].tactics.phase = Phase::Withdraw;
    far.units[0].tactics.rally = Some([-12., -8.]);
    far.units[0].tactics.patient = Some(0);
    far.units[0].tactics.since = 100;
    far.units[0].position = [-12., -8.];
    far.units[1].position = [10., 8.];
    far.units[1].tactics.phase = Phase::Engage;
    far.tick = 100;
    far.step();
    assert!(
        !far.tactical_events
            .iter()
            .any(|e| e.unit == 1 && e.kind == Kind::WithdrawalJoined)
    );
}

#[test]
fn both_playable_team_sizes_finish_without_unbounded_replans_or_illegal_goals() {
    for four in [false, true] {
        for seed in [42, 43, 44] {
            let (_, _, mut b) = encounter(seed, four);
            while !b.finished {
                b.step();
                for u in &b.units {
                    assert!(u.tactics.approach <= 2);
                    assert!(!b.arena.blocked(u.position), "fighter entered solid ground");
                    if tactics::manoeuvring(&u.tactics) {
                        let half = b.arena.half_size();
                        assert!(u.target.iter().all(|x| x.is_finite()));
                        assert!(u.target[0].abs() < half[0] && u.target[1].abs() < half[1]);
                    }
                }
                assert!(b.tactical_events.len() <= 128);
            }
            assert!(
                b.tick < phalanx::BATTLE_TICK_LIMIT,
                "recovery caused a timeout in {seed}/{four}"
            );
            for u in &b.units {
                assert!(
                    b.tactical_events
                        .iter()
                        .filter(|e| e.unit == u.id && e.kind == Kind::ApproachAbandoned)
                        .count()
                        <= 2
                );
            }
        }
    }
}

#[test]
fn recording_tactical_events_preserves_actual_combat_and_source_healing_receipts() {
    let (_, _, mut b) = encounter(42, false);
    let mut reference = b.clone();
    let dir = std::env::temp_dir().join(format!("phalanx-recovery-study-{}", std::process::id()));
    let mut recorder = phalanx::study::Recorder::start(&dir, &b).unwrap();
    while !b.finished {
        recorder.step(&mut b).unwrap();
        reference.step();
    }
    assert_eq!(
        serde_json::to_value(&b).unwrap(),
        serde_json::to_value(&reference).unwrap()
    );
    let lines = std::fs::read_to_string(recorder.directory.join("decisions.jsonl")).unwrap();
    let receipts: Vec<_> = lines
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .filter(|v| v["event"] == "tactical")
        .map(|v| v["receipt"].clone())
        .collect();
    assert_eq!(
        receipts,
        serde_json::to_value(&b.tactical_events)
            .unwrap()
            .as_array()
            .unwrap()
            .clone()
    );
    std::fs::remove_dir_all(dir).unwrap();
}
