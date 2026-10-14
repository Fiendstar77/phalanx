//! Headless runner with controlled trait contrasts and optional sequential decision records.
use phalanx::{ACTIONS, Battle, Mind, terrain::Arena};
use serde_json::json;
use std::{
    fs::File,
    io::{BufWriter, Write},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help") {
        println!(
            "phalanx-batch [--seed N] [--arena ridge|choke|open] [--swap] [--trait teamwork|caution --low 0.2 --high 0.9] [--output FILE] [--trace FILE]"
        );
        return Ok(());
    }
    let value = |key: &str| -> Result<Option<String>, String> {
        if let Some(i) = args.iter().position(|a| a == key) {
            return args
                .get(i + 1)
                .filter(|v| !v.starts_with("--"))
                .cloned()
                .map(Some)
                .ok_or_else(|| format!("missing value for {key}"));
        }
        Ok(None)
    };
    let seed = value("--seed")?
        .map(|s| s.parse::<u64>())
        .transpose()?
        .unwrap_or(42);
    let arena = value("--arena")?
        .map(|s| Arena::parse(&s).ok_or("arena must be ridge, choke, or open"))
        .transpose()?
        .unwrap_or_default();
    let swapped = args.iter().any(|a| a == "--swap");
    let mut battle = Battle::with_arena(seed, swapped, arena);
    let trait_name = value("--trait")?;
    let low = value("--low")?
        .map(|s| s.parse::<f32>())
        .transpose()?
        .unwrap_or(0.2);
    let high = value("--high")?
        .map(|s| s.parse::<f32>())
        .transpose()?
        .unwrap_or(0.9);
    if !low.is_finite()
        || !high.is_finite()
        || !(0.0..=1.0).contains(&low)
        || !(0.0..=1.0).contains(&high)
        || low >= high
    {
        return Err("trait levels must be finite, within [0,1], and low < high".into());
    }
    if let Some(name) = &trait_name {
        if !["teamwork", "caution"].contains(&name.as_str()) {
            return Err("trait must be teamwork or caution".into());
        }
        for u in &mut battle.units {
            u.mind = Mind {
                aggression: 0.5,
                caution: 0.5,
                courage: 0.5,
                teamwork: 0.5,
                patience: 0.5,
                self_preservation: 0.5,
                discipline: 0.5,
            };
            let level = if (u.team == 0) != swapped { low } else { high };
            if name == "teamwork" {
                u.mind.teamwork = level;
            } else {
                u.mind.caution = level;
            }
        }
    }
    let mut trace = value("--trace")?
        .map(File::create)
        .transpose()?
        .map(BufWriter::new);
    if let Some(w) = &mut trace {
        writeln!(
            w,
            "{}",
            json!({"event":"start","schema_version":1,"simulation_version":env!("CARGO_PKG_VERSION"),"seed":seed,"arena":arena,"swapped":swapped,"trait":trait_name,"low":low,"high":high,"feature_names":phalanx::telemetry::FEATURE_NAMES,"actions":ACTIONS,"initial_units":battle.units})
        )?;
    }
    while !battle.finished {
        let tick = battle.tick;
        let previous = trace
            .as_ref()
            .map(|_| battle.units.iter().map(|u| u.health).collect::<Vec<_>>());
        let records = if trace.is_some() {
            battle.step_recorded()
        } else {
            battle.step();
            Vec::new()
        };
        if let (Some(w), Some(previous)) = (&mut trace, previous) {
            for record in records {
                writeln!(w, "{}", serde_json::to_string(&record)?)?;
            }
            for u in &battle.units {
                let old = previous[u.id];
                if u.health < old {
                    writeln!(
                        w,
                        "{}",
                        json!({"event":"damage","tick":tick,"unit":u.id,"before":old,"after":u.health})
                    )?;
                }
            }
        }
    }
    if let Some(w) = &mut trace {
        writeln!(
            w,
            "{}",
            json!({"event":"end","tick":battle.tick,"result":battle.result})
        )?;
        w.flush()?;
    }
    let json = serde_json::to_string_pretty(&battle)?;
    if let Some(path) = value("--output")? {
        std::fs::write(path, json)?;
    } else {
        println!("{json}");
    }
    Ok(())
}
