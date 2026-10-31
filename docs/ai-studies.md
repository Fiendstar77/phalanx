# Study the AI, then watch the squad change

Phalanx is an AI systems testbed with a playable campaign. Its evidence has
three distinct layers:

| System | What changes | Evidence |
| --- | --- | --- |
| Campaign adaptation | Persistent experience, class/weapon practice, habits, conditioned memories and rival counters | Before/after learning receipts, save/load and the next battle's fighter state |
| PyTorch policy | Frozen trained policy proposes an eligible action | Same-state Utility/PyTorch comparison, raw decision records and controlled duel results |
| Bonsai experiment | Native language model proposes text or an action on a recorded observation | Raw prompt/completion, validity, fallback reason, deadline, model/runtime hashes |

Campaign adaptation is implemented with explicit rules. It is not online
gradient training. Neither the bundled PyTorch weights nor Bonsai weights
change during play. Adaptation can include fear and bad habits; more experience
does not establish better tactics.

## See growth in the game

Finish a Founders battle. The post-battle screen opens automatically and now
shows **YOUR PHALANX IS CHANGING**. Each fighter's receipt explains actual
before/after changes: supporting a teammate reinforces teamwork, a costly
battle increases self-preservation, and remembered isolation or ally loss can
condition later reactions. Class practice shows progress toward the next rank.

These are the same persisted values used by subsequent battles. The screen
does not invent a lesson from generated prose. Older saves without receipts
still load; new battles record them. The normal 3v3 to 4v4 progression remains
unchanged; five tutorial teams are sequential opponents.

## Automatic battle reports

Starting combat, including single-stepping, creates a unique directory under
`outputs/studies/`. A completed battle saves:

| File | Contents |
| --- | --- |
| `report.md` | Readable outcome, action counts and Utility/PyTorch agreement |
| `report.json` | Machine-readable counts, missing predictions, fallbacks and simulation-step timing |
| `decisions.jsonl` | Initial setup; exact decision-time perceived contacts, features, eligibility, scores, controller proposals and executed actions; damage/healing receipts; final result |
| `battle.json` | Full final simulation state |
| `learning.md`, `learning.json` | Campaign settlement and each fighter's actual before/after adaptation, when campaign rewards settle |
| `status.json` | `complete=true` only after the battle and report files finish successfully |

The in-game battle report shows the directory. Reports are independent of the
player save. Exiting or resetting an unfinished battle leaves an incomplete
recording, not a completed result. A recording error is logged and does not stop
combat; incomplete artifacts should not enter an outcome study. Trace output
is buffered, so an interrupted process can lose its final buffered records.

The raw initial setup and the final state contain world information for auditing.
Decision features and contacts reflect what the acting fighter perceived.
Do not feed the complete world snapshot or future outcomes to a partial-observation
controller. Interactive controller switches are visible in individual decision
records. Changes made interactively can prevent exact reproduction from the
initial setup alone.

Reports distinguish a missing prediction from disagreement. Step timings measure
simulation plus observation capture, excluding rendering and trace disk writes;
they are not isolated PyTorch or Bonsai inference measurements. Decisions within
a battle are correlated. Use independent battle/seed blocks for outcome uncertainty.

## Run an isolated evolution study

This command runs three real 3v3 battles in a new campaign, saving and reloading
the campaign between them. It never reads or writes `outputs/phalanx-save.json`.
Choose a fresh output directory for each study:

```bash
cargo run --offline --locked --no-default-features --bin phalanx-study -- \
  --battles 3 --seed 200 --output outputs/learning-study
```

Open `outputs/learning-study/report.md`. It links each battle study and learning
receipt. `campaign-before.json` and `campaign-after.json` let you inspect the
persistent changes. `--controller learned` instead executes the frozen PyTorch
policy on both teams, preserving utility as eligibility authority and fallback.
This command studies persistent campaign evolution; it is not a counterbalanced
test of controller superiority. The earlier [controlled duel](results/v05/README.md)
documents that separate protocol.

## Ask Bonsai about actual recorded decisions

With the [local native prerequisites](bonsai.md) installed, choose one completed
trace from a game or isolated study:

```bash
python3 tools/bonsai_smoke.py \
  --trace outputs/learning-study/BATTLE_DIRECTORY/decisions.jsonl \
  --samples 12 --budget-ms 1500 --output outputs/bonsai-tactics
```

Replace `BATTLE_DIRECTORY` with a directory listed by the study. This samples
decision indices uniformly across the trace and asks Bonsai to select an action
using the same 29 observation features and the rule-provided eligibility mask.
Utility scores, reference choices, targets, hidden transforms and future events
are excluded from the prompt. Floating point inputs are rounded to five decimal
places for the prompt; the original trace retains full values. The model sees
action descriptions in addition to those feature values.

The only accepted response is an exact one-key JSON object such as
`{"action":"SEEK COVER"}`. Malformed JSON, duplicate keys, extra fields, unknown
or ineligible actions, runtime failure, timeout or a missed deadline select the
recorded utility fallback. No repair step silently converts invalid output into
a valid proposal. Raw completions remain available for inspection.

`report.md` compares actual, Utility, PyTorch and hypothetical Bonsai-governed
choices. `report.json` records provenance, accepted-only agreement denominators,
missing learned references, timing and per-case fallback reasons. The command
exits successfully when a complete tactical study is written, even if every
proposal falls back; acceptance rate is an observed result, not hidden by a
test pass label. Each request reloads the model, so latency includes loading.

This is **shadow evaluation**: Bonsai does not change the recorded battle.
Agreement, eligibility or a fluent explanation does not demonstrate stronger
play. A later resident worker and controlled rollouts must establish its effect
on outcomes, latency and player experience before live tactical integration is
claimed.
