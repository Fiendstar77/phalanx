//! Local persistent squads and bounded battle-conditioned progression. No online authority.
use crate::{Battle, CommandTitle, FighterClass, Mind, learned::Controller, terrain::Arena};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};
/// Earned-gold slot prices, indexed by the destination league (4 through 20).
/// Content after league four is priced here but not yet playable.
pub const SLOT_COSTS: [u32; 17] = [
    1_000, 1_800, 3_000, 5_000, 8_000, 12_000, 18_000, 26_000, 38_000, 55_000, 80_000, 115_000,
    165_000, 235_000, 330_000, 460_000, 650_000,
];
pub fn slot_cost(destination: usize) -> Option<u32> {
    destination
        .checked_sub(4)
        .and_then(|i| SLOT_COSTS.get(i).copied())
}
pub fn gold_reward(league: usize, won: bool) -> u32 {
    let n = league.clamp(3, 20) as u32 - 3;
    let victory = 30 + 12 * n + 2 * n * n;
    if won { victory } else { victory / 2 }
}
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
    #[serde(default)]
    pub lost_limbs: u8,
    #[serde(default)]
    pub prosthetics: u8,
}
/// Cosmetic choices only. Defaults keep older local saves readable.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Appearance {
    pub female: Option<bool>,
    pub hair: u8,
    pub face: u8,
    pub eyes: u8,
    pub nose: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WoundRecord {
    pub battle_id: u64,
    pub limb: u8,
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
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub lost_limbs: u8,
    #[serde(default)]
    pub prosthetics: u8,
    #[serde(default)]
    pub wounds: Vec<WoundRecord>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Phalanx {
    pub name: String,
    pub color: [f32; 3],
    pub fighters: Vec<Fighter>,
    #[serde(default)]
    pub salvage: u32,
    #[serde(default = "starting_gold")]
    pub gold: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Campaign {
    pub schema_version: u32,
    pub teams: [Phalanx; 2],
    pub next_battle_id: u64,
    pub last_settled_id: Option<u64>,
}
fn starting_gold() -> u32 {
    100
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
                salvage: 0,
                gold: starting_gold(),
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
                    appearance: Appearance::default(),
                    lost_limbs: 0,
                    prosthetics: 0,
                    wounds: Vec::new(),
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
                || team.salvage > 1_000_000
                || team.gold > 1_000_000
                || !(3..=4).contains(&team.fighters.len())
                || team.fighters.len() != self.teams[0].fighters.len()
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
                        ]
                        .get(slot)
                        .copied()
                        .unwrap_or(CommandTitle::Member)
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
                    || f.wounds.len() > 4
                    || f.wounds.iter().any(|w| {
                        ![1, 2, 4, 8].contains(&w.limb) || w.battle_id >= self.next_battle_id
                    })
                    || f.wounds
                        .iter()
                        .map(|w| w.limb)
                        .collect::<BTreeSet<_>>()
                        .len()
                        != f.wounds.len()
                    || f.lost_limbs > 15
                    || f.prosthetics & !f.lost_limbs != 0
                    || f.appearance.hair > 6
                    || f.appearance.face > 16
                    || f.appearance.eyes > 15
                    || f.appearance.nose > 16
                    || f.experience > 1_000_000
                    || f.battles > 1_000_000
                    || f.wins > f.battles
                    || f.history.len() > 20
                    || f.history.iter().any(|h| {
                        h.lost_limbs > 15
                            || h.prosthetics & !h.lost_limbs != 0
                            || !h.damage.is_finite()
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
        let mut b = Battle::with_roster(seed, false, arena, true, self.teams[0].fighters.len());
        b.controller = controller;
        let size = b.team_size();
        for u in &mut b.units {
            let f = &self.teams[u.team].fighters[u.id % size];
            u.command_title = Some(f.title);
            u.class = Some(f.class);
            u.role = f.class.role();
            u.fighter_id = Some(f.fighter_id.clone());
            u.experience = f.experience;
            u.battle_count = f.battles;
            u.mind = f.mind;
            u.lost_limbs = f.lost_limbs;
            u.prosthetics = f.prosthetics;
            u.health = u.max_health();
        }
        Ok((id, b))
    }
    /// First crafting recipe: one tier-I iron limb costs five battle salvage.
    pub fn craft_prosthetic(&mut self, team: usize, slot: usize) -> Result<&'static str, String> {
        self.validate()?;
        let squad = self.teams.get_mut(team).ok_or("invalid squad")?;
        let fighter = squad.fighters.get_mut(slot).ok_or("invalid fighter")?;
        let missing = fighter.lost_limbs & !fighter.prosthetics;
        if missing == 0 {
            return Err("No limb needs replacement".into());
        }
        if squad.gold < 15 {
            return Err("Need 15 gold for the workshop".into());
        }
        if squad.salvage < 5 {
            return Err("Need 5 salvage; earn it by completing a battle".into());
        }
        let index = missing.trailing_zeros();
        let bit = 1_u8 << index;
        fighter.prosthetics |= bit;
        squad.salvage -= 5;
        squad.gold -= 15;
        Ok(["Left arm", "Right arm", "Left leg", "Right leg"][index as usize])
    }
    /// First league milestone. Local opponent expands equally; no paid combat advantage.
    pub fn unlock_fourth(&mut self) -> Result<(), String> {
        self.validate()?;
        if self.teams[0].fighters.len() != 3 {
            return Err("League 4 is already unlocked".into());
        }
        let cost = slot_cost(4).unwrap();
        if self.teams[0].gold < cost {
            return Err(format!("Fourth slot: {} / {cost} gold", self.teams[0].gold));
        }
        let mut updated = self.clone();
        updated.teams[0].gold -= cost;
        for team in &mut updated.teams {
            let mut recruit = team.fighters[0].clone();
            recruit.fighter_id = format!("{}-recruit-4", recruit.fighter_id);
            recruit.name = "Recruit".into();
            recruit.title = CommandTitle::Member;
            recruit.class = FighterClass::Warrior;
            recruit.experience = 0;
            recruit.battles = 0;
            recruit.wins = 0;
            recruit.history.clear();
            recruit.wounds.clear();
            recruit.lost_limbs = 0;
            recruit.prosthetics = 0;
            recruit.appearance = Appearance::default();
            team.fighters.push(recruit);
        }
        updated.validate()?;
        *self = updated;
        Ok(())
    }
    /// Retraining preserves identity, experience and wounds. Generals remain Paladins.
    pub fn set_class(
        &mut self,
        team: usize,
        slot: usize,
        class: FighterClass,
    ) -> Result<(), String> {
        self.validate()?;
        let squad = self.teams.get_mut(team).ok_or("invalid squad")?;
        let f = squad.fighters.get_mut(slot).ok_or("invalid fighter")?;
        if f.title == CommandTitle::General {
            return Err("The General is your Paladin".into());
        }
        if f.class == class {
            return Err("Already trained in this class".into());
        }
        if squad.gold < 10 {
            return Err("Retraining costs 10 gold".into());
        }
        f.class = class;
        squad.gold -= 10;
        Ok(())
    }
    pub fn capture_minds(&mut self, b: &Battle) -> Result<(), String> {
        if !b.founders || b.units.len() != self.teams[0].fighters.len() * 2 {
            return Err("expected a Founders battle".into());
        }
        for u in &b.units {
            if u.id >= b.units.len() || u.team >= 2 || u.team != u.id / b.team_size() {
                return Err("invalid battle roster index".into());
            }
            let f = &self.teams[u.team].fighters[u.id % b.team_size()];
            if u.fighter_id.as_deref() != Some(f.fighter_id.as_str()) {
                return Err("fighter snapshot mismatch".into());
            }
        }
        let mut updated = self.clone();
        for u in &b.units {
            updated.teams[u.team].fighters[u.id % b.team_size()].mind = u.mind;
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
        for (team, squad) in updated.teams.iter_mut().enumerate() {
            squad.gold = squad
                .gold
                .saturating_add(gold_reward(b.team_size(), winner == Some(team)))
                .min(1_000_000);
            squad.salvage = squad
                .salvage
                .saturating_add(if winner == Some(team) { 10 } else { 6 })
                .min(1_000_000);
        }
        for u in &b.units {
            let f = &mut updated.teams[u.team].fighters[u.id % b.team_size()];
            let won = winner == Some(u.team);
            let survived = u.health > 0.0;
            // The prototype recovers fighters after battle, but wounds persist.
            for bit in [1, 2, 4, 8] {
                if u.lost_limbs & bit != 0 && !f.wounds.iter().any(|w| w.limb == bit) {
                    f.wounds.push(WoundRecord {
                        battle_id: id,
                        limb: bit,
                    });
                }
            }
            f.lost_limbs = u.lost_limbs;
            f.prosthetics = u.prosthetics;
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
                lost_limbs: u.lost_limbs,
                prosthetics: u.prosthetics,
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
