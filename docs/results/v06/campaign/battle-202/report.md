# Phalanx battle study

Red wins · THE BROKEN GATE · seed 202

Duration 78.8s. Survivors: 2 / 0.
Recorded from tick 0: 788 steps and 725 decision records.

## Controller comparison

Utility/PyTorch agreement: 66.3% across 725 valid comparisons; 0 unavailable predictions.
Learned-controller fallbacks: 0. Bonsai does not control this battle.

| Action | Decisions in battle |
| --- | ---: |
| ADVANCE | 306 |
| ATTACK | 323 |
| HIGH GROUND | 12 |
| FLANK | 3 |
| REGROUP | 25 |
| RETREAT | 33 |
| HOLD | 1 |
| SEEK COVER | 0 |
| SUPPORT ALLY | 22 |

## Evidence

[Full battle state](battle.json) · [Machine-readable report](report.json) · [Decision trace](decisions.jsonl)

Decision records contain perceived contacts, features, eligibility masks, baseline scores, controller proposals and the actual executed action. Damage and healing have simulation receipts. If campaign rewards settle, learning.json and learning.md record persisted before/after changes.

## Interpretation

This is one battle. Repeated decisions are correlated, and agreement does not establish better tactics. The PyTorch policy is frozen; campaign habits and memories adapt through explicit rules. Timing measures simulation plus observation capture, excludes rendering and trace disk writes, and does not isolate inference. Interrupted/reset recordings retain complete=false in status.json.
