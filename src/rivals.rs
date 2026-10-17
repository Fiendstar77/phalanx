//! Authored local competitors. Progress is earned by real completed matches, not elapsed time.
use crate::{
    FighterClass as C, Mind,
    equipment::Gear,
    heraldry::Emblem,
    progression::{Campaign, PALETTE, Phalanx},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RivalStyle {
    Obsidian,
    Sentinel,
    Bloodhound,
    Vanguard,
    Null,
}
impl RivalStyle {
    pub const ALL: [Self; 5] = [
        Self::Obsidian,
        Self::Sentinel,
        Self::Bloodhound,
        Self::Vanguard,
        Self::Null,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Obsidian => "The Obsidian Coven",
            Self::Sentinel => "The Iron Sentinel",
            Self::Bloodhound => "The Bloodhound Syndicate",
            Self::Vanguard => "The Vanguard of the Sun",
            Self::Null => "The Null Collective",
        }
    }
    pub fn doctrine(self) -> &'static str {
        match self {
            Self::Obsidian => "Hex & range -> protection hybrids",
            Self::Sentinel => "Mitigation -> defensive rally coverage",
            Self::Bloodhound => "Speed & aggression -> rage/healing hybrids",
            Self::Vanguard => "Cohesion & auras -> offensive mastery",
            Self::Null => "Counters recorded player composition and healing",
        }
    }
    fn primary(self) -> [C; 3] {
        match self {
            Self::Obsidian => [C::Hexer, C::RockThrower, C::Ranger],
            Self::Sentinel => [C::Guardian, C::Paladin, C::Guardian],
            Self::Bloodhound => [C::Scout, C::Berserker, C::Ranger],
            Self::Vanguard => [C::BannerBearer, C::Warrior, C::Paladin],
            Self::Null => [C::Guardian, C::Hexer, C::RockThrower],
        }
    }
    fn secondary(self) -> [C; 3] {
        match self {
            Self::Obsidian => [C::Paladin, C::Healer, C::Guardian],
            Self::Sentinel => [C::BannerBearer, C::Guardian, C::Healer],
            Self::Bloodhound => [C::Berserker, C::Healer, C::Scout],
            Self::Vanguard => [C::Warrior, C::Scout, C::BannerBearer],
            Self::Null => [C::RockThrower, C::Guardian, C::Hexer],
        }
    }
}
pub fn initialize(c: &mut Campaign) {
    if !c.rivals.is_empty() {
        return;
    }
    let template = c.teams[1].clone();
    for (index, style) in RivalStyle::ALL.into_iter().enumerate() {
        if template.rival_style == Some(style) {
            continue;
        }
        let mut team = template.clone();
        team.name = style.name().into();
        team.rival_style = Some(style);
        team.color = PALETTE[(index + 2) % PALETTE.len()];
        team.gold = 100;
        team.salvage = 0;
        team.inventory.clear();
        team.heraldry.emblem = Emblem::ALL[index];
        for (slot, f) in team.fighters.iter_mut().enumerate() {
            let identity = template.fighters[slot]
                .fighter_id
                .bytes()
                .fold(0xcbf29ce484222325_u64, |h, b| {
                    (h ^ b as u64).wrapping_mul(0x100000001b3)
                });
            f.fighter_id = format!("rival-{index}-{slot}-{identity:016x}");
            f.name = format!(
                "{} {}",
                ["Coven", "Sentinel", "Hound", "Solar", "Null"][index],
                slot + 1
            );
            f.class = style.primary()[slot % 3];
            f.experience = 0;
            f.battles = 0;
            f.wins = 0;
            f.history.clear();
            f.wounds.clear();
            f.lost_limbs = 0;
            f.prosthetics = 0;
            f.mastery = Default::default();
            f.skills = None;
            f.equipment = Default::default();
            f.psyche = Default::default();
            f.appearance = Default::default();
            f.mind = if style == RivalStyle::Bloodhound {
                Mind::aggressive()
            } else {
                Mind::disciplined()
            };
            if style == RivalStyle::Vanguard {
                f.mind.teamwork = 0.95;
                f.mind.discipline = 0.9;
            }
            if style == RivalStyle::Obsidian {
                f.mind.caution = 0.85;
                f.mind.patience = 0.8;
            }
        }
        c.rivals.push(team);
    }
}
/// Spend the same earned currencies as the player. No mastery, gold or equipment is granted for free.
pub fn evolve(team: &mut Phalanx, player: &Phalanx, last_healing: f32) {
    let Some(style) = team.rival_style else {
        return;
    };
    let loadout = |f: &crate::progression::Fighter| {
        f.skills
            .unwrap_or_else(|| crate::skills::SkillLoadout::defaults(f.class, f.mastery))
    };
    let ranged = player
        .fighters
        .iter()
        .filter(|f| {
            use crate::weapons::ProjectileWeapon as W;
            let weapon = f.equipment.projectile_weapon().or_else(|| {
                use crate::skills::Skill as S;
                let selected = loadout(f);
                [S::Stonecast, S::Archery, S::Hex]
                    .into_iter()
                    .any(|s| selected.has(s))
                    .then_some(if selected.has(S::Archery) {
                        W::Bow
                    } else {
                        W::Rock
                    })
            });
            weapon.is_some_and(|w| w != W::Bow || f.lost_limbs & !f.prosthetics & 3 == 0)
        })
        .count();
    let quick = player
        .fighters
        .iter()
        .filter(|f| loadout(f).has(crate::skills::Skill::Fleetfoot))
        .count();
    for (slot, f) in team.fighters.iter_mut().enumerate() {
        let primary = style.primary()[slot % 3];
        let secondary = style.secondary()[slot % 3];
        let target = if style == RivalStyle::Null {
            if last_healing > 60.0 {
                [C::Hexer, C::Ranger, C::Scout][slot % 3]
            } else if ranged >= 2 {
                [C::Scout, C::Guardian, C::Ranger][slot % 3]
            } else if quick > 0 {
                [C::Guardian, C::Hexer, C::RockThrower][slot % 3]
            } else {
                primary
            }
        } else if f.mastery.retained(primary) && !f.mastery.retained(secondary) {
            secondary
        } else {
            primary
        };
        if f.class != target && team.gold >= 10 {
            if let Some(skills) = &mut f.skills {
                skills.retrain(target, f.mastery);
            }
            f.class = target;
            team.gold -= 10;
        }
        let missing = f.lost_limbs & !f.prosthetics;
        if missing != 0 && team.gold >= 15 && team.salvage >= 5 {
            f.prosthetics |= 1 << missing.trailing_zeros();
            team.gold -= 15;
            team.salvage -= 5;
        }
        let armor = if style == RivalStyle::Bloodhound || style == RivalStyle::Vanguard {
            Gear::TrailArmor
        } else {
            Gear::IronArmor
        };
        use crate::weapons::ProjectileWeapon as W;
        let weapon = if f.class == C::Ranger || f.mastery.retained(C::Ranger) {
            Gear::BalancedWeapon
        } else if f.mastery.weapon_points(W::Atlatl) >= 100 {
            Gear::Bow
        } else if f.mastery.weapon_points(W::Spear) >= 100 {
            Gear::Atlatl
        } else if f.mastery.weapon_points(W::Rock) >= 100 {
            Gear::ThrowingSpear
        } else {
            Gear::BalancedWeapon
        };
        for gear in [armor, weapon, Gear::WarBanner] {
            if gear == Gear::WarBanner
                && (!f.mastery.retained(C::BannerBearer) || slot == team.standard_bearer)
            {
                continue;
            }
            let (gold, salvage) = gear.cost();
            if !f.equipment.owns(gear) {
                if team.gold < gold || team.salvage < salvage {
                    continue;
                }
                team.gold -= gold;
                team.salvage -= salvage;
                f.equipment
                    .insert(crate::equipment::Item::new(gear))
                    .expect("bounded rival recipes fit inventory");
            }
            f.equipment.migrate();
            f.equipment.equipped[gear.slot()] = f
                .equipment
                .items
                .iter()
                .position(|i| i.is_some_and(|i| i.gear == gear));
            f.equipment.sync();
        }
    }
}
