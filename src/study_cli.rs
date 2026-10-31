//! Isolated multi-battle study of the actual persistent Founders campaign loop.
use phalanx::{learned::Controller, progression::Campaign, study::Recorder};
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let mut output = None;
    let mut battles = 3_u64;
    let mut seed = 200_u64;
    let mut controller = Controller::Utility;
    let mut args = std::env::args().skip(1);
    while let Some(key) = args.next() {
        if key == "--help" {
            println!(
                "phalanx-study --output NEW_DIRECTORY [--battles 3] [--seed 200] [--controller utility|learned]\nRuns an isolated new 3v3 campaign; never reads or writes your player save."
            );
            return Ok(());
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {key}"))?;
        match key.as_str() {
            "--output" => output = Some(PathBuf::from(value)),
            "--battles" => battles = value.parse()?,
            "--seed" => seed = value.parse()?,
            "--controller" => {
                controller = match value.as_str() {
                    "utility" => Controller::Utility,
                    "learned" => Controller::Learned,
                    _ => return Err("controller must be utility or learned".into()),
                }
            }
            _ => return Err(format!("unknown option: {key}").into()),
        }
    }
    if !(1..=100).contains(&battles) || seed.checked_add(battles - 1).is_none() {
        return Err("require 1–100 battles and a seed range that fits u64".into());
    }
    let output = output.ok_or("--output NEW_DIRECTORY is required")?;
    if output.exists() {
        return Err("output directory already exists; choose a fresh directory".into());
    }
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(&output)?;
    let mut campaign = Campaign::new();
    campaign.save(&output.join("campaign-before.json"))?;
    let mut text = String::from(
        "# Phalanx evolution study\n\nIsolated new 3v3 campaign, using real simulation and settlement rules. No player save is loaded or changed.\n\nExperience, practice, traits, memories and rival adaptation persist between these battles. PyTorch and Bonsai weights are unchanged.\n\n",
    );
    for index in 0..battles {
        let trial_seed = seed + index;
        let arena = phalanx::pressure::generate(&campaign.teams, trial_seed);
        let (id, mut battle) = campaign.begin_battle(trial_seed, arena, controller)?;
        let mut recorder = Recorder::start(&output, &battle)?;
        while !battle.finished {
            recorder.step(&mut battle)?;
        }
        campaign.settle(id, &battle)?;
        let report = campaign
            .last_report
            .as_ref()
            .ok_or("missing settled report")?;
        recorder.save_learning(report)?;
        let name = recorder.directory.file_name().unwrap().to_string_lossy();
        text.push_str(&format!("## Battle {}: {}\n\n[Battle study]({name}/report.md) · [Learning receipts]({name}/learning.md)\n\n", index + 1, battle.result));
        for fighter in &report.fighters[0] {
            text.push_str(&format!("**{}**\n\n", fighter.name));
            if let Some(learning) = &fighter.learning {
                for change in learning.changes() {
                    text.push_str(&format!("- {change}\n"));
                }
            }
            text.push('\n');
        }
        campaign.save(&output.join("campaign-after.json"))?;
        // Exercise the same persistence boundary the player uses between sessions.
        campaign = Campaign::load(&output.join("campaign-after.json"))?;
        std::fs::write(output.join("report.md"), &text)?;
        println!(
            "Battle {}: {} | {}",
            index + 1,
            battle.result,
            recorder.directory.display()
        );
    }
    println!("Study: {}", output.join("report.md").display());
    Ok(())
}
