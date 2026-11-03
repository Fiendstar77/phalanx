//! Authored self-play profiles, real simulation traces, separate from the player's save.
use phalanx::{
    FighterClass as C, Mind, learned::Controller, progression::Campaign, terrain::Arena,
};
fn main() {
    let mut args = std::env::args().skip(1);
    let output = std::path::PathBuf::from(
        args.next()
            .expect("usage: cognition_corpus NEW_DIRECTORY [SEEDS]"),
    );
    let seeds: u64 = args
        .next()
        .map_or(12, |s| s.parse().expect("integer seeds"));
    assert!((3..=96).contains(&seeds), "use 3 to 96 independent seeds");
    std::fs::create_dir(&output).expect("choose a new output directory");
    let profiles = [
        (
            [C::Warrior, C::Scout, C::Ranger],
            [C::Guardian, C::Warrior, C::Healer],
        ),
        (
            [C::Guardian, C::Ranger, C::Paladin],
            [C::Scout, C::Berserker, C::RockThrower],
        ),
        (
            [C::Healer, C::Ranger, C::Scout],
            [C::Warrior, C::Scout, C::Guardian],
        ),
    ];
    let mut summaries = Vec::new();
    for seed in 0..seeds {
        for (profile, (player, opposition)) in profiles.iter().enumerate() {
            let mut campaign = Campaign::new();
            campaign
                .choose_founders("Cognition fixture", *player)
                .unwrap();
            for (team, squad) in campaign.teams.iter_mut().enumerate() {
                for (slot, f) in squad.fighters.iter_mut().enumerate() {
                    f.fighter_id = format!("cognition-{seed}-{profile}-{team}-{slot}");
                    if team == 1 {
                        f.class = opposition[slot];
                    }
                    f.mind = if (team + profile) % 2 == 0 {
                        Mind::aggressive()
                    } else {
                        Mind::disciplined()
                    };
                    if profile == 2 && team == 0 {
                        f.mind.self_preservation = 0.95;
                        f.mind.aggression = 0.25;
                        f.mind.patience = 0.9;
                    }
                }
            }
            let arena =
                [Arena::OpenField, Arena::Ridge, Arena::Ruins][(seed as usize + profile) % 3];
            let (id, mut battle) = campaign
                .begin_battle(seed, arena, Controller::Utility)
                .unwrap();
            let mut recorder = phalanx::study::Recorder::start_with_context(
                &output.join("studies"),
                &battle,
                Some(id),
            )
            .unwrap();
            while !battle.finished {
                recorder.step(&mut battle).unwrap();
            }
            campaign.settle(id, &battle).unwrap();
            recorder
                .save_learning(campaign.last_report.as_ref().unwrap())
                .unwrap();
            println!(
                "seed={seed} profile={profile} ticks={} {}",
                battle.tick, battle.result
            );
            summaries.push(serde_json::json!({"seed":seed,"profile":profile,"ticks":battle.tick,
                "directory":recorder.directory.file_name().unwrap().to_str().unwrap(),"result":battle.result}));
        }
    }
    let manifest = serde_json::json!({"schema_version":1,"simulation_build":phalanx::challenges::BUILD,
        "fixture":true,"scope":"Three authored posture/class profiles, three existing arena families, real 3v3 self-play. No human player data; no starting training grants.",
        "independent_group":"seed across all profiles and maps","seeds":seeds,"battles":summaries});
    std::fs::write(
        output.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap() + "\n",
    )
    .unwrap();
}
