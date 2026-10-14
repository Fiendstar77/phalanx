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

#[test]
fn summary_counts_real_pairs_and_excludes_missing_predictions() {
    use phalanx::learned::{Comparison, ComparisonSummary};
    let mut summary = ComparisonSummary::default();
    assert_eq!(summary.agreement_rate(), None);
    let mut c = Comparison {
        tick: 0,
        unit: 0,
        utility: 8,
        learned: Some(1),
        executed: 8,
        confidence: None,
        controller: Controller::Utility,
        disagreed: true,
    };
    summary.record(&c);
    summary.record(&c);
    c.utility = 7;
    c.learned = Some(3);
    summary.record(&c);
    c.utility = 1;
    c.learned = Some(1);
    summary.record(&c);
    c.learned = None;
    summary.record(&c);
    assert_eq!(
        (summary.compared, summary.agreements, summary.unavailable),
        (4, 1, 1)
    );
    assert_eq!(summary.agreement_rate(), Some(0.25));
    assert_eq!(summary.action_pairs.iter().flatten().sum::<u32>(), 4);
    let top = summary.top_disagreements(3);
    assert_eq!(top.len(), 2);
    assert_eq!(
        (top[0].utility, top[0].learned, top[0].count),
        ("SUPPORT ALLY", "ATTACK", 2)
    );
    assert_eq!(
        (top[1].utility, top[1].learned, top[1].count),
        ("SEEK COVER", "FLANK", 1)
    );
    let output = serde_json::to_value(&summary).unwrap();
    assert_eq!(output["agreement_rate"], 0.25);
    assert_eq!(output["top_disagreements"][0]["count"], 2);
}

#[test]
fn live_summary_conserves_decisions_across_controller_switch_and_reset() {
    let mut battle = Battle::new(42, false);
    for _ in 0..150 {
        battle.step();
    }
    let before = battle.comparison_summary.compared;
    battle.controller = Controller::Learned;
    while !battle.finished {
        battle.step();
    }
    let summary = &battle.comparison_summary;
    assert!(summary.compared > before);
    assert_eq!(
        summary.compared + summary.unavailable,
        battle
            .units
            .iter()
            .flat_map(|u| u.decision_counts)
            .sum::<u32>()
    );
    assert_eq!(
        summary.compared - summary.agreements,
        battle.disagreements.len() as u32
    );
    for u in 0..ACTION_COUNT {
        for l in 0..ACTION_COUNT {
            if u != l {
                assert_eq!(
                    summary.action_pairs[u][l],
                    battle
                        .disagreements
                        .iter()
                        .filter(|c| c.utility == u && c.learned == Some(l))
                        .count() as u32
                );
            }
        }
    }
    battle = Battle::new(42, false);
    assert_eq!(battle.comparison_summary.compared, 0);
    assert_eq!(battle.comparison_summary.agreement_rate(), None);
    battle.policy = None;
    battle.step();
    assert_eq!(battle.comparison_summary.unavailable, 10);
    assert_eq!(battle.comparison_summary.agreement_rate(), None);
}
