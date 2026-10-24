//! Local team exchange without graphics or campaign mutation.
use phalanx::{
    async_exchange::{ChallengeService, LocalExchange},
    challenges::{Challenge, ChallengeResult, TeamSnapshot},
    progression::Campaign,
};
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "--help") {
        println!(
            "phalanx-challenge export --save CAMPAIGN --output TEAM.json [--team 0|1]\nphalanx-challenge fight --home TEAM.json --away ENEMY.json --output RESULT.json [--seed N]\nphalanx-challenge verify --result RESULT.json\nphalanx-challenge emulate --home TEAM.json --away ENEMY.json --output RESULT.json [--seed N]\n\nLocal exhibition matches. Campaign saves and rewards are never changed.\nExports require a new output filename."
        );
        return Ok(());
    }
    let value = |key: &str| -> Result<&str, String> {
        let index = args
            .iter()
            .position(|s| s == key)
            .ok_or_else(|| format!("Missing {key}"))?;
        args.get(index + 1)
            .filter(|s| !s.starts_with("--"))
            .map(String::as_str)
            .ok_or_else(|| format!("Missing value for {key}"))
    };
    let allowed: &[&str] = match args[0].as_str() {
        "export" => &["--save", "--output", "--team"],
        "fight" => &["--home", "--away", "--output", "--seed"],
        "verify" => &["--result"],
        "emulate" => &["--home", "--away", "--output", "--seed"],
        _ => return Err("Expected export, fight, verify, or emulate; use --help".into()),
    };
    for pair in args[1..].chunks(2) {
        if pair.len() != 2 || !allowed.contains(&pair[0].as_str()) || pair[1].starts_with("--") {
            return Err(format!("Invalid argument {}; use --help", pair[0]));
        }
        if args.iter().filter(|arg| *arg == &pair[0]).count() != 1 {
            return Err(format!("Duplicate argument {}", pair[0]));
        }
    }
    match args[0].as_str() {
        "emulate" => {
            let home = TeamSnapshot::load(Path::new(value("--home")?))?;
            let away = TeamSnapshot::load(Path::new(value("--away")?))?;
            let seed = if args.iter().any(|s| s == "--seed") {
                value("--seed")?.parse().map_err(|_| "Invalid seed")?
            } else {
                42
            };
            let mut service = LocalExchange::default();
            let opponent = service.publish("published-owner", away)?;
            // The owner's client is gone; only serializable service state remains.
            let mut service: LocalExchange =
                serde_json::from_str(&serde_json::to_string(&service).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let challenger = service.publish("challenger", home)?;
            let ticket = service.issue(challenger.id, opponent.id, seed)?;
            let result = ticket.challenge.run()?;
            let receipt = service.submit(ticket.id, result)?;
            receipt.result.save(Path::new(value("--output")?))?;
            println!(
                "Local asynchronous emulator: {} publication(s) discovered; result replay-verified and delivered to {} owner inbox(es). No accounts, networking or campaign rewards.",
                service
                    .discover(receipt.result.challenge.teams[0].team.fighters.len())
                    .len(),
                receipt.owners.len()
            );
        }
        "export" => {
            let campaign = Campaign::load(Path::new(value("--save")?))?;
            let side = if args.iter().any(|s| s == "--team") {
                value("--team")?
                    .parse()
                    .map_err(|_| "Invalid squad index")?
            } else {
                0
            };
            let snapshot = TeamSnapshot::from_campaign(&campaign, side)?;
            snapshot.save(Path::new(value("--output")?))?;
            println!("Exported {}", snapshot.summary());
        }
        "fight" => {
            let home = TeamSnapshot::load(Path::new(value("--home")?))?;
            let away = TeamSnapshot::load(Path::new(value("--away")?))?;
            let seed = if args.iter().any(|s| s == "--seed") {
                value("--seed")?.parse().map_err(|_| "Invalid seed")?
            } else {
                42
            };
            let result = Challenge::new(home, away, seed)?.run()?;
            result.save(Path::new(value("--output")?))?;
            println!(
                "{} | {} ticks | result exported",
                result.battle["result"], result.battle["tick"]
            );
        }
        "verify" => {
            let result = ChallengeResult::load(Path::new(value("--result")?))?;
            result.verify()?;
            println!("Verified: replay matches the complete recorded battle. Campaigns unchanged.");
        }
        _ => unreachable!(),
    }
    Ok(())
}
