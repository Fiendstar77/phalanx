# Prototype 0.2

## Simulation model

The core in `src/lib.rs` is independent of Bevy. Each tick advances exactly 0.1 simulated seconds. Living units select actions every five ticks: advance, attack, high ground, flank, regroup, retreat, hold, and seek cover. Negative scores exclude actions; stateless noise is keyed by seed, tick, unit ID, and action.

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

Enemy influence uses weapon range and clear segments. It is relative to the moving unit's team. Routes refresh periodically, when targets move materially, and when personality changes. Overlays draw remaining route waypoints; the inspector exposes the cost of the last full plan, not the remaining journey's cost.

## Combat roles

Each team has three melee fighters and two rock throwers, all starting with 100 health and 2.2 base movement speed. Melee fighters deal 10 base damage at 1.5 range with a 0.8-second cooldown. Throwers deal 7 base damage at 6.5 range with a 1.6-second cooldown. Elevation grants a modest positional damage bonus to either role.

Throws require a clear segment, aim at the enemy's position when launched, and travel at 10 world units per second. At impact, enemies within 0.75 units of the aim point receive damage. Moving targets can evade the rock. Hits are target contacts: one rock may damage multiple enemies. There is no friendly fire. Rendered rock arcs follow the corresponding simulation projectile; the arc is visual, while hit resolution uses the aimed ground position.

Damage resolves simultaneously. Rocks persist after their thrower's death, and elimination does not end a battle while rocks remain airborne. The battle ends on team elimination with no pending rocks, or at 120 simulated seconds. Timeouts are draws.

## Current limits

- Global enemy knowledge; no perception memory, view cone, or occlusion-based concealment.
- Fixed authored presets; no terrain editor or arbitrary arena/profile saving.
- Grid routes and ally separation, rather than rigid-body collision or navmesh movement.
- Abstract cover reduction for both melee and rocks; no directional cover or ballistic physics.
- Initial presets change multiple traits, so comparing them does not isolate a single trait.
- Scores represent the most recent decision. Route costs represent the most recent full plan.
- Reset discards individual personality edits and restores initial presets for the current arena/orientation/seed.
- JSON is a battle summary, not a sequential decision/event dataset.
- No LLM, learned controller, Python evaluation, or model training yet.

## Validation

Thirteen tests cover deterministic replay, personality-dependent action and route selection, simultaneous fatal attacks, terminal finite state, team composition, choke clearance, rough-ground slowdown, blocked throws, delayed impacts, misses, cover protection, and rocks surviving their throwers. The finite-state test uses 20 seeds and both profile orientations; another test tracks wall clearance through a complete battle on every arena.

Local checks include formatting, full graphics compilation, and Clippy. Desktop checks cover preset changes, terrain/threat/elevation overlays, route display, selection, and battle controls. The development machine uses NVIDIA Vulkan; transient swapchain validation errors occurred during renderer startup, although rendering recovered. Other platforms have not been runtime-tested.

Recordings and screenshots in `docs/images/` show the actual application. They illustrate the prototype; they are not evidence of statistical superiority for a profile or controller. The evaluation protocol remains a draft for later controlled studies.
