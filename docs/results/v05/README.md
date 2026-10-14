# Live learned controller: one controlled comparison

**Design question:** can a policy trained to imitate utility decisions remain useful when it controls an interactive battle?

The frozen v0.4 PyTorch policy now runs inside the game. Both controllers evaluate the same observation and eligibility mask. Utility remains the default controller, reference decision, and fallback; **C / Utility–Learned** switches which eligible action executes at the next decision. Movement, targeting, damage, and perception retain their simulation rules.

![Controller comparison](comparison.png)

## Protocol

One authored map: **Choke**. Thirty-two fresh seeds **100–131**, each run twice with the learned team on opposite spawn sides: **64 battles**. Both teams have identical seven traits (all 0.5), three melee fighters, and two rock throwers. No retraining or tuning on these seeds. The frozen checkpoint was trained on seeds 0–31 and selected on 32–39; its original offline test used 40–47.

The primary measure was specified by the runner before collection: learned-minus-utility match score (win = 1, draw = 0.5, loss = 0), averaged across both orientations of each seed. The interval uses 4,000 bootstrap resamples of the **32 seed blocks**, preserving paired spawn sides. Simulation version 0.5.0, source commit and SHA-256 fingerprints are recorded in [manifest.json](manifest.json). [battles.csv](battles.csv) contains every outcome and behavioral aggregate; [report.json](report.json) contains the summary.

| Measure | Observed result |
| --- | ---: |
| Learned / utility wins | 37 / 25 |
| Draws, including timeouts | 2 (1 timeout) |
| Mean learned-minus-utility score | +0.188 |
| 95% seed-block bootstrap interval | −0.047 to +0.406 |
| Same-state masked action agreement on learned trajectories | 93.4% |
| Learned-controller fallbacks | 0 |

## Interpretation and limitation

The learned team won more battles in this configuration, but the interval includes zero: **superiority is not established**. Its 93.4% decision agreement is lower than the earlier 97.9% offline test agreement. The populations differ: these are fresh neutral-profile battles on Choke with learned actions changing subsequent states. This comparison does not isolate the cause of that difference.

This is a small exploratory study on one map and one frozen imitation policy. It does not establish generalization across maps, calibrated confidence, human enjoyment, or a generative-AI benefit. Eligibility still comes from utility rules; the policy has not learned action legality. Timeout draws remain in the primary analysis. Decision agreement is descriptive, not thousands of independent experimental observations.

## Live demonstration and reproduction

The inspector shows **UTILITY / PYTORCH / AGREEMENT / EXECUTED**, including the controller that made the last executed decision. Changing modes preserves the battle state. Disagreements are recorded in `outputs/disagreements-*.jsonl`, with seed, arena, run ID, tick, unit, both proposals, executed action, mode, and masked confidence. The session header maps numeric action IDs to names and identifies the checkpoint. A [real desktop log excerpt](runtime-disagreements.jsonl) records disagreements from the shown seed-42 demo; it is separate from the controlled batch. Confidence is an uncalibrated masked softmax score.

The game uses exported PyTorch tensors through an equivalent Rust CPU forward pass; no Python process runs on the gameplay thread. Tests compare actual PyTorch logits and choices on 32 real held-out observations (absolute logit tolerance 0.0002), validate masking/fallback, check deterministic learned replay, and verify that shadow predictions leave utility combat unchanged.

```bash
cargo run --locked --bin phalanx -- --arena choke
# Press C to switch controllers; Utility is the default.
python3 -m analysis.duel --output outputs/my-choke-duel
.venv/bin/python -m analysis.plots --duel outputs/my-choke-duel --output comparison.png
```

The fresh directory contains all 64 full battle summaries. Their hashes are preserved in the published manifest; raw summaries are regenerated rather than added to the portfolio repository.
