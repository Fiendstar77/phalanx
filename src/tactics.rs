//! Deliberate campaign opening from each fighter's own contacts and nearby allies.
//! No enemy world transforms or unseen health are consulted.
use crate::{FighterClass, Unit, distance, terrain::Arena};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Phase {
    #[default]
    Reconnaissance,
    Assemble,
    Engage,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct State {
    pub scout: Option<usize>,
    pub first_report: Option<u32>,
    pub phase: Phase,
}
pub fn is_blank(state: &State) -> bool {
    state.scout.is_none()
}
pub struct Intent {
    pub goal: [f32; 2],
    pub reason: String,
}

fn scout_value(u: &Unit) -> f32 {
    (if matches!(u.class, Some(FighterClass::Scout | FighterClass::Shinobi)) {
        4.
    } else {
        0.
    }) + u
        .theus
        .as_ref()
        .map_or(0., |a| f32::from(a.sheet.rating(3, 2)) * 0.1)
        + u.movement_multiplier()
        + u.mind.caution * 0.4
        + u.mind.patience * 0.3
        - (if u.has_skill(FighterClass::Healer) {
            2.
        } else {
            0.
        })
        - u.flesh.soreness() * 0.2
}
fn legal_goal(arena: Arena, desired: [f32; 2]) -> [f32; 2] {
    let half = arena.half_size();
    let clamp = |p: [f32; 2]| {
        [
            p[0].clamp(-half[0] + 0.8, half[0] - 0.8),
            p[1].clamp(-half[1] + 0.8, half[1] - 0.8),
        ]
    };
    let wanted = clamp(desired);
    if let Some(cover) = arena.nearest_cover(wanted)
        && distance(wanted, cover) < 2.5
        && !arena.blocked(cover)
    {
        return cover;
    }
    for radius in 0..8 {
        for offset in [
            [0., 1.],
            [0., -1.],
            [1., 0.],
            [-1., 0.],
            [1., 1.],
            [-1., -1.],
        ] {
            let p = clamp([
                wanted[0] + offset[0] * radius as f32,
                wanted[1] + offset[1] * radius as f32,
            ]);
            if !arena.blocked(p) {
                return p;
            }
        }
    }
    wanted
}

pub(crate) fn intent(
    unit: &mut Unit,
    allies: &[&Unit],
    arena: Arena,
    tick: u32,
    team_size: usize,
) -> Option<Intent> {
    if unit.theus.is_none() || unit.tactics.phase == Phase::Engage {
        return None;
    }
    if unit.tactics.scout.is_none() {
        unit.tactics.scout = std::iter::once(&*unit)
            .chain(allies.iter().copied())
            .max_by(|a, b| {
                scout_value(a)
                    .total_cmp(&scout_value(b))
                    .then(b.id.cmp(&a.id))
            })
            .map(|u| u.id);
    }
    if unit.tactics.first_report.is_none() && !unit.contacts.is_empty() {
        unit.tactics.first_report = Some(tick);
    }
    let deadline = unit
        .tactics
        .first_report
        .map_or(240 + (unit.mind.patience * 80.) as u32, |first| {
            first.saturating_add(55).max(85)
        });
    if tick >= deadline {
        unit.tactics.phase = Phase::Engage;
        return None;
    }
    let close = unit.contacts.iter().any(|c| {
        c.visible && distance(unit.position, c.position) < if unit.is_ranged() { 7.0 } else { 3.8 }
    });
    if close
        || unit.health_fraction() < 0.90
        || unit.flesh.bleeding()
        || allies.iter().any(|a| a.health_fraction() < 0.55)
    {
        return None;
    }
    unit.tactics.phase = if unit.tactics.first_report.is_some() {
        Phase::Assemble
    } else {
        Phase::Reconnaissance
    };
    let half = arena.half_size();
    let side = if unit.team == 0 { -1. } else { 1. };
    let scouting = unit.tactics.scout == Some(unit.id);
    let goal = if scouting {
        if let Some(contact) = unit.contacts.iter().filter(|c| c.visible).min_by(|a, b| {
            distance(unit.position, a.position).total_cmp(&distance(unit.position, b.position))
        }) {
            let d = distance(unit.position, contact.position).max(0.01);
            let desired = [
                contact.position[0] + (unit.position[0] - contact.position[0]) / d * 6.5,
                contact.position[1] + (unit.position[1] - contact.position[1]) / d * 6.5,
            ];
            let covered = legal_goal(arena, desired);
            if distance(covered, contact.position) >= 5.5 {
                covered
            } else if !arena.blocked(desired) {
                desired
            } else {
                unit.position
            }
        } else {
            // Sweep two lanes and the centre, using geography rather than a live
            // opponent location. Keep one appointed scout when radio range breaks.
            let lane = ((tick / 90 + (unit.id % team_size.max(1)) as u32) % 3) as f32 - 1.;
            legal_goal(arena, [side * half[0] * 0.10, lane * half[1] * 0.45])
        }
    } else {
        let slot = allies.iter().filter(|a| a.id < unit.id).count() as f32;
        let x = if unit.tactics.phase == Phase::Assemble {
            0.40
        } else {
            0.62
        };
        legal_goal(
            arena,
            [
                side * half[0] * x,
                ((slot - 1.) * 1.8).clamp(-half[1] * 0.5, half[1] * 0.5),
            ],
        )
    };
    let reason = if scouting {
        if unit.tactics.phase == Phase::Assemble {
            "Recon: maintain sight while the formation assembles"
        } else {
            "Recon: scout a lane and report contacts before commitment"
        }
    } else if unit.tactics.phase == Phase::Assemble {
        "Strategy: report received; assemble the approach before committing"
    } else if unit.has_skill(FighterClass::Healer) {
        "Recon: medic stages behind the scout and screen"
    } else {
        "Recon: stage the formation while the appointed scout searches"
    };
    Some(Intent {
        goal,
        reason: reason.into(),
    })
}
