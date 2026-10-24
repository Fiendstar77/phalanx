use phalanx::{
    Battle, learned::Controller, pressure, progression::Campaign, styles::BattleStyle,
    terrain::Arena, tutorial,
};
#[test]
fn legacy_campaigns_start_a_curriculum_without_losing_fighters_or_granting_slots() {
    let c = Campaign::new();
    let mut json = serde_json::to_value(&c).unwrap();
    json.as_object_mut().unwrap().remove("tutorial");
    let mut loaded: Campaign = serde_json::from_value(json).unwrap();
    let before = serde_json::to_string(&loaded.teams[0]).unwrap();
    let lesson = tutorial::prepare_next(&mut loaded).unwrap();
    assert_eq!(lesson.rival, tutorial::LESSONS[0].rival);
    assert_eq!(serde_json::to_string(&loaded.teams[0]).unwrap(), before);
    assert_eq!(loaded.teams[0].fighters.len(), 3);
    assert!(loaded.unlock_fourth().is_err());
    assert_eq!(loaded.tutorial.completed(), 0);
    loaded.validate().unwrap();
}
#[test]
fn tutorial_attempts_and_real_victories_settle_once_and_survive_disk() {
    let mut wins = 0;
    for seed in 0..12 {
        let mut c = Campaign::new();
        tutorial::prepare_next(&mut c).unwrap();
        let arena = pressure::generate(&c.teams, seed);
        let (id, mut b) = c.begin_battle(seed, arena, Controller::Utility).unwrap();
        assert!(c.settle(id, &b).is_err());
        assert_eq!(c.tutorial.attempts, [0; 5]);
        while !b.finished {
            b.step();
        }
        let won = b.winner() == Some(0);
        c.settle(id, &b).unwrap();
        assert_eq!(c.tutorial.attempts, [1, 0, 0, 0, 0]);
        assert_eq!(c.tutorial.cleared, [won, false, false, false, false]);
        assert_eq!(c.teams[0].fighters.len(), 3);
        assert!(c.teams[0].fighters.iter().all(|f| f.experience > 0));
        assert_eq!(
            c.teams[0].gold,
            100 + phalanx::progression::gold_reward(3, won)
        );
        let saved = serde_json::to_string(&c).unwrap();
        assert!(c.settle(id, &b).is_err());
        assert_eq!(saved, serde_json::to_string(&c).unwrap());
        let mut loaded: Campaign = serde_json::from_str(&saved).unwrap();
        loaded.validate().unwrap();
        assert_eq!(loaded.tutorial.cleared, c.tutorial.cleared);
        let lesson = tutorial::prepare_next(&mut loaded).unwrap();
        assert_eq!(lesson.rival, tutorial::LESSONS[usize::from(won)].rival);
        wins += usize::from(won);
    }
    assert!(
        wins > 0,
        "starter curriculum must have a real winning opening"
    );
}
#[test]
fn authored_styles_change_tactics_and_replay_under_both_controllers() {
    let mut trajectories = std::collections::BTreeSet::new();
    for style in BattleStyle::ALL {
        for controller in [Controller::Utility, Controller::Learned] {
            let mut a = Battle::with_founders(42, false, Arena::Ridge);
            a.controller = controller;
            for u in &mut a.units {
                if u.team == 0 {
                    u.mind = style.mind();
                }
            }
            let mut b = a.clone();
            for _ in 0..120 {
                a.step();
                b.step();
            }
            assert_eq!(
                serde_json::to_string(&a).unwrap(),
                serde_json::to_string(&b).unwrap()
            );
            assert_eq!(a.controller, controller);
            if controller == Controller::Utility {
                trajectories.insert(
                    serde_json::to_string(
                        &a.units
                            .iter()
                            .map(|u| (u.position, u.decision_counts))
                            .collect::<Vec<_>>(),
                    )
                    .unwrap(),
                );
            }
        }
    }
    assert!(
        trajectories.len() > 1,
        "styles should affect actual routes/decisions"
    );
}
