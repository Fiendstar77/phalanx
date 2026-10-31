# Working on PHALANX

Read [the Core Player-Fantasy and Development Law](PHALANX_CORE_PLAYER_FANTASY_AND_DEVELOPMENT_LAW.md)
before changing gameplay, progression, AI, telemetry or monetization. It defines
the intended game; it is not a claim that every described system is implemented.
Use [the implementation status](docs/development-law-status.md) to distinguish
working foundations from future work.

- Keep Theus responsible for capability and consequence. Autonomous planners and
  learned or generative models choose or propose actions within those rules.
- Extraordinary acts must execute legal primitives against actual world state.
  Record proposals, interruptions and resolved consequences separately.
- Derive history and epithets from witnessed events. Preserve the underlying
  receipts; do not rank legendary lives with an arbitrary aggregate score.
- Preserve persistent identities and earned history. Cosmetic epithets confer
  no combat bonus. Any real-money monetization must remain cosmetic; earned
  in-game gold is a separate progression currency.
- Favor interacting systems, behavioral believability, attachment and readability.
  Report measured behavior and verification honestly, including staged fixtures
  and the limits of local tests, model experiments and hosted CI.

For Rust changes, run focused tests, `cargo fmt --check`, and strict headless
Clippy. Run the affected graphics tests when UI or presentation changes.
Keep Founders' 3v3 then 4v4 progression distinct from the five sequential NPC
tutorial teams and the original Lab rosters.
