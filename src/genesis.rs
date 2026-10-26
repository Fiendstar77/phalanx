//! Versioned word-seeded founder reveals. Authored traits and cosmetics, never trained weights.
use crate::{FighterClass, Mind, progression::Appearance, styles::BattleStyle};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Origin {
    pub version: u32,
    pub word: String,
    pub rolls: [u32; 3],
}
impl Origin {
    pub fn valid(&self) -> bool {
        self.version == 1
            && word_seed(&self.word).is_ok()
            && self.rolls.iter().all(|r| *r < 1_000_000)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Founder {
    #[serde(default)]
    pub theus: Option<crate::theus::Sheet>,
    pub name: String,
    pub class: FighterClass,
    pub mind: Mind,
    pub appearance: Appearance,
}
pub fn word_seed(word: &str) -> Result<u64, String> {
    let word = word.trim().to_lowercase();
    if word.is_empty() || word.chars().count() > 40 || word.chars().any(char::is_control) {
        return Err("Use a seed word or phrase of 1 to 40 characters".into());
    }
    Ok(word.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    }))
}
pub fn reveal(word: &str, slot: usize, roll: u32, class: FighterClass) -> Result<Founder, String> {
    if slot >= 20 || roll >= 1_000_000 {
        return Err("Invalid founder reveal".into());
    }
    let seed = word_seed(word)?;
    let mut state = crate::procedural::hash(
        seed ^ (slot as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15) ^ (roll as u64).wrapping_mul(131),
    );
    let mut pick = |n: u64| {
        state = crate::procedural::hash(state);
        state % n
    };
    let first = [
        "Pen", "Mor", "Ves", "Cal", "Nyx", "Ori", "Thal", "Ash", "Bel", "Ser", "Dra", "Cor",
    ];
    let last = [
        "elia", "rik", "ara", "ven", "ora", "ith", "enna", "os", "ira", "ax", "ian", "elle",
    ];
    let name = format!(
        "{}{}",
        first[pick(first.len() as u64) as usize],
        last[pick(last.len() as u64) as usize]
    );
    let style = BattleStyle::ALL[pick(3) as usize];
    let mut mind = style.mind();
    mind.courage = (mind.courage + (pick(101) as f32 / 100. - 0.5) * 0.12).clamp(0., 1.);
    mind.patience = (mind.patience + (pick(101) as f32 / 100. - 0.5) * 0.12).clamp(0., 1.);
    // Keep the same seeded draw for other traits; the starting Queen defaults to female.
    let female = pick(2) == 1;
    let appearance = Appearance {
        female: Some(slot == 1 || female),
        hair: 1 + pick(10) as u8,
        face: pick(17) as u8,
        eyes: pick(16) as u8,
        nose: pick(17) as u8,
        skin: pick(8) as u8,
        hair_color: pick(7) as u8,
        cheeks: pick(5) as u8,
        mouth: pick(5) as u8,
        brows: pick(5) as u8,
    };
    Ok(Founder {
        theus: Some(crate::theus::Sheet::from_temperament([
            mind.aggression,
            mind.self_preservation,
            mind.patience,
            mind.courage,
        ])),
        name,
        class,
        mind,
        appearance,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_starting_queen_is_female_across_seed_words_rerolls_and_classes() {
        for word in ["BONES", "Ash", "Queen", "A different company"] {
            for roll in 0..16 {
                for class in FighterClass::ALL {
                    let queen = reveal(word, 1, roll, class).unwrap();
                    assert_eq!(queen.appearance.female, Some(true));
                }
            }
        }
    }

    #[test]
    fn words_reproduce_unique_founders_and_rerolls_do_not_change_neighbours() {
        let encode = |f: Founder| serde_json::to_string(&f).unwrap();
        let a = reveal("Bones", 0, 0, FighterClass::Warrior).unwrap();
        assert_eq!(
            encode(a.clone()),
            encode(reveal(" BONES ", 0, 0, FighterClass::Warrior).unwrap())
        );
        assert_ne!(
            encode(a.clone()),
            encode(reveal("Bones", 1, 0, FighterClass::Warrior).unwrap())
        );
        assert_ne!(
            encode(a),
            encode(reveal("Bones", 0, 1, FighterClass::Warrior).unwrap())
        );
        for i in 0..20 {
            let f = reveal("Bones", i, 0, FighterClass::Warrior).unwrap();
            assert!((1..=10).contains(&f.appearance.hair));
        }
        assert!(word_seed("").is_err());
        assert!(word_seed("bad\nword").is_err());
    }
}
