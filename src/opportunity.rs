//! Bounded forward search over legal movement/strike primitives, followed by execution receipts.
//! No animation, model or proposed plan can award a successful hit.
use crate::{Battle, Unit, distance, ninjutsu, perception::Contact, skills::Skill, terrain::Arena};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_STEPS: usize = 4;
pub const MAX_EXPANSIONS: usize = 32;
pub const MAX_EVENTS: usize = 256;
pub const MAX_ATTEMPTS_PER_FIGHTER: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Primitive {
    Run,
    WallVault,
    Strike,
}
impl Primitive {
    pub fn name(self) -> &'static str {
        match self {
            Self::Run => "RUN",
            Self::WallVault => "WALL VAULT",
            Self::Strike => "STRIKE",
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Step {
    pub primitive: Primitive,
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub effort: f32,
}
#[derive(Clone, Debug, Serialize)]
pub struct Proposal {
    pub target: usize,
    pub observed_position: [f32; 2],
    pub steps: Vec<Step>,
    pub expanded: usize,
}

pub fn qualified(unit: &Unit) -> bool {
    unit.health > 0.
        && !unit.is_ranged()
        && unit.lost_limbs & !unit.prosthetics & 3 != 3
        && unit.equipped_skill(Skill::WallDance)
        && unit
            .theus
            .as_ref()
            .is_some_and(|a| a.sheet.attributes[1] >= 12 && a.sheet.rank(2) >= 1)
}
fn vault_allowed(unit: &Unit, tick: u32) -> bool {
    unit.lost_limbs & !unit.prosthetics & 12 == 0
        && unit
            .ninja
            .as_ref()
            .is_some_and(|n| n.wall_ready <= tick && n.bound_until <= tick)
        && unit
            .theus
            .as_ref()
            .is_some_and(|a| a.sheet.rating(2, 0) >= 12 && a.stagger_until <= tick)
}

#[derive(Clone)]
struct Node {
    position: [f32; 2],
    effort: f32,
    cost: f32,
    steps: Vec<Step>,
}

/// Search receives one perceived contact, never the opponent's hidden transform.
pub fn compose(unit: &Unit, contact: &Contact, arena: Arena, tick: u32) -> Option<Proposal> {
    if !qualified(unit)
        || ninjutsu::in_transit(unit)
        || contact.health <= 0.
        || tick.saturating_sub(contact.seen_tick) > crate::perception::MEMORY_TICKS
        || distance(unit.position, contact.position) > 10.
    {
        return None;
    }
    let stamina = unit.theus.as_ref()?.stamina;
    // The goal already has a normal attack solution. Do not invent movement
    // merely to produce a multi-step plan.
    if distance(unit.position, contact.position) < unit.attack_range()
        && arena.clear_segment(unit.position, contact.position)
    {
        return None;
    }
    let mut open = vec![Node {
        position: unit.position,
        effort: 0.,
        cost: 0.,
        steps: Vec::new(),
    }];
    let mut visited = BTreeSet::new();
    for expanded in 1..=MAX_EXPANSIONS {
        let index = open
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.cost.total_cmp(&b.1.cost))?
            .0;
        let node = open.remove(index);
        let key = (
            (node.position[0] * 20.).round() as i32,
            (node.position[1] * 20.).round() as i32,
        );
        if !visited.insert(key) {
            continue;
        }
        let d = distance(node.position, contact.position);
        if d < unit.attack_range() * 0.9
            && arena.clear_segment(node.position, contact.position)
            && node.effort + 1.5 <= stamina
            && !node.steps.is_empty()
            && node.steps.len() < MAX_STEPS
        {
            let mut steps = node.steps;
            steps.push(Step {
                primitive: Primitive::Strike,
                from: node.position,
                to: contact.position,
                effort: 1.5,
            });
            return Some(Proposal {
                target: contact.enemy,
                observed_position: contact.position,
                steps,
                expanded,
            });
        }
        if node.steps.len() + 1 >= MAX_STEPS {
            continue;
        }
        let delta = [
            node.position[0] - contact.position[0],
            node.position[1] - contact.position[1],
        ];
        let length = d.max(0.001);
        let mut runs = vec![[
            contact.position[0] + delta[0] / length * unit.attack_range() * 0.75,
            contact.position[1] + delta[1] / length * unit.attack_range() * 0.75,
        ]];
        for (center, size) in arena.walls().iter() {
            let axis = usize::from(size[1] < size[0]);
            let other = 1 - axis;
            for sign in [-1., 1.] {
                let mut point = node.position;
                point[axis] = center[axis] + sign * (size[axis] * 0.5 + 1.1);
                point[other] = point[other].clamp(
                    center[other] - size[other] * 0.5 + 0.1,
                    center[other] + size[other] * 0.5 - 0.1,
                );
                if distance(node.position, point) <= 4.5 {
                    runs.push(point);
                }
            }
        }
        for to in runs {
            let travel = distance(node.position, to);
            if travel < 0.4 || !arena.walk_segment(node.position, to) {
                continue;
            }
            let mut steps = node.steps.clone();
            steps.push(Step {
                primitive: Primitive::Run,
                from: node.position,
                to,
                effort: 0.,
            });
            open.push(Node {
                position: to,
                effort: node.effort,
                cost: node.cost + travel,
                steps,
            });
        }
        if vault_allowed(unit, tick)
            && node.effort + 8.5 <= stamina
            && !node
                .steps
                .iter()
                .any(|s| s.primitive == Primitive::WallVault)
        {
            let mut projected = unit.clone();
            projected.position = node.position;
            if let Some(leap) = ninjutsu::wall_plan(&projected, arena, contact.position, tick) {
                let to = [leap.to[0], leap.to[2]];
                let mut steps = node.steps.clone();
                steps.push(Step {
                    primitive: Primitive::WallVault,
                    from: node.position,
                    to,
                    effort: 7.,
                });
                open.push(Node {
                    position: to,
                    effort: node.effort + 7.,
                    cost: node.cost
                        + distance(node.position, to)
                        + 1.
                        + 3. * (1. - unit.health_fraction()) * unit.mind.caution,
                    steps,
                });
            }
        }
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Phase {
    Proposed,
    PrimitiveCompleted,
    Cancelled,
    Completed,
}
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub sequence: u64,
    pub tick: u32,
    pub actor: usize,
    pub plan: u64,
    pub phase: Phase,
    pub primitive: Option<Primitive>,
    pub detail: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub proposed_steps: Vec<Step>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultWitness {
    pub sequence: u64,
    pub tick: u32,
    pub landed_tick: u32,
    pub wall: usize,
    pub from: [f32; 3],
    pub to: [f32; 3],
}
#[derive(Clone, Debug, Serialize)]
pub struct Completed {
    pub strike_selected_tick: u32,
    pub actor: usize,
    pub plan: u64,
    pub started_tick: u32,
    pub completed_tick: u32,
    pub primitives: Vec<Primitive>,
    pub distance: f32,
    pub starting_health: f32,
    pub starting_max_health: f32,
    pub starting_wounds: u8,
    pub vault: Option<VaultWitness>,
    pub strike: crate::theus::Event,
}
#[derive(Clone, Debug, Serialize)]
pub struct Active {
    pub strike_selected_tick: Option<u32>,
    pub actor: usize,
    pub id: u64,
    pub proposal: Proposal,
    pub cursor: usize,
    pub started_tick: u32,
    pub step_tick: u32,
    pub distance: f32,
    pub previous_position: [f32; 2],
    pub starting_health: f32,
    pub starting_max_health: f32,
    pub starting_wounds: u8,
    pub vault: Option<VaultWitness>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Runtime {
    pub active: Vec<Active>,
    pub events: Vec<Event>,
    pub completed: Vec<Completed>,
    cooldowns: Vec<(usize, u32)>,
    next_plan: u64,
    attempts: Vec<(usize, u8)>,
}
impl Runtime {
    pub fn empty(&self) -> bool {
        self.active.is_empty() && self.events.is_empty() && self.completed.is_empty()
    }
    fn emit(
        &mut self,
        tick: u32,
        active: &Active,
        phase: Phase,
        primitive: Option<Primitive>,
        detail: &str,
    ) {
        let sequence = self.events.last().map_or(0, |e| e.sequence + 1);
        self.events.push(Event {
            sequence,
            tick,
            actor: active.actor,
            plan: active.id,
            phase,
            primitive,
            detail: detail.into(),
            proposed_steps: if phase == Phase::Proposed {
                active.proposal.steps.clone()
            } else {
                Vec::new()
            },
        });
        if self.events.len() > MAX_EVENTS {
            self.events.remove(0);
        }
    }
    pub fn target(&self, actor: usize) -> Option<usize> {
        Some(
            self.active
                .iter()
                .find(|a| a.actor == actor)?
                .proposal
                .target,
        )
    }
    pub fn owns_mobility(&self, actor: usize) -> bool {
        self.active.iter().any(|a| a.actor == actor)
    }
    pub fn record_action(&mut self, actor: usize, action: usize, tick: u32) {
        if action == 1
            && let Some(active) = self.active.iter_mut().find(|a| a.actor == actor)
            && active.proposal.steps[active.cursor].primitive == Primitive::Strike
        {
            active.strike_selected_tick.get_or_insert(tick);
        }
    }
    pub fn description(&self, actor: usize) -> String {
        self.active
            .iter()
            .find(|a| a.actor == actor)
            .map_or_else(String::new, |active| {
                format!(
                    "Opportunity #{}: {}",
                    active.id,
                    active
                        .proposal
                        .steps
                        .iter()
                        .map(|s| s.primitive.name())
                        .collect::<Vec<_>>()
                        .join(" -> ")
                )
            })
    }
    pub fn attempt_count(&self) -> usize {
        self.attempts.iter().map(|(_, count)| *count as usize).sum()
    }
    pub fn wall_goal(&self, actor: usize) -> Option<[f32; 2]> {
        let active = self.active.iter().find(|a| a.actor == actor)?;
        let step = active.proposal.steps.get(active.cursor)?;
        (step.primitive == Primitive::WallVault).then_some(step.to)
    }
    pub fn vault_started(&mut self, actor: usize, leap: &ninjutsu::Leap, sequence: u64) {
        if let Some(active) = self.active.iter_mut().find(|a| a.actor == actor) {
            active.vault = Some(VaultWitness {
                sequence,
                tick: leap.start,
                landed_tick: 0,
                wall: leap.wall,
                from: leap.from,
                to: leap.to,
            });
        }
    }
    pub fn directive(
        &self,
        unit: &Unit,
        mask: &[bool; crate::ACTION_COUNT],
    ) -> Option<(usize, [f32; 2])> {
        let active = self.active.iter().find(|a| a.actor == unit.id)?;
        let step = active.proposal.steps.get(active.cursor)?;
        let action = if step.primitive == Primitive::Strike {
            1
        } else {
            0
        };
        if !mask[action] {
            return None;
        }
        if action == 1
            && !unit
                .contacts
                .iter()
                .any(|c| c.enemy == active.proposal.target && c.visible)
        {
            return None;
        }
        Some((
            action,
            if step.primitive == Primitive::WallVault {
                unit.position
            } else {
                step.to
            },
        ))
    }
}

/// Called after actual vault motion advances, before new arts or ground movement.
pub fn observe(battle: &mut Battle, record_movement: bool) {
    if !battle.founders {
        return;
    }
    let tick = battle.tick;
    let pending = std::mem::take(&mut battle.opportunities.active);
    battle
        .opportunities
        .cooldowns
        .retain(|(_, ready)| *ready > tick);
    for mut active in pending {
        let unit = &battle.units[active.actor];
        if record_movement {
            active.distance += distance(active.previous_position, unit.position);
            active.previous_position = unit.position;
        }
        let step = &active.proposal.steps[active.cursor];
        let strike = (step.primitive == Primitive::Strike)
            .then(|| {
                battle
                    .theus_events
                    .iter()
                    .find(|e| {
                        !e.ranged
                            && active
                                .strike_selected_tick
                                .is_some_and(|selected| e.tick >= selected)
                            && e.attacker == unit.id
                            && e.defender == active.proposal.target
                            && e.tick >= active.step_tick
                    })
                    .cloned()
            })
            .flatten();
        if let Some(event) = strike {
            if event.resolution.damage > 0. {
                battle.opportunities.emit(
                    tick,
                    &active,
                    Phase::PrimitiveCompleted,
                    Some(Primitive::Strike),
                    "Actual Theus hit",
                );
                battle.opportunities.emit(
                    tick,
                    &active,
                    Phase::Completed,
                    None,
                    "Goal reached by actual movement and strike receipts",
                );
                battle.opportunities.completed.push(Completed {
                    strike_selected_tick: active
                        .strike_selected_tick
                        .expect("filtered for a committed strike"),
                    actor: unit.id,
                    plan: active.id,
                    started_tick: active.started_tick,
                    completed_tick: event.tick,
                    primitives: active.proposal.steps.iter().map(|s| s.primitive).collect(),
                    distance: active.distance,
                    starting_health: active.starting_health,
                    starting_max_health: active.starting_max_health,
                    starting_wounds: active.starting_wounds,
                    vault: active.vault,
                    strike: event,
                });
                battle.opportunities.cooldowns.push((unit.id, tick + 60));
                continue;
            }
            battle.opportunities.emit(
                tick,
                &active,
                Phase::Cancelled,
                Some(Primitive::Strike),
                "Theus strike missed or was defended",
            );
            battle.opportunities.cooldowns.push((unit.id, tick + 30));
            continue;
        }
        let reason = if unit.health <= 0. {
            Some("Actor downed before completing the plan")
        } else if tick.saturating_sub(active.started_tick) > 120 {
            Some("Plan time budget exhausted")
        } else if !qualified(unit) {
            Some("Required training, equipment or usable hand unavailable")
        } else if !unit
            .contacts
            .iter()
            .any(|c| c.enemy == active.proposal.target)
        {
            Some("Target contact expired")
        } else if step.primitive == Primitive::Run
            && !battle.arena.walk_segment(unit.position, step.to)
        {
            Some("Ground path no longer clear")
        } else if step.primitive == Primitive::WallVault
            && active.vault.is_none()
            && !vault_allowed(unit, tick)
        {
            Some("Vault requirements no longer hold")
        } else if step.primitive == Primitive::WallVault
            && active.vault.is_none()
            && ninjutsu::wall_plan(unit, battle.arena, step.to, tick).is_none()
        {
            Some("Vault surface or landing no longer legal")
        } else if unit.theus.as_ref().is_none_or(|a| {
            a.stamina
                < active.proposal.steps[active.cursor..]
                    .iter()
                    .map(|s| {
                        if s.primitive == Primitive::WallVault && active.vault.is_some() {
                            0.
                        } else {
                            s.effort
                        }
                    })
                    .sum::<f32>()
        }) {
            Some("Remaining effort budget unavailable")
        } else {
            None
        };
        if let Some(reason) = reason {
            battle.opportunities.emit(
                tick,
                &active,
                Phase::Cancelled,
                Some(step.primitive),
                reason,
            );
            battle.opportunities.cooldowns.push((unit.id, tick + 30));
            continue;
        }
        let finished = match step.primitive {
            Primitive::Run => distance(unit.position, step.to) <= 0.35,
            Primitive::WallVault => {
                active.vault.as_ref().is_some_and(|v| tick >= v.tick + 14)
                    && !ninjutsu::in_transit(unit)
                    && distance(unit.position, step.to) < 0.35
            }
            Primitive::Strike => false,
        };
        if finished {
            if step.primitive == Primitive::WallVault
                && let Some(vault) = &mut active.vault
            {
                vault.landed_tick = tick;
            }
            battle.opportunities.emit(
                tick,
                &active,
                Phase::PrimitiveCompleted,
                Some(step.primitive),
                "Observed physical endpoint reached",
            );
            active.cursor += 1;
            active.step_tick = tick;
        }
        battle.opportunities.active.push(active);
    }
}

pub fn finish(battle: &mut Battle) {
    let pending = std::mem::take(&mut battle.opportunities.active);
    for active in pending {
        battle.opportunities.emit(
            battle.tick,
            &active,
            Phase::Cancelled,
            None,
            "Battle ended before the plan completed",
        );
    }
}

pub fn prepare(battle: &mut Battle) {
    observe(battle, false);
    let tick = battle.tick;
    if !battle.founders || !tick.is_multiple_of(5) {
        return;
    }
    for unit in &battle.units {
        if !qualified(unit)
            || battle.opportunities.owns_mobility(unit.id)
            || battle
                .opportunities
                .cooldowns
                .iter()
                .any(|(id, _)| *id == unit.id)
            || battle
                .opportunities
                .attempts
                .iter()
                .any(|(id, count)| *id == unit.id && *count >= MAX_ATTEMPTS_PER_FIGHTER)
        {
            continue;
        }
        let contact = unit
            .contacts
            .iter()
            .filter(|c| c.health > 0.)
            .min_by(|a, b| {
                distance(unit.position, a.position)
                    .total_cmp(&distance(unit.position, b.position))
                    .then(a.enemy.cmp(&b.enemy))
            });
        let Some(proposal) = contact.and_then(|c| compose(unit, c, battle.arena, tick)) else {
            continue;
        };
        let id = battle.opportunities.next_plan;
        battle.opportunities.next_plan += 1;
        if let Some((_, count)) = battle
            .opportunities
            .attempts
            .iter_mut()
            .find(|(actor, _)| *actor == unit.id)
        {
            *count += 1;
        } else {
            battle.opportunities.attempts.push((unit.id, 1));
        }
        let active = Active {
            strike_selected_tick: None,
            actor: unit.id,
            id,
            proposal,
            cursor: 0,
            started_tick: tick,
            step_tick: tick,
            distance: 0.,
            previous_position: unit.position,
            starting_health: unit.health,
            starting_max_health: unit.max_health(),
            starting_wounds: unit.lost_limbs & !unit.prosthetics,
            vault: None,
        };
        battle.opportunities.emit(
            tick,
            &active,
            Phase::Proposed,
            None,
            "Bounded legal-primitive search",
        );
        battle.opportunities.active.push(active);
    }
}
