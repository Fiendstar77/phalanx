//! Local persistent squads and bounded battle-conditioned progression. No online authority.
use crate::{Battle, CommandTitle, FighterClass, Mind, learned::Controller, terrain::Arena};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};
pub const PALETTE: [[f32; 3]; 8] = [
    [1.0, 0.72, 0.08],
    [0.10, 0.80, 0.72],
    [0.65, 0.35, 0.95],
    [1.0, 0.42, 0.32],
    [0.20, 0.60, 0.98],
    [0.58, 0.85, 0.20],
    [0.98, 0.35, 0.70],
    [0.88, 0.90, 0.85],
];
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
    pub battle_id: u64,
    pub seed: u64,
    pub arena: Arena,
    pub won: bool,
    pub survived: bool,
    pub damage: f32,
    pub experience_gained: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fighter {
    pub fighter_id: String,
    pub name: String,
    pub title: CommandTitle,
    pub class: FighterClass,
    pub mind: Mind,
    pub experience: u32,
    pub battles: u32,
    pub wins: u32,
    pub history: Vec<History>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Phalanx {
    pub name: String,
    pub color: [f32; 3],
    pub fighters: Vec<Fighter>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Campaign {
    pub schema_version: u32,
    pub teams: [Phalanx; 2],
    pub next_battle_id: u64,
    pub last_settled_id: Option<u64>,
}
impl Default for Campaign {
    fn default() -> Self {
        Self::new()
    }
}
impl Campaign {
    pub fn new() -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self {
            schema_version: 1,
            next_battle_id: 0,
            last_settled_id: None,
            teams: std::array::from_fn(|team| Phalanx {
                name: if team == 0 {
                    "Sunward".into()
                } else {
                    "Moonward".into()
                },
                color: PALETTE[team],
                fighters: [
                    CommandTitle::King,
                    CommandTitle::Queen,
                    CommandTitle::General,
                ]
                .into_iter()
                .enumerate()
                .map(|(slot, title)| Fighter {
                    fighter_id: format!("{stamp:x}-{team}-{slot}"),
                    name: title.name().into(),
                    title,
                    class: [
                        FighterClass::Warrior,
                        FighterClass::RockThrower,
                        FighterClass::Paladin,
                    ][slot],
                    mind: if team == 0 {
                        Mind::aggressive()
                    } else {
                        Mind::disciplined()
                    },
                    experience: 0,
                    battles: 0,
                    wins: 0,
                    history: Vec::new(),
                })
                .collect(),
            }),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.next_battle_id > 1_000_000
            || self
                .last_settled_id
                .is_some_and(|id| id >= self.next_battle_id)
        {
            return Err("invalid campaign schema or battle sequence".into());
        }
        let mut ids = BTreeSet::new();
        let valid_name =
            |s: &str| !s.is_empty() && s.chars().count() <= 64 && !s.chars().any(char::is_control);
        for team in &self.teams {
            if !valid_name(&team.name)
                || team.fighters.len() != 3
                || team
                    .color
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            {
                return Err("invalid squad or color".into());
            }
            for (slot, f) in team.fighters.iter().enumerate() {
                let m = f.mind;
                if !valid_name(&f.name)
                    || f.fighter_id.is_empty()
                    || f.fighter_id.len() > 128
                    || !ids.insert(&f.fighter_id)
                    || f.title
                        != [
                            CommandTitle::King,
                            CommandTitle::Queen,
                            CommandTitle::General,
                        ][slot]
                    || (f.title == CommandTitle::General && f.class != FighterClass::Paladin)
                    || [
                        m.aggression,
                        m.caution,
                        m.courage,
                        m.teamwork,
                        m.patience,
                        m.self_preservation,
                        m.discipline,
                    ]
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                    || f.experience > 1_000_000
                    || f.battles > 1_000_000
                    || f.wins > f.battles
                    || f.history.len() > 20
                    || f.history.iter().any(|h| {
                        !h.damage.is_finite()
                            || h.damage < 0.0
                            || h.battle_id >= self.next_battle_id
                            || h.experience_gained > 55
                    })
                {
                    return Err("invalid persistent fighter".into());
                }
            }
        }
        Ok(())
    }
    pub fn begin_battle(
        &mut self,
        seed: u64,
        arena: Arena,
        controller: Controller,
    ) -> Result<(u64, Battle), String> {
        self.validate()?;
        if self.next_battle_id >= 1_000_000 {
            return Err("battle sequence exhausted".into());
        }
        let id = self.next_battle_id;
        self.next_battle_id = self
            .next_battle_id
            .checked_add(1)
            .ok_or("battle sequence overflow")?;
        let mut b = Battle::with_founders(seed, false, arena);
        b.controller = controller;
        for u in &mut b.units {
            let f = &self.teams[u.team].fighters[u.id % 3];
            u.command_title = Some(f.title);
            u.class = Some(f.class);
            u.role = f.class.role();
            u.fighter_id = Some(f.fighter_id.clone());
            u.experience = f.experience;
            u.battle_count = f.battles;
            u.mind = f.mind;
            u.health = u.max_health();
        }
        Ok((id, b))
    }
    pub fn capture_minds(&mut self, b: &Battle) -> Result<(), String> {
        if !b.founders || b.units.len() != 6 {
            return Err("expected a Founders battle".into());
        }
        for u in &b.units {
            if u.id >= 6 || u.team >= 2 || u.team != u.id / 3 {
                return Err("invalid battle roster index".into());
            }
            let f = &self.teams[u.team].fighters[u.id % 3];
            if u.fighter_id.as_deref() != Some(f.fighter_id.as_str()) {
                return Err("fighter snapshot mismatch".into());
            }
        }
        let mut updated = self.clone();
        for u in &b.units {
            updated.teams[u.team].fighters[u.id % 3].mind = u.mind;
        }
        updated.validate()?;
        *self = updated;
        Ok(())
    }
    pub fn settle(&mut self, id: u64, b: &Battle) -> Result<(), String> {
        if !b.finished
            || id >= self.next_battle_id
            || self.last_settled_id.is_some_and(|last| id <= last)
        {
            return Err("unfinished, stale or already rewarded battle".into());
        }
        let mut updated = self.clone();
        updated.capture_minds(b)?;
        let winner = b.winner();
        for u in &b.units {
            let f = &mut updated.teams[u.team].fighters[u.id % 3];
            let won = winner == Some(u.team);
            let survived = u.health > 0.0;
            let xp = 25
                + u32::from(won) * 15
                + u32::from(survived) * 5
                + (u.damage_dealt / 20.0).clamp(0.0, 10.0) as u32;
            f.experience = f.experience.saturating_add(xp).min(1_000_000);
            f.battles += 1;
            f.wins += u32::from(won);
            // Deterministic habits, not online PyTorch training or a promise of better tactics.
            if u.health_fraction() < 0.5 {
                f.mind.self_preservation = (f.mind.self_preservation + 0.01).min(1.0);
            }
            if u.decision_counts[8] > 0 {
                f.mind.teamwork = (f.mind.teamwork + 0.005).min(1.0);
            }
            if won {
                f.mind.courage = (f.mind.courage + 0.005).min(1.0);
            }
            if survived && u.decision_counts[5] > 0 {
                f.mind.discipline = (f.mind.discipline + 0.005).min(1.0);
            }
            f.history.push(History {
                battle_id: id,
                seed: b.seed,
                arena: b.arena,
                won,
                survived,
                damage: u.damage_dealt,
                experience_gained: xp,
            });
            if f.history.len() > 20 {
                f.history.remove(0);
            }
        }
        updated.last_settled_id = Some(id);
        updated.validate()?;
        *self = updated;
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let c: Self = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        c.validate()?;
        Ok(c)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let temporary = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        use std::io::Write;
        let mut file = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        if path.exists() {
            std::fs::copy(path, path.with_extension("json.bak")).map_err(|e| e.to_string())?;
        }
        std::fs::rename(temporary, path).map_err(|e| e.to_string())
    }
}
