//! Headless baseline runner; produces real simulation records without a GPU.
use phalanx::{Battle, terrain::Arena};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value = |key: &str| args.windows(2).find(|w| w[0] == key).map(|w| w[1].clone());
    let seed = value("--seed")
        .map(|s| s.parse::<u64>().expect("seed must be an unsigned integer"))
        .unwrap_or(42);
    let arena = value("--arena")
        .map(|s| Arena::parse(&s).expect("arena must be ridge, choke, or open"))
        .unwrap_or_default();
    let mut battle = Battle::with_arena(seed, args.iter().any(|a| a == "--swap"), arena);
    while !battle.finished {
        battle.step();
    }
    let json = serde_json::to_string_pretty(&battle).unwrap();
    if let Some(path) = value("--output") {
        std::fs::write(path, json).expect("cannot write telemetry");
    } else {
        println!("{json}");
    }
}
