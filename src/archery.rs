//! Archers reason from their visible contacts and known allies, then submit ordinary movement intents.
use crate::{Unit, collision::BODY_RADIUS, distance, perception::Contact, terrain::Arena};
pub const MIN_RANGE: f32 = 2.2;
pub const GRIP_HEIGHT: f32 = 0.85;
pub const REOPEN_RANGE: f32 = 3.4;
pub fn muzzle(position: [f32; 2], target: [f32; 2]) -> [f32; 2] {
    let d = distance(position, target).max(0.001);
    let f = [(target[0] - position[0]) / d, (target[1] - position[1]) / d];
    [
        position[0] + f[0] * 0.20 + f[1] * 0.06,
        position[1] + f[1] * 0.20 - f[0] * 0.06,
    ]
}
pub fn arrow_origin(position: [f32; 2], target: [f32; 2]) -> [f32; 2] {
    let m = muzzle(position, target);
    let d = distance(position, target).max(0.001);
    [
        m[0] + (target[0] - position[0]) / d * 0.105,
        m[1] + (target[1] - position[1]) / d * 0.105,
    ]
}
pub fn clear_shot(arena: Arena, from: [f32; 2], to: [f32; 2], allies: &[&Unit]) -> bool {
    let d = [to[0] - from[0], to[1] - from[1]];
    let length = d[0] * d[0] + d[1] * d[1];
    arena.clear_segment(from, to)
        && !allies.iter().any(|a| {
            let p = [a.position[0] - from[0], a.position[1] - from[1]];
            let t = (p[0] * d[0] + p[1] * d[1]) / length.max(0.001);
            t > 0.0
                && t < 1.0
                && distance(a.position, [from[0] + d[0] * t, from[1] + d[1] * t])
                    < BODY_RADIUS + 0.12
        })
}
pub struct Plan {
    pub action: usize,
    pub goal: [f32; 2],
    pub reason: &'static str,
}
fn candidates(unit: &Unit, enemy: [f32; 2], allies: &[&Unit]) -> Vec<[f32; 2]> {
    let d = distance(unit.position, enemy).max(0.001);
    let f = [
        (enemy[0] - unit.position[0]) / d,
        (enemy[1] - unit.position[1]) / d,
    ];
    let mut out = vec![unit.position];
    for step in [1.25, 2.5] {
        for (forward, side) in [
            (-1., 0.),
            (-0.7, -0.7),
            (-0.7, 0.7),
            (0., -1.),
            (0., 1.),
            (0.5, -0.8),
            (0.5, 0.8),
        ] {
            out.push([
                unit.position[0] + (f[0] * forward - f[1] * side) * step,
                unit.position[1] + (f[1] * forward + f[0] * side) * step,
            ]);
        }
    }
    for ally in allies.iter().filter(|a| !a.is_ranged()) {
        for side in [-1.3, 1.3] {
            out.push([
                ally.position[0] - f[0] * 1.7 - f[1] * side,
                ally.position[1] - f[1] * 1.7 + f[0] * side,
            ]);
        }
    }
    out
}
pub fn plan(
    arena: Arena,
    unit: &Unit,
    allies: &[&Unit],
    enemy: &Contact,
    tick: u32,
) -> Option<Plan> {
    if !enemy.visible {
        return None;
    }
    let d = distance(unit.position, enemy.position);
    if unit.bow_melee && (d < 1.55 || tick >= unit.archery_retreat_until) {
        return Some(Plan {
            action: if d < 1.5 { 1 } else { 0 },
            goal: enemy.position,
            reason: "Rushed archer draws a sidearm",
        });
    }
    let retreat = d < 4.0 && tick < unit.archery_retreat_until;
    let clear = clear_shot(
        arena,
        muzzle(unit.position, enemy.position),
        enemy.position,
        allies,
    );
    let exposed = allies.iter().any(|a| {
        !a.is_ranged()
            && distance(a.position, unit.position) < 3.0
            && distance(a.position, enemy.position) > d - 0.5
    });
    if !retreat && clear && !exposed {
        return Some(Plan {
            action: if d < unit.attack_range() { 1 } else { 0 },
            goal: enemy.position,
            reason: "Archer has a clear firing lane",
        });
    }
    let goal = candidates(unit, enemy.position, allies)
        .into_iter()
        .filter(|p| {
            distance(*p, unit.position) < 4.5
                && arena.walk_segment(unit.position, *p)
                && !allies
                    .iter()
                    .any(|a| distance(*p, a.position) < BODY_RADIUS * 2. + 0.15)
                && (if retreat {
                    distance(*p, enemy.position) > d + 0.5
                } else {
                    distance(*p, enemy.position) >= MIN_RANGE + 0.4
                })
                && !unit
                    .contacts
                    .iter()
                    .any(|c| c.visible && distance(*p, c.position) < MIN_RANGE)
                && clear_shot(arena, muzzle(*p, enemy.position), enemy.position, allies)
        })
        .min_by(|a, b| {
            let score = |p: [f32; 2]| {
                let range = distance(p, enemy.position);
                let screened = allies.iter().any(|a| {
                    !a.is_ranged()
                        && distance(a.position, enemy.position) + 1.0 < range
                        && distance(p, a.position) < 3.8
                });
                (range - 6.0).abs() * 0.45 + distance(p, unit.position) * 0.35
                    - if screened { 1.8 } else { 0.0 }
            };
            score(*a).total_cmp(&score(*b))
        });
    if let Some(goal) = goal.filter(|p| distance(*p, unit.position) > 0.35) {
        Some(Plan {
            action: if retreat { 5 } else { 0 },
            goal,
            reason: if retreat {
                "Archer makes a short retreat to reopen bow range"
            } else {
                "Archer sidesteps into a screened firing lane"
            },
        })
    } else if clear && d < unit.attack_range() && !unit.bow_melee {
        Some(Plan {
            action: 1,
            goal: enemy.position,
            reason: "Archer commits to a shot; no safer firing lane",
        })
    } else {
        None
    }
}
