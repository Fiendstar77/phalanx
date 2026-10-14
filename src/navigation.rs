//! Deterministic Dijkstra search over a one-unit grid. All edge costs are positive.
use crate::{Mind, Unit, distance, terrain::Arena};
use serde::Serialize;
use std::{cmp::Ordering, collections::BinaryHeap};
const WIDTH: usize = 22;
const HEIGHT: usize = 16;
const COUNT: usize = WIDTH * HEIGHT;
fn point(id: usize) -> [f32; 2] {
    [(id % WIDTH) as f32 - 10.5, (id / WIDTH) as f32 - 7.5]
}
fn nearest(arena: Arena, p: [f32; 2]) -> Option<usize> {
    (0..COUNT)
        .filter(|i| !arena.blocked(point(*i)) && arena.clear_segment(p, point(*i)))
        .min_by(|a, b| distance(p, point(*a)).total_cmp(&distance(p, point(*b))))
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct PathCost {
    pub distance: f32,
    pub terrain: f32,
    pub exposure: f32,
    pub threat: f32,
    pub uphill: f32,
}
impl PathCost {
    pub fn total(self) -> f32 {
        self.distance + self.terrain + self.exposure + self.threat + self.uphill
    }
    fn add(&mut self, other: Self) {
        self.distance += other.distance;
        self.terrain += other.terrain;
        self.exposure += other.exposure;
        self.threat += other.threat;
        self.uphill += other.uphill;
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Route {
    pub points: Vec<[f32; 2]>,
    pub cost: PathCost,
}
#[derive(Clone, Copy)]
struct Entry {
    cost: f32,
    id: usize,
}
impl PartialEq for Entry {
    fn eq(&self, o: &Self) -> bool {
        self.cost == o.cost && self.id == o.id
    }
}
impl Eq for Entry {}
impl Ord for Entry {
    fn cmp(&self, o: &Self) -> Ordering {
        o.cost
            .total_cmp(&self.cost)
            .then_with(|| o.id.cmp(&self.id))
    }
}
impl PartialOrd for Entry {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

pub fn threat_at(arena: Arena, p: [f32; 2], team: usize, units: &[Unit]) -> f32 {
    units
        .iter()
        .filter(|u| u.team != team && u.health > 0.0 && arena.clear_segment(p, u.position))
        .map(|u| (1.0 - distance(p, u.position) / (u.role.range() + 2.0)).max(0.0))
        .sum::<f32>()
        .min(2.0)
}
fn edge(arena: Arena, a: [f32; 2], b: [f32; 2], mind: Mind, threat: f32) -> PathCost {
    let d = distance(a, b);
    PathCost {
        distance: d,
        terrain: d * arena.roughness(b) * 1.4,
        exposure: d * (1.0 - arena.cover(b)) * mind.caution * 1.6,
        threat: d
            * threat
            * (0.2 + mind.caution * 1.6 + mind.self_preservation)
            * (1.0 - arena.cover(b) * 0.45),
        uphill: (arena.elevation(b) - arena.elevation(a)).max(0.0) * (0.4 + mind.patience),
    }
}
pub fn plan(
    arena: Arena,
    start: [f32; 2],
    goal: [f32; 2],
    mind: Mind,
    team: usize,
    units: &[Unit],
) -> Route {
    if arena.blocked(start) || arena.blocked(goal) {
        return Route::default();
    }
    let (Some(s), Some(g)) = (nearest(arena, start), nearest(arena, goal)) else {
        return Route::default();
    };
    let mut costs = [f32::INFINITY; COUNT];
    let mut previous = [usize::MAX; COUNT];
    let mut heap = BinaryHeap::new();
    let threat: Vec<_> = (0..COUNT)
        .map(|i| {
            if arena.blocked(point(i)) {
                0.0
            } else {
                threat_at(arena, point(i), team, units)
            }
        })
        .collect();
    costs[s] = 0.0;
    heap.push(Entry { cost: 0.0, id: s });
    while let Some(Entry { cost, id }) = heap.pop() {
        if cost > costs[id] {
            continue;
        }
        if id == g {
            break;
        }
        let x = (id % WIDTH) as i32;
        let z = (id / WIDTH) as i32;
        for dz in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let nx = x + dx;
                let nz = z + dz;
                if nx < 0 || nx >= WIDTH as i32 || nz < 0 || nz >= HEIGHT as i32 {
                    continue;
                }
                let next = nz as usize * WIDTH + nx as usize;
                if !arena.clear_segment(point(id), point(next)) {
                    continue;
                }
                let candidate =
                    cost + edge(arena, point(id), point(next), mind, threat[next]).total();
                if candidate < costs[next] {
                    costs[next] = candidate;
                    previous[next] = id;
                    heap.push(Entry {
                        cost: candidate,
                        id: next,
                    });
                }
            }
        }
    }
    if !costs[g].is_finite() {
        return Route::default();
    }
    let mut ids = vec![g];
    let mut at = g;
    while at != s {
        at = previous[at];
        ids.push(at);
    }
    ids.reverse();
    let mut points = vec![start];
    let skip = usize::from(ids.len() > 1 && arena.clear_segment(start, point(ids[1])));
    points.extend(ids.iter().skip(skip).map(|i| point(*i)));
    points.push(goal);
    let mut cost = PathCost::default();
    for pair in points.windows(2) {
        cost.add(edge(
            arena,
            pair[0],
            pair[1],
            mind,
            threat_at(arena, pair[1], team, units),
        ));
    }
    Route { points, cost }
}
