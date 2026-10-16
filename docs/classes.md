# Launch classes

These are the eight roadmap launch classes plus the existing Rock Thrower and Paladin. They change the actual deterministic Founders simulation. They are not yet a finished equipment/animation catalog: ranged classes currently share the rock projectile and basic throwing clip.

| Class | Current mechanic | Tradeoff |
|---|---|---|
| Warrior | Baseline melee | No specialty |
| Guardian | +30 maximum HP, 35% incoming-damage reduction | 20% less damage and movement speed |
| Ranger | Ranged projectile every 1.0 seconds, versus 1.6 for rocks | Needs sight and clear firing line |
| Scout | 45% faster movement | −15 maximum HP |
| Healer | Restores 12 HP to a living wounded ally within 5 units, 1.2-second cooldown | Uses its attack opportunity to heal; 35% weaker damage |
| Berserker | Damage multiplier rises from 1.15 to 2.0 as HP falls | Takes 15% more damage |
| Hexer | Ranged attacks; enemies within 5 units deal 25% less damage | Aura needs clear line and a living source |
| Banner Bearer | Allies within 5 units gain 15% damage and movement | Aura needs clear line and a living source; no stacking |
| Rock Thrower | Ranged projectile every 1.6 seconds | Slow attack rate |
| Paladin | Existing protection, damage and motivation blessing within 4 units | Must remain nearby and alive; no self-blessing |

Class retraining costs 10 earned gold and is available before a Founders battle or after it finishes. Every fighter, including the General, can select any class. Retraining preserves the fighter's identity, experience, mind, wounds and appearance. Mid-battle changes are rejected. Post-battle changes apply on the next battle.

Utility remains the default controller. The frozen PyTorch model predates these classes and persistent wounds: its agreement figures are historical imitation results, not evidence of mastering these synergies. The Lab's baseline mechanics remain unchanged when no classes are assigned.

Active Healers consume the attack opportunity when healing. Retained healing uses a separate healing cooldown, allowing a hybrid to heal and attack in the same tick; it does not relabel the logged Utility/Learned decision. The inspector exposes the healing target and reason.


## Titles and permanent mastery

The active class is the current training discipline. Titles grant independent passives and can be reassigned freely in the workshop, including to another body or former title holder. An occupied title swaps holders. King, Queen, General, Captain and Champion are unique within a squad; Member is unrestricted. No title is required and no title restricts a class or appearance.

Each completed battle credits its earned XP to the active discipline's permanent practice. At 100 practice its supported skills remain available after switching class. Ranks 1/2/3 unlock at 100/300/600 practice; healer rank increases restoration to 15/18/21 HP from the initial 12 HP. These are authored progression rules, not model training. Other disciplines currently retain their existing mechanics at the first threshold; their higher ranks do not yet add new abilities.

| Mastered discipline | Retained mechanic |
| --- | --- |
| Warrior | +5% damage |
| Guardian | HP, resistance, damage and movement tradeoffs remain together |
| Scout | Movement bonus and HP penalty remain together |
| Berserker | Rage damage and incoming-damage penalty remain together |
| Healer | Healing with its own 1.2-second cooldown; supports simultaneous ranged attacks |
| Paladin | Blessing and motivation aura, independent of title |
| Ranger | Ranged attack capability and faster projectile cadence |
| Rock Thrower | Ranged attack capability |
| Hexer | Ranged attack capability and weakening aura |
| Banner Bearer | Rally aura while carrying the squad standard or an equipped war banner; active Banner Bearers carry a standard automatically |

Every squad has a designated standard-bearer who carries the squad flag without consuming a gear slot. Carrying alone grants no unlearned skill. A war banner costs 25 gold + 8 salvage and occupies the charm slot, allowing additional fighters to carry the same squad flag while competing with the vitality charm. All standards are mounted on the bearer's back. Equipping it without Banner Bearer mastery does not grant the aura; its recipe explicitly states the requirement. Mastered aura sources obey the same range, line-of-sight and living-source rules as their active-class equivalents. Bonuses and retained tradeoffs can combine across disciplines.

Utility behavior now gives healers a wounded-ally destination behind the patient, a retreat response to visible close threats, and greater urgency for support. A hybrid already within healing range can keep attacking while healing independently. Scouts favor available flanks, guardians and paladins favor support, berserkers favor available attacks, and banner bearers favor available regrouping. These are distinct utility preferences, not proof that tactical quality improved. The frozen learned policy has not been trained on mastery or the new title mechanics.

This is the first hybrid foundation. Javelins, atlatls, separate bows, black-powder weapons, ammunition and a historical technology tree remain unimplemented; ranged combat still uses the existing projectile simulation. There is no new technology unlock implied by a mastery rank.
