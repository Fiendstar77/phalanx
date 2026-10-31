//! Local study artifacts from actual simulation steps. Recording never selects actions.
use crate::{
    ACTIONS, Battle, DT,
    telemetry::{DecisionRecord, FEATURE_NAMES},
};
use serde::Serialize;
use std::{
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    time::Instant,
};

pub struct Recorder {
    pub directory: PathBuf,
    trace: BufWriter<File>,
    initial_tick: u32,
    steps: u32,
    decisions: usize,
    total_step_ms: f64,
    max_step_ms: f64,
    complete: bool,
    next_opportunity: u64,
}

fn write_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let mut out = BufWriter::new(File::create(path)?);
    serde_json::to_writer_pretty(&mut out, value)?;
    writeln!(out)?;
    out.flush()
}

impl Recorder {
    pub fn start(root: &Path, battle: &Battle) -> io::Result<Self> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        fs::create_dir_all(root)?;
        let directory = root.join(format!(
            "battle-{}-{stamp}-{}",
            battle.seed,
            std::process::id()
        ));
        fs::create_dir(&directory)?;
        let mut trace = BufWriter::new(File::create(directory.join("decisions.jsonl"))?);
        writeln!(
            trace,
            "{}",
            serde_json::json!({
                "event": "start", "schema_version": 1,
                "simulation_version": battle.simulation_version,
                "seed": battle.seed, "arena": battle.initial_arena, "swapped": battle.swapped,
                "founders": battle.founders, "initial_tick": battle.tick,
                "controller": battle.controller, "learned_team": battle.learned_team,
                "policy_source_sha256": battle.policy.as_ref().map(|p| &p.source_sha256),
                "feature_names": FEATURE_NAMES, "actions": ACTIONS,
                "initial_units": battle.units
            })
        )?;
        trace.flush()?;
        write_json(
            &directory.join("status.json"),
            &serde_json::json!({"complete": false, "initial_tick": battle.tick}),
        )?;
        Ok(Self {
            directory,
            trace,
            initial_tick: battle.tick,
            steps: 0,
            decisions: 0,
            total_step_ms: 0.,
            max_step_ms: 0.,
            complete: false,
            next_opportunity: 0,
        })
    }

    pub fn step(&mut self, battle: &mut Battle) -> io::Result<()> {
        if battle.finished || self.complete {
            return Ok(());
        }
        let tick = battle.tick;
        let start = Instant::now();
        let records = battle.step_recorded();
        let elapsed = start.elapsed().as_secs_f64() * 1000.;
        self.steps += 1;
        self.total_step_ms += elapsed;
        self.max_step_ms = self.max_step_ms.max(elapsed);
        for record in records {
            self.record(&record)?;
        }
        for event in battle.damage_events.iter().filter(|e| e.tick == tick) {
            writeln!(
                self.trace,
                "{}",
                serde_json::json!({"event": "damage", "receipt": event})
            )?;
        }
        for event in battle.heals.iter().filter(|e| e.tick == tick) {
            writeln!(
                self.trace,
                "{}",
                serde_json::json!({"event": "healing", "receipt": event})
            )?;
        }
        for event in battle
            .opportunities
            .events
            .iter()
            .filter(|e| e.sequence >= self.next_opportunity)
        {
            writeln!(
                self.trace,
                "{}",
                serde_json::json!({"event": "opportunity", "receipt": event})
            )?;
        }
        if let Some(event) = battle.opportunities.events.last() {
            self.next_opportunity = event.sequence + 1;
        }
        if battle.finished {
            self.finish(battle)?;
        }
        Ok(())
    }

    fn record(&mut self, record: &DecisionRecord) -> io::Result<()> {
        writeln!(self.trace, "{}", serde_json::to_string(record)?)?;
        self.decisions += 1;
        Ok(())
    }

    fn finish(&mut self, battle: &Battle) -> io::Result<()> {
        writeln!(
            self.trace,
            "{}",
            serde_json::json!({"event": "end", "tick": battle.tick, "result": battle.result})
        )?;
        self.trace.flush()?;
        write_json(&self.directory.join("battle.json"), battle)?;
        let actions: Vec<_> = ACTIONS.iter().enumerate().map(|(i, name)| {
            serde_json::json!({"action": name, "decisions": battle.units.iter().map(|u| u.decision_counts[i]).sum::<u32>()})
        }).collect();
        let report = serde_json::json!({
            "schema_version": 1, "complete": true,
            "simulation_version": battle.simulation_version,
            "seed": battle.seed, "arena": battle.initial_arena, "founders": battle.founders,
            "initial_tick": self.initial_tick, "final_tick": battle.tick,
            "recorded_steps": self.steps, "decision_records": self.decisions,
            "result": battle.result, "duration_seconds": battle.tick as f32 * DT,
            "survivors": [battle.survivors(0), battle.survivors(1)],
            "actions": actions, "comparison": battle.comparison_summary,
            "learned_fallbacks": battle.learned_fallbacks,
            "opportunity_attempts": battle.opportunities.attempt_count(),
            "opportunity_completions": battle.opportunities.completed.len(),
            "mean_recorded_step_wall_ms": self.total_step_ms / f64::from(self.steps.max(1)),
            "max_recorded_step_wall_ms": self.max_step_ms,
            "step_timing_scope": "Simulation and observation capture; excludes trace disk writes, rendering and model-specific timing.",
            "bonsai_controls_game": false,
            "limitations": ["Single battle; no superiority claim or independent-sample inference from ticks.", "PyTorch weights are frozen. Campaign habit and memory adaptation is rule-based.", "Initial setup and per-decision controller are recorded; interactive edits can prevent exact replay."]
        });
        write_json(&self.directory.join("report.json"), &report)?;
        let agreement = battle
            .comparison_summary
            .agreement_rate()
            .map(|r| format!("{:.1}%", r * 100.))
            .unwrap_or_else(|| "N/A".into());
        let mut text = format!(
            "# Phalanx battle study\n\n{} · {} · seed {}\n\nDuration {:.1}s. Survivors: {} / {}.\nRecorded from tick {}: {} steps and {} decision records.\n\n## Controller comparison\n\nUtility/PyTorch agreement: {} across {} valid comparisons; {} unavailable predictions.\nLearned-controller fallbacks: {}. Bonsai does not control this battle.\n\n| Action | Decisions in battle |\n| --- | ---: |\n",
            battle.result,
            battle.initial_arena.name(),
            battle.seed,
            battle.tick as f32 * DT,
            battle.survivors(0),
            battle.survivors(1),
            self.initial_tick,
            self.steps,
            self.decisions,
            agreement,
            battle.comparison_summary.compared,
            battle.comparison_summary.unavailable,
            battle.learned_fallbacks
        );
        for (i, action) in ACTIONS.iter().enumerate() {
            text.push_str(&format!(
                "| {action} | {} |\n",
                battle
                    .units
                    .iter()
                    .map(|u| u.decision_counts[i])
                    .sum::<u32>()
            ));
        }
        text.push_str("\n## Evidence\n\n[Full battle state](battle.json) · [Machine-readable report](report.json) · [Decision trace](decisions.jsonl)\n\nDecision records contain perceived contacts, features, eligibility masks, baseline scores, controller proposals and the actual executed action. Damage and healing have simulation receipts. If campaign rewards settle, learning.json and learning.md record persisted before/after changes.\n\n## Interpretation\n\nThis is one battle. Repeated decisions are correlated, and agreement does not establish better tactics. The PyTorch policy is frozen; campaign habits and memories adapt through explicit rules. Timing measures simulation plus observation capture, excludes rendering and trace disk writes, and does not isolate inference. Interrupted/reset recordings retain complete=false in status.json.\n");
        fs::write(self.directory.join("report.md"), text)?;
        write_json(
            &self.directory.join("status.json"),
            &serde_json::json!({"complete": true, "initial_tick": self.initial_tick, "final_tick": battle.tick}),
        )?;
        self.complete = true;
        Ok(())
    }

    pub fn save_learning(&self, report: &crate::progression::BattleReport) -> io::Result<()> {
        write_json(&self.directory.join("learning.json"), report)?;
        let entries: Vec<_> = report
            .fighters
            .iter()
            .flatten()
            .flat_map(|f| &f.legends)
            .collect();
        write_json(&self.directory.join("legends.json"), &entries)?;
        let mut ledger = String::from(
            "# Legend Ledger\n\nWitnessed compositions from this battle. Epithets confer no mechanical bonus.\n\n",
        );
        for fighter in report.fighters.iter().flatten() {
            for entry in &fighter.legends {
                ledger.push_str(&format!("{}\n\n", entry.summary(&fighter.name)));
            }
        }
        if entries.is_empty() {
            ledger.push_str("No qualifying completed composition.\n");
        }
        fs::write(self.directory.join("legend-ledger.md"), ledger)?;
        let mut text = String::from(
            "# How the phalanxes changed\n\nPersisted experience, practice, habits and conditioned memories. Frozen model weights are unchanged.\n\n",
        );
        for (team, fighters) in report.fighters.iter().enumerate() {
            text.push_str(&format!("## Team {}\n\n", team + 1));
            for fighter in fighters {
                text.push_str(&format!("### {}\n\n", fighter.name));
                if let Some(learning) = &fighter.learning {
                    for line in learning.changes() {
                        text.push_str(&format!("- {line}\n"));
                    }
                }
                text.push('\n');
            }
        }
        fs::write(self.directory.join("learning.md"), text)
    }
}
