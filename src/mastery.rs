//! Permanent practice in each discipline, independent of the current class and title.
use crate::FighterClass;
use serde::{Deserialize, Serialize};
pub const RETAIN_AT: u32 = 100;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mastery {
    #[serde(deserialize_with = "deserialize_classes")]
    pub practice: [u32; 18],
    #[serde(default, deserialize_with = "crate::weapons::deserialize_practice")]
    pub weapons: [u32; 4],
}
impl Mastery {
    pub fn weapon_points(self, weapon: crate::weapons::ProjectileWeapon) -> u32 {
        self.weapons[weapon.index()]
    }
    pub fn train_weapon(&mut self, weapon: crate::weapons::ProjectileWeapon, points: u32) {
        self.weapons[weapon.index()] = self.weapons[weapon.index()]
            .saturating_add(points)
            .min(1_000_000);
    }
    pub fn weapon_summary(self) -> String {
        format!(
            "Weapon practice: Rock {}/100 -> Spear {}/100 -> Atlatl {}/100 -> Bow {}\nEarn practice by firing in completed battles; recipes stay learned across retraining.",
            self.weapons[0], self.weapons[1], self.weapons[2], self.weapons[3]
        )
    }
    pub fn points(self, class: FighterClass) -> u32 {
        self.practice[class.index()]
    }
    pub fn retained(self, class: FighterClass) -> bool {
        self.points(class) >= RETAIN_AT
    }
    pub fn rank(self, class: FighterClass) -> u32 {
        match self.points(class) {
            600.. => 3,
            300.. => 2,
            100.. => 1,
            _ => 0,
        }
    }
    pub fn train(&mut self, class: FighterClass, points: u32) {
        let value = &mut self.practice[class.index()];
        *value = value.saturating_add(points).min(1_000_000);
    }
    pub fn valid(self) -> bool {
        self.practice
            .iter()
            .chain(self.weapons.iter())
            .all(|p| *p <= 1_000_000)
    }
    pub fn summary(self, current: FighterClass) -> String {
        let retained = FighterClass::ALL
            .into_iter()
            .filter(|c| self.retained(*c))
            .map(|c| format!("{} R{}", c.name(), self.rank(c)))
            .collect::<Vec<_>>();
        format!(
            "Training {}: {} practice | retain at 100; ranks at 100/300/600\nRetained: {}",
            current.name(),
            self.points(current),
            if retained.is_empty() {
                "none yet".into()
            } else {
                retained.join(" / ")
            }
        )
    }
}

fn deserialize_classes<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[u32; 18], D::Error> {
    let values = Vec::<u32>::deserialize(d)?;
    if values.len() != 10 && values.len() != 18 {
        return Err(serde::de::Error::custom(
            "expected 10 legacy or 18 class tracks",
        ));
    }
    let mut tracks = [0; 18];
    tracks[..values.len()].copy_from_slice(&values);
    Ok(tracks)
}
