# Persistent squads and the Paladin

Founders now runs a local progression loop on the same simulation as PHALANX Lab. Each squad keeps its King, Queen, and General across battles. The two local squads are named **Sunward** and **Moonward** by default; their heraldry is a saved preference rather than their identity.

![Paladin aura, saved progress, and chosen heraldry](images/paladin-progression.png)

## Play the loop

1. Select a fighter, adjust its mind, and choose a team color using the palette buttons. Colors apply to the selected fighter's whole squad.
2. Run a battle. The Paladin's gold ring shows its aura radius; blessed allies have a small halo.
3. A completed battle records XP, wins, survival, damage, and history exactly once, then autosaves.
4. Click **Next battle** to advance the seed and restore the same fighters at full health with their earned progress.

**Save** writes your local squads. **Load** reloads the saved squads and starts a fresh battle; it does not resume mid-combat. Startup automatically loads an existing valid save and continues from the last completed seed unless `--seed` is supplied. **Reset** restarts the battle without rewarding an unfinished attempt. Arena changes retain squad identity and progression. **Swap profiles** exchanges the squads' mind configurations while preserving identities and colors.

The current save is `outputs/phalanx-save.json`. Writes use a temporary file and preserve the previous file as `phalanx-save.json.bak`. Invalid saves are left untouched at startup; automatic writes are disabled until an explicit Save. Both files are local and ignored by Git. Version-1 save validation checks schema, fighter IDs, titles/classes, colors, trait bounds, progression, and history. This is local persistence, not an online anti-cheat system.

## General: Paladin

The General is a **Paladin** with melee combat and a support aura. At the beginning of each simulation tick, living allies within **4 horizontal units** and a clear segment through the arena receive:

| Effect | Rule |
| --- | --- |
| Blessing | +5% outgoing damage; 10% less incoming damage |
| Motivation | +0.10 effective courage and discipline, capped at 1.0 |

The aura excludes its caster, does not affect enemies, does not stack, and stops on the next tick after the source dies or an ally leaves range. Walls block it. The temporary motivation boost does not overwrite persistent mind traits. Royalty, class, and experience are independent properties.

## First progression rules

A completed battle grants each fighter **25 participation XP**, plus **15 for victory**, **5 for survival**, and up to **10 for damage contribution**. Both local squads progress. Every 100 XP adds a level, capped at **level 10**. Each level above the first adds **2 maximum HP and 1% base damage**. The King/Queen's existing royal bonus remains separate. Surviving and eliminated fighters both return for the next battle; permanent injuries are not implemented yet.

Battle-conditioned habits are deterministic and small:

- Finishing below half health: +0.01 self-preservation.
- Using support: +0.005 teamwork.
- Winning: +0.005 courage.
- Surviving after retreating: +0.005 discipline.

Traits remain within 0–1. These are authored adaptation rules, **not online PyTorch training**, and they do not establish that veterans make better decisions. The frozen imitation policy remains available for same-state comparison. Improvements in tactical quality need their own experiment.

## Roadmap position

Persistent fighter data now lives separately in `src/progression.rs`; `Unit` remains the temporary combat state. Validated saved squads can create a deterministic battle and produce a serialized result, with a save/reload replay test. This closes Phase A's snapshot foundation.

Phase B now has stable IDs, names, command titles, classes, minds, XP, records, and recent history. Body bases, persistent injuries, relationships, and richer psychological state remain unfinished. Phase C has persistent three-member squads and save/load, but a dedicated roster screen and recruitment are still missing. Class architecture currently supports Warrior, Rock Thrower, and Paladin; the eight-class milestone is unfinished.

Lab rosters, seed-42 Choke results, and frozen experiment artifacts remain unchanged. Lab battles do not grant campaign XP. Five additional Rust tests cover real aura damage/range/death, persistent growth, duplicate rewards, snapshot replay, invalid data, backups, colors, and bounded progression.
