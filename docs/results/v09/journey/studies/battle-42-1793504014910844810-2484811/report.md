# Phalanx battle study

Blue wins · OPEN FIELD · seed 42

Duration 22.3s. Survivors: 0 / 1.
Recorded from tick 0: 223 steps and 90 decision records.

## Controller comparison

Utility/PyTorch agreement: N/A across 0 valid comparisons; 90 unavailable predictions.
Learned-controller fallbacks: 0. Bonsai does not control this battle.

| Action | Decisions in battle |
| --- | ---: |
| ADVANCE | 16 |
| ATTACK | 74 |
| HIGH GROUND | 0 |
| FLANK | 0 |
| REGROUP | 0 |
| RETREAT | 0 |
| HOLD | 0 |
| SEEK COVER | 0 |
| SUPPORT ALLY | 0 |

## Evidence

[Full battle state](battle.json) · [Machine-readable report](report.json) · [Decision trace](decisions.jsonl)

Decision records contain perceived contacts, features, eligibility masks, baseline scores, controller proposals and the actual executed action. Damage and healing have simulation receipts. If campaign rewards settle, learning.json and learning.md record persisted before/after changes.

## Interpretation

This is one battle. Repeated decisions are correlated, and agreement does not establish better tactics. The PyTorch policy is frozen; campaign habits and memories adapt through explicit rules. Timing measures simulation plus observation capture, excludes rendering and trace disk writes, and does not isolate inference. Interrupted/reset recordings retain complete=false in status.json.
