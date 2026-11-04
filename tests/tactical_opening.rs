use phalanx::{learned::Controller, progression::Campaign, tactics::Phase, terrain::Arena};
fn staged() -> phalanx::Battle {
    let mut c = Campaign::new();
    let arena = phalanx::pressure::generate(&c.teams, 42);
    c.begin_battle(42, arena, Controller::Utility).unwrap().1
}
#[test]
fn an_appointed_scout_moves_while_the_others_stage_and_reports_precede_commitment() {
    let mut b = staged();
    b.step();
    assert!(b.units.iter().all(|u| u.tactics.scout.is_some()));
    for team in 0..2 {
        let scouts: std::collections::BTreeSet<_> = b
            .units
            .iter()
            .filter(|u| u.team == team)
            .map(|u| u.tactics.scout)
            .collect();
        assert_eq!(scouts.len(), 1);
    }
    assert!(b.units.iter().any(|u| u.decision_reason.contains("scout")));
    assert!(b.damage_events.is_empty());
    let mut assembly = false;
    let mut engaged = false;
    while !b.finished && b.tick < 400 {
        b.step();
        assembly |= b
            .units
            .iter()
            .any(|u| u.tactics.phase == Phase::Assemble && u.tactics.first_report.is_some());
        engaged |= b.units.iter().any(|u| u.tactics.phase == Phase::Engage);
    }
    assert!(assembly);
    assert!(engaged);
    assert!(b.damage_events.iter().all(|e| e.tick > 0));
}
#[test]
fn unseen_opponent_changes_do_not_change_the_initial_recon_plan() {
    let mut a = staged();
    let mut b = a.clone();
    let half = b.arena.half_size();
    for enemy in b.units.iter_mut().filter(|u| u.team == 1) {
        enemy.position = [half[0] - 1., half[1] - 1. - enemy.id as f32 * 0.3];
        enemy.mind.aggression = 1.;
    }
    a.step();
    b.step();
    for i in 0..a.team_size() {
        assert!(a.units[i].contacts.is_empty() && b.units[i].contacts.is_empty());
        assert_eq!(a.units[i].tactics.scout, b.units[i].tactics.scout);
        assert_eq!(a.units[i].target, b.units[i].target);
        assert_eq!(a.units[i].decision_reason, b.units[i].decision_reason);
    }
}
#[test]
fn a_nearby_enemy_is_not_given_immunity_during_recon() {
    let mut c = Campaign::new();
    let (_, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    b.units[0].position = [0., 0.];
    b.units[3].position = [1., 0.];
    for _ in 0..40 {
        if b.finished {
            break;
        }
        b.step();
    }
    assert!(!b.damage_events.is_empty());
}
#[test]
fn cautious_fighters_can_decline_a_legal_wall_traversal() {
    let mut c = Campaign::new();
    let (_, mut b) = c
        .begin_battle(42, Arena::Choke, Controller::Utility)
        .unwrap();
    let u = &mut b.units[0];
    u.position = [-2., -5.];
    u.theus.as_mut().unwrap().sheet.attributes[1] = 14;
    let leap = phalanx::ninjutsu::wall_plan(u, b.arena, [4., -5.], 0).unwrap();
    u.mind = phalanx::styles::BattleStyle::Skirmisher.mind();
    u.mind.courage = 0.25;
    assert!(!phalanx::ninjutsu::wants_wall(u, &leap, b.arena));
    assert!(phalanx::ninjutsu::wall_plan(u, b.arena, [4., -5.], 0).is_some());
    u.mind = phalanx::styles::BattleStyle::Vanguard.mind();
    assert!(phalanx::ninjutsu::wants_wall(u, &leap, b.arena));
}
