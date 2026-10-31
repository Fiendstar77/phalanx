//! Staged geometry/ledger demonstration; not a survival or controller-strength study.
#[path = "../tests/support/opportunity.rs"]
mod support;
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let mut output = PathBuf::from("outputs/opportunity-demo");
    let mut seed = 6;
    let mut args = std::env::args().skip(1);
    while let Some(key) = args.next() {
        if key == "--help" {
            println!(
                "opportunity [--seed 6] [--output NEW_DIRECTORY]\nStaged wounded-Scout wall approach; autonomous planning and normal Theus strike resolution."
            );
            return Ok(());
        }
        let value = args.next().ok_or("missing argument value")?;
        match key.as_str() {
            "--seed" => seed = value.parse()?,
            "--output" => output = value.into(),
            _ => return Err(format!("unknown argument {key}").into()),
        }
    }
    if output.exists() {
        return Err("choose a fresh output directory".into());
    }
    let (mut campaign, id, mut battle) = support::fixture(seed);
    let mut recorder = phalanx::study::Recorder::start(&output, &battle)?;
    campaign.save(&output.join("campaign-before.json"))?;
    while !battle.finished {
        recorder.step(&mut battle)?;
    }
    campaign.settle(id, &battle)?;
    recorder.save_learning(campaign.last_report.as_ref().unwrap())?;
    campaign.save(&output.join("campaign-after.json"))?;
    let audit = serde_json::json!({
        "scripted_fixture": true, "human_playtest": false,
        "setup": "Rank-one acrobat Scout, one arm missing, 28% initial health, remembered opponent beyond a wall. Other roster members are downed; the Guardian can move and defend but its attack cooldown is held high.",
        "resolution": "Normal seeded Theus; no successful hit or critical is forced.",
        "purpose": "Autonomous composition, physical execution and ledger proof; not a survival-probability or superiority study.",
        "seed": seed, "attempts": battle.opportunities.attempt_count(),
        "completions": battle.opportunities.completed.len(),
        "legends": campaign.teams[0].fighters[0].legends,
        "study_directory": recorder.directory
    });
    std::fs::write(
        output.join("audit.json"),
        serde_json::to_string_pretty(&audit)?,
    )?;
    for event in &battle.opportunities.events {
        println!("{}", serde_json::to_string(event)?);
    }
    for entry in &campaign.teams[0].fighters[0].legends {
        println!("{}", entry.summary("Mira"));
    }
    println!("Study: {}", recorder.directory.display());
    Ok(())
}
