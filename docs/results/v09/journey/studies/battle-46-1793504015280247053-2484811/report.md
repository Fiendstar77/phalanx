# Phalanx battle study

Blue wins · THE CHOKE · seed 46

Duration 77.9s. Survivors: 0 / 2.
Recorded from tick 0: 779 steps and 668 decision records.

## Controller comparison

Utility/PyTorch agreement: 58.8% across 668 valid comparisons; 0 unavailable predictions.
Learned-controller fallbacks: 0. Bonsai does not control this battle.

| Action | Decisions in battle |
| --- | ---: |
| ADVANCE | 173 |
| ATTACK | 327 |
| HIGH GROUND | 0 |
| FLANK | 33 |
| REGROUP | 15 |
| RETREAT | 57 |
| HOLD | 1 |
| SEEK COVER | 5 |
| SUPPORT ALLY | 57 |

## Evidence

[Full battle state](battle.json) · [Machine-readable report](report.json) · [Decision trace](decisions.jsonl)

Decision records contain perceived contacts, features, eligibility masks, baseline scores, controller proposals and the actual executed action. Damage and healing have simulation receipts. If campaign rewards settle, learning.json and learning.md record persisted before/after changes.

## Interpretation

This is one battle. Repeated decisions are correlated, and agreement does not establish better tactics. The PyTorch policy is frozen; campaign habits and memories adapt through explicit rules. Timing measures simulation plus observation capture, excludes rendering and trace disk writes, and does not isolate inference. Interrupted/reset recordings retain complete=false in status.json.
