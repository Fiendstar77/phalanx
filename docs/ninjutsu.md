# Ninjutsu and the occult arsenal

This is the first playable ninja arts pass in Phalanx. Wall movement, smoke,
snares, returning blades and binding spells change combat state. The renderer
reads those states and cast receipts; visual effects do not award damage.

## Equip the arts

Open **Workshop → Skills**. **Wall dance**, **Smoke bombs** and **Grave binding**
are universal starter arts, available to any title or class. They occupy the
existing active slots. Default loadouts include them when capacity permits;
explicitly saved custom loadouts keep their choices, so equip the new skills
from the catalogue. Class abilities and retained mastery still share the same
10 active / 3 passive capacity.

Open **Forge** to craft a **Chain-sickle** or **Returning razor wheel** for
**5 gold and no salvage**. These are deliberately accessible starter recipes.
Crafting puts a physical item into the fighter's personal inventory; move it
into the weapon slot to use it. Both support the existing item forging and
inventory/save systems. The older inspector workshop action also crafts and
equips them. A crafted ninja weapon replaces the current weapon; a ranged
class can use it without secretly continuing to shoot its class weapon.

## What the abilities actually do

| Art | Combat behaviour |
| --- | --- |
| Wall dance | A fighter approaches a nearby wall face, climbs/kicks upward, crosses above its top and lands on walkable ground. The full 1.4-second path is checked against solid scenery and other reserved vaults. It costs 7 effort, requires intact legs and Acrobatics 12, and recovers after 5.5 seconds. |
| Smoke bombs | Two charges per battle, 3 effort per deployment and a 6.5-second recovery. A cloud grows to a 2.6 m radius and dissipates after 4.2 seconds. Both teams lose direct sight through it; contact memory remains. Touching distance and sight above the cloud remain possible. |
| Grave binding | An Insight/Coordination check opposes the target's Resolve. It costs 8 effort, recovers after 9 seconds, and requires clear current sight within 6.5 m. A successful seal reduces movement to 20% and exposes the victim for 2.2 seconds. Resisted casts spend effort but do not bind. It does not directly inflict damage or bind an active wall dancer. |
| Reaper's hook | A successful chain-sickle strike can snare a target for 0.8 seconds and pull it up to 0.7 m on the next accepted physical step. Weapon reach is 3.3 m. Walls and body contacts constrain the pull; the hook recovers after 2.8 seconds. |
| Demon wheel | A real 3D blade flies toward the observed target point, then returns toward its owner. It can contact an enemy once outbound and once on return. Each contact still passes through Theus attack/defence resolution. Throws cost 4 effort and recover after 2.2 seconds. Walls turn back or stop the blade. |

The AI uses these arts autonomously. Smoke deployment reacts to injury or
pressure from multiple visible enemies. Fighters leave obscurants by a flank
calculated from remembered contacts. Wall dancers follow a known contact or
their existing navigation objective; they do not inspect hidden enemy positions.
Conflicting vault trajectories are reserved sequentially. An unproductive blind
search advances to another authored patrol objective instead of repeating an
irrelevant wall crossing.

Smoke can therefore break a volley, create a route for a flank, or cover a wall
approach. It also blinds allies. A fighter above the cloud can regain direct
sight. Binding and chain snares create real exposure windows for Theus partners.

## Presentation and limits

The hand-held chain-sickle and eight-bladed wheel are procedural meshes. The
moving wheel, chain links, soft smoke billboards and orbiting seal papers come
from bounded pools: at most eight clouds, eight flying wheels and 128 retained
cast receipts. Captions name actual arts, including resisted binding attempts.
Wall movement uses the existing skeletal wall-kick clip with authoritative
world movement. The current 52-clip set is reused; these are not newly acquired
motion-capture assets.

Wall vaults and razor-wheel sweeps have 3D solid clearance. Existing ground
arrows, darts and stones keep their earlier planar flight/interception model
and cannot hit a wall dancer above their height band. Theus's earlier launch
arcs remain partly presentation-driven. This is not a replacement of the whole
physics engine with full articulated 3D combat. Landing contacts, hand/foot
placement and effect styling still need art refinement.

Equipment and equipped skills persist. Smoke charges, bindings, flight plans
and flying wheels belong to a battle and reset at preparation. Load Game still
restores campaign preparation rather than resuming a spell or projectile in
mid-flight. The original Lab research rosters do not receive the ninja runtime.

## Validation and review

`tests/ninjutsu.rs` covers real wall traversal, clearance, competing vaults,
leg/effort restrictions, smoke visibility for both teams, finite charges,
resistance rolls, actual hooks, physical return blades and persisted loadouts.
The full procedural and archer tests now distinguish legal flight above a wall
from penetration into its solid volume, and use the current breached arena.

Run the simulation, graphics and strict simulation lint checks:

```sh
cargo test --offline --locked --no-default-features
cargo test --offline --locked --bin phalanx
cargo clippy --offline --locked --no-default-features --all-targets -- -D warnings
```

For a disposable rendered review, make a fresh directory containing an `assets`
symlink and launch the built game there:

```sh
WGPU_ADAPTER_NAME=AMD /path/to/Phalanx/target/debug/phalanx --visual-witness --ninja-demo --seed 6
```

This stages a real 3v3 encounter with the new weapons equipped, disables
campaign autosave, waits for render warm-up and records a bounded excerpt.
`outputs/ninja-demo/audit.json` reports observed arts, physical wall height and
smoke observations; `events.json` retains the actual cast receipts. PNGs are
requested at 24 FPS. This is a scripted fixture, not a human playtest, a full
campaign, or a promise that the fight finishes within the excerpt.
