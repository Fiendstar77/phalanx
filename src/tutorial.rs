//! Local NPC curriculum. Completion records real campaign victories, never online ranking.
use crate::{progression::Campaign, rivals::RivalStyle};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Progress {
    pub cleared: [bool; 5],
    pub attempts: [u32; 5],
}
#[derive(Clone, Copy)]
pub struct Lesson {
    pub rival: RivalStyle,
    pub title: &'static str,
    pub advice: &'static str,
}
pub const LESSONS: [Lesson; 5] = [
    Lesson {
        rival: RivalStyle::Bloodhound,
        title: "1 / Survive the rush",
        advice: "Scouts and Berserkers punish an unsupported back line. Try a Protector near your ranged fighter; compare positioning with a Vanguard's pursuit.",
    },
    Lesson {
        rival: RivalStyle::Sentinel,
        title: "2 / Crack the shell",
        advice: "Guardians and a Paladin reward patience. Flank their protection, vary your class mix and spend salvage on a deliberate loadout.",
    },
    Lesson {
        rival: RivalStyle::Obsidian,
        title: "3 / Cross the firing line",
        advice: "Hexes and ranged pressure punish an open charge. Try cover, a Skirmisher's cautious approach or a Scout who can reach the shooters.",
    },
    Lesson {
        rival: RivalStyle::Vanguard,
        title: "4 / Break the formation",
        advice: "Banner and Paladin auras depend on nearby allies and clear lines. Split the formation; combine disciplines, equipment and minds to disrupt support.",
    },
    Lesson {
        rival: RivalStyle::Null,
        title: "5 / Pass the counter-test",
        advice: "The Collective adapts through authored rules to your recorded composition and healing. Change your build, not only its numbers. Build a phalanx that survives whatever comes next.",
    },
];
impl Progress {
    pub fn next(&self) -> Option<usize> {
        self.cleared.iter().position(|v| !*v)
    }
    pub fn completed(&self) -> usize {
        self.cleared.iter().filter(|v| **v).count()
    }
    pub fn valid(&self) -> bool {
        self.attempts
            .iter()
            .zip(self.cleared)
            .all(|(n, won)| *n <= 1_000_000 && (!won || *n > 0))
    }
    pub fn record(&mut self, style: Option<RivalStyle>, won: bool) {
        if let Some(i) = LESSONS.iter().position(|l| Some(l.rival) == style) {
            self.attempts[i] = self.attempts[i].saturating_add(1).min(1_000_000);
            self.cleared[i] |= won;
        }
    }
}
pub fn prepare_next(c: &mut Campaign) -> Result<Lesson, String> {
    let i=c.tutorial.next().ok_or("Five tutorial teams cleared. Rematch rivals to explore other builds; the wider campaign is still in development.")?;
    let lesson = LESSONS[i];
    if c.teams[1].rival_style != Some(lesson.rival) {
        let rival = c
            .rivals
            .iter()
            .position(|r| r.rival_style == Some(lesson.rival))
            .ok_or("Tutorial opponent is missing from this save")?;
        c.select_rival(rival)?;
    }
    Ok(lesson)
}
pub fn summary(c: &Campaign) -> String {
    let mut out = format!(
        "FIVE TEAMS. FIVE LESSONS. | {} / 5 teams cleared\nEarn battle XP, gold and salvage. Buy extra fighter slots with gold.\n\n",
        c.tutorial.completed()
    );
    for (i, l) in LESSONS.iter().enumerate() {
        out.push_str(&format!(
            "{} {} / {}\n{}\n\n",
            if c.tutorial.cleared[i] {
                "CLEARED"
            } else {
                "TO LEARN"
            },
            l.title,
            l.rival.name(),
            l.advice
        ));
    }
    out
}
