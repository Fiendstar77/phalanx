# Bonsai tactical shadow study

3 recorded decisions; 0 accepted proposals; 3 utility fallbacks.

The simulation already executed these decisions. Governed actions below are hypothetical; this study changes no battle. Utility/PyTorch labels and score magnitudes were withheld from the prompt. Eligibility comes from existing rules; the model has not learned legality.

| Tick / unit | Actual | Utility | PyTorch | Bonsai governed | Result | Request ms |
| --- | --- | --- | --- | --- | --- | ---: |
| 0 / 0 | ADVANCE | ADVANCE | ADVANCE | ADVANCE | timeout | 1509.0 |
| 245 / 1 | ATTACK | ATTACK | ATTACK | ATTACK | timeout | 1510.0 |
| 565 / 4 | ATTACK | ATTACK | SUPPORT ALLY | ATTACK | timeout | 1510.0 |

[Full report and provenance](report.json). Raw prompts and completions are retained beside it.

Requests reload the model and include loading in their latency. They do not measure a resident worker. Timeout and malformed, extra-field, duplicate-key, unknown or ineligible proposals fall back to utility. This is a small correlated sample, not an independent test set, training, live control or evidence of stronger tactics. Win-rate comparison requires separate controlled rollouts with the model actually choosing actions.
