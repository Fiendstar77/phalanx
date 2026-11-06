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
    stream_sequence: u64,
    frames: u32,
    theus_receipts: u32,
    previous_frame: Vec<[f32; 2]>,
    frame_tick: u32,
}

fn write_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let mut out = BufWriter::new(File::create(path)?);
    serde_json::to_writer_pretty(&mut out, value)?;
    writeln!(out)?;
    out.flush()
}

impl Recorder {
    pub fn start(root: &Path, battle: &Battle) -> io::Result<Self> {
        Self::start_with_context(root, battle, None)
    }

    pub fn start_with_context(
        root: &Path,
        battle: &Battle,
        campaign_battle_id: Option<u64>,
    ) -> io::Result<Self> {
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
                "telemetry_schema_version": 2, "stream_sequence": 0,
                "simulation_build": crate::challenges::BUILD,
                "campaign_battle_id": campaign_battle_id,
                "coordinate_system": "arena_local_xz", "world_context": null,
                "frame_authority": "research_truth_not_an_agent_observation",
                "decision_authority": "actor_perception_before_movement",
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
            stream_sequence: 1,
            frames: 0,
            theus_receipts: 0,
            previous_frame: battle.units.iter().map(|u| u.position).collect(),
            frame_tick: battle.tick,
        })
    }

    fn emit(&mut self, mut value: serde_json::Value) -> io::Result<()> {
        value["stream_sequence"] = self.stream_sequence.into();
        writeln!(self.trace, "{value}")?;
        self.stream_sequence += 1;
        Ok(())
    }

    fn frame(&mut self, battle: &Battle) -> io::Result<()> {
        let elapsed = (battle.tick - self.frame_tick).max(1) as f32 * DT;
        let actors: Vec<_> = battle
            .units
            .iter()
            .enumerate()
            .map(|(i, u)| {
                let old = self.previous_frame.get(i).copied().unwrap_or(u.position);
                serde_json::json!({"unit":u.id,"fighter_id":u.fighter_id,"team":u.team,
                "position":u.position,"ground_height":battle.arena.elevation(u.position),
                "mean_velocity":[(u.position[0]-old[0])/elapsed,(u.position[1]-old[1])/elapsed],
                "health":u.health,"max_health":u.max_health(),"lost_limbs":u.lost_limbs,
                "flesh":u.flesh,"bleeding_loss":u.bleeding_loss,"tactics":u.tactics,
                "prosthetics":u.prosthetics,"action":ACTIONS[u.action],
                "effort":u.theus.as_ref().map(|a|a.stamina),
                "theus_practice":u.theus.as_ref().map(|a|a.sheet.practice),
                "technique":u.theus.as_ref().map(|a|a.technique),
                "airborne_until":u.theus.as_ref().map(|a|a.airborne_until),
                "windup_target":u.theus.as_ref().and_then(|a|a.windup_target)})
            })
            .collect();
        self.emit(
            serde_json::json!({"event":"combat_frame","schema_version":1,
            "tick":battle.tick,"phase":"post_step_research_truth","truth_only":true,
            "interval_seconds":elapsed,"actors":actors}),
        )?;
        self.previous_frame = battle.units.iter().map(|u| u.position).collect();
        self.frame_tick = battle.tick;
        self.frames += 1;
        self.trace.flush()
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
            self.emit(serde_json::json!({"event": "damage", "receipt": event}))?;
        }
        for event in battle.heals.iter().filter(|e| e.tick == tick) {
            self.emit(serde_json::json!({"event": "healing", "receipt": event}))?;
        }
        for event in battle.bleed_events.iter().filter(|e| e.tick == tick) {
            self.emit(serde_json::json!({"event":"bleeding","receipt":event}))?;
        }
        for event in battle.tactical_events.iter().filter(|e| e.tick == tick) {
            self.emit(serde_json::json!({"event":"tactical","receipt":event}))?;
        }
        for event in battle.theus_events.iter().filter(|e| e.tick == tick) {
            self.emit(serde_json::json!({"event":"theus","receipt":event}))?;
            self.theus_receipts += 1;
        }
        let next_opportunity = self.next_opportunity;
        for event in battle
            .opportunities
            .events
            .iter()
            .filter(|e| e.sequence >= next_opportunity)
        {
            self.emit(serde_json::json!({"event": "opportunity", "receipt": event}))?;
        }
        if let Some(event) = battle.opportunities.events.last() {
            self.next_opportunity = event.sequence + 1;
        }
        if self.steps.is_multiple_of(5) || battle.finished {
            self.frame(battle)?;
        }
        if battle.finished {
            self.finish(battle)?;
        }
        Ok(())
    }

    fn record(&mut self, record: &DecisionRecord) -> io::Result<()> {
        self.emit(serde_json::to_value(record)?)?;
        self.decisions += 1;
        Ok(())
    }

    fn finish(&mut self, battle: &Battle) -> io::Result<()> {
        self.emit(
            serde_json::json!({"event": "end", "tick": battle.tick, "result": battle.result}),
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
            "telemetry_schema_version": 2, "simulation_build":crate::challenges::BUILD,
            "combat_frames":self.frames,"theus_receipts":self.theus_receipts,
            "stream_events":self.stream_sequence,"trace_bytes":self.trace.get_ref().metadata()?.len(),
            "result": battle.result, "duration_seconds": battle.tick as f32 * DT,
            "survivors": [battle.survivors(0), battle.survivors(1)],
            "actions": actions, "comparison": battle.comparison_summary,
            "learned_fallbacks": battle.learned_fallbacks,
            "opportunity_attempts": battle.opportunities.attempt_count(),
            "opportunity_completions": battle.opportunities.completed.len(),
            "tactical_events":battle.tactical_events,
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
        text.push_str(&format!("\n## Deep telemetry\n\nBuild `{}`. {} ordered events, {} post-step frames and {} opposed Theus receipts. Trace size: {} bytes. Frames include resolved positions, mean motion, health, effort, wounds, practice and technique state. Theus receipts retain dice, defence, critical and actual damage.\n\nFrames are research truth, separate from decision-time perceived observations. Coordinates remain local to this arena; world context is unknown. A stream sequence detects gaps and ordering errors.\n",crate::challenges::BUILD,self.stream_sequence,self.frames,self.theus_receipts,self.trace.get_ref().metadata()?.len()));
        if !battle.tactical_events.is_empty() {
            text.push_str("\n## Tactical history\n\n");
            for e in &battle.tactical_events {
                text.push_str(&format!(
                    "- {:.1}s, fighter {}: {}{}\n",
                    e.tick as f32 * DT,
                    e.unit,
                    e.description(),
                    if e.kind == crate::tactics::Kind::TreatmentReceived {
                        format!(" (+{:.1} HP)", e.amount)
                    } else {
                        String::new()
                    }
                ));
            }
        }
        text.push_str("\n## Evidence\n\n[Full battle state](battle.json) · [Machine-readable report](report.json) · [Decision trace](decisions.jsonl)\n\nDecision records contain perceived contacts, features, eligibility masks, baseline scores, controller proposals and the actual executed action. Damage and healing have simulation receipts. If campaign rewards settle, learning.json and learning.md record persisted before/after changes.\n\n## Interpretation\n\nThis is one battle. Repeated decisions are correlated, and agreement does not establish better tactics. The live PyTorch policy is frozen; separate research models may be trained from eligible studies. Actual practice and event memories persist. Match results grant no temperament or growth multiplier. Timing measures simulation plus observation capture, excludes rendering and trace disk writes, and does not isolate inference. Interrupted/reset recordings retain complete=false in status.json.\n");
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
            "# How the phalanxes changed\n\nPersisted practice and witnessed event memories. Completion credit is equal for all results; temperament and model weights receive no automatic result reinforcement.\n\n",
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
