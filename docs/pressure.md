# Procedural battlefields

Founders battles now use a seeded battlefield grammar: four terrain families, continuous seed-driven wall spacing and relief, covered flanks, exposed central crossings, and breachable timber among permanent stone ruins. The same terrain rules drive rendering, visibility, path costs, movement and combat.

Open **Create a procedural battlefield** in the main menu, or press **G** before starting a Founders battle. The game opens a paused preview with a tactical briefing. **New battlefield** generates another seed; **Fight on this battlefield** starts the match. Generation is also available after a finished battle. A fight in progress cannot be rerolled. **Reset** preserves the exact field and seed. **Inspect / prepare a fighter** restores the inspector; **Map briefing** opens the tactical panel again. These previews keep fighter identity, equipment, experience, wounds and resources.

## Four battlefield families

| Family | Tactical problem |
| --- | --- |
| The Broken Gate | A short central gate, screened wings and longer covered flanks. Timber breaches change crossfire and access to the courtyard. |
| Crown of Stone | Two contested heights overlooking an exposed saddle. Ruins interrupt firing lanes; protected outer routes cost distance. |
| The Long Causeway | Two dry lanes cross a slow basin. Commit through the open center, use the causeways or take the covered outer approach. |
| The Sunken Court | A low center beneath raised shoulders. Broken corners screen movement around the court; heights command the approaches. |

Rendered previews: [Broken Gate](images/battlefield-broken-gate.png), [Crown of Stone](images/battlefield-crown-of-stone.png), [Long Causeway](images/battlefield-long-causeway.png), [Sunken Court](images/battlefield-sunken-court.png). [Live combat on a generated field](images/battlefield-combat.png).

Every new field has four timber barricades and six permanent stone walls. Timber can be destroyed by the existing combat rules; stone remains. Ruin shapes, widths, spacing, high-ground height, cover pockets and biome palette vary with the seed. Green vegetation marks cover, dark earth marks the slow basin, and pale strata make elevation readable. Four biome palettes provide reclaimed grassland, ochre ruins, slate and weathered stone.

There are three connected crossings: the center and both outer flanks. Cover and travel distance differ between them. Body clearance is included in collision and navigation. Deployment is clear; X reflection gives both teams the same terrain at each corresponding position. A four-member field grows from **36 x 26** to **48 x 34**, including its terrain features. No decorative obstacle is added to a walkable corridor.

## Team pressure

Both squads still determine the pressure lesson. The lowest shared preparedness is selected, avoiding a lesson either squad just fought; the seed resolves near ties. Low cohesion/isolation reactions invite split-lane pressure, impatience invites elevation, recklessness invites exposure, and low mobility invites a longer rough crossing. The chosen lesson adjusts the generated field while its family comes from the seed. A ranged class label does not select a hostile map.

Lab arenas remain authored. **Arena** / **M** cycles those maps; **G** returns to generated Founders fields. **Next battle** generates a new field from both current squads and their histories. Leagues 5-20 remain outside this build.

## Export a tactical map without graphics

```bash
cargo run --locked --no-default-features --bin phalanx-map -- \
  --seed 6 --league 4 --output broken-gate.svg --simulate
```

This writes a top-down SVG and the complete serialized arena in `broken-gate.json`, then optionally runs an actual battle and prints its ticks, damage and winner. It does not read or write campaign saves and refuses to overwrite earlier exports. Repeat a seed and league to reproduce the geometry; Founders gameplay also uses the actual teams to select its pressure lesson.

## Compatibility and validation

`PressureField.generator` versions the terrain grammar. Absent/zero retains the exact original terrain functions and eight legacy barricade layouts. Version one identifies the new battlefields. Navigation caches retain the seed for this version because geometry is seed-dependent. Snapshot challenges include the new arena and remain restricted to the same game build.

The procedural regression suite checks 96 seed/size combinations for complete floor connectivity, spawn and search-objective clearance, mirrored geometry/heights/cover/roughness, accessible cover and high ground, plus three crossing routes using the real path planner. Separate checks exercise actual combat breaches, seed-specific graph caching, exact replay, and completed battles in every family and both playable leagues. A known-threat comparison across twelve seeds verifies longer routes with lower threat exposure. Generated-map patrols revisit the shared crossings after losing contact. These checks establish map legality and observed engagements; balance and player enjoyment still need playtesting.

[Recorded local validation](results/battlefields/validation.json): 135 simulation tests and 13 game tests passed, along with formatting, the CI simulation Clippy gate and the game build. Four separate rendered previews were inspected; a rendered generated battle completed with visible motion and combat effects. CLI exports were parsed as SVG and overwrite/unsupported-league rejection was checked. An additional strict graphics Clippy run reports 15 existing diagnostics in unchanged workshop/effects modules; that extra check is not the graphics CI gate. Hosted CI has not been run for these local changes.

To reproduce a preview without touching the player's campaign, use a disposable directory with an `assets` symlink and run the built game with `--visual-witness --battlefield-witness --seed 6`. Add `--league-four` for 4v4. The screenshot is written to that directory's `outputs/battlefield-witness.png`. `--visual-witness --refinement-witness --seed 6` runs the generated battle and captures live combat instead.

## Veteran pursuit recovery

Founders now sweep the arena after finding an empty opposing deployment zone, including small and generated maps. Saturated scars can still cause a brief flinch, but scar-induced HOLD is bounded to an eight-tick window per eighty ticks, staggered per unit, and cannot interrupt a legal close attack. Healthy units recover from fear-induced retreat, while serious injuries and threatened ranged units retain withdrawal. Attack/advance receive stronger commitment, and regroup/support/cover cannot indefinitely outweigh the combat objective. Early isolation reactions remain distinct between fearful and reckless personalities.

Summed ally avoidance is capped at 30% of the movement step in Founders. Large or wounded squads therefore retain forward movement instead of being pushed backward by additive separation. Lab movement and utility scoring retain their previous baseline. No hidden enemy coordinates are added to pursuit/search decisions. The frozen learned model has not been retrained on these Founders utility changes.

A local saved-squad probe at seed 64 reproduced zero-damage Open Field timeout and an Approach-field timeout. After the fix those same squad copies finished at 130 and 256 ticks respectively; Ridge, Choke and Ruins also finished. This is a regression witness, not a general win-rate or balance study. Synthetic regression tests cover saturated scars, empty territory and crowded wounded movement in `tests/pursuit.rs`.
