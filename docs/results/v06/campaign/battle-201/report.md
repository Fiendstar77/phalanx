# Phalanx battle study

Blue wins · THE BROKEN GATE · seed 201

Duration 67.0s. Survivors: 0 / 2.
Recorded from tick 0: 670 steps and 717 decision records.

## Controller comparison

Utility/PyTorch agreement: 52.0% across 717 valid comparisons; 0 unavailable predictions.
Learned-controller fallbacks: 0. Bonsai does not control this battle.

| Action | Decisions in battle |
| --- | ---: |
| ADVANCE | 220 |
| ATTACK | 431 |
| HIGH GROUND | 0 |
| FLANK | 6 |
| REGROUP | 12 |
| RETREAT | 11 |
| HOLD | 0 |
| SEEK COVER | 0 |
| SUPPORT ALLY | 37 |

## Evidence

[Full battle state](battle.json) · [Machine-readable report](report.json) · [Decision trace](decisions.jsonl)

Decision records contain perceived contacts, features, eligibility masks, baseline scores, controller proposals and the actual executed action. Damage and healing have simulation receipts. If campaign rewards settle, learning.json and learning.md record persisted before/after changes.

## Interpretation

This is one battle. Repeated decisions are correlated, and agreement does not establish better tactics. The PyTorch policy is frozen; campaign habits and memories adapt through explicit rules. Timing measures simulation plus observation capture, excludes rendering and trace disk writes, and does not isolate inference. Interrupted/reset recordings retain complete=false in status.json.
