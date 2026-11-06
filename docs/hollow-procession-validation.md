# Hollow Procession validation - 2026-11-06

This records local verification of the first larger location and movement pass.
It is not hosted CI, a performance benchmark or proof that the character art is
finished.

- Full headless suite: **207 passed**.
- Graphics binary unit tests: **80 passed**, including rotated/scaled leg IK,
  held-pose restoration and the 4K surface's filtered mip chain and sampled heights.
- Strict headless Clippy (`--all-targets -- -D warnings`), formatting and diff checks passed.
- Four-fighter campaign seeds 6, 42 and 72 completed with actual damage and exact
  serialized replay equality. Recorded start, combat frames and end retained the
  location context. Measured recorded steps averaged 0.17-0.25 ms locally, with
  observed maxima of 8-38 ms; this excludes rendering and trace disk writes.
- Save/load and return tests preserved fighter identities, geography and breached
  timber. Their persistence fixture sets a resolved breach directly; existing
  procedural tests separately exercise destruction by actual combat. Settlement
  still gives equal gold/salvage and cannot award a completed encounter twice.
- Serial rendered `--region-witness` runs inspected overview, close and movement
  captures. The final staged fixture reached tick 201 with 11,618 ankle correction
  applications, maximum correction below 0.27 m, and zero invalid poses. These
  counts are frame applications, not distinct footsteps. Paused rendering retained
  byte-identical serialized Battle state.
- The player campaign save SHA-256 was unchanged across those isolated fixtures:
  `a149f1f10ed5e30701b008300c72569940995504311bc5f16a4bba0f516dfa3e`.

Local render artifacts are in `outputs/hollow-procession/`. The normal campaign
and isolated demonstration remain different verification paths. The normal saved
4v4 roster was also opened and its in-game scene inspected; a single audio buffer
underrun was logged at launch. The world is a
bounded location with sealed gates; reload returns to encounter preparation.
Autonomous world travel and connected locations are future work.
