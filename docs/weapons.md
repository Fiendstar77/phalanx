# Projectile weapon evolution

Weapon technology is independent of class and office. A Queen with Warrior as her current class can equip an atlatl, retain mastered healing, and wear a war banner. No retraining or title change is needed to equip a discovered weapon.

| Weapon | Range | Base damage | Supply per battle | Windup / recovery | Flight |
|---|---:|---:|---:|---|---|
| Rock | 6.5 | 7 | Unlimited | Immediate / 1.6s | Slow lob, existing landing splash |
| Throwing spear | 8 | 18 | 6 | 0.5s / 1.7s | Faster shaft; first enemy body hit |
| Atlatl | 10.5 | 26 | 4 | 0.9s / 2.3s | Fast fletched dart; first enemy body hit |
| Bow | 2.2–10 | 13 | 12 | 0.65s / 0.85s | Arrow; first body intercepts; allies block without damage |

Damage still receives class, title, level, armor, height and cover modifiers. Ranger training reduces recovery by 37.5%, including its existing 1.0s rock cycle. Sight still matters: maximum atlatl reach often requires a high vantage point because ordinary flat-ground sight is 10 units.

## Discovering and crafting

1. Throw rocks in completed battles. A fighter who releases at least one projectile earns that battle's XP as practice with that weapon. Idle fighters gain no weapon practice. Practice is separate from class mastery.
2. At **100 rock practice**, craft a throwing spear bundle for **30 gold + 8 salvage**.
3. Throw spears in completed battles. At **100 spear practice**, craft an atlatl bundle for **45 gold + 12 salvage**.
4. At **100 atlatl practice or 100 Ranger class practice**, craft a bow for **55 gold + 14 salvage**. Active or mastered Rangers already have a starter bow without a recipe; this also works with tempered/balanced fittings. A crafted bow gives any class the same archery behavior.
5. Owned equipment switches freely in the existing weapon slot. Spears and atlatls replace tempered/balanced weapon fittings; armor and the charm slot stay independent. Removing the weapon returns to the fighter's class/retained attack capability.

Battle reports show earned weapon practice and newly discovered recipes. The squad workshop shows accumulated practice, locked-recipe requirements and next-battle ammunition. Selecting equipment before battle updates the staged fighter immediately; post-battle changes apply to the next battle. Equipment ownership and all weapon practice persist through saves, title swaps and class changes. Older saves default to zero weapon practice. The existing six recipe bit positions are preserved.

## Combat behavior

Spear and atlatl users plant their feet during windup. A target leaving sight or range cancels the attempt without spending ammunition; heavy hits (12 or more incoming damage in one tick) interrupt an unfinished windup. Ammunition is spent on release. Released shafts follow a fixed trajectory and can miss; they do not home onto a moving target. They intercept the first opposing living body along their flight using the collision system's resolved movement trace, with impact coordinates and relative velocity passed to blood effects. Friendly bodies do not take projectile damage.

Empty bundles cause the fighter to approach with a melee sidearm (1.5 range, 8 base damage). Supplies refill automatically when staging a new battle, with no ammunition purchase cost in this slice. Retained healing and banner auras continue while drawing, throwing or fighting with the sidearm. Rivals use the same practice requirements and earned currencies to progress through these weapons.

Spears and darts have bone-mounted hand models; the atlatl has a short hooked lever and a longer fletched dart. In flight, shafts align with their trajectory. Black powder, ammunition crafting and battlefield ammunition recovery remain future work.

## Validation

`cargo test --no-default-features --test weapons` covers unlock/payment atomicity, distinct windups, cancellation and interruption, swept first-body interception, finite ammunition and sidearm fallback, save compatibility, retained healing/banner combinations, and settlement without duplicate practice. The graphical control tests cover pre-battle role/ammunition updates and post-battle snapshot preservation.

Staged graphical witnesses (launch from an isolated working directory to protect player saves):

```bash
phalanx --visual-witness --weapon-witness --weapon-loadout-witness --arena open --seed 42
phalanx --visual-witness --weapon-witness --spear-witness --arena open --seed 42
phalanx --visual-witness --weapon-witness --weapon-workshop-witness --spear-witness --arena open --seed 42
```

These stage a fresh Queen/Warrior with earned-skill fixtures and crafted equipment. They are QA evidence, not player progression. The witness logs actual weapon shots, windups, airborne shafts and restored HP.

[Atlatl loadout](images/atlatl-loadout-witness.png) · [Atlatl battle](images/atlatl-battle-witness.png) · [Spear battle](images/spear-battle-witness.png) · [Recipe requirements](images/weapon-recipes-witness.png) · [Recipe discovery](images/weapon-report-witness.png)


## Proper archery

Rangers use actual bows rather than rock projectiles. Bow users choose walkable firing lanes from visible enemies and known allies, preferring positions screened by melee allies. An ally in the shooting lane prevents drawing; an ally entering it cancels a draw without consuming ammunition. Allies crossing an already released arrow stop it without friendly damage. Arrows remain fixed shots after release and can miss moving targets.

Archers commit to drawing with planted feet, keep the bow aimed during follow-through, and use short retreats to reopen range. Retreats last at most 1.2s and cannot restart for another 2.4s. Repositioning is slower than an ordinary advance, so melee fighters can close. A threat inside 2.2 units forces a melee sidearm; the bow becomes available again beyond 3.4 units. No ammunition is lost merely for drawing a sidearm. Losing an unprosthetized arm also prevents drawing; fitting a prosthetic restores bow use. Retained healing and banner abilities continue independently.

The bow is mounted at the left wrist; an arm solver places both hands at the shared grip/string geometry. The string and limbs flex during the draw, and the nocked arrow disappears on release. A hip quiver shows each remaining arrow. Ground-plane launch coordinates and visual grip/arrow origins share the same definition; presentation does not change simulation state. Older three-weapon practice arrays and saved reports migrate to four entries with bow practice initially zero.

Archery checks: `cargo test --no-default-features --test archery`. Staged close views use `--visual-witness --weapon-witness --bow-witness --bow-draw-witness --arena open`; add `--male-bow-witness` for the other rig. Omit the draw flag to run a battle, and add `--hybrid-bow-witness` to demonstrate a Queen/Warrior using crafted archery, retained healing and a banner. Use an isolated working directory for these QA fixtures.

[Draw pose](images/bow-draw-witness.png) · [Other rig](images/bow-male-draw-witness.png) · [Hybrid battle](images/bow-battle-witness.png) · [Workshop](images/bow-workshop-witness.png)

Null's counter-building reads equipped ranged weapons and retained training, so a squad of Warriors carrying bows is recognized as a ranged build.


The [occult arsenal](ninjutsu.md) adds two physical weapon-slot items: the chain-sickle, with 3.3 m reach and hit-triggered snares, and an eight-bladed returning wheel with real outgoing/return sweeps. Both starter recipes cost 5 gold and no salvage. They use the existing inventory, equipping, forging and save flow.
