# Code and test walkthrough

This map ties engineering claims to implementation and falsifiable checks.
Tests cover the stated fixtures and interfaces; they do not certify general
tactical intelligence or player enjoyment.

## Trace one decision

Start with [the published observations](../results/v06/tactical/observations.json).
Each entry contains the exact allowlisted payload sent to Bonsai, its raw
completion, the governor result, and reference choices that were withheld from
the prompt. `trace_line` points to the corresponding record in
[the source decision trace](../results/v06/campaign/battle-200/decisions.jsonl).

The sample at tick 135, unit 3 contains remembered enemy information with
`enemy_visible=0` and reduced contact confidence. The utility reference advances;
Bonsai proposes seeking cover. Both are eligible. The log permits inspection
of that disagreement; it does not tell us which would have produced a better
battle. No explanation generated after the choice is treated as its cause.

Follow the computation through these files:

| Question | Source | Check to inspect |
| --- | --- | --- |
| What could the fighter know? | [perception.rs](../../src/perception.rs): `update_in_field`, `Contact::confidence` | [Perception tests](../../tests/perception_and_coordination.rs): direct-sight limits, relay source, expiration, unseen-enemy counterfactual |
| How do teammates affect a choice? | [cognition.rs](../../src/cognition.rs): `focus_target`, `support_ally`, `flank_position` | Same test file: cooperative focus, known wounded ally, engaged-ally flank |
| How does personality change movement? | [navigation.rs](../../src/navigation.rs): `edge`, `plan_known` | [Terrain tests](../../tests/terrain_and_rocks.rs): cautious routes differ; walls constrain travel and shots |
| Which actions can execute? | [lib.rs](../../src/lib.rs): guarded scores, mask, utility/learned selector | [learned.rs tests](../../tests/learned.rs): eligibility, replay and missing-policy fallback |
| Are inputs recorded before the consequence? | [telemetry.rs](../../src/telemetry.rs): `DecisionRecord`, `FEATURE_NAMES` | [Telemetry tests](../../tests/telemetry.rs): recording equivalence and pre-choice flank-state features |
| Are displayed outcomes physical? | [lib.rs](../../src/lib.rs), [theus_combat.rs](../../src/theus_combat.rs) | [Theus](../../tests/theus.rs), [weapons](../../tests/weapons.rs), [healing](../../tests/healing.rs): real costs, hits and restoration |

The simulation timestep is 0.1 seconds; tactical evaluation occurs every five
ticks. Comparison records identify the controller responsible for the executed
decision. Changing the UI's controller affects later choices rather than
rewriting earlier records.

## Review the learned policy

| Stage | Source and artifact | What to check |
| --- | --- | --- |
| Collect | [experiments.py](../../analysis/experiments.py), [v0.4 manifests](../results/v04/README.md) | Seed/side/map configuration, source/binary hashes, complete traces |
| Split | [train.py](../../analysis/train.py): `load` | Whole seeds split before normalization; reject inconsistent schemas and learned-controller teacher headers |
| Fit | Same file: `run`, `Policy` | Train-only mean/std; masked class-weighted cross entropy; AdamW; validation checkpoint selection |
| Evaluate | [metrics.json](../results/v04/policy/metrics.json) | Held-out membership, per-class support, confusion matrix, simple baselines, raw eligibility and timing scope |
| Export | [export.py](../../analysis/export.py), [bundled policy](../../assets/policy.json) | Actual tensors, normalization, feature/action order and source checkpoint hash |
| Execute | [learned.rs](../../src/learned.rs) | Bounded schema, dimensions, finite values, ReLU placement, mask and unavailable prediction |
| Verify | [learned tests](../../tests/learned.rs), [PyTorch witness](../../assets/policy_witness.json) | Cross-language logits within 0.0002, equal masked choice, deterministic replay, utility fallback |

The 29 → 64 → 64 → 9 architecture has
`(29×64+64) + (64×64+64) + (64×9+9) = 6,665` parameters.
Training occurs offline. The Rust controller applies the trained weights; it
does not call a Python service or update them online.

