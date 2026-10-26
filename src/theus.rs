//! Theus v1: portable character, skill, effort and opposed combat rules.
//! Original terminology and critical effects; no renderer or Phalanx world types.
use serde::{Deserialize, Serialize};
pub const VERSION: u32 = 1;
pub const NAME: &str = "Theus";
pub const ATTRIBUTE_NAMES: [&str; 4] = ["Might", "Agility", "Insight", "Resolve"];
pub const ATTRIBUTE_BUDGET: u16 = 48;
pub const SKILL_NAMES: [&str; 6] = [
    "Melee",
    "Marksmanship",
    "Acrobatics",
    "Awareness",
    "Coordination",
    "Medicine",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sheet {
    pub version: u32,
    pub attributes: [u8; 4],
    pub practice: [u16; 6],
}
impl Default for Sheet {
    fn default() -> Self {
        Self {
            version: VERSION,
            attributes: [11; 4],
            practice: [0; 6],
        }
    }
}
impl Sheet {
    pub fn from_temperament(values: [f32; 4]) -> Self {
        Self {
            attributes: values.map(|v| 10 + (v.clamp(0., 1.) * 2.) as u8),
            ..Self::default()
        }
    }
    pub fn valid(self) -> bool {
        self.version == VERSION
            && self.practice.iter().all(|v| *v <= 600)
            && self.attributes.iter().all(|v| (6..=16).contains(v))
            && self.attributes.iter().map(|v| *v as u16).sum::<u16>() <= ATTRIBUTE_BUDGET
    }
    pub fn unspent(self) -> u16 {
        ATTRIBUTE_BUDGET.saturating_sub(self.attributes.iter().map(|v| *v as u16).sum())
    }
    pub fn adjust(&mut self, attribute: usize, delta: i8) -> bool {
        let Some(&old) = self.attributes.get(attribute) else {
            return false;
        };
        let next = old as i16 + delta as i16;
        if !(6..=16).contains(&next) || (delta > 0 && delta as u16 > self.unspent()) {
            return false;
        }
        self.attributes[attribute] = next as u8;
        true
    }
    pub fn rank(self, skill: usize) -> i16 {
        (self.practice.get(skill).copied().unwrap_or(0).min(600) / 100) as i16
    }
    pub fn rating(self, skill: usize, training: i16) -> i16 {
        let attribute = match skill {
            0..=2 => 1,
            3 | 4 => 2,
            _ => 3,
        };
        (self.attributes[attribute] as i16 + 2 + self.rank(skill) + training).clamp(6, 18)
    }
    pub fn endurance(self) -> f32 {
        (self.attributes[0] + self.attributes[3]) as f32 * 1.7
    }
    pub fn train(&mut self, skill: usize, amount: u16) {
        if let Some(v) = self.practice.get_mut(skill) {
            *v = v.saturating_add(amount).min(600);
        }
    }
}
fn mix(mut n: u64) -> u64 {
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    n ^ (n >> 31)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    pub dice: [u8; 3],
    pub target: i16,
    pub margin: i16,
}
pub fn check(seed: u64, target: i16) -> Check {
    let dice = std::array::from_fn(|i| {
        1 + (mix(seed ^ (i as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15)) % 6) as u8
    });
    Check {
        dice,
        target,
        margin: target - dice.iter().map(|v| *v as i16).sum::<i16>(),
    }
}
pub fn open_percentile(seed: u64) -> u16 {
    let mut total = 0;
    for i in 0..4 {
        let roll = 1 + (mix(seed ^ (i as u64).wrapping_mul(7919)) % 100) as u16;
        total += roll;
        if roll < 97 {
            break;
        }
    }
    total
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Miss,
    Dodge,
    Parry,
    Hit,
    Critical,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Critical {
    None,
    OffBalance,
    GuardSplit,
    Launch,
}
impl Critical {
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "Clean hit",
            Self::OffBalance => "Off balance",
            Self::GuardSplit => "Guard split",
            Self::Launch => "Launched",
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Strike {
    pub attack: i16,
    pub defence: i16,
    pub damage: f32,
    pub armour: f32,
    pub parry: bool,
    pub exposed: bool,
    pub critical_bonus: i16,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    pub attack: Check,
    pub defence: Option<Check>,
    pub outcome: Outcome,
    pub damage: f32,
    pub critical_roll: u16,
    pub critical: Critical,
}
pub fn resolve(seed: u64, strike: Strike) -> Resolution {
    let attack = check(seed, strike.attack);
    let mut result = Resolution {
        attack,
        defence: None,
        outcome: Outcome::Miss,
        damage: 0.,
        critical_roll: 0,
        critical: Critical::None,
    };
    if attack.margin < 0 {
        return result;
    }
    let defence = check(
        seed ^ 0x88c9a32d71,
        strike.defence - if strike.exposed { 3 } else { 0 },
    );
    result.defence = Some(defence);
    // A well-placed strike can beat an otherwise successful defence.
    if defence.margin >= 0 && defence.margin + 2 >= attack.margin {
        result.outcome = if strike.parry {
            Outcome::Parry
        } else {
            Outcome::Dodge
        };
        return result;
    }
    result.outcome = Outcome::Hit;
    let severity = (open_percentile(seed ^ 0x177b923f) as i32
        + attack.margin.max(0) as i32 * 5
        + strike.critical_bonus as i32)
        .max(0) as u16;
    result.critical_roll = severity;
    result.critical = match severity {
        0..=99 => Critical::None,
        100..=124 => Critical::OffBalance,
        125..=159 => Critical::GuardSplit,
        _ => Critical::Launch,
    };
    if result.critical != Critical::None {
        result.outcome = Outcome::Critical;
    }
    let quality = 1.
        + attack.margin.max(0) as f32 * 0.035
        + if result.outcome == Outcome::Critical {
            0.25
        } else {
            0.
        };
    result.damage = (strike.damage * quality - strike.armour).max(strike.damage * 0.18);
    result
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Technique {
    #[default]
    None,
    BurstStep,
    WallKick,
    Cartwheel,
    AegisLaunch,
    MeteorRelay,
    Crossfire,
    Pincer,
    Counter,
}
impl Technique {
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "Strike",
            Self::BurstStep => "Burst step",
            Self::WallKick => "Wall kick",
            Self::Cartwheel => "Rolling evasion",
            Self::AegisLaunch => "Aegis launch",
            Self::MeteorRelay => "Meteor relay",
            Self::Crossfire => "Crossfire",
            Self::Pincer => "Scissor pincer",
            Self::Counter => "Riposte",
        }
    }
    pub fn combination(self) -> bool {
        matches!(
            self,
            Self::AegisLaunch | Self::MeteorRelay | Self::Crossfire | Self::Pincer
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub sheet: Sheet,
    pub stamina: f32,
    pub clock: u32,
    pub stagger_until: u32,
    pub exposed_until: u32,
    pub airborne_since: u32,
    pub airborne_until: u32,
    pub opener: Option<usize>,
    pub launcher: Option<usize>,
    pub combo_ready: u32,
    pub burst_until: u32,
    pub burst_ready: u32,
    pub technique: Technique,
    pub technique_since: u32,
    pub technique_until: u32,
    pub windup_target: Option<usize>,
    pub strike_at: u32,
}
impl Actor {
    pub fn new(sheet: Sheet) -> Self {
        Self {
            stamina: sheet.endurance(),
            sheet,
            clock: 0,
            stagger_until: 0,
            exposed_until: 0,
            airborne_since: 0,
            airborne_until: 0,
            opener: None,
            launcher: None,
            combo_ready: 0,
            burst_until: 0,
            burst_ready: 0,
            technique: Technique::None,
            technique_since: 0,
            technique_until: 0,
            windup_target: None,
            strike_at: 0,
        }
    }
    pub fn tired(&self) -> bool {
        self.stamina < self.sheet.endurance() * 0.2
    }
    pub fn movement(&self) -> f32 {
        if self.stagger_until > self.clock {
            0.5
        } else if self.burst_until > self.clock {
            1.9
        } else if self.tired() {
            0.85
        } else {
            1.
        }
    }
    pub fn spend(&mut self, effort: f32) -> bool {
        if self.stamina < effort {
            false
        } else {
            self.stamina -= effort;
            true
        }
    }
    pub fn height(&self, tick: u32) -> f32 {
        self.height_at(tick as f32)
    }
    pub fn height_at(&self, tick: f32) -> f32 {
        let arc = |since: u32, until: u32, apex: f32| {
            if tick < since as f32 || tick >= until as f32 || until <= since {
                return 0.;
            }
            let t = (tick - since as f32) / (until - since) as f32;
            (std::f32::consts::PI * t).sin() * apex
        };
        arc(self.airborne_since, self.airborne_until, 3.5).max(match self.technique {
            Technique::MeteorRelay => arc(self.technique_since, self.technique_until, 4.2),
            Technique::WallKick => arc(self.technique_since, self.technique_until, 2.5),
            _ => 0.,
        })
    }
    pub fn perform(&mut self, technique: Technique, tick: u32, duration: u32) {
        self.technique = technique;
        self.technique_since = tick;
        self.technique_until = tick + duration;
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub sequence: u64,
    pub tick: u32,
    pub attacker: usize,
    pub defender: usize,
    pub technique: Technique,
    pub partner: Option<usize>,
    pub resolution: Resolution,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_checks_are_reproducible_and_training_improves_success() {
        let mut low = 0;
        let mut high = 0;
        for seed in 0..4096 {
            let a = check(seed, 9);
            assert_eq!(a, check(seed, 9));
            assert!(a.dice.iter().all(|v| (1..=6).contains(v)));
            low += usize::from(a.margin >= 0);
            high += usize::from(check(seed, 14).margin >= 0);
        }
        assert!(high > low * 2);
        assert!((0..10000).any(|seed| open_percentile(seed) > 100));
        assert!((0..10000).all(|seed| (1..=400).contains(&open_percentile(seed))));
    }
    #[test]
    fn defended_and_missed_strikes_do_no_damage_and_armour_reduces_hits() {
        let input = Strike {
            attack: 14,
            defence: 10,
            damage: 20.,
            armour: 0.,
            parry: true,
            exposed: false,
            critical_bonus: 0,
        };
        let mut defended = false;
        let mut hit = false;
        let mut missed = false;
        for seed in 0..300 {
            let a = resolve(seed, input);
            let b = resolve(
                seed,
                Strike {
                    armour: 5.,
                    ..input
                },
            );
            assert_eq!(a, resolve(seed, input));
            match a.outcome {
                Outcome::Miss => {
                    missed = true;
                    assert_eq!(a.damage, 0.);
                }
                Outcome::Dodge | Outcome::Parry => {
                    defended = true;
                    assert_eq!(a.damage, 0.);
                }
                _ => {
                    hit = true;
                    assert!(b.damage < a.damage);
                }
            }
        }
        assert!(defended && hit && missed);
    }
    #[test]
    fn point_buy_effort_and_airtime_are_bounded() {
        let mut sheet = Sheet::default();
        assert!(sheet.adjust(0, 4));
        assert!(!sheet.adjust(1, 1));
        assert!(sheet.valid());
        assert!(!sheet.adjust(0, 10));
        let mut actor = Actor::new(sheet);
        assert!(!actor.spend(1000.));
        assert!(actor.spend(5.));
        actor.perform(Technique::MeteorRelay, 10, 10);
        assert_eq!(actor.height(9), 0.);
        assert_eq!(actor.height(20), 0.);
        assert!((actor.height(15) - 4.2).abs() < 0.001);
    }
}
