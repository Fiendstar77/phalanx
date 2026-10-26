//! Learned abilities are permanent; equipped abilities have explicit capacity.
use crate::{FighterClass as C, mastery::Mastery};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Skill {
    Heal,
    Bless,
    Rally,
    Hex,
    Stonecast,
    Archery,
    ShieldBash,
    Bulwark,
    Fleetfoot,
    Bloodrage,
    WeaponDrill,
    Quickdraw,
    WallDance,
    SmokeBomb,
    ShadowSeal,
    Fencing,
    Ambush,
    Flow,
    Veilcraft,
    Reaping,
    ToxicSmoke,
    Occult,
    Springheel,
}
impl Skill {
    pub const ALL: [Self; 23] = [
        Self::Heal,
        Self::Bless,
        Self::Rally,
        Self::Hex,
        Self::Stonecast,
        Self::Archery,
        Self::ShieldBash,
        Self::Bulwark,
        Self::Fleetfoot,
        Self::Bloodrage,
        Self::WeaponDrill,
        Self::Quickdraw,
        Self::WallDance,
        Self::SmokeBomb,
        Self::ShadowSeal,
        Self::Fencing,
        Self::Ambush,
        Self::Flow,
        Self::Veilcraft,
        Self::Reaping,
        Self::ToxicSmoke,
        Self::Occult,
        Self::Springheel,
    ];
    pub fn class(self) -> C {
        match self {
            Self::Heal => C::Healer,
            Self::Bless => C::Paladin,
            Self::Rally => C::BannerBearer,
            Self::Hex => C::Hexer,
            Self::Stonecast => C::RockThrower,
            Self::Archery | Self::Quickdraw => C::Ranger,
            Self::ShieldBash | Self::Bulwark => C::Guardian,
            Self::Fleetfoot => C::Scout,
            Self::Bloodrage => C::Berserker,
            Self::WeaponDrill => C::Warrior,
            Self::WallDance | Self::SmokeBomb => C::Scout,
            Self::ShadowSeal => C::Hexer,
            Self::Fencing => C::Duelist,
            Self::Ambush => C::Assassin,
            Self::Flow => C::Monk,
            Self::Veilcraft => C::Shinobi,
            Self::Reaping => C::Reaper,
            Self::ToxicSmoke => C::Alchemist,
            Self::Occult => C::Warlock,
            Self::Springheel => C::Acrobat,
        }
    }
    pub fn passive(self) -> bool {
        matches!(
            self,
            Self::Bulwark
                | Self::Fleetfoot
                | Self::Bloodrage
                | Self::WeaponDrill
                | Self::Quickdraw
                | Self::Fencing
                | Self::Ambush
                | Self::Flow
                | Self::Veilcraft
                | Self::Reaping
                | Self::Occult
                | Self::Springheel
        )
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Heal => "Mend wounds",
            Self::Bless => "Blessing aura",
            Self::Rally => "Banner rally",
            Self::Hex => "Hex field",
            Self::Stonecast => "Stone casting",
            Self::Archery => "Archery",
            Self::ShieldBash => "Shield retaliation",
            Self::Bulwark => "Bulwark",
            Self::Fleetfoot => "Fleetfoot",
            Self::Bloodrage => "Bloodrage",
            Self::WeaponDrill => "Weapon drill",
            Self::Quickdraw => "Quickdraw",
            Self::WallDance => "Wall dance",
            Self::SmokeBomb => "Smoke bombs",
            Self::ShadowSeal => "Grave binding",
            Self::Fencing => "Fencing",
            Self::Ambush => "Opportunist ambush",
            Self::Flow => "Flow breathing",
            Self::Veilcraft => "Veilcraft",
            Self::Reaping => "Reaping edge",
            Self::ToxicSmoke => "Toxic smoke",
            Self::Occult => "Occult resonance",
            Self::Springheel => "Springheel",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Fencing => "Active parries gain +2 defence and stronger ripostes.",
            Self::Ambush => "Exploit exposed enemies for +35% damage; fast but fragile.",
            Self::Flow => "Recover effort faster; fluid unarmed counters and greater endurance.",
            Self::Veilcraft => "Three smoke bombs; faster wall recovery and smoke ambushes.",
            Self::Reaping => "Longer melee reach; increasingly dangerous to wounded enemies.",
            Self::ToxicSmoke => "Toxic smoke drains enemy effort while covering allies.",
            Self::Occult => "Stronger resisted bindings drain enemy effort and last longer.",
            Self::Springheel => "Fast traversal; improved acrobatics and longer wall holds.",

            Self::Heal => "Heal nearby injured allies every 1.2s",
            Self::Bless => "Nearby allies gain damage and protection",
            Self::Rally => "Rally allies; requires a physical banner",
            Self::Hex => "Nearby enemies deal 25% less damage",
            Self::Stonecast => "Throw stones without crafted ranged gear",
            Self::Archery => "Use a starter bow and archer positioning",
            Self::ShieldBash => "Retaliate against a stopped charge",
            Self::Bulwark => "+90 HP, strong mitigation; slower and weaker attacks",
            Self::Fleetfoot => "+45% movement, -45 HP",
            Self::Bloodrage => "Damage rises as HP falls; +15% incoming damage",
            Self::WeaponDrill => "+5% damage after 100 Warrior practice",
            Self::Quickdraw => "37.5% shorter projectile recovery",
            Self::WallDance => "Climb and vault nearby walls; costs 7 effort; intact legs required",
            Self::SmokeBomb => "Two bombs per battle; smoke obscures both teams and enables escape",
            Self::ShadowSeal => {
                "Resisted shadow binding slows and exposes a visible enemy; costs 8 effort"
            }
        }
    }
    pub fn unlocked(self, current: C, mastery: Mastery) -> bool {
        matches!(self, Self::WallDance | Self::SmokeBomb | Self::ShadowSeal)
            || self.class() == current
            || mastery.retained(self.class())
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillLoadout {
    pub active: [Option<Skill>; 10],
    pub passive: [Option<Skill>; 3],
}
impl SkillLoadout {
    pub fn defaults(current: C, mastery: Mastery) -> Self {
        let mut result = Self::default();
        // Current discipline first; retained disciplines fill remaining capacity.
        for class in std::iter::once(current).chain(C::ALL.into_iter().filter(|c| *c != current)) {
            for skill in Skill::ALL
                .into_iter()
                .filter(|s| s.class() == class && s.unlocked(current, mastery))
            {
                let slots: &mut [Option<Skill>] = if skill.passive() {
                    &mut result.passive
                } else {
                    &mut result.active
                };
                if let Some(slot) = slots.iter_mut().find(|s| s.is_none()) {
                    *slot = Some(skill);
                }
            }
        }
        result
    }
    pub fn has(self, skill: Skill) -> bool {
        self.active
            .iter()
            .chain(self.passive.iter())
            .any(|s| *s == Some(skill))
    }
    pub fn valid(self, current: C, mastery: Mastery) -> bool {
        let mut seen = Vec::new();
        self.active
            .iter()
            .enumerate()
            .chain(self.passive.iter().enumerate().map(|(i, s)| (i + 10, s)))
            .all(|(i, s)| {
                s.is_none_or(|s| {
                    let valid = s.passive() == (i >= 10)
                        && s.unlocked(current, mastery)
                        && !seen.contains(&s);
                    seen.push(s);
                    valid
                })
            })
    }
    pub fn assign(
        &mut self,
        passive: bool,
        index: usize,
        skill: Option<Skill>,
        current: C,
        mastery: Mastery,
    ) -> Result<(), String> {
        if index >= if passive { 3 } else { 10 } {
            return Err("Invalid skill slot".into());
        }
        if let Some(s) = skill {
            if s.passive() != passive {
                return Err("Active and passive slots are separate".into());
            }
            if !s.unlocked(current, mastery) {
                return Err("Master this discipline (100 practice) to retain its skills".into());
            }
            for v in self.active.iter_mut().chain(self.passive.iter_mut()) {
                if *v == Some(s) {
                    *v = None;
                }
            }
        }
        if passive {
            self.passive[index] = skill;
        } else {
            self.active[index] = skill;
        }
        Ok(())
    }
    pub fn retrain(&mut self, current: C, mastery: Mastery) {
        for slot in self.active.iter_mut().chain(self.passive.iter_mut()) {
            if slot.is_some_and(|s| !s.unlocked(current, mastery)) {
                *slot = None;
            }
        }
        for skill in Skill::ALL.into_iter().filter(|s| s.class() == current) {
            if !self.has(skill) {
                let slots: &mut [Option<Skill>] = if skill.passive() {
                    &mut self.passive
                } else {
                    &mut self.active
                };
                if let Some(s) = slots.iter_mut().find(|s| s.is_none()) {
                    *s = Some(skill);
                }
            }
        }
    }
}
