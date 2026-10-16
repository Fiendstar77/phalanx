//! Symmetric pressure fields for Founders. The lesson comes from both squads.
//! Ranged fraction is observed and deliberately unused: the field never seals a class out.
use crate::{
    FighterClass,
    progression::{Fighter, History, Phalanx},
    psyche::Reaction,
    terrain::Arena,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lesson {
    Split,
    Ridge,
    Exposure,
    Approach,
}
impl Lesson {
    fn rank(self) -> u8 {
        match self {
            Self::Split => 0,
            Self::Ridge => 1,
            Self::Exposure => 2,
            Self::Approach => 3,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Split => "TWO LANES",
            Self::Ridge => "CONTESTED HILL",
            Self::Exposure => "OPEN COVER",
            Self::Approach => "LONG CROSSING",
        }
    }
    pub fn reason(self) -> &'static str {
        match self {
            Self::Split => "Two equal lanes. Cohesion and isolation fear have to cover both.",
            Self::Ridge => "One centered hill. Impatience gives the high ground away.",
            Self::Exposure => "Mirrored cover on open ground. Recklessness is exposed.",
            Self::Approach => "Both sides cross the same rough ground. Slow fighters must commit.",
        }
    }
}
/// Every feature is mirrored. `cover_extra` only adds cover. `wide` only eases a 4-fighter spawn.
/// `reach` grows the footprint with the playable league: 0 is the original 22×16 field,
/// 1 is the 3-fighter skirmish, 2 is the 4-fighter company. Older saves omit it and stay at 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PressureField {
    pub lesson: Lesson,
    pub cover_extra: u8,
    pub wide: bool,
    #[serde(default)]
    pub reach: u8,
}
const SPLIT_SMALL: [([f32; 2], [f32; 2]); 1] = [([0.0, 0.0], [2.2, 4.0])];
const SPLIT_SKIRMISH: [([f32; 2], [f32; 2]); 1] = [([0.0, 0.0], [2.2, 9.0])];
/// Center wall plus a mirrored pair of posts. Both north and south lanes stay open.
const SPLIT_COMPANY: [([f32; 2], [f32; 2]); 3] = [
    ([0.0, 0.0], [2.2, 14.0]),
    ([-12.0, 0.0], [2.2, 4.0]),
    ([12.0, 0.0], [2.2, 4.0]),
];
impl PressureField {
    pub fn name(self) -> &'static str {
        self.lesson.name()
    }
    pub fn reason(self) -> &'static str {
        self.lesson.reason()
    }
    pub fn half(self) -> [f32; 2] {
        match self.reach {
            0 => [11.0, 8.0],
            1 => [18.0, 13.0],
            _ => [24.0, 17.0],
        }
    }
    pub fn spawn_x(self) -> f32 {
        match self.reach {
            0 => match self.lesson {
                Lesson::Approach => 9.4,
                Lesson::Split if !self.wide => 6.4,
                _ => 8.0,
            },
            1 => match self.lesson {
                Lesson::Approach => 16.4,
                Lesson::Split if !self.wide => 11.0,
                _ => 13.0,
            },
            _ => match self.lesson {
                Lesson::Approach => 22.4,
                Lesson::Split if !self.wide => 14.0,
                _ => 17.0,
            },
        }
    }
    pub fn cover_level(self) -> u8 {
        match self.lesson {
            Lesson::Exposure => 2,
            _ => 1 + self.cover_extra.min(1),
        }
    }
    pub fn walls(self) -> &'static [([f32; 2], [f32; 2])] {
        if self.lesson != Lesson::Split {
            return &[];
        }
        match self.reach {
            0 => &SPLIT_SMALL,
            1 => &SPLIT_SKIRMISH,
            _ => &SPLIT_COMPANY,
        }
    }
    pub fn cover_anchors(self) -> &'static [[f32; 2]] {
        match self.reach {
            0 => &[
                [-5.0, 5.0],
                [5.0, 5.0],
                [-5.0, -5.0],
                [5.0, -5.0],
                [-4.2, 0.0],
                [4.2, 0.0],
            ],
            1 => &[
                [-9.0, 8.0],
                [9.0, 8.0],
                [-9.0, -8.0],
                [9.0, -8.0],
                [-8.0, 0.0],
                [8.0, 0.0],
            ],
            _ => &[
                [-13.0, 11.0],
                [13.0, 11.0],
                [-13.0, -11.0],
                [13.0, -11.0],
                [-11.0, 0.0],
                [11.0, 0.0],
            ],
        }
    }
    pub fn blocked(self, p: [f32; 2]) -> bool {
        let half = self.half();
        p[0].abs() > half[0] - 0.3
            || p[1].abs() > half[1] - 0.3
            || self.walls().iter().any(|(center, extent)| {
                (p[0] - center[0]).abs() < extent[0] * 0.5 + 0.3
                    && (p[1] - center[1]).abs() < extent[1] * 0.5 + 0.3
            })
    }
    pub fn elevation(self, p: [f32; 2]) -> f32 {
        if self.lesson != Lesson::Ridge {
            return 0.0;
        }
        let (rx, rz, height) = match self.reach {
            0 => (4.0, 6.0, 1.7),
            1 => (7.0, 9.0, 2.4),
            _ => (10.0, 12.0, 3.0),
        };
        let x = (1.0 - p[0].abs() / rx).max(0.0);
        let z = (1.0 - p[1].abs() / rz).max(0.0);
        x * z * height
    }
    pub fn cover(self, p: [f32; 2]) -> f32 {
        let (flank, pocket) = match self.reach {
            0 => (
                p[0].abs() >= 2.5 && p[0].abs() <= 7.0 && p[1].abs() >= 3.5 && p[1].abs() <= 6.5,
                p[0].abs() >= 3.0 && p[0].abs() <= 5.5 && p[1].abs() <= 1.4,
            ),
            1 => (
                p[0].abs() >= 5.0 && p[0].abs() <= 13.0 && p[1].abs() >= 6.0 && p[1].abs() <= 11.0,
                p[0].abs() >= 6.0 && p[0].abs() <= 10.0 && p[1].abs() <= 2.5,
            ),
            _ => (
                p[0].abs() >= 8.0 && p[0].abs() <= 18.0 && p[1].abs() >= 8.0 && p[1].abs() <= 14.5,
                p[0].abs() >= 8.0 && p[0].abs() <= 14.0 && p[1].abs() <= 3.0,
            ),
        };
        f32::from(flank || (self.cover_level() >= 2 && pocket))
    }
    pub fn roughness(self, p: [f32; 2]) -> f32 {
        let band = match self.reach {
            0 => 3.0,
            1 => 6.0,
            _ => 8.0,
        };
        f32::from(self.lesson == Lesson::Approach && p[0].abs() < band)
    }
}
#[derive(Clone, Copy, Debug)]
struct SquadRead {
    mobility: f32,
    cohesion: f32,
    caution: f32,
    patience: f32,
    aggression: f32,
    isolation: f32,
}
fn class_mobility(class: FighterClass) -> f32 {
    match class {
        FighterClass::Scout => 0.9,
        FighterClass::Guardian => 0.25,
        _ => 0.55,
    }
}
fn fighter_mobility(fighter: &Fighter) -> f32 {
    let missing_legs = ((fighter.lost_limbs & !fighter.prosthetics) & 0b1100).count_ones() as f32;
    (class_mobility(fighter.class) * (1.0 - 0.2 * missing_legs)).clamp(0.0, 1.0)
}
fn isolation_weight(fighter: &Fighter) -> f32 {
    let scar = fighter.psyche.isolation;
    match scar.reaction {
        Reaction::Fear | Reaction::Hesitation | Reaction::Attachment => scar.intensity,
        _ => 0.0,
    }
}
fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}
fn read_squad(squad: &Phalanx) -> SquadRead {
    SquadRead {
        mobility: mean(
            &squad
                .fighters
                .iter()
                .map(fighter_mobility)
                .collect::<Vec<_>>(),
        ),
        cohesion: mean(
            &squad
                .fighters
                .iter()
                .map(|f| f.mind.teamwork)
                .collect::<Vec<_>>(),
        ),
        caution: mean(
            &squad
                .fighters
                .iter()
                .map(|f| f.mind.caution)
                .collect::<Vec<_>>(),
        ),
        patience: mean(
            &squad
                .fighters
                .iter()
                .map(|f| f.mind.patience)
                .collect::<Vec<_>>(),
        ),
        aggression: mean(
            &squad
                .fighters
                .iter()
                .map(|f| f.mind.aggression)
                .collect::<Vec<_>>(),
        ),
        isolation: mean(
            &squad
                .fighters
                .iter()
                .map(isolation_weight)
                .collect::<Vec<_>>(),
        ),
    }
}
/// 1 means the squad is already prepared for the lesson. Lower means the field can teach it.
fn preparedness(lesson: Lesson, squad: SquadRead) -> f32 {
    match lesson {
        Lesson::Split => squad.cohesion * 0.6 + (1.0 - squad.isolation) * 0.4,
        Lesson::Ridge => {
            squad.patience * 0.55 + squad.caution * 0.25 + (1.0 - squad.aggression) * 0.20
        }
        Lesson::Exposure => squad.caution * 0.7 + (1.0 - squad.aggression) * 0.3,
        Lesson::Approach => squad.mobility * 0.75 + squad.patience * 0.25,
    }
}
fn last_lesson(squad: &Phalanx) -> Option<Lesson> {
    squad
        .fighters
        .iter()
        .filter_map(|fighter| fighter.history.last())
        .max_by_key(|entry| entry.battle_id)
        .and_then(|entry| match entry.arena {
            Arena::Pressure(field) => Some(field.lesson),
            _ => None,
        })
}
/// The lowest shared preparedness wins. A lesson just fought is skipped.
/// Ties within 0.02 use the seed. The map stays symmetric either way.
pub fn generate(teams: &[Phalanx; 2], seed: u64) -> Arena {
    let left = read_squad(&teams[0]);
    let right = read_squad(&teams[1]);
    let mut ranked = [
        Lesson::Split,
        Lesson::Ridge,
        Lesson::Exposure,
        Lesson::Approach,
    ]
    .map(|lesson| {
        (
            lesson,
            0.5 * (preparedness(lesson, left) + preparedness(lesson, right)),
        )
    });
    ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.rank().cmp(&b.0.rank())));
    let banned = [last_lesson(&teams[0]), last_lesson(&teams[1])];
    let banned = |lesson: Lesson| banned.contains(&Some(lesson));
    let best = ranked[0].1;
    let mut band: Vec<Lesson> = ranked
        .iter()
        .filter(|(lesson, score)| *score <= best + 0.02 && !banned(*lesson))
        .map(|(lesson, _)| *lesson)
        .collect();
    if band.is_empty() {
        band = ranked
            .iter()
            .map(|(lesson, _)| *lesson)
            .filter(|lesson| !banned(*lesson))
            .collect();
    }
    if band.is_empty() {
        band.push(ranked[0].0);
    }
    let lesson = band[(seed as usize) % band.len()];
    let count = teams[0].fighters.len();
    Arena::Pressure(PressureField {
        lesson,
        cover_extra: (seed % 2) as u8,
        wide: count >= 4,
        reach: if count >= 4 { 2 } else { 1 },
    })
}
/// Test helper for a history entry that records only which lesson was just fought.
pub fn fought(lesson: Lesson) -> History {
    History {
        battle_id: 0,
        seed: 1,
        arena: Arena::Pressure(PressureField {
            lesson,
            cover_extra: 0,
            wide: false,
            reach: 0,
        }),
        won: false,
        survived: true,
        damage: 0.0,
        experience_gained: 0,
        lost_limbs: 0,
        prosthetics: 0,
    }
}
