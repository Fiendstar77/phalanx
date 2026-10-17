# Character sheets, skill slots and the forge

Open **Squad workshop**, then select any member. Every member has the same controls, regardless of title or class. The Character page shows a full-body, rotatable preview, missing limbs, fitted iron prosthetics, and weapon/armor/charm slots. An unfitted severed limb stays visibly absent. The preview comes from the saved fighter; it never changes a completed battle or report.

Drag equipment between the personal inventory, matching equipment slots, and the shared phalanx chest. Replaced equipment stays in the personal inventory. Each physical item keeps its own type, level and alloy, including when another member takes it from the chest. Personal storage has 24 item positions including equipped items; the shared chest holds 128. Invalid transfers and full inventories leave items and currency untouched. Older ownership-bit saves materialize one item per owned recipe with their worn slots preserved.

## Skills

The Skills page has **ten active slots and three passive slots**. Drag unlocked skills from the catalog into the appropriate slots; right-click an equipped slot to clear it. Moving an already equipped skill moves its assignment instead of duplicating it. Current-class abilities are available immediately. At 100 class practice, those abilities stay unlocked after retraining. Retraining clears unmastered abilities from the previous class and fills empty slots with the new class's abilities; mastered assignments remain. Titles retain their separate bonuses.

Seven implemented active abilities are exposed: healing, blessing, banner rally, hex field, stone casting, archery, and shield retaliation. Five passive choices expose existing mechanics: bulwark, fleetfoot, bloodrage, weapon drill and quickdraw. The ten active slots allow expansion; empty slots do not pretend that additional abilities already exist. Removing a skill disables its corresponding mechanic. Crafted ranged weapons remain usable through equipment, independently of starter ranged skills.

## Crafting and reforging

Craft new items on the Forge page. Crafting adds an item to personal storage; drag it into a slot to wear it. The older inspector/training recipe shortcuts still craft-and-equip or toggle existing gear.

Select an inventory item or drag it to the anvil. Upgrade it from **level 1 to level 5**, then **melt and reforge** that same item into the next alloy at level 1:

**Iron → Steel → Crucible steel → Meteoric alloy**

An item's power is `1 + 0.06 × (level - 1) + 0.35 × alloy_index`. A level-1 item in the next alloy is stronger than the previous alloy at level 5. Upgrading weapons increases damage, armor improves mitigation, vitality charms increase HP, and war banners strengthen an equipped, learned rally aura. Alloy names describe the item's metal or fittings; a bow still has a wooden bow and string. The original item type and equipped/storage position survive reforging. The consumed item is replaced once, rather than copied into a second item. Meteoric level 5 is the cap.

The displayed gold/salvage cost is charged once per successful operation. Forge animation supplies furnace glow, hammer motion and sparks; the completed transaction saves immediately, so closing the screen cannot duplicate or lose a queued item.

## Longer, bloodier battles

Maximum HP is **three times the previous formula**, across both campaign and Lab fighters, including class, title, equipment and level contributions. Starter HP is 300, or 330 for royalty. Ordinary damage and healing amounts stay unchanged. The battle cap is 360 simulated seconds. Injured fighters can withdraw but must periodically re-engage; local collision steering preserves valid ramp routes.

Blood bursts emit **ten times as many droplets**, using a bounded pool of 8,192 particles. Larger droplets, larger death/severance pools, and more frequent wound trails accumulate on the persistent terrain stain canvas. Harmless contact still produces no blood.

Equipment, selected skills, personal inventories, shared chests, currency, levels and alloys use the existing validated campaign save/backup path. Mid-battle changes are rejected. Changes before the first tick update the staged fighters; changes after battle affect the next battle while preserving historical reports.

## Rendered checks

[Member character sheet](images/rpg-character-witness.png), [King character sheet](images/rpg-king-witness.png), [skill slots](images/rpg-skills-witness.png), [forge animation and reforged steel item](images/rpg-forge-witness.png), and [tenfold gore in combat](images/gore-tenfold-witness.png).

The isolated pointer witness equips an item, returns it to the shared chest, carries it back into personal storage and reloads the save. Separate pointer witnesses move a healing skill to slot ten and drag an iron level-5 item onto the anvil before clicking the actual forge button. These QA fixtures never replace the player's campaign.
