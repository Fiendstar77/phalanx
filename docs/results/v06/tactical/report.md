# Bonsai tactical shadow study

12 recorded decisions; 12 accepted proposals; 0 utility fallbacks.

The simulation already executed these decisions. Governed actions below are hypothetical; this study changes no battle. Utility/PyTorch labels and score magnitudes were withheld from the prompt. Eligibility comes from existing rules; the model has not learned legality.

| Tick / unit | Actual | Utility | PyTorch | Bonsai governed | Result | Request ms |
| --- | --- | --- | --- | --- | --- | ---: |
| 0 / 0 | ADVANCE | ADVANCE | ADVANCE | ADVANCE | accepted | 2688.0 |
| 40 / 4 | ADVANCE | ADVANCE | HIGH GROUND | SEEK COVER | accepted | 2675.0 |
| 90 / 0 | ADVANCE | ADVANCE | ADVANCE | ADVANCE | accepted | 2717.0 |
| 135 / 3 | ADVANCE | ADVANCE | REGROUP | SEEK COVER | accepted | 2768.0 |
| 180 / 1 | ATTACK | ATTACK | ATTACK | ATTACK | accepted | 2750.0 |
| 220 / 5 | ATTACK | ATTACK | ATTACK | ADVANCE | accepted | 2761.0 |
| 265 / 3 | ATTACK | ATTACK | ATTACK | ADVANCE | accepted | 2795.0 |
| 310 / 1 | ATTACK | ATTACK | ATTACK | ATTACK | accepted | 2780.0 |
| 350 / 5 | ATTACK | ATTACK | ATTACK | ADVANCE | accepted | 2741.0 |
| 400 / 1 | ATTACK | ATTACK | ATTACK | ADVANCE | accepted | 2701.0 |
| 480 / 3 | ATTACK | ATTACK | ATTACK | ATTACK | accepted | 2784.0 |
| 565 / 4 | ATTACK | ATTACK | SUPPORT ALLY | ATTACK | accepted | 2818.0 |

[Full report and provenance](report.json). Raw prompts and completions are retained beside it.

Requests reload the model and include loading in their latency. They do not measure a resident worker. Timeout and malformed, extra-field, duplicate-key, unknown or ineligible proposals fall back to utility. This is a small correlated sample, not an independent test set, training, live control or evidence of stronger tactics. Win-rate comparison requires separate controlled rollouts with the model actually choosing actions.
