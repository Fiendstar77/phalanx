//! Rare lasting injury, independent of ordinary damage and cosmetic blood volume.
use crate::{Unit, procedural::hash};

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
