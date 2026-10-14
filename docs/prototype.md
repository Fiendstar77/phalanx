# Prototype 0.5

## Simulation model

The core in `src/lib.rs` is independent of Bevy. Each tick advances exactly 0.1 simulated seconds. Living units select actions every five ticks: advance, attack, high ground, flank, regroup, retreat, hold, seek cover, and support ally. Negative scores exclude actions; stateless noise is keyed by seed, tick, unit ID, and action.

Aggression and courage influence engagement. Caution, patience, and discipline influence positioning. Teamwork and ally isolation influence regrouping. Injury, caution, self-preservation, and enemy concentration influence retreat. Nearby enemy pressure adds a retreat incentive to rock throwers. Scores and eligibility conditions remain explicit in source.

## Terrain and navigation

`src/terrain.rs` defines the arena rules used by rendering, movement, combat, and navigation. Ridge has elevation; Choke adds two solid wall segments around a central passage; Open Field removes terrain bonuses as a baseline. Cover zones grant 30% incoming-damage reduction. Rough ground halves base movement speed. Uphill movement is also slower.

`src/navigation.rs` runs deterministic Dijkstra search over a one-unit grid with diagonal edges and segment clearance. A 0.3-unit allowance keeps agents away from choke walls. Failed routes return no movement rather than crossing an obstacle. Separation adjustments are checked against the same wall rules.

Positive edge costs combine:

- distance;
- terrain difficulty (1.4 × edge length on rough ground);
- exposure (edge length × uncovered fraction × caution × 1.6);
- threat (nearby enemy influence × edge length × a caution/self-preservation weight, reduced by cover);
- uphill elevation change weighted by patience.

Enemy influence uses only that unit’s contacts, their last observed weapon range and position, fading confidence, and clear segments. Routes refresh periodically, when targets move materially, and when personality changes. Overlays draw remaining route waypoints; the inspector exposes the cost of the last full plan, not the remaining journey's cost.

## Perception and coordination

`src/perception.rs` is the observation boundary. Radial sight starts at 10 units, gains range from observer elevation, loses range when a target occupies cover, and checks walls and terrain height. Decisions use contacts with observed position, health, role, timestamp, source, and direct-visibility status. Lost contacts retain their last observed position with decreasing confidence for up to four simulated seconds; observation of a body or inspection of an empty last-known location can clear them earlier.

Nearby teammates share current direct sightings when both participants have `(teamwork + discipline) / 2 >= 0.5` and are within eight units. Memory reports are not retransmitted, so sharing cannot keep a stale contact alive indefinitely. Friendly information is available through direct sight or nearby cooperative communication. All updates use the same pre-movement snapshot to avoid team update-order advantages.

`src/cognition.rs` selects targets from the recipient’s contacts, with teamwork-weighted preferences for an allied focus. High-teamwork agents can move to support a known injured or pressured ally. Supporting is tactical movement, not healing. A visible enemy engaged by an ally can make a disciplined agent prefer a flank; a fixed flank anchor and cooldown discourage repeated orbiting.

An attack requires a current direct sighting and valid range/clearance. Reports and memories can guide movement but cannot authorize attacking an unseen target. Navigation receives perceived threat sources rather than hidden unit positions. Without contacts, agents search the level's authored opposing deployment zone; this objective contains no live enemy coordinates.

The inspector exposes visible/known/report counts, selected contact source and age, support target, scores, and the last decision's eligibility reason. Threat overlays use known contacts. Knowledge markers show reported positions; an older marker may differ from the enemy's true position.

## Presentation and performance

The simulation remains fixed at 10 ticks per simulated second. While running, rendered pawn positions interpolate between actual previous/current states, with up to one simulated tick of presentation delay (100 ms at x1). Pause and single-step show current simulation positions. Picking and selection markers use the same displayed position. No predicted future movement is fed into the simulation.

Development builds optimize the simulation crate as well as dependencies. Route anchoring avoids clearance checks for grid cells farther than the current best candidate. Unchanged inspector/status text and camera transforms are not marked dirty every frame. See `docs/performance.md` for measured step timing and replay checks.

## Combat roles

Each team has three melee fighters and two rock throwers, all starting with 100 health and 2.2 base movement speed. Melee fighters deal 10 base damage at 1.5 range with a 0.8-second cooldown. Throwers deal 7 base damage at 6.5 range with a 1.6-second cooldown. Elevation grants a modest positional damage bonus to either role.

Throws require a clear segment, aim at the enemy's position when launched, and travel at 10 world units per second. At impact, enemies within 0.75 units of the aim point receive damage. Moving targets can evade the rock. Hits are target contacts: one rock may damage multiple enemies. There is no friendly fire. Rendered rock arcs follow the corresponding simulation projectile; the arc is visual, while hit resolution uses the aimed ground position.

Damage resolves simultaneously. Rocks persist after their thrower's death, and elimination does not end a battle while rocks remain airborne. The battle ends on team elimination with no pending rocks, or at 120 simulated seconds. Timeouts are draws.

## Current limits

- Radial sight and proximity communication; no facing/view cones, audible combat cues, or probabilistic identification.
- Fixed authored presets; no terrain editor or arbitrary arena/profile saving.
- Grid routes and ally separation, rather than rigid-body collision or navmesh movement.
- Abstract cover reduction for both melee and rocks; no directional cover or ballistic physics.
- Initial presets change multiple traits, so comparing them does not isolate a single trait.
- Scores represent the most recent decision. Route costs represent the most recent full plan.
- Reset discards individual personality edits and restores initial presets for the current arena/orientation/seed.
- JSON summaries are complemented by optional decision/damage JSONL traces; aggregate living-time and ally-distance metrics are recorded.
- A PyTorch imitation model has been trained and evaluated offline. The live game retains utility AI; no generative model or learned in-game controller is installed.

