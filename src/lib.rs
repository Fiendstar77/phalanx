//! Fixed-step combat model, independent of rendering and wall-clock time.
use serde::Serialize;
pub mod cognition;
pub mod learned;
pub mod navigation;
pub mod perception;
pub mod telemetry;
pub mod terrain;
use learned::{Comparison, ComparisonSummary, Controller, LearnedPolicy};
use navigation::{Route, plan_known};
use perception::{COMMUNICATION_RANGE, Contact, can_see, cooperative, threats};
use std::sync::Arc;
use terrain::Arena;

pub const ACTION_COUNT: usize = 9;

pub const DT: f32 = 0.1;
pub const ACTIONS: [&str; ACTION_COUNT] = [
    "ADVANCE",
    "ATTACK",
    "HIGH GROUND",
    "FLANK",
    "REGROUP",
    "RETREAT",
    "HOLD",
    "SEEK COVER",
    "SUPPORT ALLY",
];

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Mind {
    pub aggression: f32,
    pub caution: f32,
    pub courage: f32,
    pub teamwork: f32,
    pub patience: f32,
    pub self_preservation: f32,
    pub discipline: f32,
}
impl Mind {
    pub fn aggressive() -> Self {
        Self {
            aggression: 0.9,
            caution: 0.2,
            courage: 0.85,
            teamwork: 0.25,
            patience: 0.2,
            self_preservation: 0.25,
            discipline: 0.3,
        }
    }
    pub fn disciplined() -> Self {
        Self {
            aggression: 0.5,
            caution: 0.8,
            courage: 0.65,
            teamwork: 0.9,
            patience: 0.8,
            self_preservation: 0.65,
            discipline: 0.9,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Role {
    Fighter,
    RockThrower,
}
impl Role {
    pub fn range(self) -> f32 {
        if self == Self::Fighter { 1.5 } else { 6.5 }
    }
    pub fn name(self) -> &'static str {
        if self == Self::Fighter {
            "MELEE"
        } else {
            "ROCK THROWER"
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CommandTitle {
    King,
    Queen,
    General,
}
impl CommandTitle {
    pub fn name(self) -> &'static str {
        match self {
            Self::King => "KING",
            Self::Queen => "QUEEN",
            Self::General => "GENERAL",
        }
    }
    pub fn royal(self) -> bool {
        matches!(self, Self::King | Self::Queen)
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Rock {
    pub shooter: usize,
    pub team: usize,
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub age: u32,
    pub flight_ticks: u32,
    pub damage: f32,
}
#[derive(Clone, Debug, Serialize)]
pub struct Unit {
    pub id: usize,
    pub team: usize,
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_title: Option<CommandTitle>,
    pub position: [f32; 2],
    pub health: f32,
    pub mind: Mind,
    pub action: usize,
    pub scores: [f32; ACTION_COUNT],
    pub target: [f32; 2],
    pub enemy: Option<usize>,
    pub cooldown: f32,
    pub damage_dealt: f32,
    pub distance_moved: f32,
    pub decision_counts: [u32; ACTION_COUNT],
    pub route: Route,
    pub path_cursor: usize,
    pub route_goal: [f32; 2],
    pub route_mind: Mind,
    pub cover_ticks: u32,
    pub rough_ticks: u32,
    pub alive_ticks: u32,
    pub high_ground_ticks: u32,
    pub rocks_thrown: u32,
    pub rocks_hit: u32,
    pub contacts: Vec<Contact>,
    pub visible_enemies: usize,
    pub shared_reports: u32,
    pub support_target: Option<usize>,
    pub flank_anchor: Option<[f32; 2]>,
    pub flank_until: u32,
    pub decision_reason: String,
    pub comparison: Option<Comparison>,
    #[serde(skip_serializing_if = "search_not_started")]
    pub search_waypoint: u32,
}
impl Unit {
    pub fn max_health(&self) -> f32 {
        if self.command_title.is_some_and(CommandTitle::royal) {
            110.0
        } else {
            100.0
        }
    }
    pub fn health_fraction(&self) -> f32 {
        self.health / self.max_health()
    }
    pub fn damage_multiplier(&self) -> f32 {
        if self.command_title.is_some_and(CommandTitle::royal) {
            1.10
        } else {
            1.0
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Battle {
    pub seed: u64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub founders: bool,
    pub tick: u32,
    pub units: Vec<Unit>,
    pub finished: bool,
    pub result: String,
    pub swapped: bool,
    pub arena: Arena,
    pub rocks: Vec<Rock>,
    pub schema_version: u32,
    pub controller: Controller,
    pub learned_team: Option<usize>,
    #[serde(skip)]
    pub policy: Option<Arc<LearnedPolicy>>,
    pub disagreements: Vec<Comparison>,
    pub comparison_summary: ComparisonSummary,
    pub learned_fallbacks: u32,
    pub simulation_version: String,
    pub ally_distance_sum: [f64; 2],
    pub ally_distance_samples: [u32; 2],
}
pub fn elevation(p: [f32; 2]) -> f32 {
    let x = (1.0 - p[0].abs() / 3.5).max(0.0);
    let z = (1.0 - p[1].abs() / 7.0).max(0.0);
    x * z * 1.7
}
pub fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}
// Stateless noise isolates decision variation from the number of previous random draws.
fn noise(seed: u64, tick: u32, id: usize, action: usize) -> f32 {
    let mut n = seed
        ^ (tick as u64).wrapping_mul(0x9e3779b97f4a7c15)
        ^ ((id * ACTION_COUNT + action) as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((n ^ (n >> 31)) as u32 as f32 / u32::MAX as f32 - 0.5) * 0.035
}
fn search_not_started(leg: &u32) -> bool {
    *leg == 0
}
impl Battle {
    pub fn new(seed: u64, swapped: bool) -> Self {
        Self::with_arena(seed, swapped, Arena::Ridge)
    }
    pub fn with_arena(seed: u64, swapped: bool, arena: Arena) -> Self {
        Self::with_setup(seed, swapped, arena, false)
    }
    pub fn with_founders(seed: u64, swapped: bool, arena: Arena) -> Self {
        Self::with_setup(seed, swapped, arena, true)
    }
    pub fn team_size(&self) -> usize {
        if self.founders {
            3
        } else {
            self.arena.team_size()
        }
    }
    pub fn unit_label(&self, id: usize) -> String {
        let u = &self.units[id];
        let team = if u.team == 0 { "RED" } else { "BLUE" };
        if let Some(title) = u.command_title {
            format!("{team} {}", title.name())
        } else {
            format!("{team}-{:02}", id % self.team_size() + 1)
        }
    }
    pub fn with_setup(seed: u64, swapped: bool, arena: Arena, founders: bool) -> Self {
        let team_size = if founders { 3 } else { arena.team_size() };
        let units = (0..2 * team_size)
            .map(|id| {
                let team = id / team_size;
                let aggressive = (team == 0) != swapped;
                let position = if founders {
                    [arena.spawn(team, 0)[0], (id % team_size) as f32 * 1.6 - 1.6]
                } else {
                    arena.spawn(team, id % team_size)
                };
                Unit {
                    id,
                    team,
                    command_title: if founders {
                        Some(
                            [
                                CommandTitle::King,
                                CommandTitle::Queen,
                                CommandTitle::General,
                            ][id % team_size],
                        )
                    } else {
                        None
                    },
                    role: if (founders && id % team_size == 1)
                        || (!founders && id % team_size >= arena.fighters())
                    {
                        Role::RockThrower
                    } else {
                        Role::Fighter
                    },
                    position,
                    health: if founders && id % team_size < 2 {
                        110.0
                    } else {
                        100.0
                    },
                    mind: if aggressive {
                        Mind::aggressive()
                    } else {
                        Mind::disciplined()
                    },
                    action: 6,
                    scores: [0.0; ACTION_COUNT],
                    target: position,
                    enemy: None,
                    cooldown: 0.0,
                    damage_dealt: 0.0,
                    distance_moved: 0.0,
                    decision_counts: [0; ACTION_COUNT],
                    route: Route::default(),
                    path_cursor: 0,
                    route_goal: position,
                    route_mind: Mind::aggressive(),
                    cover_ticks: 0,
                    rough_ticks: 0,
                    alive_ticks: 0,
                    high_ground_ticks: 0,
                    rocks_thrown: 0,
                    rocks_hit: 0,
                    contacts: Vec::new(),
                    visible_enemies: 0,
                    shared_reports: 0,
                    support_target: None,
                    flank_anchor: None,
                    flank_until: 0,
                    decision_reason: "Awaiting first decision".into(),
                    comparison: None,
                    search_waypoint: 0,
                }
            })
            .collect();
        Self {
            seed,
            founders,
            tick: 0,
            units,
            finished: false,
            result: "Ready".into(),
            swapped,
            arena,
            rocks: Vec::new(),
            schema_version: 2,
            controller: Controller::Utility,
            learned_team: None,
            policy: Some(LearnedPolicy::bundled()),
            disagreements: Vec::new(),
            comparison_summary: ComparisonSummary::default(),
            learned_fallbacks: 0,
            simulation_version: env!("CARGO_PKG_VERSION").into(),
            ally_distance_sum: [0.0; 2],
            ally_distance_samples: [0; 2],
        }
    }
    /// Living agents whose latest decision has this exact, valid disagreement pair.
    pub fn disagreement_units(&self, utility: usize, learned: usize) -> Vec<usize> {
        self.units
            .iter()
            .filter(|u| {
                u.health > 0.0
                    && utility != learned
                    && u.comparison
                        .as_ref()
                        .is_some_and(|c| c.utility == utility && c.learned == Some(learned))
            })
            .map(|u| u.id)
            .collect()
    }
    pub fn survivors(&self, team: usize) -> usize {
        self.units
            .iter()
            .filter(|u| u.team == team && u.health > 0.0)
            .count()
    }
    pub fn step(&mut self) {
        self.advance(None);
    }
    pub fn step_recorded(&mut self) -> Vec<telemetry::DecisionRecord> {
        let mut records = Vec::new();
        self.advance(Some(&mut records));
        records
    }
    fn advance(&mut self, mut records: Option<&mut Vec<telemetry::DecisionRecord>>) {
        if self.finished {
            return;
        }
        perception::update(&mut self.units, self.arena, self.tick);
        let snapshot = self.units.clone();
        let mut damage = vec![0.0; self.units.len()];
        let arena = self.arena;
        let team_size = self.team_size();
        let mut landed = Vec::new();
        for rock in &mut self.rocks {
            rock.age += 1;
            if rock.age >= rock.flight_ticks {
                for target in &snapshot {
                    if target.team != rock.team
                        && target.health > 0.0
                        && distance(target.position, rock.to) < 0.75
                    {
                        let dealt = rock.damage * (1.0 - 0.3 * arena.cover(target.position));
                        damage[target.id] += dealt;
                        landed.push((rock.shooter, dealt));
                    }
                }
            }
        }
        self.rocks.retain(|r| r.age < r.flight_ticks);
        for (id, dealt) in landed {
            self.units[id].damage_dealt += dealt;
            self.units[id].rocks_hit += 1;
        }
        for unit in &mut self.units {
            if unit.health <= 0.0 {
                continue;
            }
            unit.alive_ticks += 1;
            unit.high_ground_ticks += u32::from(arena.elevation(unit.position) > 0.8);
            unit.cover_ticks += u32::from(arena.cover(unit.position) > 0.0);
            unit.rough_ticks += u32::from(arena.roughness(unit.position) > 0.0);
            unit.cooldown = (unit.cooldown - DT).max(0.0);
            let allies: Vec<_> = snapshot
                .iter()
                .filter(|a| {
                    a.team == unit.team
                        && a.id != unit.id
                        && a.health > 0.0
                        && distance(unit.position, a.position) <= COMMUNICATION_RANGE
                        && (can_see(arena, unit.position, a.position)
                            || (cooperative(unit.mind) && cooperative(a.mind)))
                })
                .collect();
            let enemy = cognition::focus_target(unit, &allies, self.tick);
            // Enemy deployment zones are authored objectives, not live enemy positions.
            let mut objective =
                arena.search_waypoint(unit.team, unit.id % team_size, unit.search_waypoint);
            if arena == Arena::Citadel
                && enemy.is_none()
                && distance(unit.position, objective) < 1.8
            {
                unit.search_waypoint += 1;
                objective =
                    arena.search_waypoint(unit.team, unit.id % team_size, unit.search_waypoint);
            }
            if self.founders && unit.search_waypoint.is_multiple_of(5) {
                objective = [
                    arena.spawn(1 - unit.team, 0)[0],
                    (unit.id % team_size) as f32 * 1.6 - 1.6,
                ];
            }
            let enemy_position = enemy.as_ref().map(|e| e.position).unwrap_or(objective);
            let d = enemy
                .as_ref()
                .map(|e| distance(unit.position, e.position))
                .unwrap_or(f32::INFINITY);
            let known_threats = threats(unit, self.tick);
            let support = cognition::support_ally(unit, &allies, self.tick);
            let center = if allies.is_empty() {
                unit.position
            } else {
                [
                    allies.iter().map(|a| a.position[0]).sum::<f32>() / allies.len() as f32,
                    allies.iter().map(|a| a.position[1]).sum::<f32>() / allies.len() as f32,
                ]
            };
            let isolation = (distance(unit.position, center) / 5.0).min(1.0);
            let injury = 1.0 - unit.health_fraction();
            let local_threat = (unit
                .contacts
                .iter()
                .filter(|c| distance(unit.position, c.position) < 3.5)
                .map(|c| c.confidence(self.tick))
                .sum::<f32>()
                / 3.0)
                .min(1.0);
            let ridge = if arena == Arena::Citadel {
                arena.high_ground(unit.position)
            } else {
                [
                    if unit.team == 0 { -0.6 } else { 0.6 },
                    unit.position[1].clamp(-2.0, 2.0),
                ]
            };
            let mut decision_features = Vec::new();
            let old_flank_until = unit.flank_until;
            let old_flank_anchor = unit.flank_anchor;
            if self.tick.is_multiple_of(5) {
                let m = unit.mind;
                let ranged = unit.role == Role::RockThrower;
                let in_range = enemy.as_ref().is_some_and(|e| e.visible)
                    && d < unit.role.range()
                    && arena.clear_segment(unit.position, enemy_position);
                let arrived = distance(unit.position, ridge) < 0.9;
                let previous_action = unit.action;
                unit.scores = [
                    0.25 + 0.65 * m.aggression + 0.2 * m.courage - 0.25 * m.caution * local_threat,
                    if in_range {
                        0.85 + 0.4 * m.aggression + 0.2 * m.courage
                            - 0.15 * injury * m.self_preservation
                    } else {
                        -1.0
                    },
                    if !arena.has_high_ground()
                        || arrived
                        || in_range
                        || (arena == Arena::Citadel && enemy.is_none() && self.tick >= 180)
                    {
                        -1.0
                    } else {
                        0.25 + 0.35 * m.caution + 0.25 * m.discipline + 0.2 * m.patience
                            - 0.015 * distance(unit.position, ridge)
                    },
                    if let Some(e) = &enemy {
                        let engaged = allies
                            .iter()
                            .filter(|a| {
                                a.enemy == Some(e.enemy)
                                    && a.action == 1
                                    && (can_see(arena, unit.position, a.position)
                                        || (cooperative(unit.mind) && cooperative(a.mind)))
                            })
                            .count() as f32;
                        let ongoing = unit.action == 3
                            && unit
                                .flank_anchor
                                .is_some_and(|p| distance(unit.position, p) > 0.8);
                        if e.visible
                            && !in_range
                            && d > 2.0
                            && d < 9.0
                            && engaged > 0.0
                            && (ongoing || self.tick >= unit.flank_until)
                        {
                            0.25 + 0.2 * m.aggression
                                + 0.35 * m.discipline
                                + 0.3 * m.teamwork
                                + 0.2 * engaged.min(2.0)
                                + if ongoing { 0.1 } else { 0.0 }
                        } else {
                            -1.0
                        }
                    } else {
                        -1.0
                    },
                    if isolation > 0.4 {
                        0.1 + 0.75 * m.teamwork * isolation + 0.2 * m.discipline
                            - 0.15 * m.aggression
                    } else {
                        -1.0
                    },
                    if injury > 0.55 || (ranged && d < 2.7) {
                        (if ranged && d < 2.7 { 1.1 } else { 0.15 })
                            + injury * (0.65 * m.self_preservation + 0.55 * m.caution)
                            + 0.25 * local_threat
                            - 0.2 * m.courage
                    } else {
                        -1.0
                    },
                    if arena.has_high_ground() && arrived && d > 3.0 && self.tick < 180 {
                        0.35 + 0.35 * m.patience + 0.2 * m.discipline
                    } else {
                        -1.0
                    },
                    if arena.cover(unit.position) < 0.5
                        && !in_range
                        && arena.nearest_cover(unit.position).is_some()
                    {
                        0.1 + 0.4 * m.caution
                            + 0.3 * m.self_preservation
                            + 0.2 * injury
                            + 0.15 * local_threat
                    } else {
                        -1.0
                    },
                    if let Some((ally, urgency)) = support {
                        0.15 + 0.65 * m.teamwork + 0.25 * m.discipline + 0.45 * urgency
                            - 0.025 * distance(unit.position, ally.position)
                    } else {
                        -1.0
                    },
                ];
                for (i, score) in unit.scores.iter_mut().enumerate() {
                    if *score >= 0.0 {
                        *score += noise(self.seed, self.tick, unit.id, i);
                    }
                }
                let m = unit.mind;
                let engaged = enemy
                    .as_ref()
                    .map(|e| {
                        allies
                            .iter()
                            .filter(|a| a.enemy == Some(e.enemy) && a.action == 1)
                            .count()
                    })
                    .unwrap_or(0);
                decision_features = vec![
                    m.aggression,
                    m.caution,
                    m.courage,
                    m.teamwork,
                    m.patience,
                    m.self_preservation,
                    m.discipline,
                    unit.health_fraction(),
                    f32::from(unit.role == Role::RockThrower),
                    unit.position[0] / arena.half_size()[0],
                    unit.position[1] / arena.half_size()[1],
                    arena.elevation(unit.position)
                        / if arena == Arena::Citadel { 3.8 } else { 1.7 },
                    arena.cover(unit.position),
                    arena.roughness(unit.position),
                    (d / 20.0).min(1.0),
                    f32::from(enemy.as_ref().is_some_and(|e| e.visible)),
                    unit.contacts.len() as f32 / team_size as f32,
                    enemy.as_ref().map(|e| e.health / 100.0).unwrap_or(0.0),
                    enemy
                        .as_ref()
                        .map(|e| e.confidence(self.tick))
                        .unwrap_or(0.0),
                    allies.len() as f32 / (team_size - 1) as f32,
                    isolation,
                    local_threat,
                    support
                        .map(|(_, urgency)| (urgency / 2.0).min(1.0))
                        .unwrap_or(0.0),
                    engaged as f32 / (team_size - 1) as f32,
                    f32::from(distance(unit.position, ridge) < 0.9),
                    f32::from(self.tick < 180),
                    f32::from(self.tick >= old_flank_until),
                    f32::from(enemy.is_some()),
                    old_flank_anchor
                        .map(|p| (distance(unit.position, p) / 10.0).min(1.0))
                        .unwrap_or(1.0),
                ];
                let utility_action = unit
                    .scores
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .unwrap()
                    .0;
                let mask = std::array::from_fn(|i| unit.scores[i] >= 0.0);
                let prediction = self
                    .policy
                    .as_ref()
                    .and_then(|model| model.predict(&decision_features, &mask));
                let learned_action = prediction
                    .map(|p| p.0)
                    .filter(|i| *i < ACTION_COUNT && mask[*i]);
                let active_controller = self
                    .learned_team
                    .map(|team| {
                        if unit.team == team {
                            Controller::Learned
                        } else {
                            Controller::Utility
                        }
                    })
                    .unwrap_or(self.controller);
                unit.action = if active_controller == Controller::Learned {
                    if let Some(action) = learned_action {
                        action
                    } else {
                        self.learned_fallbacks += 1;
                        utility_action
                    }
                } else {
                    utility_action
                };
                let comparison = Comparison {
                    tick: self.tick,
                    unit: unit.id,
                    utility: utility_action,
                    learned: learned_action,
                    executed: unit.action,
                    confidence: prediction.map(|p| p.1),
                    controller: active_controller,
                    disagreed: learned_action.is_some_and(|a| a != utility_action),
                };
                self.comparison_summary.record(&comparison);
                if comparison.disagreed {
                    self.disagreements.push(comparison.clone());
                }
                unit.comparison = Some(comparison);
                unit.decision_counts[unit.action] += 1;
                if unit.action == 3 && previous_action != 3 {
                    unit.flank_anchor = enemy
                        .as_ref()
                        .map(|e| cognition::flank_position(arena, unit, e));
                    unit.flank_until = self.tick + 60;
                } else if unit.action != 3 {
                    unit.flank_anchor = None;
                }
                unit.decision_reason = match unit.action {
                    0 => {
                        if arena == Arena::Citadel && enemy.is_none() && unit.search_waypoint > 0 {
                            "Scout patrol after deployment search"
                        } else if enemy.is_some() {
                            "Advance on observed/reported contact"
                        } else {
                            "Search opposing deployment zone"
                        }
                    }
                    1 => "Current sighting inside weapon range",
                    2 => "Caution, discipline and patience favor ridge",
                    3 => "Ally engages target; approach another angle",
                    4 => "Teamwork favors reducing ally separation",
                    5 => "Injury or close threat favors withdrawal",
                    6 => "Patience favors holding ridge position",
                    7 => "Exposure and self-preservation favor cover",
                    _ => "Known ally injury or pressure favors support",
                }
                .into();
                if active_controller == Controller::Learned && learned_action.is_some() {
                    unit.decision_reason = "Learned proposal; scores show utility baseline".into();
                }
            }
            unit.enemy = enemy.as_ref().map(|e| e.enemy);
            unit.support_target = if unit.action == 8 {
                support.map(|(ally, _)| ally.id)
            } else {
                None
            };
            unit.target = match unit.action {
                2 => ridge,
                3 => unit.flank_anchor.unwrap_or(enemy_position),
                4 => center,
                5 => [
                    (unit.position[0] + (unit.position[0] - enemy_position[0]))
                        .clamp(-arena.half_size()[0] + 0.5, arena.half_size()[0] - 0.5),
                    (unit.position[1] + (unit.position[1] - enemy_position[1]))
                        .clamp(-arena.half_size()[1] + 1.0, arena.half_size()[1] - 1.0),
                ],
                6 => unit.position,
                7 => arena.nearest_cover(unit.position).unwrap_or(unit.position),
                8 => support
                    .map(|(ally, _)| cognition::support_position(arena, unit, ally))
                    .unwrap_or(unit.position),
                _ => enemy_position,
            };
            if self.tick.is_multiple_of(5)
                && let Some(sink) = records.as_deref_mut()
            {
                sink.push(telemetry::DecisionRecord::capture(
                    self.tick,
                    unit,
                    decision_features.clone(),
                ));
            }
            if unit.action == 1
                && enemy.as_ref().is_some_and(|e| e.visible)
                && d < unit.role.range()
                && arena.clear_segment(unit.position, enemy_position)
                && unit.cooldown <= 0.0
            {
                let advantage = 1.0
                    + 0.1
                        * (arena.elevation(unit.position) - arena.elevation(enemy_position))
                            .max(0.0);
                if unit.role == Role::RockThrower {
                    self.rocks.push(Rock {
                        shooter: unit.id,
                        team: unit.team,
                        from: unit.position,
                        to: enemy_position,
                        age: 0,
                        flight_ticks: (d / 10.0 / DT).ceil().max(1.0) as u32,
                        damage: 7.0 * unit.damage_multiplier() * advantage,
                    });
                    unit.rocks_thrown += 1;
                    unit.cooldown = 1.6;
                } else {
                    let dealt = 10.0
                        * unit.damage_multiplier()
                        * advantage
                        * (1.0 - 0.3 * arena.cover(enemy_position));
                    damage[enemy.as_ref().unwrap().enemy] += dealt;
                    unit.damage_dealt += dealt;
                    unit.cooldown = 0.8;
                }
            }
            let stationary = unit.action == 6
                || (unit.action == 1
                    && enemy.as_ref().is_some_and(|e| e.visible)
                    && d < unit.role.range()
                    && arena.clear_segment(unit.position, enemy_position));
            if stationary {
                unit.route = Route::default();
                unit.path_cursor = 0;
            } else {
                let expired = self.tick.is_multiple_of(15)
                    || unit.route.points.is_empty()
                    || distance(unit.route_goal, unit.target) > 1.0
                    || unit.route_mind != unit.mind;
                if expired {
                    unit.route =
                        plan_known(arena, unit.position, unit.target, unit.mind, &known_threats);
                    unit.path_cursor = 1;
                    unit.route_goal = unit.target;
                    unit.route_mind = unit.mind;
                }
                while unit.path_cursor < unit.route.points.len()
                    && distance(unit.position, unit.route.points[unit.path_cursor]) < 0.18
                {
                    unit.path_cursor += 1;
                }
                if let Some(waypoint) = unit.route.points.get(unit.path_cursor).copied() {
                    let remaining = distance(unit.position, waypoint);
                    if remaining > 0.001 {
                        let uphill =
                            (arena.elevation(waypoint) - arena.elevation(unit.position)).max(0.0);
                        let speed =
                            2.2 / (1.0 + uphill * 0.35 + arena.roughness(unit.position) * 1.0);
                        let amount = (speed * DT).min(remaining);
                        let mut next = [
                            unit.position[0]
                                + (waypoint[0] - unit.position[0]) / remaining * amount,
                            unit.position[1]
                                + (waypoint[1] - unit.position[1]) / remaining * amount,
                        ];
                        for ally in &allies {
                            let gap = distance(next, ally.position);
                            if gap > 0.001 && gap < 0.65 {
                                let separated = [
                                    next[0] + (next[0] - ally.position[0]) / gap * 0.045,
                                    next[1] + (next[1] - ally.position[1]) / gap * 0.045,
                                ];
                                if arena.walk_segment(unit.position, separated) {
                                    next = separated;
                                }
                            }
                        }
                        next[0] =
                            next[0].clamp(-arena.half_size()[0] + 0.5, arena.half_size()[0] - 0.5);
                        next[1] =
                            next[1].clamp(-arena.half_size()[1] + 0.5, arena.half_size()[1] - 0.5);
                        if arena.walk_segment(unit.position, next) {
                            unit.distance_moved += distance(unit.position, next);
                            unit.position = next;
                        }
                    }
                }
            }
        }
        // Simultaneous damage avoids giving the first team update-order immunity.
        for unit in &mut self.units {
            unit.health = (unit.health - damage[unit.id]).max(0.0);
            if unit.health <= 0.0 {
                unit.visible_enemies = 0;
                for c in &mut unit.contacts {
                    c.visible = false;
                }
            }
        }
        for team in 0..2 {
            let alive: Vec<_> = self
                .units
                .iter()
                .filter(|u| u.team == team && u.health > 0.0)
                .collect();
            let mut sum = 0.0;
            let mut pairs = 0;
            for (i, a) in alive.iter().enumerate() {
                for b in alive.iter().skip(i + 1) {
                    sum += distance(a.position, b.position) as f64;
                    pairs += 1;
                }
            }
            if pairs > 0 {
                self.ally_distance_sum[team] += sum / pairs as f64;
                self.ally_distance_samples[team] += 1;
            }
        }
        self.tick += 1;
        let a = self.survivors(0);
        let b = self.survivors(1);
        if ((a == 0 || b == 0) && self.rocks.is_empty()) || self.tick >= 1200 {
            self.finished = true;
            self.result = if a == 0 && b == 0 {
                "Draw"
            } else if a == 0 {
                "Blue wins"
            } else if b == 0 {
                "Red wins"
            } else {
                "Timeout / draw"
            }
            .into();
        } else {
            self.result = "Battle in progress".into();
        }
    }
}
