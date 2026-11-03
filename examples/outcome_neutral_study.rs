//! Actual repeated solo sparring, with equal completion credit and no result-based drift.
use phalanx::{learned::Controller, progression::Campaign, terrain::Arena};
fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: outcome_neutral_study NEW_OUTPUT_DIRECTORY");
    let path = std::path::Path::new(&path);
    assert!(!path.exists(), "choose a new directory");
    std::fs::create_dir_all(path).unwrap();
    let mut campaign = Campaign::new_prologue();
    let initial = campaign.clone();
    let mut results = Vec::new();
    for seed in 42..50 {
        // Deliberate diagnostic rematches: remain in the solo chapter after a win.
        // This is not the normal story flow, which would proceed to the next meeting.
        let (id, mut battle) = campaign
            .begin_battle(seed, Arena::OpenField, Controller::Utility)
            .unwrap();
        let mut recorder = phalanx::study::Recorder::start(&path.join("studies"), &battle).unwrap();
        while !battle.finished {
            recorder.step(&mut battle).unwrap();
        }
        campaign.settle(id, &battle).unwrap();
        let report = campaign.last_report.as_ref().unwrap();
        recorder.save_learning(report).unwrap();
        let profiles: Vec<_> = campaign.teams.iter().enumerate().map(|(team, squad)| {
            let f = &squad.fighters[0];
            assert_eq!(f.mind, initial.teams[team].fighters[0].mind);
            assert_eq!(f.psyche.confidence, 0.);
            serde_json::json!({"xp":f.experience,"class_practice":f.mastery.points(f.class),
                "gold":squad.gold,"salvage":squad.salvage,"theus_practice":f.theus.unwrap().practice,
                "event_memories":f.psyche,"mind":f.mind,"wins":f.wins,"battles":f.battles})
        }).collect();
        for key in ["xp", "class_practice", "gold", "salvage"] {
            assert_eq!(profiles[0][key], profiles[1][key]);
        }
        println!(
            "seed={} result={} xp={}/{} gold={}/{}",
            seed,
            battle.result,
            profiles[0]["xp"],
            profiles[1]["xp"],
            profiles[0]["gold"],
            profiles[1]["gold"]
        );
        results.push(serde_json::json!({"seed":seed,"winner":battle.winner(),"tick":battle.tick,"profiles":profiles}));
    }
    campaign.save(&path.join("campaign-after.json")).unwrap();
    let summary = serde_json::json!({"source_build":phalanx::challenges::BUILD,
        "scope":"Eight real solo sparring rematches; authored starting people, Utility controller, seeds 42-49. Chapter advance deliberately deferred. No forced hit or winner.",
        "policy":phalanx::progression::PROGRESSION_POLICY,"battles":results});
    std::fs::write(
        path.join("summary.json"),
        serde_json::to_string_pretty(&summary).unwrap() + "\n",
    )
    .unwrap();
}
