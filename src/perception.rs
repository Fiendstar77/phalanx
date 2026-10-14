//! Observations are sampled from reality here. Decisions receive contacts, never hidden transforms.
use crate::{Mind, Role, Unit, distance, navigation::Threat, terrain::Arena};
use serde::Serialize;
pub const MEMORY_TICKS: u32 = 40;
pub const COMMUNICATION_RANGE: f32 = 8.0;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Source {
    OwnSight,
    Ally(usize),
}
#[derive(Clone, Debug, Serialize)]
pub struct Contact {
    pub enemy: usize,
    pub position: [f32; 2],
    pub health: f32,
    pub role: Role,
    pub seen_tick: u32,
    pub source: Source,
    pub visible: bool,
}
impl Contact {
    pub fn confidence(&self, tick: u32) -> f32 {
        (1.0 - tick.saturating_sub(self.seen_tick) as f32 / (MEMORY_TICKS + 1) as f32).max(0.0)
    }
}
pub fn cooperative(mind: Mind) -> bool {
    (mind.teamwork + mind.discipline) * 0.5 >= 0.5
}
pub fn can_see(arena: Arena, observer: [f32; 2], target: [f32; 2]) -> bool {
    let sight = 10.0 + arena.elevation(observer) * 1.5 - arena.cover(target) * 2.0;
    if distance(observer, target) > sight || !arena.clear_segment(observer, target) {
        return false;
    }
    let from = arena.elevation(observer) + 0.95;
    let to = arena.elevation(target) + 0.95;
    let steps = (distance(observer, target) / 0.25).ceil().max(1.0) as usize;
    (1..steps).all(|i| {
        let t = i as f32 / steps as f32;
        let p = [
            observer[0] + (target[0] - observer[0]) * t,
            observer[1] + (target[1] - observer[1]) * t,
        ];
        arena.elevation(p) < from + (to - from) * t
    })
}
/// Direct reports are collected simultaneously before sharing. Memories never echo each other.
pub fn update(units: &mut [Unit], arena: Arena, tick: u32) {
    let old = units.to_vec();
    let direct: Vec<Vec<Contact>> = old
        .iter()
        .map(|u| {
            if u.health <= 0.0 {
                return Vec::new();
            }
            old.iter()
                .filter(|e| {
                    e.team != u.team && e.health > 0.0 && can_see(arena, u.position, e.position)
                })
                .map(|e| Contact {
                    enemy: e.id,
                    position: e.position,
                    health: e.health,
                    role: e.role,
                    seen_tick: tick,
                    source: Source::OwnSight,
                    visible: true,
                })
                .collect()
        })
        .collect();
    for unit in units {
        if unit.health <= 0.0 {
            unit.contacts.clear();
            unit.visible_enemies = 0;
            continue;
        }
        let mut contacts = old[unit.id].contacts.clone();
        contacts.retain(|c| {
            if tick.saturating_sub(c.seen_tick) > MEMORY_TICKS {
                return false;
            }
            // Observed bodies and an inspected empty last-known location clear stale beliefs.
            let observed_dead = old.iter().any(|e| {
                e.id == c.enemy && e.health <= 0.0 && can_see(arena, unit.position, e.position)
            });
            let searched = distance(unit.position, c.position) < 1.2
                && can_see(arena, unit.position, c.position)
                && !direct[unit.id].iter().any(|v| v.enemy == c.enemy);
            !observed_dead && !searched
        });
        for c in &mut contacts {
            c.visible = false;
        }
        if cooperative(unit.mind) {
            for ally in &old {
                if ally.id == unit.id
                    || ally.team != unit.team
                    || ally.health <= 0.0
                    || !cooperative(ally.mind)
                    || distance(unit.position, ally.position) > COMMUNICATION_RANGE
                {
                    continue;
                }
                for sighting in &direct[ally.id] {
                    // A direct current observation always wins over a shared report.
                    if direct[unit.id].iter().any(|c| c.enemy == sighting.enemy) {
                        continue;
                    }
                    let mut report = sighting.clone();
                    report.source = Source::Ally(ally.id);
                    report.visible = false;
                    if let Some(existing) = contacts.iter_mut().find(|c| c.enemy == report.enemy) {
                        if existing.seen_tick < report.seen_tick {
                            *existing = report;
                            unit.shared_reports += 1;
                        }
                    } else {
                        contacts.push(report);
                        unit.shared_reports += 1;
                    }
                }
            }
        }
        for sighting in &direct[unit.id] {
            if let Some(existing) = contacts.iter_mut().find(|c| c.enemy == sighting.enemy) {
                *existing = sighting.clone();
            } else {
                contacts.push(sighting.clone());
            }
        }
        contacts.sort_by_key(|c| c.enemy);
        unit.visible_enemies = direct[unit.id].len();
        unit.contacts = contacts;
    }
}
pub fn threats(unit: &Unit, tick: u32) -> Vec<Threat> {
    unit.contacts
        .iter()
        .map(|c| Threat {
            position: c.position,
            range: c.role.range(),
            confidence: c.confidence(tick),
        })
        .collect()
}
