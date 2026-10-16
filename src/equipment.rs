//! Persistent workshop recipes. One weapon, armor and charm can be worn together.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gear {
    TemperedWeapon,
    BalancedWeapon,
    IronArmor,
    TrailArmor,
    VitalityCharm,
}
impl Gear {
    pub const ALL: [Self; 5] = [
        Self::TemperedWeapon,
        Self::BalancedWeapon,
        Self::IronArmor,
        Self::TrailArmor,
        Self::VitalityCharm,
    ];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|g| *g == self).unwrap()
    }
    pub fn slot(self) -> usize {
        match self {
            Self::TemperedWeapon | Self::BalancedWeapon => 0,
            Self::IronArmor | Self::TrailArmor => 1,
            Self::VitalityCharm => 2,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::TemperedWeapon => "Tempered weapon",
            Self::BalancedWeapon => "Balanced weapon",
            Self::IronArmor => "Iron armor",
            Self::TrailArmor => "Trail armor",
            Self::VitalityCharm => "Vitality charm",
        }
    }
    pub fn effect(self) -> &'static str {
        match self {
            Self::TemperedWeapon => "+20% damage, -8% speed",
            Self::BalancedWeapon => "+8% damage, +5% speed",
            Self::IronArmor => "-20% incoming damage, -15% speed",
            Self::TrailArmor => "+12% speed, +8% incoming damage",
            Self::VitalityCharm => "+20 maximum HP",
        }
    }
    pub fn cost(self) -> (u32, u32) {
        match self {
            Self::TemperedWeapon => (25, 8),
            Self::BalancedWeapon => (20, 6),
            Self::IronArmor => (30, 10),
            Self::TrailArmor => (20, 6),
            Self::VitalityCharm => (35, 12),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equipment {
    pub owned: u8,
    pub slots: [Option<Gear>; 3],
}
impl Equipment {
    pub fn owns(self, gear: Gear) -> bool {
        self.owned & (1 << gear.index()) != 0
    }
    pub fn valid(self) -> bool {
        self.owned & !31 == 0
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
            1.20
        } else if self.has(Gear::BalancedWeapon) {
            1.08
        } else {
            1.0
        }
    }
    pub fn incoming(self) -> f32 {
        if self.has(Gear::IronArmor) {
            0.80
        } else if self.has(Gear::TrailArmor) {
            1.08
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
            20.0
        } else {
            0.0
        }
    }
    pub fn summary(self) -> String {
        self.slots
            .iter()
            .map(|g| g.map(Gear::name).unwrap_or("Starter"))
            .collect::<Vec<_>>()
            .join(" / ")
    }
}
