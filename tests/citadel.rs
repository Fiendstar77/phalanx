use phalanx::{Battle, Role, learned::Controller, navigation::plan_known, terrain::Arena};
#[test]
fn citadel_has_five_times_the_footprint_and_requested_rosters() {
    let a = Arena::Citadel.half_size();
    let b = Arena::Ridge.half_size();
    assert_eq!(a[0] * a[1], 5.0 * b[0] * b[1]);
    let battle = Battle::with_arena(42, false, Arena::Citadel);
    for team in 0..2 {
        let units: Vec<_> = battle.units.iter().filter(|u| u.team == team).collect();
        assert_eq!(units.len(), 10);
        assert_eq!(units.iter().filter(|u| u.role == Role::Fighter).count(), 6);
        assert_eq!(
            units.iter().filter(|u| u.role == Role::RockThrower).count(),
            4
        );
        assert!(units.iter().all(|u| !Arena::Citadel.blocked(u.position)));
    }
    assert!(Arena::Citadel.elevation([-6.0, -9.0]) > 3.0);
    assert!(Arena::Citadel.blocked([0.0, 0.0]));
    assert!(!Arena::Citadel.blocked([0.0, 6.0]));
    let route = plan_known(
        Arena::Citadel,
        [-18.0, 0.0],
        [18.0, 0.0],
        battle.units[0].mind,
        &[],
    );
    assert!(!route.points.is_empty());
    assert!(
        route
            .points
            .windows(2)
            .all(|p| Arena::Citadel.clear_segment(p[0], p[1]))
    );
    assert!(route.points.iter().any(|p| p[1].abs() > 4.0));
}
#[test]
fn twenty_agent_battles_respect_walls_and_finish_in_both_controller_modes() {
    for controller in [Controller::Utility, Controller::Learned] {
        let mut b = Battle::with_arena(42, false, Arena::Citadel);
        b.controller = controller;
        while !b.finished {
            b.step();
            for u in &b.units {
                assert!(u.position.iter().all(|v| v.is_finite()));
                assert!(!b.arena.blocked(u.position), "{} {:?}", u.id, u.position);
                if let Some(c) = &u.comparison {
                    assert!(u.scores[c.executed] >= 0.0);
                }
            }
        }
        assert!(b.tick <= phalanx::BATTLE_TICK_LIMIT);
        if controller == Controller::Utility {
            assert!(
                b.tick < phalanx::BATTLE_TICK_LIMIT,
                "seed-42 search must not stall until timeout"
            );
        }
        assert_eq!(b.learned_fallbacks, 0);
    }
}
#[test]
fn live_pair_filter_excludes_dead_and_superseded_decisions() {
    let mut b = Battle::with_arena(42, false, Arena::Choke);
    let mut witness = None;
    for _ in 0..400 {
        b.step();
        witness = b.units.iter().find_map(|u| {
            u.comparison
                .as_ref()
                .filter(|c| c.disagreed && u.health > 0.0)
                .and_then(|c| c.learned.map(|learned| (u.id, c.utility, learned)))
        });
        if witness.is_some() {
            break;
        }
    }
    let (id, utility, learned) = witness.expect("actual disagreement witness");
    assert!(b.disagreement_units(utility, learned).contains(&id));
    assert!(b.disagreement_units(utility, utility).is_empty());
    let mut dead = b.clone();
    dead.units[id].health = 0.0;
    assert!(!dead.disagreement_units(utility, learned).contains(&id));
    b.units[id].comparison = None;
    assert!(!b.disagreement_units(utility, learned).contains(&id));
}

#[test]
fn empty_deployment_search_continues_and_passarelles_require_ramps() {
    let arena = Arena::Citadel;
    assert!(arena.walk_segment([-12.0, 6.0], [-8.0, 6.0]));
    assert!(arena.walk_segment([-8.0, 6.0], [8.0, 6.0]));
    assert!(arena.walk_segment([8.0, 6.0], [12.0, 6.0]));
    assert!(!arena.walk_segment([0.0, 4.5], [0.0, 6.0]));
    let mut b = Battle::with_arena(42, false, arena);
    // Observer-independent search: no enemies in sight and an already reached deployment waypoint.
    b.units[0].position = arena.spawn(1, 0);
    for u in b.units.iter_mut().filter(|u| u.team == 1) {
        u.health = 0.0;
    }
    b.tick = 200;
    b.step();
    assert_eq!(b.units[0].search_waypoint, 1);
    assert_ne!(b.units[0].target, arena.spawn(1, 0));
    assert_eq!(b.units[0].scores[2], -1.0);
}
