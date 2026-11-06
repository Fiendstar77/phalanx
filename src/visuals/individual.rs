//! Persistent identity drives performance; this module cannot change combat rules.
#[cfg(test)]
use super::*;
pub(super) const EXPRESSIONS: [&str; 24] = [
    "quiet focus",
    "suspicion",
    "anticipation",
    "concern",
    "reassurance",
    "resentment",
    "concentration",
    "effort",
    "alarm",
    "pain",
    "defiance",
    "rapture",
    "relief",
    "exhaustion",
    "threat",
    "disgust",
    "grief",
    "astonishment",
    "resolve",
    "hesitation",
    "affection",
    "deadpan",
    "smirk",
    "delight",
];
#[derive(Clone, Copy, Debug)]
pub(super) struct Signature {
    pub gait: usize,
    pub sway: f32,
    pub bounce: f32,
    pub lean: f32,
    pub turn: f32,
    pub cadence: f32,
    pub phase: f32,
}
pub(super) fn key(u: &phalanx::Unit) -> u64 {
    u.fighter_id
        .as_deref()
        .unwrap_or("lab")
        .bytes()
        .fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
        })
}
pub(super) fn signature(u: &phalanx::Unit) -> Signature {
    let k = phalanx::procedural::hash(key(u));
    let family = if u.mind.courage < 0.4 {
        3
    } else if u.mind.aggression > 0.72 {
        2
    } else if u.mind.caution > 0.6 {
        1
    } else {
        0
    };
    let style = (family * 2 + (k as usize % 2)) % 8;
    let (gait, sway, bounce, lean, turn, cadence) = match style {
        0 => (28, 0.25, 0.15, 0.015, 0.25, 0.98), // measured guard
        1 => (1, 0.65, 0.35, -0.035, 0.8, 0.88),  // loose swagger
        2 => (25, 0.18, 0.1, 0.11, 0.15, 0.85),   // stalking
        3 => (28, 0.35, 0.12, 0.045, 0.4, 1.06),  // watchful sidling
        4 => (1, 0.85, 0.6, -0.04, 1.1, 1.12),    // rolling bravado
        5 => (27, 0.3, 0.9, 0.08, 0.3, 1.16),     // springing blade carrier
        6 => (25, 0.55, 0.32, 0.09, 0.8, 1.05),   // furtive
        _ => (1, 0.35, 0.75, 0.025, 0.3, 0.92),   // hesitant high step
    };
    Signature {
        gait,
        sway,
        bounce,
        lean,
        turn,
        cadence,
        phase: (k % 997) as f32 / 997. * std::f32::consts::TAU,
    }
}
pub(super) fn expression(
    u: &phalanx::Unit,
    tick: u32,
    clock: f32,
    hurt: f32,
    attack: f32,
    laugh: f32,
    finished: bool,
) -> usize {
    if hurt > 0.35 {
        return if u.mind.courage > 0.65 {
            10
        } else if u.mind.courage < 0.35 {
            8
        } else {
            9
        };
    }
    if u.health_fraction() < 0.30 {
        return if u.mind.aggression > 0.75 {
            11
        } else if u.mind.courage > 0.65 {
            18
        } else {
            8
        };
    }
    let tired = u
        .theus
        .as_ref()
        .is_some_and(|a| a.stamina < a.sheet.endurance() * 0.2);
    if tired {
        return 13;
    }
    if let Some(r) = u
        .social
        .receipts
        .iter()
        .rev()
        .find(|r| r.battle_id.is_none() && tick.saturating_sub(r.tick) < 45)
    {
        match r.kind {
            phalanx::social::Kind::HealedMe => return if u.mind.teamwork > 0.6 { 20 } else { 12 },
            phalanx::social::Kind::StoodBetween => return 4,
            _ => {}
        }
    }
    if finished {
        return if u.mind.patience > 0.6 { 12 } else { 23 };
    }
    if u.support_target.is_some_and(|id| u.social.bond(id) > 0.3) {
        return if u.action == 8 { 3 } else { 4 };
    }
    if u.enemy.is_some_and(|id| u.social.caution(id) > 0.1) {
        return if u.mind.caution > u.mind.aggression {
            1
        } else {
            5
        };
    }
    if laugh > 0.2 {
        return if u.mind.aggression > 0.7 {
            11
        } else if u.mind.caution > 0.6 {
            17
        } else {
            22
        };
    }
    if attack > 0.12 || u.action == 1 {
        return if u.mind.discipline > 0.65 {
            6
        } else if u.mind.courage > 0.7 {
            14
        } else {
            7
        };
    }
    if phalanx::tactics::recovering(&u.tactics) {
        return if u.mind.courage < 0.5 { 19 } else { 18 };
    }
    let epoch = (clock / 3.7) as u64;
    let variant = phalanx::procedural::hash(key(u) ^ epoch.rotate_left(17)) as usize % 3;
    let bank = if u.mind.courage < 0.4 {
        [1, 19, 3]
    } else if u.mind.aggression > 0.72 {
        [2, 22, 14]
    } else if u.mind.patience > 0.65 {
        [0, 21, 6]
    } else {
        [0, 2, 22]
    };
    bank[variant]
}
pub(super) fn pose(expression: usize, blink: f32) -> [f32; 6] {
    let mut p = match expression {
        0 => [0.08, 0., 0.02, 0.04, 0., 0.],
        1 => [0.18, 0.02, 0.22, 0., 0., 0.],
        2 => [0.12, 0., 0.08, 0.25, 0.03, 0.],
        3 => [0., 0.12, 0.37, 0.02, 0., 0.],
        4 => [0.02, 0., 0.10, 0.35, 0., 0.],
        5 => [0.45, 0.10, 0.05, 0.02, 0., 0.],
        6 => [0.34, 0.08, 0., 0., 0., 0.],
        7 => [0.36, 0.20, 0.06, 0., 0.26, 0.],
        8 => [0., 0.15, 0.88, 0., 0.15, 0.],
        9 => [0.05, 0.93, 0.12, 0., 0.20, 0.],
        10 => [0.68, 0.30, 0., 0.10, 0.10, 0.],
        11 => [0.18, 0., 0.03, 0.85, 0.28, 0.],
        12 => [0., 0.08, 0., 0.35, 0.03, 0.],
        13 => [0., 0.40, 0.10, 0., 0.18, 0.],
        14 => [0.74, 0., 0.04, 0.08, 0.24, 0.],
        15 => [0.30, 0.26, 0.10, 0., 0., 0.],
        16 => [0., 0.48, 0.28, 0., 0.06, 0.],
        17 => [0., 0., 0.65, 0.12, 0.34, 0.],
        18 => [0.40, 0.12, 0., 0.05, 0., 0.],
        19 => [0.04, 0.08, 0.30, 0.01, 0., 0.],
        20 => [0., 0., 0.04, 0.50, 0., 0.],
        21 => [0., 0., 0., 0., 0., 0.],
        22 => [0.12, 0., 0., 0.52, 0.02, 0.],
        _ => [0., 0., 0.03, 0.78, 0.22, 0.],
    };
    p[5] = blink;
    let sum = p[..5].iter().sum::<f32>();
    if sum > 1.35 {
        for v in &mut p[..5] {
            *v *= 1.35 / sum;
        }
    }
    p
}
#[cfg(test)]
mod tests {
    #[cfg(test)]
    use super::*;
    #[test]
    fn expression_library_is_distinct_bounded_and_preserves_a_blink() {
        let mut poses = std::collections::BTreeSet::new();
        for i in 0..EXPRESSIONS.len() {
            let p = pose(i, 1.);
            assert_eq!(p[5], 1.);
            assert!(p[..5].iter().sum::<f32>() <= 1.35001);
            poses.insert(p.map(|v| (v * 1000.).round() as i32));
        }
        assert_eq!(poses.len(), 24);
    }
    #[test]
    fn saved_identity_keeps_its_signature_across_roster_positions() {
        let mut b = phalanx::Battle::with_founders(42, false, Arena::OpenField);
        let mut u = b.units.remove(0);
        u.fighter_id = Some("test-veteran".into());
        let first = signature(&u);
        u.id = 7;
        assert_eq!(first.gait, signature(&u).gait);
        assert_eq!(first.phase, signature(&u).phase);
        let styles: std::collections::BTreeSet<_> = (0..100)
            .map(|i| {
                u.fighter_id = Some(format!("fighter-{i}"));
                signature(&u).phase.to_bits()
            })
            .collect();
        assert!(styles.len() > 80);
    }
}
