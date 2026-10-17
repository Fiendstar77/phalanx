//! Learned projectile technologies. Equipment selects the weapon; class and title stay independent.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectileWeapon {
    #[default]
    Rock,
    Spear,
    Atlatl,
    Bow,
}
impl ProjectileWeapon {
    pub const ALL: [Self; 4] = [Self::Rock, Self::Spear, Self::Atlatl, Self::Bow];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Rock => "Rock",
            Self::Spear => "Throwing spear",
            Self::Atlatl => "Atlatl",
            Self::Bow => "Bow",
        }
    }
    pub fn range(self) -> f32 {
        match self {
            Self::Rock => 6.5,
            Self::Spear => 8.0,
            Self::Atlatl => 10.5,
            Self::Bow => 10.0,
        }
    }
    pub fn damage(self) -> f32 {
        match self {
            Self::Rock => 7.0,
            Self::Spear => 18.0,
            Self::Atlatl => 26.0,
            Self::Bow => 13.0,
        }
    }
    pub fn speed(self) -> f32 {
        match self {
            Self::Rock => 10.0,
            Self::Spear => 17.0,
            Self::Atlatl => 23.0,
            Self::Bow => 26.0,
        }
    }
    /// Rocks keep the existing immediate release; new technologies require a planted windup.
    pub fn windup(self) -> f32 {
        match self {
            Self::Rock => 0.0,
            Self::Spear => 0.5,
            Self::Atlatl => 0.9,
            Self::Bow => 0.65,
        }
    }
    pub fn recovery(self) -> f32 {
        match self {
            Self::Rock => 1.6,
            Self::Spear => 1.7,
            Self::Atlatl => 2.3,
            Self::Bow => 0.85,
        }
    }
    /// 255 is the unlimited starter-rock supply. Crafted bundles refill between battles.
    pub fn ammunition(self) -> u8 {
        match self {
            Self::Rock => u8::MAX,
            Self::Spear => 6,
            Self::Atlatl => 4,
            Self::Bow => 12,
        }
    }
    pub fn arc(self) -> f32 {
        match self {
            Self::Rock => 1.4,
            Self::Spear => 0.55,
            Self::Atlatl => 0.35,
            Self::Bow => 0.55,
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Rock => "6.5 range | 7 damage | unlimited rocks | 1.6s recovery",
            Self::Spear => "8 range | 18 damage | 6 spears | 0.5s windup + 1.7s recovery",
            Self::Atlatl => "10.5 range | 26 damage | 4 darts | 0.9s windup + 2.3s recovery",
            Self::Bow => "2.2-10 range | 13 damage | 12 arrows | 0.65s draw + 0.85s recovery",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Windup {
    pub weapon: ProjectileWeapon,
    pub target: usize,
    pub remaining: f32,
}

/// Earliest swept-disc interception of a moving projectile against a body this tick.
pub fn intercept(
    from: [f32; 2],
    to: [f32; 2],
    body_from: [f32; 2],
    body_to: [f32; 2],
    radius: f32,
) -> Option<f32> {
    let p = [from[0] - body_from[0], from[1] - body_from[1]];
    let v = [
        to[0] - from[0] - (body_to[0] - body_from[0]),
        to[1] - from[1] - (body_to[1] - body_from[1]),
    ];
    let c = p[0] * p[0] + p[1] * p[1] - radius * radius;
    if c <= 0.0 {
        return Some(0.0);
    }
    let a = v[0] * v[0] + v[1] * v[1];
    let b = p[0] * v[0] + p[1] * v[1];
    let disc = b * b - a * c;
    if a < 1e-10 || disc < 0.0 {
        return None;
    }
    let t = (-b - disc.sqrt()) / a;
    (0.0..=1.0).contains(&t).then_some(t)
}

/// Older saves and reports stored only rock/spear/atlatl practice. Preserve those indices.
pub fn deserialize_practice<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<[u32; 4], D::Error> {
    let values = Vec::<u32>::deserialize(deserializer)?;
    if !(3..=4).contains(&values.len()) {
        return Err(serde::de::Error::custom(
            "expected three or four weapon practice values",
        ));
    }
    let mut out = [0; 4];
    out[..values.len()].copy_from_slice(&values);
    Ok(out)
}
