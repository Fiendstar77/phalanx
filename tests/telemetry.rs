use phalanx::{Battle, telemetry::FEATURE_NAMES};
#[test]
fn recording_decisions_does_not_change_the_battle() {
    let mut plain = Battle::new(42, false);
    let mut recorded = plain.clone();
    let mut count = 0;
    while !plain.finished {
        plain.step();
        for record in recorded.step_recorded() {
            count += 1;
            assert_eq!(record.features.len(), FEATURE_NAMES.len());
            assert!(record.features.iter().all(|v| v.is_finite()));
            let chosen = phalanx::ACTIONS
                .iter()
                .position(|a| *a == record.action)
                .unwrap();
            assert!(record.action_mask[chosen]);
            assert!(record.contacts.iter().all(|c| c.seen_tick <= record.tick));
        }
    }
    assert!(count > 0);
    assert_eq!(
        serde_json::to_string(&plain).unwrap(),
        serde_json::to_string(&recorded).unwrap()
    );
}
#[test]
fn telemetry_counts_only_living_unit_time_and_handles_no_ally_pairs() {
    let mut b = Battle::new(42, false);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[0].health = 100.0;
    b.step();
    assert_eq!(b.units[0].alive_ticks, 1);
    assert_eq!(b.units[1].alive_ticks, 0);
    assert_eq!(b.ally_distance_samples, [0, 0]);
}

#[test]
fn recorded_features_do_not_include_the_selected_flank_state() {
    let mut b = Battle::with_arena(42, false, phalanx::terrain::Arena::OpenField);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[0].health = 100.0;
    b.units[0].mind = phalanx::Mind::disciplined();
    b.units[0].position = [-3.6, 0.0];
    b.units[1].health = 100.0;
    b.units[1].position = [-1.0, 0.0];
    b.units[1].enemy = Some(5);
    b.units[1].action = 1;
    b.units[5].health = 100.0;
    b.units[5].position = [0.4, 0.0];
    let records = b.step_recorded();
    let r = records.iter().find(|r| r.unit == 0).unwrap();
    assert_eq!(r.action, "FLANK");
    assert!(b.units[0].flank_anchor.is_some());
    let ready = FEATURE_NAMES
        .iter()
        .position(|n| *n == "flank_ready")
        .unwrap();
    let remaining = FEATURE_NAMES
        .iter()
        .position(|n| *n == "flank_remaining")
        .unwrap();
    assert_eq!(r.features[ready], 1.0);
    assert_eq!(r.features[remaining], 1.0);
}
