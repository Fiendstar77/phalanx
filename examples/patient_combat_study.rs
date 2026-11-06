//! Authored local witness. Uses no player save; injuries and outcomes resolve normally.
use phalanx::{
    FighterClass as C, learned::Controller, progression::Campaign, styles::BattleStyle,
    tactics::Phase,
};
fn main() {
    let out = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: patient_combat_study NEW_DIRECTORY"),
    );
    std::fs::create_dir(&out).expect("use a new output directory");
    let mut campaign = Campaign::new();
    for (team, squad) in campaign.teams.iter_mut().enumerate() {
        for (slot, f) in squad.fighters.iter_mut().enumerate() {
            f.class = [C::Warrior, C::Scout, C::Healer][slot];
            f.mind = BattleStyle::ALL[(slot + team) % 3].mind();
        }
    }
    let mut summaries = Vec::new();
    for seed in 42..50 {
        let arena = phalanx::pressure::generate(&campaign.teams, seed);
        let (id, mut battle) = campaign
            .begin_battle(seed, arena, Controller::Utility)
            .unwrap();
        let mut recorder =
            phalanx::study::Recorder::start_with_context(&out.join("studies"), &battle, Some(id))
                .unwrap();
        let mut first_report = None;
        let mut first_hit = None;
        let mut recon_frames = 0;
        let mut assembly_frames = 0;
        while !battle.finished {
            recorder.step(&mut battle).unwrap();
            if first_report.is_none() {
                first_report = battle
                    .units
                    .iter()
                    .filter_map(|u| u.tactics.first_report)
                    .min();
            }
            if first_hit.is_none() {
                first_hit = battle.damage_events.first().map(|e| e.tick);
            }
            recon_frames += u32::from(
                battle
                    .units
                    .iter()
                    .any(|u| u.tactics.phase == Phase::Reconnaissance),
            );
            assembly_frames += u32::from(
                battle
                    .units
                    .iter()
                    .any(|u| u.tactics.phase == Phase::Assemble),
            );
        }
        let wounds = battle
            .units
            .iter()
            .map(|u| {
                u.flesh
                    .marks
                    .iter()
                    .filter(|w| w.source_battle.is_none())
                    .count()
            })
            .sum::<usize>();
        let bleeding = battle.units.iter().map(|u| u.bleeding_loss).sum::<f32>();
        campaign.settle(id, &battle).unwrap();
        recorder
            .save_learning(campaign.last_report.as_ref().unwrap())
            .unwrap();
        summaries.push(serde_json::json!({"seed":seed,"first_report_tick":first_report,"first_hit_tick":first_hit,
            "recon_seconds":recon_frames as f32*phalanx::DT,"assembly_seconds":assembly_frames as f32*phalanx::DT,
            "ticks":battle.tick,"result":battle.result,"new_flesh_wounds":wounds,"bleeding_hp":bleeding,
            "tactical_events":battle.tactical_events}));
        println!(
            "seed {seed}: first report {first_report:?}, first hit {first_hit:?}, {wounds} wounds, {bleeding:.1} bleeding HP"
        );
        campaign.save(&out.join("campaign-after.json")).unwrap();
        campaign = Campaign::load(&out.join("campaign-after.json")).unwrap();
    }
    std::fs::write(out.join("summary.json"),serde_json::to_string_pretty(&serde_json::json!({
        "scripted_starting_profiles":true,"human_playtest":false,"real_combat":true,"player_save_opened":false,
        "simulation_build":phalanx::challenges::BUILD,"battles":summaries})).unwrap()+"\n").unwrap();
}
