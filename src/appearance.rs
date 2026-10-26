//! Saved cosmetic choices shared by the creator and the camp editor.
use crate::progression::Appearance;

pub const FEATURES: [&str; 10] = [
    "Body", "Hair", "Face", "Eyes", "Nose", "Skin", "Dye", "Cheeks", "Mouth", "Brows",
];
pub const COUNTS: [u8; 10] = [2, 11, 17, 16, 17, 8, 7, 5, 5, 5];
pub const FACES: [&str; 17] = [
    "Peach",
    "Heart",
    "Moon",
    "Pixie",
    "Pear",
    "Round",
    "Button",
    "Oval",
    "Doll",
    "Imp",
    "Square",
    "Long",
    "Diamond",
    "Soft heart",
    "Wide",
    "Petal",
    "Plum",
];
pub const NOSES: [&str; 17] = [
    "Button",
    "Petite",
    "Upturned",
    "Round",
    "Broad",
    "Long",
    "Pointed",
    "Snub",
    "Soft",
    "Wide button",
    "Narrow",
    "Pixie",
    "Roman",
    "Pear",
    "Flat",
    "Hooked",
    "Cherry",
];

impl Appearance {
    pub fn choice(self, feature: usize) -> u8 {
        match feature {
            0 => u8::from(self.female.unwrap_or(false)),
            1 => self.hair,
            2 => self.face,
            3 => self.eyes,
            4 => self.nose,
            5 => self.skin,
            6 => self.hair_color,
            7 => self.cheeks,
            8 => self.mouth,
            9 => self.brows,
            _ => 0,
        }
    }
    pub fn cycle(&mut self, feature: usize, direction: i32) {
        let Some(&count) = COUNTS.get(feature) else {
            return;
        };
        let value = (self.choice(feature) as i32 + direction).rem_euclid(count as i32) as u8;
        match feature {
            0 => self.female = Some(value != 0),
            1 => self.hair = value,
            2 => self.face = value,
            3 => self.eyes = value,
            4 => self.nose = value,
            5 => self.skin = value,
            6 => self.hair_color = value,
            7 => self.cheeks = value,
            8 => self.mouth = value,
            9 => self.brows = value,
            _ => {}
        }
    }
    pub fn choice_name(self, feature: usize) -> &'static str {
        let choices: &[&str] = match feature {
            0 => &["Man", "Woman"],
            1 => &[
                "Bald", "Crop", "Bob", "Sweep", "Puffs", "Tuft", "Mane", "Spikes", "Curls",
                "Crest", "Buns",
            ],
            2 => &FACES,
            3 => &[
                "Hazel", "Sky", "Moss", "Amber", "Violet", "Teal", "Chestnut", "Rose", "Gold",
                "Ice", "Forest", "Lilac", "Copper", "Navy", "Silver", "Ruby",
            ],
            4 => &NOSES,
            5 => &[
                "Porcelain",
                "Peach",
                "Honey",
                "Caramel",
                "Chestnut",
                "Umber",
                "Rose",
                "Olive",
            ],
            6 => &[
                "Natural", "Ink", "Chestnut", "Honey", "Copper", "Silver", "Plum",
            ],
            7 => &["Peachy", "Soft", "Plump", "Dimpled", "Sculpted"],
            8 => &["Sweet", "Tiny", "Wide smile", "Pout", "Smirk"],
            9 => &["Gentle", "Arched", "Straight", "Mischievous", "Worried"],
            _ => return "",
        };
        choices[self.choice(feature) as usize % choices.len()]
    }
}
