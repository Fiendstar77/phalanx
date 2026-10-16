# Pressure fields

Founders battles generate the battlefield from both squads. Lab arenas stay authored. Pass `--arena` to force an authored map for a Founders session. **Arena** in a Founders battle steps back onto that authored cycle. **Next battle** generates again.

The field is mirrored. Neither side gets a private hill, a private lane, or a private patch of rough ground. Four lessons exist:

| Lesson | Terrain | Who it pressures |
| --- | --- | --- |
| Two lanes | A centered wall with an equal north lane and south lane | Low cohesion, or fear, hesitation, or attachment about being isolated |
| Contested hill | One hill in the middle | Low patience and high aggression |
| Open cover | Open ground with the same cover on both flanks | Low caution and high aggression |
| Long crossing | A longer spawn and a rough band both sides must cross | Low mobility: Guardians and missing unrepaired legs |

Preparedness is the average of the two squads. The least prepared lesson is chosen. Two scores within 0.02 are a tie, broken by the battle seed. A lesson either squad just fought is skipped for the next one. A 4-fighter split uses the standard spawn distance. A 3-fighter split starts closer.

Ranged and melee labels use the same mobility unless the class is a Scout or a Guardian. Swapping Warriors for Rangers does not change the field. The two-lane map still leaves both flank lanes open, so a ranged team that does reach that lesson can still shoot. On the 4-fighter field a mirrored pair of posts narrows the middle and both lanes stay open past them.

The footprint grows with the playable league, and the features move with it. Three fighters get a 36×26 skirmish. Four fighters get a 48×34 company field with a longer hill, a wider rough crossing, and cover set farther from the spawns. The original 22×16 field remains readable from older saves. League 5–20, with several fronts and a larger roster, is still ahead.

## Veteran pursuit recovery

Founders now sweep the arena after finding an empty opposing deployment zone, including small and generated maps. Saturated scars can still cause a brief flinch, but scar-induced HOLD is bounded to an eight-tick window per eighty ticks, staggered per unit, and cannot interrupt a legal close attack. Healthy units recover from fear-induced retreat, while serious injuries and threatened ranged units retain withdrawal. Attack/advance receive stronger commitment, and regroup/support/cover cannot indefinitely outweigh the combat objective. Early isolation reactions remain distinct between fearful and reckless personalities.

Summed ally avoidance is capped at 30% of the movement step in Founders. Large or wounded squads therefore retain forward movement instead of being pushed backward by additive separation. Lab movement and utility scoring retain their previous baseline. No hidden enemy coordinates are added to pursuit/search decisions. The frozen learned model has not been retrained on these Founders utility changes.

A local saved-squad probe at seed 64 reproduced zero-damage Open Field timeout and an Approach-field timeout. After the fix those same squad copies finished at 130 and 256 ticks respectively; Ridge, Choke and Ruins also finished. This is a regression witness, not a general win-rate or balance study. Synthetic regression tests cover saturated scars, empty territory and crowded wounded movement in `tests/pursuit.rs`.
