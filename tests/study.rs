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
    // Controlled exposure fixture ensures this test exercises habits and memories.
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
    assert!(learning.mind_after.self_preservation > learning.mind_before.self_preservation);
    assert!(learning.mind_after.teamwork > learning.mind_before.teamwork);
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
