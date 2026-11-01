# Phalanx battle study

Draw · THE CHOKE · seed 47

Duration 106.2s. Survivors: 0 / 0.
Recorded from tick 0: 1062 steps and 885 decision records.

## Controller comparison

Utility/PyTorch agreement: 61.5% across 885 valid comparisons; 0 unavailable predictions.
Learned-controller fallbacks: 0. Bonsai does not control this battle.

| Action | Decisions in battle |
| --- | ---: |
| ADVANCE | 209 |
| ATTACK | 388 |
| HIGH GROUND | 0 |
| FLANK | 29 |
| REGROUP | 38 |
| RETREAT | 86 |
| HOLD | 0 |
| SEEK COVER | 26 |
| SUPPORT ALLY | 109 |

## Evidence

[Full battle state](battle.json) · [Machine-readable report](report.json) · [Decision trace](decisions.jsonl)

Decision records contain perceived contacts, features, eligibility masks, baseline scores, controller proposals and the actual executed action. Damage and healing have simulation receipts. If campaign rewards settle, learning.json and learning.md record persisted before/after changes.

## Interpretation

This is one battle. Repeated decisions are correlated, and agreement does not establish better tactics. The PyTorch policy is frozen; campaign habits and memories adapt through explicit rules. Timing measures simulation plus observation capture, excludes rendering and trace disk writes, and does not isolate inference. Interrupted/reset recordings retain complete=false in status.json.
