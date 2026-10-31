//! A bounded history of witnessed accomplishments. Epithets never alter mechanics.
use crate::{
    Battle,
    opportunity::{Primitive, VaultWitness},
    progression::Campaign,
    theus::{Check, Critical, Outcome},
};
use serde::{Deserialize, Serialize};
pub const RETAINED_ENTRIES: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrikeWitness {
    pub sequence: u64,
    pub tick: u32,
    pub attack: Check,
    pub defence: Option<Check>,
    pub outcome: Outcome,
    pub critical: Critical,
    pub damage: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub strike_selected_tick: u32,
    pub battle_id: u64,
    pub seed: u64,
    pub simulation_build: String,
    pub actor_fighter_id: String,
    pub opponent: String,
    pub target_fighter_id: String,
    pub arena: crate::terrain::Arena,
    pub plan: u64,
    pub started_tick: u32,
    pub completed_tick: u32,
    pub primitives: Vec<Primitive>,
    pub distance: f32,
    pub starting_health: f32,
    pub starting_max_health: f32,
    pub final_health: f32,
    pub starting_wounds: u8,
    pub vault: VaultWitness,
    pub strike: StrikeWitness,
}
impl Entry {
    pub fn valid(&self, next_battle_id: u64, fighter_id: &str) -> bool {
        let check = |c: Check| {
            c.dice.iter().all(|d| (1..=6).contains(d))
                && (0..=32).contains(&c.target)
                && c.margin == c.target - c.dice.iter().map(|d| *d as i16).sum::<i16>()
        };
        self.battle_id < next_battle_id
            && self.actor_fighter_id == fighter_id
            && !fighter_id.is_empty()
            && !self.target_fighter_id.is_empty()
            && self.target_fighter_id.len() <= 128
            && !self.opponent.is_empty()
            && self.opponent.chars().count() <= 64
            && !self.opponent.chars().any(char::is_control)
            && !self.simulation_build.is_empty()
            && self.simulation_build.len() <= 128
            && self.started_tick <= self.vault.tick
            && self
                .vault
                .tick
                .checked_add(14)
                .is_some_and(|end| end <= self.vault.landed_tick)
            && self.vault.landed_tick <= self.strike_selected_tick
            && self.strike_selected_tick <= self.strike.tick
            && self.strike.tick == self.completed_tick
            && self.completed_tick <= crate::BATTLE_TICK_LIMIT
            && (2..=crate::opportunity::MAX_STEPS).contains(&self.primitives.len())
            && self.primitives.last() == Some(&Primitive::Strike)
            && self.primitives.contains(&Primitive::WallVault)
            && [
                self.distance,
                self.starting_health,
                self.starting_max_health,
                self.final_health,
                self.strike.damage,
            ]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.)
            && self.distance <= 1000.
            && self.starting_health > 0.
            && self.starting_max_health > 0.
            && self.starting_health <= self.starting_max_health
            && self.final_health <= self.starting_max_health
            && self.starting_wounds <= 15
            && self.strike.damage > 0.
            && matches!(self.strike.outcome, Outcome::Hit | Outcome::Critical)
            && check(self.strike.attack)
            && self.strike.defence.is_none_or(check)
            && self
                .vault
                .from
                .iter()
                .chain(&self.vault.to)
                .all(|v| v.is_finite() && v.abs() <= 1000.)
    }
    pub fn title(&self) -> String {
        format!("The Vault at {}", self.arena.name())
    }
    pub fn epithet(&self) -> &'static str {
        "Wallrunner"
    }
    pub fn summary(&self, name: &str) -> String {
        format!(
            "{} — {} traveled {:.1} m, completed a wall vault and struck {} for {:.1} damage. Started at {:.1}/{:.1} HP; finished the battle {}.",
            self.title(),
            name,
            self.distance,
            self.opponent,
            self.strike.damage,
            self.starting_health,
            self.starting_max_health,
            if self.final_health > 0. {
                format!("with {:.1} HP", self.final_health)
            } else {
                "downed".into()
            }
        )
    }
}

/// Retain the first qualifying witnessed composition per fighter/battle.
/// This provisional detector is not a ranking of legendary value; full completed
/// compositions remain available in the battle and study receipts.
pub fn capture(
    battle_id: u64,
    battle: &Battle,
    actor: usize,
    campaign: &Campaign,
) -> Option<Entry> {
    let unit = battle.units.get(actor)?;
    let completed = battle.opportunities.completed.iter().find(|c| {
        c.actor == actor
            && c.vault
                .as_ref()
                .is_some_and(|v| v.landed_tick >= v.tick + 14)
            && (c.starting_health <= c.starting_max_health * 0.3
                || c.starting_wounds != 0
                || c.strike.resolution.critical != Critical::None)
    })?;
    let target = battle.units.get(completed.strike.defender)?;
    let opponent = &campaign.teams[target.team].fighters[target.id % battle.team_size()];
    let resolution = &completed.strike.resolution;
    Some(Entry {
        strike_selected_tick: completed.strike_selected_tick,
        battle_id,
        seed: battle.seed,
        simulation_build: crate::challenges::BUILD.into(),
        actor_fighter_id: campaign.teams[unit.team].fighters[unit.id % battle.team_size()]
            .source_identity()
            .to_string(),
        opponent: opponent.name.clone(),
        target_fighter_id: opponent.source_identity().to_string(),
        arena: battle.initial_arena,
        plan: completed.plan,
        started_tick: completed.started_tick,
        completed_tick: completed.completed_tick,
        primitives: completed.primitives.clone(),
        distance: completed.distance,
        starting_health: completed.starting_health,
        starting_max_health: completed.starting_max_health,
        final_health: unit.health.max(0.),
        starting_wounds: completed.starting_wounds,
        vault: completed.vault.clone()?,
        strike: StrikeWitness {
            sequence: completed.strike.sequence,
            tick: completed.strike.tick,
            attack: resolution.attack,
            defence: resolution.defence,
            outcome: resolution.outcome,
            critical: resolution.critical,
            damage: resolution.damage,
        },
    })
}
