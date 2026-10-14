use phalanx::{Battle, Mind};

#[test]
fn fixed_seed_replays_identically() {
    let mut a = Battle::new(42, false);
    let mut b = Battle::new(42, false);
    for _ in 0..1200 {
        a.step();
        b.step();
    }
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
    assert!(a.finished);
}

#[test]
fn personality_changes_the_opening_decision() {
    let mut aggressive = Battle::new(42, false);
    let mut cautious = aggressive.clone();
    cautious.units[0].mind = Mind::disciplined();
    aggressive.step();
    cautious.step();
    assert_eq!(aggressive.units[0].action, 0, "aggressive fighter advances");
    assert_eq!(
        cautious.units[0].action, 2,
        "disciplined fighter seeks the ridge"
    );
}

#[test]
fn fatal_attacks_resolve_simultaneously() {
    let mut battle = Battle::new(42, false);
    for unit in &mut battle.units {
        unit.health = 0.0;
    }
    for id in [0, 5] {
        battle.units[id].health = 5.0;
        battle.units[id].mind = Mind::aggressive();
        battle.units[id].position = [if id == 0 { -0.4 } else { 0.4 }, 0.0];
    }
    battle.step();
    assert_eq!(battle.units[0].health, 0.0);
    assert_eq!(battle.units[5].health, 0.0);
    assert_eq!(battle.result, "Draw");
}

#[test]
fn runs_remain_finite_and_finished_battles_do_not_change() {
    for seed in 0..20 {
        for swapped in [false, true] {
            let mut battle = Battle::new(seed, swapped);
            for _ in 0..1200 {
                battle.step();
            }
            assert!(battle.finished);
            for u in &battle.units {
                assert!((0.0..=100.0).contains(&u.health));
                assert!(u.position.iter().all(|p| p.is_finite()));
                assert!(u.scores.iter().all(|s| s.is_finite()));
            }
            let before = serde_json::to_string(&battle).unwrap();
            battle.step();
            assert_eq!(before, serde_json::to_string(&battle).unwrap());
        }
    }
}
