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
fn schema_two_relu_stops_before_the_logits() {
    use phalanx::telemetry::FEATURE_NAMES;
    let mut row = vec![0.0; FEATURE_NAMES.len()];
    row[0] = 1.0;
    let hidden = vec![row];
    let output = vec![vec![1.0]; ACTION_COUNT];
    let policy = LearnedPolicy::from_json(
        &serde_json::json!({
            "schema_version": 2,
            "source_sha256": "test",
            "features": FEATURE_NAMES,
            "actions": phalanx::ACTIONS,
            "mean": vec![0.0; FEATURE_NAMES.len()],
            "std": vec![1.0; FEATURE_NAMES.len()],
            "layers": [
                {"weight": hidden, "bias": [-2.0]},
                {"weight": output, "bias": vec![-3.0; ACTION_COUNT]}
            ]
        })
        .to_string(),
    )
    .unwrap();
    let mut features = vec![0.0; FEATURE_NAMES.len()];
    features[0] = 1.0;
    let quiet = policy.logits(&features).unwrap();
    assert!(quiet.iter().all(|v| (*v + 3.0).abs() < 1e-5));
    features[0] = 4.0;
    let loud = policy.logits(&features).unwrap();
    assert!(loud.iter().all(|v| (*v - -1.0).abs() < 1e-5));
}

#[test]
fn alternate_policy_schema_validates_names_and_applies_its_own_action_mask() {
    let text = serde_json::json!({
        "schema_version": 2,
        "source_sha256": "test",
        "features": ["effort", "health"],
        "actions": ["press", "recover"],
        "mean": [0.0, 0.0],
        "std": [1.0, 1.0],
        "layers": [
            {"weight": [[1.0, 0.0], [0.0, 1.0]], "bias": [0.0, 0.0]},
            {"weight": [[1.0, 0.0], [0.0, 1.0]], "bias": [0.0, 0.0]}
        ]
    })
    .to_string();
    let policy =
        LearnedPolicy::from_schema(&text, &["effort", "health"], &["press", "recover"]).unwrap();
    assert_eq!(policy.logits(&[2.0, 1.0]).unwrap(), [2.0, 1.0]);
    assert_eq!(
        policy.predict_masked(&[2.0, 1.0], &[false, true]),
        Some((1, 1.0))
    );
    assert!(policy.predict_masked(&[2.0, 1.0], &[true]).is_none());
    assert!(
        policy
            .predict_masked(&[2.0, 1.0], &[false, false])
            .is_none()
    );
    assert!(LearnedPolicy::from_json(&text).is_err());
    assert!(
        LearnedPolicy::from_schema(&text, &["health", "effort"], &["press", "recover"]).is_err()
    );
    assert!(
        LearnedPolicy::from_schema(&text, &["effort", "health"], &["recover", "press"]).is_err()
    );
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

/// Local check of the founders 1024×3 export. The bundled lab policy stays schema 1.
#[test]
#[ignore = "reads outputs/founders-policy-1024x3; not the bundled policy"]
fn founders_schema_two_matches_python_logits_on_held_out_rows() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("outputs/founders-policy-1024x3");
    let policy = std::sync::Arc::new(
        LearnedPolicy::from_json(
            &std::fs::read_to_string(dir.join("policy.json")).expect("exported policy"),
        )
        .expect("schema 2 policy"),
    );
    let witness: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("rust_witness.json")).expect("python witness"),
    )
    .unwrap();
    let mut max_error = 0.0_f32;
    for i in 0..witness["features"].as_array().unwrap().len() {
        let features: Vec<f32> = serde_json::from_value(witness["features"][i].clone()).unwrap();
        let mask: [bool; ACTION_COUNT] =
            serde_json::from_value(witness["masks"][i].clone()).unwrap();
        let expected: Vec<f32> = serde_json::from_value(witness["logits"][i].clone()).unwrap();
        let actual = policy.logits(&features).unwrap();
        for (a, b) in actual.iter().zip(&expected) {
            max_error = max_error.max((a - b).abs());
            assert!((a - b).abs() < 2e-4, "{a} != {b}");
        }
        assert_eq!(
            policy.predict(&features, &mask).unwrap().0,
            witness["predictions"][i].as_u64().unwrap() as usize
        );
    }
    eprintln!("founders logit max abs error {max_error}");

    let mut rows = Vec::new();
    let mut compared = 0_u32;
    let mut agreements = 0_u32;
    for seed in 100..108 {
        for swapped in [false, true] {
            let lesson = match seed % 4 {
                0 => phalanx::pressure::Lesson::Split,
                1 => phalanx::pressure::Lesson::Ridge,
                2 => phalanx::pressure::Lesson::Exposure,
                _ => phalanx::pressure::Lesson::Approach,
            };
            let arena = phalanx::terrain::Arena::Pressure(phalanx::procedural::decorate(
                phalanx::pressure::PressureField {
                    lesson,
                    cover_extra: (seed % 2) as u8,
                    wide: false,
                    reach: 1,
                    ..Default::default()
                },
                seed,
            ));
            let mut battle = Battle::with_setup(seed, swapped, arena, true);
            battle.policy = Some(policy.clone());
            let learned_team = if swapped { 0 } else { 1 };
            battle.learned_team = Some(learned_team);
            while !battle.finished {
                battle.step();
            }
            assert_eq!(battle.learned_fallbacks, 0);
            let winner = match battle.result.as_str() {
                "Red wins" => Some(0),
                "Blue wins" => Some(1),
                "Draw" | "Timeout / draw" => None,
                other => panic!("unexpected result {other}"),
            };
            let score = match winner {
                None => 0.5,
                Some(team) => {
                    if team == learned_team {
                        1.0
                    } else {
                        0.0
                    }
                }
            };
            compared += battle.comparison_summary.compared;
            agreements += battle.comparison_summary.agreements;
            rows.push(serde_json::json!({
                "seed": seed,
                "swapped": swapped,
                "learner_score": score,
                "result": battle.result,
                "tick": battle.tick,
                "compared": battle.comparison_summary.compared,
                "agreements": battle.comparison_summary.agreements,
            }));
        }
    }
    let agreement = agreements as f64 / compared as f64;
    assert!(
        agreement > 0.9,
        "student should still track the founders teacher, got {agreement}"
    );
    let report = serde_json::json!({
        "seeds": "100..108, both orientations, founders pressure, learned versus utility",
        "battles": rows.len(),
        "agreement": agreement,
        "learned_wins": rows.iter().filter(|r| r["learner_score"] == 1.0).count(),
        "utility_wins": rows.iter().filter(|r| r["learner_score"] == 0.0).count(),
        "draws": rows.iter().filter(|r| r["learner_score"] == 0.5).count(),
        "rows": rows,
    });
    std::fs::write(dir.join("duel-seeds-100-107.json"), report.to_string()).unwrap();
    eprintln!("{report}");
}
