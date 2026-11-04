//! Rare lasting injury, independent of ordinary damage and cosmetic blood volume.
use crate::{Unit, procedural::hash};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Region {
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}
impl Region {
    pub fn name(self) -> &'static str {
        match self {
            Self::Torso => "torso",
            Self::LeftArm => "left arm",
            Self::RightArm => "right arm",
            Self::LeftLeg => "left leg",
            Self::RightLeg => "right leg",
        }
    }
    pub fn limb(self) -> u8 {
        match self {
            Self::Torso => 0,
            Self::LeftArm => 1,
            Self::RightArm => 2,
            Self::LeftLeg => 4,
            Self::RightLeg => 8,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Laceration {
    pub source_battle: Option<u64>,
    pub hit_sequence: u64,
    pub region: Region,
    pub severity: u8,
    pub blood_remaining: f32,
    pub recovery_bouts: u8,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Flesh {
    pub marks: Vec<Laceration>,
    pub lifetime_count: u32,
}
impl Flesh {
    pub fn blank(&self) -> bool {
        self.marks.is_empty() && self.lifetime_count == 0
    }
    pub fn valid(&self) -> bool {
        self.marks.len() <= 32
            && self.lifetime_count >= self.marks.len() as u32
            && self.lifetime_count <= 1_000_000
            && self.marks.iter().all(|w| {
                (1..=3).contains(&w.severity)
                    && w.blood_remaining.is_finite()
                    && (0.0..=18.0).contains(&w.blood_remaining)
                    && w.recovery_bouts <= 2
            })
    }
    pub fn soreness(&self) -> f32 {
        self.marks
            .iter()
            .filter(|w| w.recovery_bouts > 0)
            .map(|w| f32::from(w.severity))
            .sum::<f32>()
            .min(6.)
    }
    pub fn movement(&self) -> f32 {
        1. - self.soreness() * 0.035
    }
    pub fn bleeding(&self) -> bool {
        self.marks.iter().any(|w| w.blood_remaining > 0.)
    }
    pub fn stanch(&mut self, mut amount: f32) {
        for wound in &mut self.marks {
            let treated = amount.min(wound.blood_remaining).max(0.);
            wound.blood_remaining -= treated;
            amount -= treated;
        }
    }
    pub fn settle(&mut self, battle: u64) {
        for wound in &mut self.marks {
            if wound.source_battle.is_some() {
                wound.recovery_bouts = wound.recovery_bouts.saturating_sub(1);
            } else {
                wound.source_battle = Some(battle);
            }
            // Existing between-bout medical recovery closes bleeding, not scars.
            wound.blood_remaining = 0.;
        }
    }
    pub fn rest(&mut self) {
        for wound in &mut self.marks {
            wound.blood_remaining = 0.;
            wound.recovery_bouts = 0;
        }
    }
    pub fn summary(&self) -> String {
        if self.blank() {
            return "Flesh: no recorded lacerations".into();
        }
        let active = self.marks.iter().filter(|w| w.recovery_bouts > 0).count();
        format!(
            "Flesh: {active} recovering / {} lasting scars | {}\nMovement {:.0}% | healing also stanches bleeding",
            self.lifetime_count.saturating_sub(active as u32),
            if self.bleeding() {
                "BLEEDING"
            } else {
                "bleeding controlled"
            },
            self.movement() * 100.
        )
    }
}
pub fn is_blank(flesh: &Flesh) -> bool {
    flesh.blank()
}

#[derive(Clone, Debug, Serialize)]
pub struct BleedEvent {
    pub tick: u32,
    pub unit: usize,
    pub hit_sequence: u64,
    pub region: Region,
    pub amount: f32,
}

/// A separate deterministic injury draw. Ordinary scratches and blunt hits
/// never create these wounds; this does not consume the tactical RNG stream.
pub(crate) fn lacerate(
    seed: u64,
    sequence: u64,
    unit: &mut Unit,
    amount: f32,
    cutting: bool,
) -> Option<Region> {
    if !cutting
        || unit.theus.is_none()
        || unit.health <= 0.
        || !amount.is_finite()
        || amount < unit.max_health() * 0.04
    {
        return None;
    }
    let roll =
        hash(seed ^ sequence.wrapping_mul(0x6a09e667f3bcc909) ^ (unit.id as u64).wrapping_mul(977));
    if roll % 1000 >= 120 {
        return None;
    }
    let regions = [
        Region::Torso,
        Region::LeftArm,
        Region::RightArm,
        Region::LeftLeg,
        Region::RightLeg,
    ];
    let eligible: Vec<_> = regions
        .into_iter()
        .filter(|r| r.limb() & (unit.lost_limbs | unit.prosthetics) == 0)
        .collect();
    let region = eligible[(roll >> 16) as usize % eligible.len()];
    let severity = if amount >= unit.max_health() * 0.12 {
        3
    } else if amount >= unit.max_health() * 0.075 {
        2
    } else {
        1
    };
    if unit.flesh.marks.len() == 32 {
        let old = unit
            .flesh
            .marks
            .iter()
            .position(|w| w.recovery_bouts == 0)?;
        unit.flesh.marks.remove(old);
    }
    unit.flesh.marks.push(Laceration {
        source_battle: None,
        hit_sequence: sequence,
        region,
        severity,
        blood_remaining: f32::from(severity) * 6.,
        recovery_bouts: 2,
    });
    unit.flesh.lifetime_count = unit.flesh.lifetime_count.saturating_add(1).min(1_000_000);
    Some(region)
}

pub(crate) fn bleed(battle: &mut crate::Battle) {
    for unit in &mut battle.units {
        if unit.health <= 0. {
            continue;
        }
        for wound in &mut unit.flesh.marks {
            let amount = (f32::from(wound.severity) * 0.45 * crate::DT)
                .min(wound.blood_remaining)
                .min(unit.health);
            if amount <= 0. {
                continue;
            }
            wound.blood_remaining -= amount;
            unit.health = (unit.health - amount).max(0.);
            unit.bleeding_loss += amount;
            if let Some(actor) = &mut unit.theus {
                actor.stamina = (actor.stamina - amount * 0.8).max(0.);
            }
            battle.bleed_events.push(BleedEvent {
                tick: battle.tick,
                unit: unit.id,
                hit_sequence: wound.hit_sequence,
                region: wound.region,
                amount,
            });
        }
    }
    if battle.bleed_events.len() > 256 {
        battle.bleed_events.drain(..battle.bleed_events.len() - 256);
    }
}

pub(crate) fn sever(seed: u64, sequence: u64, unit: &Unit, amount: f32, cutting: bool) -> u8 {
    if !cutting
        || unit.health <= 0.0
        || unit.health_fraction() > 0.65
        || amount < unit.max_health() * 0.04
        || !amount.is_finite()
    {
        return 0;
    }
    // A dedicated deterministic roll consumes none of the tactical RNG stream.
    let roll =
        hash(seed ^ sequence.wrapping_mul(0x9e3779b97f4a7c15) ^ (unit.id as u64).wrapping_mul(131));
    if roll % 1000 >= 30 {
        return 0;
    }
    let eligible: Vec<u8> = [1, 2, 4, 8]
        .into_iter()
        .filter(|limb| unit.lost_limbs & limb == 0)
        .collect();
    if eligible.is_empty() {
        0
    } else {
        eligible[((roll >> 16) as usize) % eligible.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Battle, terrain::Arena};
    #[test]
    fn only_rare_severe_cutting_hits_can_sever_and_the_roll_is_reproducible() {
        let mut unit = Battle::with_founders(42, false, Arena::OpenField)
            .units
            .remove(0);
        unit.health = unit.max_health() * 0.40;
        let mut losses = 0;
        for sequence in 0..10_000 {
            let limb = sever(42, sequence, &unit, unit.max_health() * 0.10, true);
            assert_eq!(
                limb,
                sever(42, sequence, &unit, unit.max_health() * 0.10, true)
            );
            assert_eq!(
                sever(42, sequence, &unit, unit.max_health() * 0.03, true),
                0
            );
            assert_eq!(
                sever(42, sequence, &unit, unit.max_health() * 0.10, false),
                0
            );
            losses += usize::from(limb != 0);
        }
        assert!(
            (200..400).contains(&losses),
            "3 percent of eligible severe hits: {losses}"
        );
        unit.lost_limbs = 15;
        assert!((0..1000).all(|s| sever(42, s, &unit, 100., true) == 0));
        unit.lost_limbs = 0;
        unit.health = 0.;
        assert!((0..1000).all(|s| sever(42, s, &unit, 100., true) == 0));
    }
}
