use phalanx::{Battle, CommandTitle, terrain::Arena};
#[test]
fn every_starting_phalanx_has_king_queen_general_with_bounded_royal_bonus() {
    let b = Battle::with_founders(42, false, Arena::Ridge);
    assert_eq!(b.team_size(), 3);
    assert_eq!(b.units.len(), 6);
    for team in 0..2 {
        let units: Vec<_> = b.units.iter().filter(|u| u.team == team).collect();
        assert_eq!(
            units
                .iter()
                .map(|u| u.command_title.unwrap())
                .collect::<Vec<_>>(),
            vec![
                CommandTitle::King,
                CommandTitle::Queen,
                CommandTitle::General
            ]
        );
        for royal in &units[..2] {
            assert_eq!(royal.health, 330.0);
            assert_eq!(royal.health_fraction(), 1.0);
            assert_eq!(royal.damage_multiplier(), 1.1);
        }
        assert_eq!(units[2].max_health(), 300.0);
        assert_eq!(units[2].damage_multiplier(), 1.0);
    }
    let lab = Battle::new(42, false);
    assert_eq!(lab.units.len(), 10);
    assert!(
        lab.units
            .iter()
            .all(|u| u.command_title.is_none() && u.max_health() == 300.0)
    );
}
#[test]
fn royal_bonus_is_applied_to_actual_melee_and_delayed_rock_damage() {
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[0].health = 110.0;
    b.units[0].position = [0.0, 0.0];
    b.units[5].health = 100.0;
    b.units[5].position = [1.0, 0.0];
    b.units[5].cooldown = 100.0;
    b.step();
    assert!((b.units[5].health - (100.0 - 11.0 * 0.90)).abs() < 1e-5);
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[1].health = 110.0;
    b.units[1].position = [0.0, 0.0];
    b.units[5].health = 100.0;
    b.units[5].position = [4.0, 0.0];
    b.units[5].cooldown = 100.0;
    b.step();
    assert_eq!(b.units[5].health, 100.0);
    assert!((b.rocks[0].damage - 7.7).abs() < 1e-5);
}
#[test]
fn founders_replay_and_comparisons_remain_valid_on_all_arenas() {
    for arena in [Arena::Ridge, Arena::Choke, Arena::OpenField, Arena::Citadel] {
        let mut a = Battle::with_founders(42, false, arena);
        let mut b = a.clone();
        while !a.finished {
            a.step();
            b.step();
            for u in &a.units {
                assert!(u.health.is_finite());
                assert!(!arena.blocked(u.position));
            }
        }
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        assert_eq!(
            a.comparison_summary.compared + a.comparison_summary.unavailable,
            a.units.iter().flat_map(|u| u.decision_counts).sum::<u32>()
        );
    }
}
