//! Fixed-step combat model, independent of rendering and wall-clock time.
use serde::Serialize;

pub const DT: f32 = 0.1;
pub const ACTIONS: [&str; 7] = [
    "ADVANCE",
    "ATTACK",
    "HIGH GROUND",
    "FLANK",
    "REGROUP",
    "RETREAT",
    "HOLD",
];

#[derive(Clone, Copy, Debug, Serialize)]
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
#[derive(Clone, Debug, Serialize)]
pub struct Unit {
    pub id: usize,
    pub team: usize,
    pub position: [f32; 2],
    pub health: f32,
    pub mind: Mind,
    pub action: usize,
    pub scores: [f32; 7],
    pub target: [f32; 2],
    pub enemy: Option<usize>,
    pub cooldown: f32,
    pub damage_dealt: f32,
    pub distance_moved: f32,
    pub decision_counts: [u32; 7],
}
#[derive(Clone, Debug, Serialize)]
pub struct Battle {
    pub seed: u64,
    pub tick: u32,
    pub units: Vec<Unit>,
    pub finished: bool,
    pub result: String,
    pub swapped: bool,
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
        ^ ((id * 7 + action) as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((n ^ (n >> 31)) as u32 as f32 / u32::MAX as f32 - 0.5) * 0.035
}
impl Battle {
    pub fn new(seed: u64, swapped: bool) -> Self {
        let units = (0..10)
            .map(|id| {
                let team = id / 5;
                let aggressive = (team == 0) != swapped;
                let position = [
                    if team == 0 { -8.0 } else { 8.0 },
                    (id % 5) as f32 * 1.6 - 3.2,
                ];
                Unit {
                    id,
                    team,
                    position,
                    health: 100.0,
                    mind: if aggressive {
                        Mind::aggressive()
                    } else {
                        Mind::disciplined()
                    },
                    action: 6,
                    scores: [0.0; 7],
                    target: position,
                    enemy: None,
                    cooldown: 0.0,
                    damage_dealt: 0.0,
                    distance_moved: 0.0,
                    decision_counts: [0; 7],
                }
            })
            .collect();
        Self {
            seed,
            tick: 0,
            units,
            finished: false,
            result: "Ready".into(),
            swapped,
        }
    }
    pub fn survivors(&self, team: usize) -> usize {
        self.units
            .iter()
            .filter(|u| u.team == team && u.health > 0.0)
            .count()
    }
    pub fn step(&mut self) {
        if self.finished {
            return;
        }
        let snapshot = self.units.clone();
        let mut damage = [0.0; 10];
        for unit in &mut self.units {
            if unit.health <= 0.0 {
                continue;
            }
            unit.cooldown = (unit.cooldown - DT).max(0.0);
            let Some(enemy) = snapshot
                .iter()
                .filter(|e| e.team != unit.team && e.health > 0.0)
                .min_by(|a, b| {
                    distance(unit.position, a.position)
                        .total_cmp(&distance(unit.position, b.position))
                })
            else {
                continue;
            };
            let d = distance(unit.position, enemy.position);
            let allies: Vec<_> = snapshot
                .iter()
                .filter(|a| a.team == unit.team && a.id != unit.id && a.health > 0.0)
                .collect();
            let center = if allies.is_empty() {
                unit.position
            } else {
                [
                    allies.iter().map(|a| a.position[0]).sum::<f32>() / allies.len() as f32,
                    allies.iter().map(|a| a.position[1]).sum::<f32>() / allies.len() as f32,
                ]
            };
            let isolation = (distance(unit.position, center) / 5.0).min(1.0);
            let injury = 1.0 - unit.health / 100.0;
            let local_threat = (snapshot
                .iter()
                .filter(|e| {
                    e.team != unit.team
                        && e.health > 0.0
                        && distance(unit.position, e.position) < 3.5
                })
                .count() as f32
                / 3.0)
                .min(1.0);
            let ridge = [
                if unit.team == 0 { -0.6 } else { 0.6 },
                unit.position[1].clamp(-2.0, 2.0),
            ];
            if self.tick.is_multiple_of(5) {
                let m = unit.mind;
                let in_range = d < 1.5;
                let arrived = distance(unit.position, ridge) < 0.9;
                unit.scores = [
                    0.25 + 0.65 * m.aggression + 0.2 * m.courage - 0.25 * m.caution * local_threat,
                    if in_range {
                        0.85 + 0.4 * m.aggression + 0.2 * m.courage
                            - 0.15 * injury * m.self_preservation
                    } else {
                        -1.0
                    },
                    if arrived || in_range {
                        -1.0
                    } else {
                        0.25 + 0.35 * m.caution + 0.25 * m.discipline + 0.2 * m.patience
                            - 0.015 * distance(unit.position, ridge)
                    },
                    if d > 2.0 && d < 7.0 {
                        0.2 + 0.25 * m.aggression + 0.25 * m.discipline + 0.2 * m.patience
                    } else {
                        -1.0
                    },
                    if isolation > 0.4 {
                        0.1 + 0.75 * m.teamwork * isolation + 0.2 * m.discipline
                            - 0.15 * m.aggression
                    } else {
                        -1.0
                    },
                    if injury > 0.55 {
                        0.15 + injury * (0.65 * m.self_preservation + 0.55 * m.caution)
                            + 0.25 * local_threat
                            - 0.2 * m.courage
                    } else {
                        -1.0
                    },
                    if arrived && d > 3.0 && self.tick < 180 {
                        0.35 + 0.35 * m.patience + 0.2 * m.discipline
                    } else {
                        -1.0
                    },
                ];
                for (i, score) in unit.scores.iter_mut().enumerate() {
                    if *score >= 0.0 {
                        *score += noise(self.seed, self.tick, unit.id, i);
                    }
                }
                unit.action = unit
                    .scores
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .unwrap()
                    .0;
                unit.decision_counts[unit.action] += 1;
            }
            unit.enemy = Some(enemy.id);
            unit.target = match unit.action {
                2 => ridge,
                3 => [
                    enemy.position[0],
                    (enemy.position[1] + if unit.id.is_multiple_of(2) { 2.8 } else { -2.8 })
                        .clamp(-6.0, 6.0),
                ],
                4 => center,
                5 => [
                    (unit.position[0] + (unit.position[0] - enemy.position[0])).clamp(-10.5, 10.5),
                    (unit.position[1] + (unit.position[1] - enemy.position[1])).clamp(-7.0, 7.0),
                ],
                6 => unit.position,
                _ => enemy.position,
            };
            if unit.action == 1 && d < 1.5 && unit.cooldown <= 0.0 {
                // Identical base damage; positional advantage belongs to terrain.
                let dealt = 10.0
                    * (1.0 + 0.1 * (elevation(unit.position) - elevation(enemy.position)).max(0.0));
                damage[enemy.id] += dealt;
                unit.damage_dealt += dealt;
                unit.cooldown = 0.8;
            } else if unit.action != 6 && (unit.action != 1 || d > 1.15) {
                let remaining = distance(unit.position, unit.target);
                if remaining > 0.02 {
                    let uphill = (elevation(unit.target) - elevation(unit.position)).max(0.0);
                    let speed = 2.2 / (1.0 + uphill * 0.35);
                    let amount = (speed * DT).min(remaining);
                    let mut next = [
                        unit.position[0] + (unit.target[0] - unit.position[0]) / remaining * amount,
                        unit.position[1] + (unit.target[1] - unit.position[1]) / remaining * amount,
                    ];
                    // Mild separation keeps living allies readable without a scripted formation.
                    for ally in &allies {
                        let gap = distance(next, ally.position);
                        if gap > 0.001 && gap < 0.65 {
                            next[0] += (next[0] - ally.position[0]) / gap * 0.045;
                            next[1] += (next[1] - ally.position[1]) / gap * 0.045;
                        }
                    }
                    next[0] = next[0].clamp(-10.5, 10.5);
                    next[1] = next[1].clamp(-7.0, 7.0);
                    unit.distance_moved += distance(unit.position, next);
                    unit.position = next;
                }
            }
        }
        // Simultaneous damage avoids giving the first team update-order immunity.
        for unit in &mut self.units {
            unit.health = (unit.health - damage[unit.id]).max(0.0);
        }
        self.tick += 1;
        let a = self.survivors(0);
        let b = self.survivors(1);
        if a == 0 || b == 0 || self.tick >= 1200 {
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