## Validation

Twenty-six Rust tests cover deterministic replay, personality-dependent action and route selection, simultaneous fatal attacks, terminal finite state, team composition, choke clearance, rough-ground slowdown, blocked throws, delayed impacts, misses, cover protection, rocks surviving their throwers, limited sight, terrain occlusion, sharing eligibility/range, memory expiry without echoes, hidden-enemy independence, support, focus selection, flanking, decision-recording invariance, valid feature/mask records, living-time metrics, and protection against leaking selected flank state into training features. The finite-state test uses 20 seeds and both profile orientations; another test tracks wall clearance through a complete battle on every arena.

Local checks include formatting, full graphics compilation, and Clippy. Desktop checks cover preset changes, terrain/threat/elevation overlays, route display, selection, battle controls, and the knowledge inspector. The user confirmed that the updated movement feels smooth. The development machine uses NVIDIA Vulkan; transient swapchain validation errors occurred during renderer startup, although rendering recovered. Other platforms have not been runtime-tested.

Recordings and screenshots in `docs/images/` show the actual application. They illustrate the prototype; they are not evidence of statistical superiority for a profile or controller. The evaluation protocol remains a draft for later controlled studies.

## Measured baseline

The Python package now runs controlled batches and trains a PyTorch policy from exact decision-time traces. See [the first results](results/v04/README.md) for 864 battles, held-out evaluation, real saved weights, and current limitations. Three Python analysis tests and a train/save/reload smoke run complement the Rust checks.

## Live learned controller

The frozen PyTorch policy is exported to `assets/policy.json` and evaluated in Rust on the same 29 pre-action features as utility AI. Both proposals use utility eligibility. Utility is default; C switches the executed controller without resetting state. Failed inference falls back to utility. The inspector retains the last decision's mode and proposals; disagreements go to JSONL. Learned explanations identify a model proposal rather than attributing the choice to a utility score. See [the focused comparison](results/v05/README.md) for measured outcomes and limits.

Four additional Rust tests validate actual PyTorch/Rust logits and masked predictions, unchanged utility combat under shadow inference, deterministic eligible learned control, and invalid-model/input fallback.

The live disagreement summary counts all teams and controller modes in the current battle, with unavailable predictions excluded from agreement. Named top pairs and the full Utility-row/Learned-column matrix are serialized in battle JSON. Two further Rust tests cover aggregation denominators, decision conservation across toggles, and reset/missing-policy behavior. The Choke batch report separately aggregates learned-controlled teams and verifies original battle hashes.

## Citadel and live pair inspection

Citadel is a 44×40 arena (five times the standard 22×16 footprint), with 6 warriors + 4 rock throwers per team. Authored twin hills rise to 3.8 units. Five barricades create central passages and lateral approaches; six cover zones and rough ground add route choices. Grid dimensions, damage accumulation, rendering, roster colors/roles, unit labels, camera framing, and selection now follow arena size. The default three arenas retain their original rules and rosters. Citadel retains the 120-second cap. The original larger-map search stalled at deployment zones; scouts now continue through authored patrol waypoints and stop camping empty hills after the opening. With the walkways and search fix, seed 42 finished in a Red win at 67.8 seconds. The model features scale position, height, and team counts to the arena while preserving the original small-map values; the frozen small-arena policy is not validated for Citadel generalization.

Top disagreement rows are clickable. A filter selects living agents whose **latest** proposals match the exact pair; pink rings and markers identify them. A found match pauses and selects an agent. D cycles matching agents; Esc/Show all clears the filter. Counts remain historical, so zero current matches is shown explicitly. Filters clear on reset/map change. UI clicks cannot also select a world pawn.

Static grid neighbor clearance is cached per authored arena while dynamic threats and personality costs remain recomputed. Choke seed-42 and Citadel seed-42 full summaries matched byte-for-byte before/after caching. Four additional tests verify the expanded roster/footprint, collision-safe routes and complete 20-agent battles in both controller modes, and exclusion of dead/superseded decisions from live filters.

Two raised passerelles cross the central lanes at 3.2 units, with ramp entrances and side rails. Walking checks surface-height steps separately from projectile/visibility clearance, preventing agents from jumping onto a deck through its side. Rendering shows the underlying ground and the deck separately. Navigation still has one traversable surface at each horizontal location: **true stacked underpass/overpass navigation is not implemented**. These are walkable raised routes, not a multi-level navigation system.

## Founders setup

The graphical app now defaults to three fighters per team: King, Queen, and General, centered in each deployment zone. Command titles are independent of melee/rock-thrower roles. Initial King and General are melee fighters; Queen is a rock thrower. Royals have 110 maximum HP and multiply base weapon damage by 1.10. General retains 100 HP and normal damage. Own injury and ally-support calculations use health relative to maximum HP; observed enemy health is represented as a percentage for utility features. Gold/silver crown meshes and a chest chevron patch follow these titles.

Founders/Lab switches between the new setup and the original experiment rosters. Reset, profile swap, and arena changes preserve the active setup and controller. Graphical `--lab` and the default headless batch preserve original Lab behavior; batch `--founders` enables the new setup. The Lab's seed-42 Choke output still matches byte-for-byte. Three new tests cover roster composition, actual royal melee/projectile damage, and deterministic finite battles on every arena.

There is no automatic loss on royal elimination, no new command aura, and no persistent fighter storage yet. This slice establishes visible squad identity on the same measured simulation core.

The original learned policy was trained on Lab rosters, so its original agreement metrics do not establish accuracy or combat quality for royal Founders battles. Utility remains the default and eligibility authority.
