//! Playable, saved opening. Chapter gates consume witnessed victories, never invented deeds.
use crate::{
    Battle, CommandTitle, FighterClass,
    progression::{Campaign, Fighter},
    terrain::Arena,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Victory {
    pub chapter: u8,
    pub battle_id: u64,
    pub seed: u64,
    pub tick: u32,
    pub participants: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Join {
    pub fighter_id: String,
    pub after_battle_id: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Progress {
    /// 0 solo, 1 queen met, 2 general met, 3 full campaign unlocked.
    pub chapter: u8,
    /// Authored people waiting offstage, not active roster slots.
    pub waiting: [Vec<Fighter>; 2],
    pub victories: Vec<Victory>,
    pub joins: Vec<Join>,
}
impl Progress {
    pub fn active(&self) -> bool {
        self.chapter < 3
    }
    pub fn title(&self) -> &'static str {
        [
            "BEFORE THE CROWN",
            "A PROMISE SHARED",
            "THE THIRD STANDARD",
            "A PHALANX IS BORN",
        ][self.chapter.min(3) as usize]
    }
    pub fn briefing(&self) -> String {
        let story = match self.chapter {
            0 => {
                "Age 16. An aspiring king has no crown, only a first road trial. Win this solo encounter to meet your future queen."
            }
            1 => {
                "One winter later, age 17. Your future queen joins as an equal. Win together to meet the general; your first scars and practice carry forward."
            }
            2 => {
                "Another year later, age 18. Your future general joins. Win as three to found the Phalanx and earn the royal offices."
            }
            _ => {
                "The opening is complete. Your veterans now enter the full 3v3 campaign; five NPC tutorial teams await. The fourth slot costs 250 earned gold."
            }
        };
        format!(
            "{} | {}\n{}\nSpace: fight / pause. Inspect your fighter or change temperament in the workshop. Outcomes use normal Theus rules.",
            self.title(),
            if self.active() {
                format!("Chapter {} / 3", self.chapter + 1)
            } else {
                "Campaign unlocked".into()
            },
            story
        )
    }
    pub fn cleared(&self) -> bool {
        self.victories.iter().any(|v| v.chapter == self.chapter)
    }
    pub(crate) fn record(&mut self, id: u64, battle: &Battle) {
        if self.active() && !self.cleared() && battle.finished && battle.winner() == Some(0) {
            self.victories.push(Victory {
                chapter: self.chapter,
                battle_id: id,
                seed: battle.seed,
                tick: battle.tick,
                participants: battle
                    .units
                    .iter()
                    .filter(|u| u.team == 0)
                    .filter_map(|u| u.fighter_id.clone())
                    .collect(),
            });
        }
    }
    pub(crate) fn validate(&self, c: &Campaign) -> Result<(), String> {
        let chapter = self.chapter as usize;
        if chapter > 3
            || !c.founders_prepared
            || c.teams[0].fighters.len() < 1 + chapter.min(2)
            || (self.active() && c.teams[0].fighters.len() != 1 + chapter)
            || self.waiting.iter().any(|w| w.len() != 2 - chapter.min(2))
            || self.victories.len() < chapter
            || self.victories.len() > (chapter + 1).min(3)
            || self.joins.len() != chapter.min(2)
        {
            return Err("invalid opening chapter or roster".into());
        }
        for (i, receipt) in self.victories.iter().enumerate() {
            if receipt.chapter as usize != i
                || receipt.battle_id >= c.next_battle_id
                || receipt.tick == 0
                || (i > 0 && self.victories[i - 1].battle_id >= receipt.battle_id)
                || receipt.participants
                    != c.teams[0]
                        .fighters
                        .iter()
                        .take(i + 1)
                        .map(|f| f.fighter_id.clone())
                        .collect::<Vec<_>>()
            {
                return Err("invalid opening victory receipt".into());
            }
        }
        for (i, join) in self.joins.iter().enumerate() {
            if join.fighter_id != c.teams[0].fighters[i + 1].fighter_id
                || join.after_battle_id != self.victories[i].battle_id
            {
                return Err("invalid companion joining receipt".into());
            }
        }
        if self.active()
            && c.teams
                .iter()
                .flat_map(|t| &t.fighters)
                .any(|f| f.title != CommandTitle::Member)
        {
            return Err("royal offices are not yet earned".into());
        }
        for waiting in &self.waiting {
            if !waiting.is_empty() {
                let mut roster = c.teams[0].clone();
                roster.fighters = waiting.clone();
                roster.standard_bearer = 0;
                roster.validate_roster(c.next_battle_id, 1)?;
                if waiting.iter().any(|f| {
                    f.battles != 0 || !f.history.is_empty() || f.title != CommandTitle::Member
                }) {
                    return Err("invalid offstage companion".into());
                }
            }
        }
        Ok(())
    }
}
impl Campaign {
    pub fn new_prologue() -> Self {
        let mut c = Self::new();
        c.founders_prepared = true;
        c.teams[0].name = "The Uncrowned".into();
        c.teams[1].name = "Road challengers".into();
        for (i, f) in c.teams[0].fighters.iter_mut().enumerate() {
            let card = crate::genesis::reveal("BEFORE THE CROWN", i, 0, f.class).unwrap();
            f.name = card.name;
            f.mind = card.mind;
            f.theus = card.theus;
            f.appearance = card.appearance;
            f.appearance.age_years = [16, 17, 24][i];
            f.title = CommandTitle::Member;
        }
        c.teams[0].fighters[0].appearance.female = Some(false);
        for (i, f) in c.teams[1].fighters.iter_mut().enumerate() {
            f.title = CommandTitle::Member;
            f.name = ["Road duelist", "Road scout", "Road sentinel"][i].into();
            f.class = [
                FighterClass::Warrior,
                FighterClass::Scout,
                FighterClass::RockThrower,
            ][i];
        }
        let waiting = std::array::from_fn(|team| {
            c.teams[team].standard_bearer = 0;
            c.teams[team].fighters.split_off(1)
        });
        c.prologue = Some(Progress {
            chapter: 0,
            waiting,
            victories: Vec::new(),
            joins: Vec::new(),
        });
        c
    }
    pub fn prologue_active(&self) -> bool {
        self.prologue.as_ref().is_some_and(Progress::active)
    }
    pub fn prologue_arena(&self) -> Option<Arena> {
        self.prologue
            .as_ref()
            .filter(|p| p.active())
            .map(|p| [Arena::OpenField, Arena::Ridge, Arena::Choke][p.chapter as usize])
    }
    /// Called between encounters. A loss retries the same chapter without discarding earned history.
    pub fn advance_prologue(&mut self) -> Result<String, String> {
        self.validate()?;
        let mut next = self.clone();
        let p = next
            .prologue
            .as_mut()
            .filter(|p| p.active())
            .ok_or("The opening is complete")?;
        if !p.cleared() {
            return Ok(
                "Try the chapter again. Practice and memories from the previous attempt remain."
                    .into(),
            );
        }
        if p.chapter < 2 {
            let after = p.victories[p.chapter as usize].battle_id;
            for team in 0..2 {
                let fighter = p.waiting[team].remove(0);
                if team == 0 {
                    p.joins.push(Join {
                        fighter_id: fighter.fighter_id.clone(),
                        after_battle_id: after,
                    });
                }
                next.teams[team].fighters.push(fighter);
            }
            // Explicit story time skips, not biological aging per combat round.
            for f in &mut next.teams[0].fighters {
                if f.appearance.age_years < 18 {
                    f.appearance.age_years += 1;
                }
            }
        } else {
            for team in &mut next.teams {
                for (f, title) in team.fighters.iter_mut().zip([
                    CommandTitle::King,
                    CommandTitle::Queen,
                    CommandTitle::General,
                ]) {
                    f.title = title;
                }
                team.standard_bearer = 2;
            }
        }
        p.chapter += 1;
        let text = p.briefing();
        next.validate()?;
        *self = next;
        Ok(text)
    }
}
