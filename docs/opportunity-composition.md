# Opportunity Composition and the Legend Ledger

> Characters do not have legendary abilities. They become legendary because of things they actually managed to do.

This is the design direction for Phalanx: an exceptional fighter can assemble a
short plan from abilities and geometry, commit to it, and live with the actual
consequences. The player has no “legendary move” button. The history records an
accomplishment after the world resolves it, not after a plan sounds impressive.

It implements a first slice of the [governing development law](../PHALANX_CORE_PLAYER_FANTASY_AND_DEVELOPMENT_LAW.md).
The [status assessment](development-law-status.md) maps that broader intent to
working foundations and remaining systems.

The Black Bridge scene—a wounded Scout wall-running, catching a chain,
striking a Guardian off the bridge and collapsing—is the target experience.
The first implementation below supports a smaller, real subset. Hanging chains,
ledge throws, bonded rescues and independent catastrophic-success consequences
are explicitly later stages.

## Working first slice

[The planner](../src/opportunity.rs) searches over three primitives:

| Primitive | Preconditions and effect |
| --- | --- |
| RUN | A currently clear walkable segment. Uses normal navigation, speed, terrain, collision and movement; no teleport or additional movement allowance. |
| WALL VAULT | Wall dance equipped; two usable legs; Acrobatics rating at least 12; wall cooldown and binding allow it; a solid-clear trajectory and walkable landing exist; competing vaults remain separated; costs 7 actual effort. |
| STRIKE | Current visible contact, legal attack range and clearance through the existing attack guard. Normal wind-up, opposed Theus rolls, armour and consequences resolve the strike. Ordinary strikes cost 1.5 effort; the existing cooperative resolver may choose a separately eligible combination. |

Composition becomes available to a melee fighter with Agility at least 12,
Acrobatics rank 1 (100 practice), Wall dance and at least one usable hand.
It is not restricted to a special command title. A missing arm can therefore
coexist with a plan; an unfitted missing leg excludes the wall primitive.
Practice and prepared equipment persist through the existing campaign.

The same search produces `RUN → STRIKE` on open ground and can produce
`RUN → WALL VAULT → RUN → STRIKE` when an opaque wall blocks the direct route.
There is no stored “vaulting ultimate” action. Ground approach candidates come
from the target's attack envelope and wall faces; vault edges come from the
same checked geometry used by the live movement system.

The search is deliberately bounded: at most four primitives, 32 node expansions,
one wall vault per plan and eight accepted composition attempts per fighter per
battle. Movement cost and injury/caution-weighted vault cost order the frontier;
effort is tracked separately as a hard minimum budget. This limited search is
not complete over all navigable routes and is not a proof of a globally optimal
plan. It adds no new training or generative-model dependency.

## Prediction is not execution

Planning reads the fighter's contact position, not the enemy's hidden transform.
Terrain is the existing known arena. A contact remembered behind a wall can
motivate a traversal; it cannot authorize a strike through the wall. The contact
used for planning can be from the preceding perception update. Attack execution
still uses the current perception and eligibility checks.

An active plan binds its intended contact and supplies a normal tactical action
and physical goal at each step. Telemetry labels those executed decisions
`Opportunity`, while retaining the utility and PyTorch reference proposals.
The normal controller remains available when a primitive cannot supply a legal
action. The imitation trainer rejects non-utility decisions even inside a trace
whose session header says Utility.

Before a primitive starts, the runtime rechecks equipment, limbs, remaining
effort, contact availability, ground clearance and vault eligibility/geometry.
A vault must also pass the existing competing-trajectory reservation checks.
The normal world systems execute it. A run completes only at an observed
endpoint; a vault completes only after its actual flight lands. A strike
completes only with an actual positive-damage melee Theus receipt against the
planned target, after an opportunity decision actually selected the strike.
An unrelated projectile cannot complete it. A miss or successful defence cancels
that attempt.

New injuries, lost contacts, exhausted effort, blocked paths, unavailable
surfaces, actor death or the 120-tick plan budget can interrupt it. The end of a
battle cancels unfinished plans. Success never means guaranteed damage,
invulnerability or a retroactive rewrite of a failed step. Reactive arts and
other combat can still consume effort or change the situation.

## A ledger of witnessed events

[The Legend Ledger](../src/legend.rs) considers only completed compositions
containing a landed vault and real strike. A qualifying event starts at no more
than 30% health, starts with an unfitted missing limb, or produces an actual
Theus critical. This is a provisional detector, not a legendary-value score.
At most one is recorded per fighter per battle: the first qualifying moment in
chronological completion order. All completed compositions remain in the battle
and study receipts, including moments not selected for the profile ledger.

An entry stores:

- Battle, seed, source build, stable fighter/opponent identities and terrain.
- Plan primitives, actual start/completion ticks and measured horizontal travel.
- Starting health/wounds and final battle health.
- The vault receipt and its observed landing tick.
- Actual strike sequence, attack/defence dice, outcome, critical and damage.

Distance is accumulated from authoritative positions after each simulation
step, rather than from the projected route or requested movement. It measures
horizontal travel sampled at the world timestep, not the full 3D arc length.
Final battle health is deliberately distinguished from health at the impact.

