//! Atomic camp transactions. Battle snapshots and historical reports never change here.
use crate::{
    equipment::{Gear, Item},
    progression::{Campaign, Phalanx},
    skills::{Skill, SkillLoadout},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    Personal(usize),
    Shared(usize),
    Equipped(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    Personal,
    Shared,
    Equipped(usize),
}
fn item(team: &Phalanx, fighter: usize, location: Location) -> Result<Item, String> {
    let e = &team
        .fighters
        .get(fighter)
        .ok_or("Invalid fighter")?
        .equipment;
    match location {
        Location::Personal(i) => e.items.get(i).copied().flatten(),
        Location::Shared(i) => team.inventory.get(i).copied(),
        Location::Equipped(s) => e
            .equipped
            .get(s)
            .copied()
            .flatten()
            .and_then(|i| e.items.get(i).copied().flatten()),
    }
    .ok_or("There is no item here".into())
}
impl Campaign {
    pub fn inventory_item(
        &self,
        team: usize,
        fighter: usize,
        location: Location,
    ) -> Result<Item, String> {
        item(
            self.teams.get(team).ok_or("Invalid squad")?,
            fighter,
            location,
        )
    }
    pub fn transfer_item(
        &mut self,
        team: usize,
        fighter: usize,
        source: Location,
        destination: Destination,
    ) -> Result<String, String> {
        self.validate()?;
        let mut squad = self.teams.get(team).ok_or("Invalid squad")?.clone();
        squad
            .fighters
            .get_mut(fighter)
            .ok_or("Invalid fighter")?
            .equipment
            .migrate();
        let object = item(&squad, fighter, source)?;
        if let Destination::Equipped(slot) = destination {
            if object.gear.slot() != slot {
                return Err("That item does not fit this equipment slot".into());
            }
        }
        let e = &mut squad.fighters[fighter].equipment;
        let source_index = match source {
            Location::Personal(i) => Some(i),
            Location::Equipped(s) => e.equipped[s],
            Location::Shared(_) => None,
        };
        match destination {
            Destination::Equipped(slot) => {
                let index = if let Some(i) = source_index {
                    i
                } else {
                    if let Location::Shared(i) = source {
                        squad.inventory.remove(i);
                    }
                    e.insert(object)?
                };
                for slot in &mut e.equipped {
                    if *slot == Some(index) {
                        *slot = None;
                    }
                }
                e.equipped[slot] = Some(index);
            }
            Destination::Personal => {
                if let Some(i) = source_index {
                    for slot in &mut e.equipped {
                        if *slot == Some(i) {
                            *slot = None;
                        }
                    }
                } else {
                    e.insert(object)?;
                    if let Location::Shared(i) = source {
                        squad.inventory.remove(i);
                    }
                }
            }
            Destination::Shared => {
                let Some(index) = source_index else {
                    return Err("Already in the phalanx inventory".into());
                };
                if squad.inventory.len() >= 128 {
                    return Err("Phalanx inventory is full".into());
                }
                for slot in &mut e.equipped {
                    if *slot == Some(index) {
                        *slot = None;
                    }
                }
                e.items[index] = None;
                squad.inventory.push(object);
            }
        }
        e.sync();
        self.teams[team] = squad;
        Ok(format!("Moved {}", object.label()))
    }
    pub fn craft_item(
        &mut self,
        team: usize,
        fighter: usize,
        gear: Gear,
    ) -> Result<String, String> {
        self.validate()?;
        let mut squad = self.teams.get(team).ok_or("Invalid squad")?.clone();
        let f = squad.fighters.get_mut(fighter).ok_or("Invalid fighter")?;
        if let Some(reason) = gear.locked_reason(f.mastery) {
            return Err(reason);
        }
        let (gold, salvage) = gear.cost();
        if squad.gold < gold || squad.salvage < salvage {
            return Err(format!("Need {gold} gold + {salvage} salvage"));
        }
        let item = Item::new(gear);
        f.equipment.insert(item)?;
        squad.gold -= gold;
        squad.salvage -= salvage;
        self.teams[team] = squad;
        Ok(format!(
            "Forged {} into the personal inventory",
            item.label()
        ))
    }
    pub fn forge_item(
        &mut self,
        team: usize,
        fighter: usize,
        location: Location,
    ) -> Result<String, String> {
        self.validate()?;
        let mut squad = self.teams.get(team).ok_or("Invalid squad")?.clone();
        squad
            .fighters
            .get_mut(fighter)
            .ok_or("Invalid fighter")?
            .equipment
            .migrate();
        let old = item(&squad, fighter, location)?;
        let (gold, salvage) = old
            .forge_cost()
            .ok_or("This item has reached its final alloy and level")?;
        if squad.gold < gold || squad.salvage < salvage {
            return Err(format!("Need {gold} gold + {salvage} salvage"));
        }
        let new = old.forged().unwrap();
        let e = &mut squad.fighters[fighter].equipment;
        match location {
            Location::Personal(i) => e.items[i] = Some(new),
            Location::Shared(i) => squad.inventory[i] = new,
            Location::Equipped(s) => e.items[e.equipped[s].unwrap()] = Some(new),
        }
        e.sync();
        squad.gold -= gold;
        squad.salvage -= salvage;
        self.teams[team] = squad;
        Ok(format!(
            "{} {} -> {}",
            if old.level == Item::MAX_LEVEL {
                "Melted and reforged"
            } else {
                "Upgraded"
            },
            old.label(),
            new.label()
        ))
    }
    pub fn set_skill(
        &mut self,
        team: usize,
        fighter: usize,
        passive: bool,
        index: usize,
        skill: Option<Skill>,
    ) -> Result<String, String> {
        self.validate()?;
        let f = self
            .teams
            .get_mut(team)
            .and_then(|t| t.fighters.get_mut(fighter))
            .ok_or("Invalid fighter")?;
        let mut loadout = f
            .skills
            .unwrap_or_else(|| SkillLoadout::defaults(f.class, f.mastery));
        loadout.assign(passive, index, skill, f.class, f.mastery)?;
        f.skills = Some(loadout);
        Ok("Skill loadout saved for the next battle".into())
    }
}
