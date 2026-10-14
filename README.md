# PHALANX

**Terrain-aware autonomous combat simulation and an experimental testbed for game AI.**

Players shape battlefields and agent personalities, then observe autonomous teams fight. PHALANX explores how terrain, individual psychology, and teamwork affect collective behavior—and how to make those decisions explainable and measurable.

> **Status: design foundation.** This repository currently contains the project specification and evaluation plan. The Rust/Bevy simulation, Python analysis tools, and optional generative controller are planned; there is no playable build or measured result yet.

## The experience

Build an arena → shape two teams' minds → run a battle → inspect decisions → change one variable → repeat.

Seven personality traits influence tactical utility: aggression, caution, courage, teamwork, patience, self-preservation, and discipline. The intended inspector exposes candidate action scores and the state behind each decision.

## Research question

**Does selective generative reasoning improve autonomous tactics enough to justify its latency and computational cost?**

The project first establishes deterministic combat and terrain-aware navigation. A later local LLM may propose an action from a validated set when tactical choices are ambiguous. The simulation retains authority over movement, damage, and world state.

## Planned stack

- **Rust / Bevy:** interactive simulation, tactical navigation, combat, and agent inspection.
- **Python:** sequential telemetry analysis, statistical comparisons, and experiment reporting.
- **Optional local LLM:** structured tactical proposals, legality validation, deterministic fallback.
- **PyTorch:** a later learned-policy baseline or training experiment, only once real simulation data and an evaluation pipeline exist.

No model has been trained or integrated at this stage.

## Milestones and acceptance criteria

| Milestone | Deliverable | Evidence required |
| --- | --- | --- |
| 1 — Make it alive | 3D arena, 5 vs 5, utility AI, run/pause/reset, clickable inspector | A recorded demo showing personality changes affecting decisions |
| 2 — Make terrain matter | Cover, elevation, terrain costs, tactical paths, arena presets | Controlled demonstrations on ridge, choke, and open-field maps |
| 3 — Make behavior measurable | Seeded batches, JSON telemetry, Python analysis | Reproducible comparisons with uncertainty and limitations |
| 4 — Evaluate generative cognition | Selective LLM proposals, validation, fallback | Baseline comparison including quality, latency, and cost |

## Evaluation approach

Compare simple heuristics, utility AI, and utility AI with selective LLM reasoning on matched maps and seeds. Swap team spawn sides to reduce positional bias. Record outcomes and decision sequences; treat battles as the experimental unit rather than treating every tick as independent evidence.

Measure win rate, survival, cohesion, cover use, action validity, decision latency, and LLM calls/tokens. Predefine success criteria, report confidence intervals, and separate exploratory observations from held-out evaluations. Human playtests should assess readability and player experience separately from combat performance.

See [the evaluation protocol](docs/evaluation.md) and [the full design brief](docs/design-brief.md).

## Relevance to interactive AI research

PHALANX is designed to connect game-design questions to measurable system behavior: translate a design need into an experiment, expose agent decisions, analyze sequential data, and evaluate whether a generative component helps the experience. It is an independent portfolio project, with no Ubisoft affiliation.

## Running the project

There is no executable yet. This initial publication documents the scope and evaluation protocol; implementation starts with milestone 1. Future run instructions will accompany a verified build.

## Author

Guillaume Heinerth · [GitHub](https://github.com/Fiendstar77)

> Do not command the phalanx. Build the minds and terrain from which the phalanx emerges.
