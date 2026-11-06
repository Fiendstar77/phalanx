# Personal memory and companion brain - 2026-11-06

The campaign now has a 57-input PyTorch policy (57 -> 128 -> 128 -> 128 -> 9;
41,609 parameters), exported to `assets/policy_context.json` for native Rust
inference. The original 29-input policy remains the Lab baseline. Utility remains
the default campaign controller; the new policy supplies shadow comparisons and
the existing optional Learned mode. Neither policy can waive Theus eligibility.

The inputs include directional bonds and recognized rivals, practiced habits,
effort/injury, communicated hound state, aerial eligibility and equipment effects.
Unknown companion health/position is masked by report freshness. Opponent inputs
still come from the recipient's contacts. Companion orders and aerial proposals
are currently rule-based autonomy, not a learned companion or maneuver planner.

## Local experiment

The research fixture contains 48 completed bouts: two episodes for each of 24
independent four-fighter campaigns, with staged training, recipe eligibility and
equipment. Each squad keeps a Houndwarden and Healer; the other two slots rotate
through all 19 classes across the corpus, including ranged weapons. This replaces
the first melee-heavy experiment.
Each seed and its persistent identities belong to only one split. Seeds 1-16
train, 17-20 validate, and 21-24 test the imitation policy. The temporal study
uses its own recorded seed/identity split. These are fixture data, not human play.

The training view retains 77,783 guarded Utility decisions. It explicitly excludes
256 Opportunity-controller decisions from imitation labels; those receipts remain
in the original studies and causal forecasting corpus. The revised hound has
shorter reach, lower sight and reduced ground speeds. The exported model passed
Rust/PyTorch logit and masked-action parity checks.

| Masked imitation on held-out campaigns | Accuracy | Macro-F1 |
| --- | --- | --- |
| Previous bundled policy, same new test cases | 52.18% | 0.347 |
| Retrained 29-input ablation, same width/depth/epochs | 84.90% | 0.572 |
| New 57-input policy | 90.73% | 0.672 |

The comparison with the old bundle changes training data and architecture as well
as features. The 29-input ablation holds training configuration fixed. The gain in
overall accuracy does not establish stronger combat, mastery of rare actions or
reliable generalization beyond these campaigns. Per-action supports and confusion
matrices are retained in the JSON reports. Raw and masked eligibility are distinct.

A separate 6,825-parameter causal model consumes two earlier private observations
and actions. On its held-out samples, accuracy was 91.1%, versus 91.3% action
persistence; NLL was 0.296 versus the Markov model's 0.365. It did not beat persistence
on overall accuracy. It remains research-only: these are the actor's own earlier
observations,
not an opponent-visible model, online learning or combat superiority evidence.

## Reproduce

Use fresh output directories:

```sh
cargo run --locked --no-default-features --example social_corpus -- outputs/context-corpus
.venv/bin/python -m analysis.prepare_context_training outputs/context-corpus outputs/context-view
.venv/bin/python -m analysis.train --data outputs/context-view --output outputs/context-policy --train-end 17 --val-end 21 --epochs 35 --hidden 128 --depth 3
.venv/bin/python -m analysis.export outputs/context-policy/policy.pt outputs/context-policy/policy.json
.venv/bin/python -m analysis.train --data outputs/context-view --output outputs/context-29 --train-end 17 --val-end 21 --epochs 35 --hidden 128 --depth 3 --input-prefix 29
.venv/bin/python -m analysis.cognition --data outputs/context-corpus --output outputs/context-forecast --train-end 17 --val-end 21 --epochs 20 --history 2 --max-corpus-mib 1536
```

The corpus generator never accesses the player's campaign save. Raw studies,
checkpoints and rendered fixtures remain local under `outputs/`; small metrics,
portable weights and reload witnesses are in the repository. Hosted CI and a human
comparison of controller quality are separate from these local results.