The initial narration is deterministic and conservative: “traveled …, completed
a wall vault and struck …; finished the battle with … HP/downed.” It does not
claim a rescue, projectile interception, railing break, a location name that
does not exist, or a survival probability that was never measured.

The first earned epithet is **Wallrunner**. It is a display name derived from a
retained witnessed event and has zero mechanical effect. It is separate from
King, Queen, General and other command titles. Fighters retain the latest 32
entries plus a lifetime recorded-event count. Settlement awards entries once;
recruitment and fresh authored rivals clear inherited history.

## What the player can see and study

The inspector's decision reason shows the actual proposed primitive sequence.
A living fighter can bark after a completed wall composition; the bark follows
the receipt, not the proposal. Existing wall-kick animation and world motion
present the traversal. The post-battle report lists earned moments and epithets,
and the fighter's workshop profile exposes retained history.

Automatic studies add opportunity proposal/completion/cancellation records to
`decisions.jsonl`. `report.json` includes attempt and completion counts.
After campaign settlement, `legends.json` and `legend-ledger.md` preserve the
newly recorded moments alongside learning receipts. Full plans and completed
world witnesses also remain in `battle.json`.

[Browse the staged execution, rendered report and complete receipts](results/v07/README.md)
without installing the game. The fixture's grants and suppressed opponent
attacks are disclosed alongside the evidence.

No UI control activates a named ultimate, and no new authored animation
pretends to be a discovered physical route. The current movement vocabulary is
small and uses existing clips; broader visual variation and player readability
still need live playtests.

## Asynchronous reputation and its provenance

Frozen team snapshots carry the fighter's ledger, count and epithet. Exhibition
copies use separate runtime IDs while preserving their original source
identity, so two copies of the same famous fighter can participate without
discarding or misbinding their history. Exhibitions preserve the original
campaign and do not grant it rewards or new legends.

Imported profiles label their legends **source-reported**. A packet's schema
validation is not authentication of every historical claim. Replaying a
challenge verifies that challenge's result for the same build and frozen
inputs; it does not retroactively verify 612 prior battles. Signed publication
chains or separately replayable historical witnesses would be a later system.

## The Black Bridge expansion

| Stage | Required world capability | Evidence before it can enter a legend |
| --- | --- | --- |
| Wall steps, kicks and linked vaults | More surface affordances, cooldown-aware repeated motion and foothold transitions | Executed surface contacts and accepted trajectories, with real effort use |
| Chain swing | Authored anchor entities, reachable grips, a usable gripping limb, swing constraints and release/landing checks | Actual anchor acquisition, constrained swing and release path |
| Projectile interception | A block/intercept primitive synchronized with the existing physical projectile sweep | A specific projectile actually stopped, redirected or absorbed |
| Throw from a ledge | Support edges, push/throw resolution, airborne/fall state, collision and landing damage | Actual loss of support and resolved fall; animation alone is insufficient |
| Bonded rescue | Persistent relationships keyed by stable fighter identities and a perceived ally-in-danger goal | A legal plan and a real intervention; the beneficiary must be identifiable |
| Catastrophic success | Independent goal success, actor injury, weapon failure, scenery damage and recovery outcomes | Each consequence separately resolved and recorded |

Bond strength, courage, confidence, fear and perceived ally urgency can change
which risks a fighter is willing to accept. They must not waive a missing
surface, unusable limb, blocked path or unavailable effort. A heuristic risk
score must not be called “30% survival” until a probability model and calibration
study justify that interpretation. Existing teamwork is not silently relabeled
as a persistent personal bond.

“Catastrophic success” should be a composition of consequences: the objective
can succeed while the weapon breaks and the actor crashes. It should not be a
single critical label that grants every consequence at once. This first slice
retains today's opposed strikes and critical outcomes; those extra physical
systems are not implemented by a new adjective.

Combat barks keep their 20-word contract. A later narrator could propose a
longer ledger sentence or epithet, but only from admitted receipts, with a
factual template fallback. Neither eloquence nor a model's explanation creates
history.

## Verification

[The integration tests](../tests/opportunity.rs) exercise geometry-dependent
plans, the node/depth limits, training/effort/limb rejection, injury interruption,
proposal-only rejection, real physical execution and opposed hits, deterministic
replay, save/snapshot continuity and zero mechanical bonuses from epithets.
The training tests additionally reject mixed-controller imitation labels.

```bash
cargo test --locked --no-default-features --test opportunity
cargo run --locked --no-default-features --example opportunity -- \
  --seed 6 --output outputs/my-opportunity-demo
```

The example writes its scripted setup and limits to `audit.json`, plus the real
decision trace, completed receipts and persisted campaign/legend files. It grants
training, a remembered contact and a wounded starting state; other roster members
are downed and the Guardian's attacks are suppressed. The planner, movement,
defence and strike rolls still run normally. This isolates composition mechanics
and must not be presented as an unassisted rescue or a survival-rate benchmark.

This establishes a bounded working foundation for the larger scene. It does not
establish that compositions improve combat outcomes or that players perceive
them as authored versus discovered. Those require controlled rollouts and
separate player tests.
