# Phalanx battle study

Blue wins · OPEN FIELD · seed 45

Duration 25.8s. Survivors: 0 / 1.
Recorded from tick 0: 258 steps and 104 decision records.

## Controller comparison

Utility/PyTorch agreement: 100.0% across 104 valid comparisons; 0 unavailable predictions.
Learned-controller fallbacks: 0. Bonsai does not control this battle.

| Action | Decisions in battle |
| --- | ---: |
| ADVANCE | 20 |
| ATTACK | 82 |
| HIGH GROUND | 0 |
| FLANK | 0 |
| REGROUP | 0 |
| RETREAT | 2 |
| HOLD | 0 |
| SEEK COVER | 0 |
| SUPPORT ALLY | 0 |

## Evidence

[Full battle state](battle.json) · [Machine-readable report](report.json) · [Decision trace](decisions.jsonl)

Decision records contain perceived contacts, features, eligibility masks, baseline scores, controller proposals and the actual executed action. Damage and healing have simulation receipts. If campaign rewards settle, learning.json and learning.md record persisted before/after changes.

## Interpretation

This is one battle. Repeated decisions are correlated, and agreement does not establish better tactics. The PyTorch policy is frozen; actual practice and event memories persist. Match results grant no temperament or growth multiplier. Timing measures simulation plus observation capture, excludes rendering and trace disk writes, and does not isolate inference. Interrupted/reset recordings retain complete=false in status.json.
