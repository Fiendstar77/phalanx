# Local performance check

The interactive prototype showed intermittent lag. A fixed-seed CPU timing harness found expensive simulation ticks, especially while several agents recalculated routes.

The changes enable optimization for the application crate in development builds, avoid unnecessary route-anchor clearance checks, stop rewriting unchanged UI text and camera transforms, and interpolate displayed positions between real fixed-step snapshots.

## CPU step timing

Single local before/after diagnostic runs, seed 42, default profiles, on the same development machine. These values time `Battle::step`, excluding graphics, asset loading, and compilation. They are not a cross-machine benchmark or a statistical comparison of controllers.

| Arena | Before p99 | After p99 | Before maximum | After maximum |
| --- | ---: | ---: | ---: | ---: |
| Ridge | 14.28 ms | 0.91 ms | 16.85 ms | 1.46 ms |
| Choke | 12.21 ms | 1.03 ms | 16.20 ms | 1.31 ms |
| Open Field | 11.24 ms | 0.80 ms | 24.39 ms | 1.91 ms |

Final JSON battle summaries for all three arenas were compared byte-for-byte before and after these performance changes and matched. The perception/coordination policy was unchanged by the optimization.

## Live rendering

The updated application was tested on the development desktop with optional frame diagnostics enabled. Steady diagnostic samples were about 60 FPS (16.67 ms per frame) on this desktop. The user confirmed that it feels smooth. Pawn rendering interpolates between real simulation positions; it does not change combat state or increase decision frequency. A displayed position can trail physics by up to one simulated tick while running.

## Repeat the check

```bash
cargo run --locked --no-default-features --example profile
RUST_LOG=info cargo run --locked --bin phalanx -- --arena choke --profile
```

The first command reports median, p95, p99, and maximum CPU step time for three complete battles. The second logs live FPS/frame timing. Startup shader/pipeline compilation and transient renderer initialization errors should be distinguished from steady gameplay.

## Live policy check (v0.5)

On the same development desktop, Choke with the bundled policy evaluated beside utility AI remained about 60 FPS (steady diagnostic averages around 16.67 ms/frame). This verifies local responsiveness, not a controller latency benchmark. The forward pass runs natively in Rust at decision time; no Python service blocks gameplay.
