# Phalanx battle study

Red wins · THE RIDGE · seed 44

Duration 65.3s. Survivors: 1 / 0.
Recorded from tick 0: 653 steps and 365 decision records.

## Controller comparison

Utility/PyTorch agreement: 53.7% across 365 valid comparisons; 0 unavailable predictions.
Learned-controller fallbacks: 0. Bonsai does not control this battle.

| Action | Decisions in battle |
| --- | ---: |
| ADVANCE | 89 |
| ATTACK | 206 |
| HIGH GROUND | 31 |
| FLANK | 1 |
| REGROUP | 6 |
| RETREAT | 32 |
| HOLD | 0 |
| SEEK COVER | 0 |
| SUPPORT ALLY | 0 |

## Evidence

[Full battle state](battle.json) · [Machine-readable report](report.json) · [Decision trace](decisions.jsonl)

Decision records contain perceived contacts, features, eligibility masks, baseline scores, controller proposals and the actual executed action. Damage and healing have simulation receipts. If campaign rewards settle, learning.json and learning.md record persisted before/after changes.

## Interpretation

This is one battle. Repeated decisions are correlated, and agreement does not establish better tactics. The PyTorch policy is frozen; campaign habits and memories adapt through explicit rules. Timing measures simulation plus observation capture, excludes rendering and trace disk writes, and does not isolate inference. Interrupted/reset recordings retain complete=false in status.json.
