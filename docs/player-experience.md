# The opening campaign

Fresh campaigns begin by naming the phalanx and choosing the classes of its three founders. King, Queen and General remain command titles; each can start in any of the ten classes. Type the team name, choose each fighter's class and confirm to enter the first trial, paused at normal speed. Initial class choices are free once; later retraining costs 10 gold. Existing saves retain their established squads and skip this setup.

For an established team, use **Begin / continue NPC tutorials** in the main menu. The opening chapter has five tactical lessons, each backed by an actual rival team. Wins clear lessons; defeats give the ordinary battle XP and resources and leave the lesson available for another attempt. Rivals retain their existing authored development, equipment and wounds. There are no fabricated victories or tutorial reward multipliers.

| Team | Lesson |
| --- | --- |
| Bloodhound | Survive the rush; protect unsupported ranged fighters |
| Sentinel | Crack defensive support with positioning and a varied build |
| Obsidian | Cross ranged pressure using cover and careful approaches |
| Vanguard | Disrupt banners, auras and formations |
| Null Collective | Change builds against authored composition counters |

The default play view has fighter cards, health bars, clear selection, a compact detail panel and contextual build ideas. Switch between your squad and the rival roster, or select a fighter directly on the field. **I** opens the existing research inspector. **Follow selected** gives a tactical close view; **Whole battlefield** restores the wider framing. Scroll the right detail panel to read longer information.

Three free, persistent **battle styles** expose meaningful choices without editing seven sliders: Vanguard pursues aggressively, Protector values team support, and Skirmisher takes a cautious approach. These are authored mind presets, not new trained policies. Combine them with existing classes, skills and equipment; the workshop retains the finer controls. Styles cannot be changed during combat. Postbattle changes prepare the next fight and preserve the completed result.

Fighters still start in a three-person squad. Slot four costs **1,000 gold**. The existing purchase ladder continues toward twenty; playable Founders leagues currently stop at four. Card pagination is checked for twenty fighters per side, preparing the interface without claiming that 20v20 campaign combat is delivered.

## Splatter and panic

The tone is caricatural splatter comedy with physically legible consequences. Actual severing events create at most **five sustained jets globally, two per fighter**. Each wound has a distinct side, source, direction, pitch and seeded variation. A fighter's wounds share a heartbeat: a fast pressure surge, falling tail and a quiet interval. Pressure stays exaggerated for an intentionally excessive eight to ten simulation seconds, then tapers away. The eightfold emission budget is an artistic quantity setting, not a measurement of physiological blood volume.

Mixed droplet sizes inherit fighter motion, fall under gravity, slow with drag, hit walls and leave terrain-conforming stains. The pool is bounded at 8,192 droplets. Body pieces tumble, bounce and settle. Accumulated stains darken where they overlap; there is no fluid-volume solver, medical bleeding model or new blood-loss damage rule.

Below 30% health, living fighters use two new authored clips: **NearDeathPanic** at rest and **WoundedStagger** while moving. Frantic glances, fear expressions and critical-health barks support the gesture. Attacks, hit reactions and deaths keep priority. These are presentation reactions to real health, not extra movement, status effects or changes to the controller's chosen action. Deaths retain their existing three variants.

## The late arena reveal

The intended full game is primarily an offline NPC campaign. These five tutorial teams form its opening chapter. A longer campaign and a late arena unlock remain future work; completing these tutorials does not claim to complete that larger campaign.

The eventual endgame reveals an **asynchronous player arena**: publish a frozen squad, challenge another player's published team while its owner is absent, then verify and deliver the result. A second arena advancement ladder should extend progression beyond the campaign while preserving the veteran team and its history; its tiers and rewards remain to be designed and implemented. Player-facing tutorial copy preserves that surprise. The current developer prototype still exposes manual snapshot exhibitions; these have no campaign reward authority. [The implemented local exchange and transport boundary](snapshot-challenges.md) prepare the underlying structure. Authentication, hosted delivery, trusted progression and the endgame unlock are not implemented.

The long-term progression goal is extreme veteran growth through time, dedication and strategic combinations. Very high levels and a broader arena experience ladder are design goals, not new rewards, level limits or balance claims in this build.

## Replacement hair concepts

[Front-view lineup on the actual character head](images/hair-lineup.png). Ten new hair-only GLBs replace the six earlier meshes: short messy, buzzed crop, swept back, bob, long straight, high ponytail, twin tails, curly/fluffy, mohawk and warrior braid. Their continuous scalps use a raycast envelope of the actual male and female heads in neutral rest pose. Source heads remain intact.

The workshop cycles through **bald plus ten styles**. Saved style numbers remain valid and use the replacement meshes. These are sculpted prototype concepts, with editable Blender sources and a reproducible generator in `tools/build_hair_meshes.py`; they still need player art review, especially ponytails from the rear and under crowns. Hair remains cosmetic, with no collision or tactical bonus.

## Reproduce the demonstration

[Short gameplay recording](media/opening-campaign.mp4) uses scripted controls on a fresh disposable QA campaign: naming the team, confirming the starting roster, styles, a paid 10-gold Healer retrain, the first tutorial match, camera changes and its real result. It uses Utility control. It is a silent rendered capture, not a human playtest or evidence of superior AI.

Build the game and run this from a **fresh disposable directory** with an `assets` symlink to this repository:

```bash
/path/to/Phalanx/target/debug/phalanx --visual-witness --player-demo --seed 6
```

`--player-demo` disables campaign autosaving for the fixture. It captures PNG frames and `audit.json` under the disposable directory's `outputs/player-demo`. To encode a recording:

```bash
ffmpeg -framerate 10 -i outputs/player-demo/%05d.png \
  -c:v libx264 -crf 22 -pix_fmt yuv420p -movflags +faststart opening-campaign.mp4
```

Reproduction checks include ordinary XP/gold settlement exactly once, legacy saves, style persistence, unchanged completed reports, real tactical trajectory differences, replay under Utility and Learned control, pulse gaps, independent wound sources, looping critical-health clips, and roster pagination. Utility remains the default; the HUD explicitly distinguishes the executing controller from shadow proposals. Visual effects consume no simulation RNG and do not change authoritative combat.
