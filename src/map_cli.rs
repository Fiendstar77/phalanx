//! Inspect/export a generated battlefield without starting graphics or touching a save.
use phalanx::{learned::Controller, pressure, procedural, progression::Campaign, terrain::Arena};
use std::fmt::Write;
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!(
            "phalanx-map [--seed N] [--league 3|4] [--output map.svg] [--simulate]\nWrites an SVG tactical plan and a JSON arena beside it. Campaign saves are never read or written."
        );
        return Ok(());
    }
    let mut seed = 42;
    let mut league = 3;
    let mut output = "battlefield.svg".to_string();
    let mut simulate = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--simulate" => simulate = true,
            "--seed" | "--league" | "--output" => {
                let value = args.get(i + 1).ok_or("missing argument value")?;
                match args[i].as_str() {
                    "--seed" => seed = value.parse::<u64>().map_err(|_| "invalid seed")?,
                    "--league" => league = value.parse::<usize>().map_err(|_| "invalid league")?,
                    _ => output = value.clone(),
                }
                i += 1;
            }
            other => return Err(format!("unknown option {other}; use --help")),
        }
        i += 1;
    }
    if ![3, 4].contains(&league) {
        return Err("playable leagues are 3 and 4".into());
    }
    let mut c = Campaign::new();
    if league == 4 {
        c.teams[0].gold = 1000;
        c.unlock_fourth()?;
    }
    let arena = pressure::generate(&c.teams, seed);
    let Arena::Pressure(field) = arena else {
        unreachable!()
    };
    if !procedural::connected(field) {
        return Err("battlefield failed route validation".into());
    }
    let path = std::path::Path::new(&output);
    if path.extension().and_then(|e| e.to_str()) != Some("svg") {
        return Err("output must end in .svg".into());
    }
    let json_path = path.with_extension("json");
    // Create both new artifacts before writing; do not overwrite campaign data or earlier exports.
    let mut svg_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let json_file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&json_path)
    {
        Ok(f) => f,
        Err(e) => {
            drop(svg_file);
            let _ = std::fs::remove_file(path);
            return Err(e.to_string());
        }
    };
    use std::io::Write as _;
    svg_file
        .write_all(svg(arena, league).as_bytes())
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(json_file, &arena).map_err(|e| e.to_string())?;
    let mut report = serde_json::json!({"name":field.name(),"seed":seed,"league":league,"routes_connected":true,"svg":output,"arena":json_path});
    if simulate {
        let (_, mut b) = c.begin_battle(seed, arena, Controller::Utility)?;
        while !b.finished {
            b.step();
        }
        report["battle"] = serde_json::json!({"ticks":b.tick,"winner":b.winner(),"damage":b.units.iter().map(|u|u.damage_dealt).sum::<f32>()});
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    );
    Ok(())
}
fn svg(arena: Arena, league: usize) -> String {
    let Arena::Pressure(f) = arena else {
        unreachable!()
    };
    let [hx, hz] = arena.half_size();
    let scale = 18.;
    let ox = 56. + hx * scale;
    let oy = 118. + hz * scale;
    let width = 112. + hx * scale * 2.;
    let height = 220. + hz * scale * 2.;
    let mut out = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{width}' height='{height}' viewBox='0 0 {width} {height}'><rect width='100%' height='100%' fill='#101b23'/><g font-family='sans-serif' fill='#e6e4ce'><text x='36' y='38' font-size='25' font-weight='bold'>{}</text><text x='36' y='65' font-size='14'>PHALANX / Seed {} / League {} / {}</text></g>",
        f.name(),
        f.seed,
        league,
        f.lesson.name()
    );
    let palette = procedural::palette(f);
    for x in -(hx as i32 * 2)..hx as i32 * 2 {
        for z in -(hz as i32 * 2)..hz as i32 * 2 {
            let p = [x as f32 * 0.5, z as f32 * 0.5];
            let mid = [p[0] + 0.25, p[1] + 0.25];
            let h = arena.elevation(mid);
            let c = if arena.cover(mid) > 0. {
                palette[1]
            } else if arena.roughness(mid) > 0.5 {
                palette[2]
            } else if h > 0.9 {
                palette[3]
            } else {
                palette[0]
            };
            let shade = if (h * 5.).fract() < 0.14 { 0.84 } else { 1. };
            write!(
                out,
                "<rect x='{:.2}' y='{:.2}' width='9.1' height='9.1' fill='rgb({},{},{})'/>",
                ox + p[0] * scale,
                oy + p[1] * scale,
                (c[0] * 255. * shade) as u8,
                (c[1] * 255. * shade) as u8,
                (c[2] * 255. * shade) as u8
            )
            .unwrap();
        }
    }
    for (i, (center, extent)) in arena.walls().iter().enumerate() {
        write!(out,"<rect x='{}' y='{}' width='{}' height='{}' rx='2' fill='{}' stroke='#111a20' stroke-width='2'/>",ox+(center[0]-extent[0]*0.5)*scale,oy+(center[1]-extent[1]*0.5)*scale,extent[0]*scale,extent[1]*scale,if i<4 {"#ab7846"} else {"#a3a7a2"}).unwrap();
    }
    for (label, p) in procedural::zones(f)
        .into_iter()
        .filter(|(l, _)| !l.starts_with("Covered"))
    {
        write!(out,"<circle cx='{}' cy='{}' r='5' fill='#e9cc74'/><text x='{}' y='{}' text-anchor='middle' font-size='12' font-family='sans-serif' fill='#fff4d9' stroke='#17232b' stroke-width='3' paint-order='stroke'>{label}</text>",ox+p[0]*scale,oy+p[1]*scale,ox+p[0]*scale,oy+p[1]*scale-12.).unwrap();
    }
    for side in 0..2 {
        for slot in 0..league {
            let p = arena.spawn(side, slot);
            write!(
                out,
                "<circle cx='{}' cy='{}' r='8' fill='{}' stroke='white' stroke-width='1.5'/>",
                ox + p[0] * scale,
                oy + p[1] * scale,
                if side == 0 { "#62c2ed" } else { "#ef8666" }
            )
            .unwrap();
        }
    }
    write!(out,"<g font-family='sans-serif' fill='#e6e4ce' font-size='13'><text x='36' y='{}'>GREEN / cover     DARK / slow basin     PALE / high ground</text><text x='36' y='{}'>TIMBER / breachable     STONE / permanent     Three connected crossings</text></g></svg>",height-49.,height-25.).unwrap();
    out
}
