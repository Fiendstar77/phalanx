use phalanx::{
    ACTION_COUNT, Battle,
    learned::{Controller, LearnedPolicy},
};
#[test]
fn rust_inference_matches_actual_pytorch_logits_and_masked_predictions() {
    let policy = LearnedPolicy::bundled();
    let witness: serde_json::Value =
        serde_json::from_str(include_str!("../assets/policy_witness.json")).unwrap();
    for i in 0..witness["features"].as_array().unwrap().len() {
        let features: Vec<f32> = serde_json::from_value(witness["features"][i].clone()).unwrap();
        let mask: [bool; ACTION_COUNT] =
            serde_json::from_value(witness["masks"][i].clone()).unwrap();
        let expected: Vec<f32> = serde_json::from_value(witness["logits"][i].clone()).unwrap();
        let actual = policy.logits(&features).unwrap();
        for (a, b) in actual.iter().zip(expected) {
            assert!((a - b).abs() < 2e-4, "{a} != {b}");
        }
        assert_eq!(
            policy.predict(&features, &mask).unwrap().0,
            witness["predictions"][i].as_u64().unwrap() as usize
        );
    }
}
#[test]
fn shadow_comparison_does_not_change_utility_rollout() {
    let mut shadow = Battle::new(42, false);
    let mut baseline = shadow.clone();
    baseline.policy = None;
    while !shadow.finished {
        shadow.step();
        baseline.step();
    }
    for (a, b) in shadow.units.iter().zip(&baseline.units) {
        assert_eq!(a.position, b.position);
        assert_eq!(a.health, b.health);
        assert_eq!(a.decision_counts, b.decision_counts);
        assert_eq!(a.damage_dealt, b.damage_dealt);
    }
    assert!(!shadow.disagreements.is_empty());
    assert_eq!(shadow.result, baseline.result);
}
#[test]
fn learned_controller_uses_only_eligible_actions_and_is_replayable() {
    let mut a = Battle::new(7, false);
    a.controller = Controller::Learned;
    let mut b = a.clone();
    while !a.finished {
        a.step();
        b.step();
        for u in &a.units {
            if let Some(c) = &u.comparison {
                assert!(u.scores[c.executed] >= 0.0);
            }
        }
    }
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
    assert_eq!(a.learned_fallbacks, 0);
}
#[test]
fn missing_policy_falls_back_to_utility_and_invalid_inputs_are_rejected() {
    let mut a = Battle::new(42, false);
    a.controller = Controller::Learned;
    a.policy = None;
    let mut b = Battle::new(42, false);
    b.policy = None;
    a.step();
    b.step();
    assert_eq!(a.learned_fallbacks, 10);
    for (a, b) in a.units.iter().zip(&b.units) {
        assert_eq!(a.action, b.action);
        assert_eq!(a.position, b.position);
    }
    let policy = LearnedPolicy::bundled();
    assert!(policy.logits(&[f32::NAN; 29]).is_none());
    assert!(policy.predict(&[0.0; 29], &[false; ACTION_COUNT]).is_none());
    assert!(LearnedPolicy::from_json("{}").is_err());
}