The current loader checks each trace header's controller for utility teacher
data. It is intended for the experiment collector's single-controller traces.
Do not assume this makes arbitrary interactively switched game traces valid
training data: review per-decision controller records and the collection
manifest before admitting another source.

The optional `founders_schema_two_matches_python_logits_on_held_out_rows` test
is ignored by default because it reads local experimental outputs. It is not
part of the bundled-policy verification claim.

## Review persistent adaptation

Read [campaign settlement](../../src/progression.rs),
[conditioned reaction rules](../../src/psyche.rs),
[class/weapon practice](../../src/mastery.rs) and
[rival adaptation](../../src/rivals.rs). Then compare a captured fighter across
[learning.json](../results/v06/campaign/battle-200/learning.json),
[the next battle's setup](../results/v06/campaign/battle-201/decisions.jsonl) and
[the final campaign](../results/v06/campaign/campaign-after.json).

The important checks are more precise than “the squad learns”:

- [Progression tests](../../tests/progression.rs) check once-only settlement,
  bounded habits, persistent identity, contrasting personality-specific scars,
  and an actual decision difference caused by conditioned reactions.
- [Study tests](../../tests/study.rs) check receipt serialization, legacy saves,
  invalid receipt rejection, save/load continuity and the next battle's state.
- [Study recording](../../src/study.rs) calls the same recorded simulation step.
  A test compares its final battle with unrecorded execution.
- Interrupted recordings retain `complete=false`. A complete-looking result
  cannot be inferred from a directory merely existing.

The visible [post-battle receipt code](../../src/learning.rs) describes saved
deltas and practice thresholds. It does not ask a language model to invent a
story about why a fighter improved. XP/stat progression, habit adaptation and
learned-policy action selection should be evaluated separately.

## Review the generative boundary

[Native runner](../../tools/bonsai_smoke.py) →
[prompt construction and governor](../../tools/bonsai_tactics.py) →
[contract tests](../../analysis/test_bonsai.py) →
[native evidence](../results/v06/tactical/report.json).

The contract tests deliberately supply an ineligible action, an unknown action,
fenced/non-JSON text, extra fields, duplicate keys, timeout, runtime failure and
a missed deadline. They require utility fallback. They also inject hidden state
and teacher scores into the source trace and check that only allowlisted
observation features and eligible actions enter the prompt.

These are model-free interface tests. The published native experiment separately
proves that the real local model was called. Its accepted proposals remain
shadow proposals. A valid action cannot establish a beneficial decision, and
the process-per-request timings cannot establish resident-worker latency.

## Run the checks

From the repository root, the first commands need Python 3.11+ and Rust, but no
graphics libraries, PyTorch installation or generative-model weights:

```bash
python3 tools/check_ai_evidence.py
python3 -m unittest analysis.test_analysis analysis.test_bonsai analysis.test_ai_evidence
cargo test --locked --no-default-features
cargo clippy --locked --no-default-features --all-targets -- -D warnings
```

Cargo may fetch locked dependencies on a first build. Add `--offline` when they
are already cached. With the source commit available locally, check its captured
file hashes as well:

```bash
python3 tools/check_ai_evidence.py --verify-source
```

For graphics, install the [Linux prerequisites](../../README.md#running-the-project),
then run `cargo test --locked --bin phalanx`. For actual PyTorch checkpoint reload
and export, follow the [analysis setup](../../analysis/README.md). The CI workflow
separates simulation, graphics, Python analysis/evidence and PyTorch smoke jobs;
it does not download Bonsai weights or run native Bonsai generation.

## Reproduce an experiment

- [Campaign studies and automatic game reports](../ai-studies.md) explain the
  artifacts and isolated three-battle runner.
- [Native setup](../bonsai.md) pins the tested model and runtime revisions.
- [v0.6 capture manifest](../results/v06/manifest.json) records source, commands,
  machine, settings and original/published file hashes.
- [v0.4 training](../../analysis/README.md) and the [v0.5 duel protocol](../results/v05/README.md)
  explain their historical snapshots and splits.

Fresh campaign identities and measured wall times can differ on repetition.
Historical estimates refer to the recorded source versions; executing today's
simulator does not silently recreate yesterday's experiment. The checked-in
observations and campaign snapshots make the original evidence inspectable.
