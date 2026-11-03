use phalanx::{Battle, learned::Controller, progression::Campaign, study::Recorder};
use std::path::PathBuf;

fn scratch(label: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "phalanx-study-{label}-{}-{stamp}",
        std::process::id()
    ))
}

#[test]
fn study_preserves_combat_and_records_complete_actual_decisions() {
    let root = scratch("recording");
    let mut recorded = Battle::new(201, false);
    let mut reference = recorded.clone();
    let mut recorder = Recorder::start(&root, &recorded).unwrap();
    while !recorded.finished {
        recorder.step(&mut recorded).unwrap();
        reference.step();
    }
    assert_eq!(
        serde_json::to_value(&recorded).unwrap(),
        serde_json::to_value(&reference).unwrap()
    );
    let trace = std::fs::read_to_string(recorder.directory.join("decisions.jsonl")).unwrap();
    let rows: Vec<serde_json::Value> = trace
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.first().unwrap()["event"], "start");
    assert_eq!(rows.last().unwrap()["event"], "end");
    for (sequence, row) in rows.iter().enumerate() {
        assert_eq!(row["stream_sequence"], sequence as u64);
    }
    assert_eq!(rows[0]["telemetry_schema_version"], 2);
    assert_eq!(rows[0]["simulation_build"], phalanx::challenges::BUILD);
    assert!(rows[0]["world_context"].is_null());
    let frames: Vec<_> = rows
        .iter()
        .filter(|r| r["event"] == "combat_frame")
        .collect();
    assert!(!frames.is_empty());
    assert!(
        frames
            .iter()
            .all(|r| r["truth_only"] == true && r["tick"].as_u64().unwrap() > 0)
    );
    let final_frame = frames.last().unwrap();
    for (row, actual) in final_frame["actors"]
        .as_array()
        .unwrap()
        .iter()
        .zip(&recorded.units)
    {
        assert_eq!(
            row["position"],
            serde_json::to_value(actual.position).unwrap()
        );
        assert_eq!(row["health"], serde_json::to_value(actual.health).unwrap());
    }
    let decisions: Vec<_> = rows.iter().filter(|r| r["event"] == "decision").collect();
    assert!(!decisions.is_empty());
    for row in &decisions {
        let actual = row["comparison"]["executed"].as_u64().unwrap() as usize;
        assert_eq!(row["action"], phalanx::ACTIONS[actual]);
        assert_eq!(row["action_mask"][actual], true);
    }
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(recorder.directory.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["decision_records"], decisions.len());
    assert_eq!(report["complete"], true);
    assert_eq!(report["bonsai_controls_game"], false);
    let status: serde_json::Value =
        serde_json::from_slice(&std::fs::read(recorder.directory.join("status.json")).unwrap())
            .unwrap();
    assert_eq!(status["complete"], true);
    assert!(recorder.directory.join("report.md").is_file());
    drop(recorder);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupted_study_is_never_labelled_complete() {
    let root = scratch("interrupted");
    let mut battle = Battle::new(202, false);
    let mut recorder = Recorder::start(&root, &battle).unwrap();
    recorder.step(&mut battle).unwrap();
    let directory = recorder.directory.clone();
    drop(recorder);
    let status: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("status.json")).unwrap()).unwrap();
    assert_eq!(status["complete"], false);
    assert!(!directory.join("report.md").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn deep_feed_records_actual_opposed_rolls_and_campaign_context() {
    let root = scratch("theus-feed");
    let mut campaign = Campaign::new();
    let (id, mut battle) = campaign
        .begin_battle(8, phalanx::terrain::Arena::OpenField, Controller::Utility)
        .unwrap();
    let mut recorder = Recorder::start_with_context(&root, &battle, Some(id)).unwrap();
    while !battle.finished {
        recorder.step(&mut battle).unwrap();
    }
    let rows: Vec<serde_json::Value> =
        std::fs::read_to_string(recorder.directory.join("decisions.jsonl"))
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
    assert_eq!(rows[0]["campaign_battle_id"], id);
    let actual: Vec<_> = rows
        .iter()
        .filter(|r| r["event"] == "theus")
        .map(|r| r["receipt"].clone())
        .collect();
    assert!(!actual.is_empty());
    assert_eq!(
        actual,
        serde_json::to_value(&battle.theus_events)
            .unwrap()
            .as_array()
            .unwrap()
            .clone()
    );
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(recorder.directory.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["theus_receipts"], actual.len());
    assert_eq!(
        report["trace_bytes"],
        std::fs::metadata(recorder.directory.join("decisions.jsonl"))
            .unwrap()
            .len()
    );
    drop(recorder);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn growth_receipts_survive_save_load_and_are_used_in_the_next_battle() {
    let root = scratch("growth");
    std::fs::create_dir(&root).unwrap();
    let mut campaign = Campaign::new();
    let (id, mut battle) = campaign
        .begin_battle(203, phalanx::terrain::Arena::OpenField, Controller::Utility)
        .unwrap();
    while !battle.finished {
        battle.step();
    }
    // Controlled exposure fixture ensures this test exercises actual practice and event memories.
    battle.units[0].health = 0.;
    battle.units[0].exposure_isolation = 120;
    battle.units[0].decision_counts[8] += 1;
    campaign.settle(id, &battle).unwrap();
    let learning = campaign.last_report.as_ref().unwrap().fighters[0][0]
        .learning
        .as_ref()
        .unwrap()
        .clone();
    assert!(learning.valid());
    assert_eq!(learning.mind_after, learning.mind_before);
    assert_ne!(learning.psyche_before, learning.psyche_after);
    assert!(learning.class_practice[1] > learning.class_practice[0]);
    campaign.save(&root.join("save.json")).unwrap();
    let mut restored = Campaign::load(&root.join("save.json")).unwrap();
    let restored_receipt = restored.last_report.as_ref().unwrap().fighters[0][0]
        .learning
        .as_ref()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&learning).unwrap(),
        serde_json::to_value(restored_receipt).unwrap()
    );
    let (_, next) = restored
        .begin_battle(204, phalanx::terrain::Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(
        serde_json::to_value(next.units[0].mind).unwrap(),
        serde_json::to_value(learning.mind_after).unwrap()
    );
    assert_eq!(next.units[0].psyche, learning.psyche_after);
    assert_eq!(
        next.units[0].mastery.points(next.units[0].class.unwrap()),
        learning.class_practice[1]
    );
    let mut legacy = serde_json::to_value(&campaign).unwrap();
    for side in legacy["last_report"]["fighters"].as_array_mut().unwrap() {
        for fighter in side.as_array_mut().unwrap() {
            fighter.as_object_mut().unwrap().remove("learning");
        }
    }
    let legacy: Campaign = serde_json::from_value(legacy).unwrap();
    legacy.validate().unwrap();
    campaign.last_report.as_mut().unwrap().fighters[0][0]
        .learning
        .as_mut()
        .unwrap()
        .mind_after
        .courage = 2.;
    assert!(campaign.validate().is_err());
    std::fs::remove_dir_all(root).unwrap();
}
