//! Deterministic swept-disc contact resolution for living fighters, on both teams.
//! Contacts are physical constraints, independent of perception and tactical knowledge.
use crate::{Unit, distance, terrain::Arena};
use std::collections::{BTreeMap, BTreeSet};
pub const BODY_RADIUS: f32 = 0.45;
pub const BODY_DIAMETER: f32 = BODY_RADIUS * 2.0;
const GAP: f32 = BODY_DIAMETER + 0.002;
#[derive(Clone, Debug)]
pub struct Frame {
    pub time: f32,
    pub positions: Vec<[f32; 2]>,
}
#[derive(Clone, Debug, Default)]
pub struct Contact {
    pub a: usize,
    pub b: usize,
    pub point: [f32; 2],
    pub velocity: [f32; 2],
    pub speed: f32,
}
#[derive(Clone, Debug, Default)]
pub struct Motion {
    pub impacts: Vec<Contact>,
    pub frames: Vec<Frame>,
    pub contacts: u32,
}
impl Motion {
    pub fn position(&self, id: usize, time: f32) -> Option<[f32; 2]> {
        let end = self.frames.partition_point(|f| f.time <= time);
        let a = self.frames.get(end.saturating_sub(1))?;
        let Some(b) = self.frames.get(end) else {
            return a.positions.get(id).copied();
        };
        let t = ((time - a.time) / (b.time - a.time).max(1e-7)).clamp(0.0, 1.0);
        Some(add(
            a.positions[id],
            mul(sub(b.positions[id], a.positions[id]), t),
        ))
    }
    fn record(&mut self, time: f32, positions: &[[f32; 2]]) {
        if self
            .frames
            .last()
            .is_some_and(|f| (f.time - time).abs() < 1e-7)
        {
            return;
        }
        self.frames.push(Frame {
            time,
            positions: positions.to_vec(),
        });
    }
}
fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}
fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn mul(a: [f32; 2], k: f32) -> [f32; 2] {
    [a[0] * k, a[1] * k]
}
fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}
fn length(a: [f32; 2]) -> f32 {
    dot(a, a).sqrt()
}
fn clipped(arena: Arena, p: [f32; 2], v: [f32; 2]) -> [f32; 2] {
    if length(v) < 1e-7 {
        return [0., 0.];
    }
    if arena.walk_segment(p, add(p, v)) {
        return v;
    }
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..12 {
        let mid = (lo + hi) * 0.5;
        if arena.walk_segment(p, add(p, mul(v, mid))) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    mul(v, (lo - 0.001).max(0.0))
}
fn unstack(arena: Arena, p: &mut [[f32; 2]], alive: &[bool]) {
    for _ in 0..160 {
        let mut worst = 0.0_f32;
        for i in 0..p.len() {
            for j in i + 1..p.len() {
                if !alive[i] || !alive[j] {
                    continue;
                }
                let delta = sub(p[j], p[i]);
                let d = length(delta);
                if d >= GAP - 0.00002 {
                    continue;
                }
                worst = worst.max(GAP - d);
                let normal = if d > 1e-6 {
                    mul(delta, 1.0 / d)
                } else {
                    let angle = ((i * 7 + j * 13) % 17) as f32 * 0.3696;
                    [angle.cos(), angle.sin()]
                };
                let correction = GAP - d;
                let a = clipped(arena, p[i], mul(normal, -correction * 0.5));
                let b = clipped(arena, p[j], mul(normal, correction * 0.5));
                p[i] = add(p[i], a);
                p[j] = add(p[j], b);
                let missing = (correction - length(a) - length(b)).max(0.0);
                if missing > 1e-6 {
                    let extra = clipped(arena, p[i], mul(normal, -missing));
                    p[i] = add(p[i], extra);
                    p[j] = add(
                        p[j],
                        clipped(arena, p[j], mul(normal, (missing - length(extra)).max(0.0))),
                    );
                }
            }
        }
        if worst < 0.00003 {
            break;
        }
    }
}
/// Time of first contact during two linear paths; testing endpoints alone permits swaps.
fn contact(p: [f32; 2], q: [f32; 2], v: [f32; 2], w: [f32; 2]) -> Option<f32> {
    let delta = sub(q, p);
    let relative = sub(w, v);
    let a = dot(relative, relative);
    let b = dot(delta, relative);
    if a < 1e-10 || b >= -1e-5 {
        return None;
    }
    let c = dot(delta, delta) - GAP * GAP;
    if c <= 1e-6 {
        return Some(0.0);
    }
    let discriminant = b * b - a * c;
    if discriminant < 0.0 {
        return None;
    }
    let t = (-b - discriminant.sqrt()) / a;
    (t >= 0.0 && t < 1.0).then_some(t)
}
fn candidate_pairs(
    p: &[[f32; 2]],
    movement: &[[f32; 2]],
    alive: &[bool],
) -> BTreeSet<(usize, usize)> {
    let mut cells: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
    for i in 0..p.len() {
        if !alive[i] {
            continue;
        }
        let end = add(p[i], movement[i]);
        let lo = std::array::from_fn::<_, 2, _>(|axis| {
            ((p[i][axis].min(end[axis]) - BODY_RADIUS) / 2.0).floor() as i32
        });
        let hi = std::array::from_fn::<_, 2, _>(|axis| {
            ((p[i][axis].max(end[axis]) + BODY_RADIUS) / 2.0).floor() as i32
        });
        for x in lo[0]..=hi[0] {
            for z in lo[1]..=hi[1] {
                cells.entry((x, z)).or_default().push(i);
            }
        }
    }
    let mut pairs = BTreeSet::new();
    for bodies in cells.values() {
        for (at, &a) in bodies.iter().enumerate() {
            for &b in &bodies[at + 1..] {
                pairs.insert((a.min(b), a.max(b)));
            }
        }
    }
    pairs
}
pub fn resolve(arena: Arena, units: &mut [Unit], before: &[Unit]) -> Motion {
    let alive = before.iter().map(|u| u.health > 0.0).collect::<Vec<_>>();
    let original = before.iter().map(|u| u.position).collect::<Vec<_>>();
    let mut p = original.clone();
    let mut intended = units
        .iter()
        .zip(before)
        .map(|(a, b)| sub(a.position, b.position))
        .collect::<Vec<_>>();
    let attempted = intended.iter().map(|v| length(*v)).collect::<Vec<_>>();
    unstack(arena, &mut p, &alive);
    // Local tangential steering lets crowds flow around contacts instead of freezing nose-to-nose.
    for i in 0..p.len() {
        if !alive[i] || attempted[i] < 1e-6 {
            continue;
        }
        let forward = mul(intended[i], 1.0 / attempted[i]);
        let sideways = [-forward[1], forward[0]];
        let mut turn = 0.0;
        for j in 0..p.len() {
            if i == j || !alive[j] {
                continue;
            }
            let delta = sub(p[j], p[i]);
            let ahead = dot(delta, forward);
            let lateral = dot(delta, sideways);
            if ahead > 0.0
                && ahead < 1.65
                && lateral.abs() < GAP + 0.1
                && arena.clear_segment(p[i], p[j])
            {
                let side = if lateral > 0.03 { -1.0 } else { 1.0 };
                turn += side
                    * (1.0 - lateral.abs() / (GAP + 0.1))
                    * (1.0 - ((ahead - GAP) / 0.75).max(0.0));
            }
        }
        let steered = add(
            intended[i],
            mul(sideways, attempted[i] * 0.8 * turn.clamp(-1.0, 1.0)),
        );
        let candidate = mul(steered, attempted[i] / length(steered).max(1e-7));
        // Steering must not turn a valid ramp route into a permanent push against its ledge.
        // Keep the original path direction when the lateral detour loses terrain clearance.
        if arena.walk_segment(p[i], add(p[i], candidate)) {
            intended[i] = candidate;
        }
    }
    let steps = (attempted.iter().copied().fold(0.0_f32, f32::max) / 0.15)
        .ceil()
        .max(1.0) as usize;
    let mut traveled = vec![0.0; p.len()];
    let mut motion = Motion::default();
    motion.record(0.0, &p);
    for step in 0..steps {
        let mut remaining = intended
            .iter()
            .map(|v| mul(*v, 1.0 / steps as f32))
            .collect::<Vec<_>>();
        let mut fraction = 0.0;
        for _ in 0..256 {
            for i in 0..p.len() {
                remaining[i] = if alive[i] {
                    clipped(arena, p[i], remaining[i])
                } else {
                    [0., 0.]
                };
            }
            let mut hit = None;
            let mut earliest = 1.0;
            for (i, j) in candidate_pairs(&p, &remaining, &alive) {
                if let Some(t) = contact(p[i], p[j], remaining[i], remaining[j])
                    && t < earliest
                {
                    earliest = t;
                    hit = Some((i, j));
                }
            }
            for i in 0..p.len() {
                let movement = mul(remaining[i], earliest);
                p[i] = add(p[i], movement);
                traveled[i] += length(movement);
                remaining[i] = mul(remaining[i], 1.0 - earliest);
            }
            fraction += (1.0 - fraction) * earliest;
            motion.record((step as f32 + fraction) / steps as f32, &p);
            let Some((i, j)) = hit else {
                break;
            };
            motion.contacts += 1;
            let normal = mul(sub(p[j], p[i]), 1.0 / distance(p[i], p[j]).max(1e-6));
            let relative = mul(
                sub(remaining[i], remaining[j]),
                steps as f32 / ((1.0 - fraction).max(1e-5) * crate::DT),
            );
            if !motion.impacts.iter().any(|c| c.a == i && c.b == j) {
                motion.impacts.push(Contact {
                    a: i,
                    b: j,
                    point: add(p[i], mul(normal, BODY_RADIUS)),
                    velocity: relative,
                    speed: dot(relative, normal).max(0.0),
                });
            }
            let closing = dot(sub(remaining[j], remaining[i]), normal);
            if closing < 0.0 {
                let wi: f32 = if attempted[i] < 1e-6 {
                    0.0
                } else if before[i].has_skill(crate::FighterClass::Guardian) {
                    0.35
                } else {
                    1.0
                };
                let wj: f32 = if attempted[j] < 1e-6 {
                    0.0
                } else if before[j].has_skill(crate::FighterClass::Guardian) {
                    0.35
                } else {
                    1.0
                };
                let total = (wi + wj).max(1e-6);
                remaining[i] = add(remaining[i], mul(normal, closing * wi / total));
                remaining[j] = sub(remaining[j], mul(normal, closing * wj / total));
            }
        }
        motion.record((step + 1) as f32 / steps as f32, &p);
    }
    for i in 0..units.len() {
        units[i].position = p[i];
        // Replace attempted path length with accepted physical travel; spawn repair is not locomotion.
        units[i].distance_moved = (units[i].distance_moved - attempted[i]).max(0.0) + traveled[i];
    }
    motion
}
