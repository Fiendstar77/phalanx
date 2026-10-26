//! Persistent workshop recipes. One weapon, armor and charm can be worn together.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gear {
    TemperedWeapon,
    BalancedWeapon,
    IronArmor,
    TrailArmor,
    VitalityCharm,
    WarBanner,
    ThrowingSpear,
    Atlatl,
    Bow,
    Kusarigama,
    RazorWheel,
}
impl Gear {
    pub const ALL: [Self; 11] = [
        Self::TemperedWeapon,
        Self::BalancedWeapon,
        Self::IronArmor,
        Self::TrailArmor,
        Self::VitalityCharm,
        Self::WarBanner,
        Self::ThrowingSpear,
        Self::Atlatl,
        Self::Bow,
        Self::Kusarigama,
        Self::RazorWheel,
    ];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|g| *g == self).unwrap()
    }
    pub fn slot(self) -> usize {
        match self {
            Self::TemperedWeapon
            | Self::BalancedWeapon
            | Self::ThrowingSpear
            | Self::Atlatl
            | Self::Bow
            | Self::Kusarigama
            | Self::RazorWheel => 0,
            Self::IronArmor | Self::TrailArmor => 1,
            Self::VitalityCharm | Self::WarBanner => 2,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::TemperedWeapon => "Tempered weapon",
            Self::BalancedWeapon => "Balanced weapon",
            Self::IronArmor => "Iron armor",
            Self::TrailArmor => "Trail armor",
            Self::VitalityCharm => "Vitality charm",
            Self::WarBanner => "War banner",
            Self::ThrowingSpear => "Throwing spear",
            Self::Atlatl => "Atlatl",
            Self::Bow => "Bow",
            Self::Kusarigama => "Chain-sickle",
            Self::RazorWheel => "Returning razor wheel",
        }
    }
    pub fn effect(self) -> &'static str {
        match self {
            Self::TemperedWeapon => "+20% damage, -8% speed",
            Self::BalancedWeapon => "+8% damage, +5% speed",
            Self::IronArmor => "-20% incoming damage, -15% speed",
            Self::TrailArmor => "+12% speed, +8% incoming damage",
            Self::VitalityCharm => "+60 maximum HP",
            Self::WarBanner => "Enables retained Banner Bearer rally aura",
            Self::ThrowingSpear => crate::weapons::ProjectileWeapon::Spear.description(),
            Self::Atlatl => crate::weapons::ProjectileWeapon::Atlatl.description(),
            Self::Bow => crate::weapons::ProjectileWeapon::Bow.description(),
            Self::Kusarigama => "3.3m reach; successful hooks snare and pull along clear ground",
            Self::RazorWheel => {
                "Spinning return blade; physical outbound and return hits; costs 4 effort"
            }
        }
    }
    pub fn locked_reason(self, mastery: crate::mastery::Mastery) -> Option<String> {
        use crate::weapons::ProjectileWeapon as W;
        if self == Self::Bow {
            return (mastery.weapon_points(W::Atlatl) < 100
                && !mastery.retained(crate::FighterClass::Ranger))
            .then(|| {
                format!(
                    "Requires 100 Atlatl practice ({}/100) or Ranger mastery ({}/100)",
                    mastery.weapon_points(W::Atlatl),
                    mastery.points(crate::FighterClass::Ranger)
                )
            });
        }
        let previous = match self {
            Self::ThrowingSpear => W::Rock,
            Self::Atlatl => W::Spear,
            _ => return None,
        };
        let points = mastery.weapon_points(previous);
        (points < 100).then(|| format!("Requires 100 {} practice ({points}/100)", previous.name()))
    }
    pub fn cost(self) -> (u32, u32) {
        match self {
            Self::TemperedWeapon => (25, 8),
            Self::BalancedWeapon => (20, 6),
            Self::IronArmor => (30, 10),
            Self::TrailArmor => (20, 6),
            Self::VitalityCharm => (35, 12),
            Self::WarBanner => (25, 8),
            Self::ThrowingSpear => (30, 8),
            Self::Atlatl => (45, 12),
            Self::Bow => (55, 14),
            Self::Kusarigama | Self::RazorWheel => (5, 0),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equipment {
    pub owned: u16,
    pub slots: [Option<Gear>; 3],
    /// Version zero is the old ownership-bit save. Materialize it before mutations.
    #[serde(default)]
    pub inventory_version: u8,
    #[serde(default)]
    pub items: [Option<Item>; 24],
    #[serde(default)]
    pub equipped: [Option<usize>; 3],
}
impl Equipment {
    pub fn projectile_weapon(self) -> Option<crate::weapons::ProjectileWeapon> {
        use crate::weapons::ProjectileWeapon as W;
        if self.has(Gear::ThrowingSpear) {
            Some(W::Spear)
        } else if self.has(Gear::Atlatl) {
            Some(W::Atlatl)
        } else if self.has(Gear::Bow) {
            Some(W::Bow)
        } else {
            None
        }
    }
    pub fn owns(self, gear: Gear) -> bool {
        self.owned & (1 << gear.index()) != 0
    }
    pub fn valid(self) -> bool {
        ((self.inventory_version == 0
            && self.items.iter().all(Option::is_none)
            && self.equipped.iter().all(Option::is_none))
            || (self.inventory_version == 1
                && self.items.iter().flatten().all(|i| i.valid())
                && self
                    .equipped
                    .iter()
                    .enumerate()
                    .all(|(slot, index)| match index {
                        Some(index) => self.items.get(*index).and_then(|i| *i).is_some_and(|i| {
                            i.gear.slot() == slot && self.slots[slot] == Some(i.gear)
                        }),
                        None => self.slots[slot].is_none(),
                    })
                && self.owned
                    == self
                        .items
                        .iter()
                        .flatten()
                        .fold(0, |mask, item| mask | (1 << item.gear.index()))))
            && self.owned & !((1 << Gear::ALL.len()) - 1) == 0
            && self
                .slots
                .iter()
                .enumerate()
                .all(|(slot, gear)| gear.is_none_or(|g| g.slot() == slot && self.owns(g)))
    }
    pub fn has(self, gear: Gear) -> bool {
        self.slots[gear.slot()] == Some(gear)
    }
    pub fn damage(self) -> f32 {
        if self.has(Gear::TemperedWeapon) {
            1.20 * self.power(0)
        } else if self.has(Gear::BalancedWeapon) {
            1.08 * self.power(0)
        } else {
            self.power(0)
        }
    }
    pub fn incoming(self) -> f32 {
        if self.has(Gear::IronArmor) {
            0.80 / self.power(1)
        } else if self.has(Gear::TrailArmor) {
            1.08 / self.power(1)
        } else {
            1.0
        }
    }
    pub fn speed(self) -> f32 {
        (if self.has(Gear::TemperedWeapon) {
            0.92
        } else if self.has(Gear::BalancedWeapon) {
            1.05
        } else {
            1.0
        }) * if self.has(Gear::IronArmor) {
            0.85
        } else if self.has(Gear::TrailArmor) {
            1.12
        } else {
            1.0
        }
    }
    pub fn health(self) -> f32 {
        if self.has(Gear::VitalityCharm) {
            20.0 * self.power(2)
        } else {
            0.0
        }
    }
    pub fn migrate(&mut self) {
        if self.inventory_version != 0 {
            return;
        }
        for gear in Gear::ALL {
            if self.owns(gear) {
                let index = self.items.iter().position(Option::is_none).unwrap();
                self.items[index] = Some(Item::new(gear));
                if self.has(gear) {
                    self.equipped[gear.slot()] = Some(index);
                }
            }
        }
        self.inventory_version = 1;
        self.sync();
    }
    pub fn sync(&mut self) {
        self.owned = self
            .items
            .iter()
            .flatten()
            .fold(0, |mask, item| mask | (1 << item.gear.index()));
        self.slots = std::array::from_fn(|s| {
            self.equipped[s]
                .and_then(|i| self.items.get(i).copied().flatten())
                .map(|i| i.gear)
        });
    }
    pub fn insert(&mut self, item: Item) -> Result<usize, String> {
        self.migrate();
        let index = self
            .items
            .iter()
            .position(Option::is_none)
            .ok_or("Personal inventory is full")?;
        self.items[index] = Some(item);
        self.sync();
        Ok(index)
    }
    pub fn worn(self, slot: usize) -> Option<Item> {
        self.equipped
            .get(slot)
            .copied()
            .flatten()
            .and_then(|i| self.items.get(i).copied().flatten())
            .or_else(|| self.slots.get(slot).copied().flatten().map(Item::new))
    }
    pub fn power(self, slot: usize) -> f32 {
        self.worn(slot).map_or(1.0, Item::power)
    }
    pub fn summary(self) -> String {
        self.slots
            .iter()
            .enumerate()
            .map(|(slot, _)| {
                self.worn(slot)
                    .map(Item::label)
                    .unwrap_or_else(|| "Starter".into())
            })
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

/// Each physical item carries its own progress. Moving it never changes its level or alloy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub gear: Gear,
    pub level: u8,
    pub alloy: u8,
}
impl Item {
    pub const MAX_LEVEL: u8 = 5;
    pub const MAX_ALLOY: u8 = 3;
    pub fn new(gear: Gear) -> Self {
        Self {
            gear,
            level: 1,
            alloy: 0,
        }
    }
    pub fn valid(self) -> bool {
        (1..=Self::MAX_LEVEL).contains(&self.level) && self.alloy <= Self::MAX_ALLOY
    }
    pub fn alloy_name(self) -> &'static str {
        ["Iron", "Steel", "Crucible steel", "Meteoric alloy"][self.alloy.min(3) as usize]
    }
    pub fn power(self) -> f32 {
        1.0 + (self.level.saturating_sub(1)) as f32 * 0.06 + self.alloy as f32 * 0.35
    }
    pub fn label(self) -> String {
        format!(
            "{} | {} Lv{}",
            self.gear.name(),
            self.alloy_name(),
            self.level
        )
    }
    pub fn forge_cost(self) -> Option<(u32, u32)> {
        if self.level == Self::MAX_LEVEL && self.alloy == Self::MAX_ALLOY {
            return None;
        }
        let (g, s) = self.gear.cost();
        let factor = if self.level == Self::MAX_LEVEL {
            4 * (self.alloy as u32 + 1)
        } else {
            self.level as u32 + self.alloy as u32 * 3
        };
        Some(((g / 3).max(5) * factor, (s / 3).max(2) * factor))
    }
    pub fn forged(self) -> Option<Self> {
        self.forge_cost()?;
        Some(if self.level == Self::MAX_LEVEL {
            Self {
                level: 1,
                alloy: self.alloy + 1,
                ..self
            }
        } else {
            Self {
                level: self.level + 1,
                ..self
            }
        })
    }
}
