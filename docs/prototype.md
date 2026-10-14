# Prototype 0.1

## Simulation model

The core in `src/lib.rs` is independent of Bevy. Every tick advances exactly 0.1 simulated seconds. Living units select actions every five ticks. Each decision scores advance, attack, high ground, flank, regroup, retreat, and hold; negative scores exclude an action from selection.

Aggression and courage increase advance/attack utility. Caution, patience, and discipline increase ridge utility. Teamwork and ally isolation affect regroup utility. Injury, self-preservation, caution, and local enemy concentration affect retreat utility. The formulas and gating conditions are explicit in the source. Small stateless noise is keyed by seed, tick, unit ID, and action.

Each unit starts with 100 health, 10 base attack damage, 1.5 attack range, a 0.8-second cooldown, and 2.2 base movement speed. Elevation adds up to a modest positional damage advantage and slows uphill movement. Damage is accumulated before updating health so both sides can land fatal attacks in the same tick.

The battle ends when a team is eliminated or at 120 simulated seconds. Timeouts are reported as draws rather than assigned a winner.

## Current limits

- Full enemy knowledge; no occlusion or visibility range.
- Direct target movement; no obstacles, navmesh, cover, or tactical route search.
- Separation only between allies; no physical collision system.
- Initial presets change multiple traits, so their comparison does not isolate teamwork or caution.
- Scores represent the most recent decision, not continuously recomputed explanations.
- Reset discards individual mind edits. Arbitrary setup saving and a seed editor are not implemented.
- JSON is a battle summary, not a sequential event dataset. No scientific or player-experience conclusions follow from it.
- No LLM, learned controller, Python evaluation, or model training yet.

## Local validation

Four simulation tests verify same-seed replay, a personality-dependent opening choice, simultaneous fatal attacks, and finite terminal state across 20 seeds with both profile orientations. Local checks also include formatting, full graphics compilation, and Clippy.

The application has been opened on the development desktop and visually inspected. Live checks confirmed a single 0.1-second step per click, unit selection, run/pause/reset, personality presets, and individual trait edits. A 10-second recording and an inspector screenshot are included in `docs/images/`. The development machine uses an NVIDIA Vulkan renderer; transient swapchain validation errors occurred during renderer startup, although the arena rendered afterward. Other platforms have not been runtime-tested.

The sample seed 42 is illustrative only. It cannot establish that either profile is superior. The evaluation protocol remains a draft for later controlled experiments.
