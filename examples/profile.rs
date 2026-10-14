//! CPU step timing harness. Run in the same profile as the graphical application.
use phalanx::{Battle, terrain::Arena};
use std::time::Instant;
fn main() {
    for arena in [Arena::Ridge, Arena::Choke, Arena::OpenField] {
        let mut b = Battle::with_arena(42, false, arena);
        let mut timings = Vec::new();
        let start = Instant::now();
        while !b.finished {
            let tick = b.tick;
            let before = Instant::now();
            b.step();
            timings.push((before.elapsed().as_secs_f64() * 1000.0, tick));
        }
        timings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let n = timings.len();
        println!(
            "{}: {} ticks / {:.1}ms total; median {:.2}ms p95 {:.2}ms p99 {:.2}ms max {:.2}ms (tick {})",
            arena.name(),
            n,
            start.elapsed().as_secs_f64() * 1000.0,
            timings[n / 2].0,
            timings[n * 95 / 100].0,
            timings[n * 99 / 100].0,
            timings[n - 1].0,
            timings[n - 1].1
        );
        println!("  slowest {:?}", &timings[n.saturating_sub(5)..]);
    }
}
