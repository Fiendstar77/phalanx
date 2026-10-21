//! Deterministic Dijkstra search over a one-unit grid. All edge costs are positive.
use crate::{Mind, Unit, distance, terrain::Arena};
use serde::Serialize;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    sync::{Arc, Mutex, OnceLock},
};
fn point(arena: Arena, id: usize) -> [f32; 2] {
    let size = arena.half_size();
    let width = (size[0] * 2.0) as usize;
    [
        (id % width) as f32 - size[0] + 0.5,
        (id / width) as f32 - size[1] + 0.5,
    ]
}
/// Static wall-clearance graph: authored terrain does not change within an arena.
/// Neighbor order matches the original dz/dx traversal to preserve deterministic ties.
fn build_graph(arena: Arena) -> Vec<Vec<usize>> {
    let size = arena.half_size();
    let width = (size[0] * 2.0) as usize;
    let height = (size[1] * 2.0) as usize;
    (0..width * height)
        .map(|id| {
            let x = (id % width) as i32;
            let z = (id / width) as i32;
            let mut edges = Vec::new();
            for dz in -1..=1 {
                for dx in -1..=1 {
                    let nx = x + dx;
                    let nz = z + dz;
                    if (dx != 0 || dz != 0)
                        && nx >= 0
                        && nx < width as i32
                        && nz >= 0
                        && nz < height as i32
                    {
                        let next = nz as usize * width + nx as usize;
                        if arena.walk_segment(point(arena, id), point(arena, next)) {
                            edges.push(next);
                        }
                    }
                }
            }
            edges
        })
        .collect()
}
fn pressure_graph(field: crate::pressure::PressureField) -> Arc<Vec<Vec<usize>>> {
    type GraphCache = HashMap<crate::pressure::PressureField, Arc<Vec<Vec<usize>>>>;
    static CACHE: OnceLock<Mutex<GraphCache>> = OnceLock::new();
    let key = crate::pressure::PressureField {
        seed: 0,
        cover_extra: 0,
        wide: false,
        ..field
    };
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(graph) = cache.get(&key) {
        return graph.clone();
    }
    let graph = Arc::new(build_graph(Arena::Pressure(key)));
    if cache.len() >= 32 {
        cache.clear();
    }
    cache.insert(key, graph.clone());
    graph
}
/// Static wall-clearance graph. Authored arenas are cached once.
/// Pressure fields share a bounded cache keyed by physical layout.
fn neighbors(arena: Arena) -> Arc<Vec<Vec<usize>>> {
    if let Arena::Pressure(field) = arena {
        return pressure_graph(field);
    }
    static RIDGE: OnceLock<Arc<Vec<Vec<usize>>>> = OnceLock::new();
    static CHOKE: OnceLock<Arc<Vec<Vec<usize>>>> = OnceLock::new();
    static OPEN: OnceLock<Arc<Vec<Vec<usize>>>> = OnceLock::new();
    static CITADEL: OnceLock<Arc<Vec<Vec<usize>>>> = OnceLock::new();
    static HIGHLANDS: OnceLock<Arc<Vec<Vec<usize>>>> = OnceLock::new();
    static RUINS: OnceLock<Arc<Vec<Vec<usize>>>> = OnceLock::new();
    static GRAND: OnceLock<Arc<Vec<Vec<usize>>>> = OnceLock::new();
    static GAUNTLET: OnceLock<Arc<Vec<Vec<usize>>>> = OnceLock::new();
    static QUARRY: OnceLock<Arc<Vec<Vec<usize>>>> = OnceLock::new();
    static MIRE: OnceLock<Arc<Vec<Vec<usize>>>> = OnceLock::new();
    let cache = match arena {
        Arena::Ridge => &RIDGE,
        Arena::Choke => &CHOKE,
        Arena::OpenField => &OPEN,
        Arena::Citadel => &CITADEL,
        Arena::Highlands => &HIGHLANDS,
        Arena::Ruins => &RUINS,
        Arena::GrandArena => &GRAND,
        Arena::Gauntlet => &GAUNTLET,
        Arena::Quarry => &QUARRY,
        Arena::Mire => &MIRE,
        Arena::Pressure(_) => unreachable!("pressure graphs are cached by field"),
    };
    cache.get_or_init(|| Arc::new(build_graph(arena))).clone()
}
fn nearest(arena: Arena, p: [f32; 2]) -> Option<usize> {
    let size = arena.half_size();
    let count = (size[0] * 2.0 * size[1] * 2.0) as usize;
    let mut best = None;
    let mut best_distance = f32::INFINITY;
    for id in 0..count {
        let candidate = point(arena, id);
        let d = distance(p, candidate);
        // Preserve stable tie order; only test clearance for a closer anchor.
        if d < best_distance && !arena.blocked(candidate) && arena.walk_segment(p, candidate) {
            best = Some(id);
            best_distance = d;
        }
    }
    best
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

#[derive(Clone, Copy, Debug)]
pub struct Threat {
    pub position: [f32; 2],
    pub range: f32,
    pub confidence: f32,
}
pub fn threat_at_known(arena: Arena, p: [f32; 2], threats: &[Threat]) -> f32 {
    threats
        .iter()
        .filter(|t| arena.clear_segment(p, t.position))
        .map(|t| (1.0 - distance(p, t.position) / (t.range + 2.0)).max(0.0) * t.confidence)
        .sum::<f32>()
        .min(2.0)
}
/// Ground-truth adapter for explicit navigation baseline tests. Agent runtime uses plan_known.
pub fn threat_at(arena: Arena, p: [f32; 2], team: usize, units: &[Unit]) -> f32 {
    let threats: Vec<_> = units
        .iter()
        .filter(|u| u.team != team && u.health > 0.0)
        .map(|u| Threat {
            position: u.position,
            range: u.attack_range(),
            confidence: 1.0,
        })
        .collect();
    threat_at_known(arena, p, &threats)
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
    let threats: Vec<_> = units
        .iter()
        .filter(|u| u.team != team && u.health > 0.0)
        .map(|u| Threat {
            position: u.position,
            range: u.attack_range(),
            confidence: 1.0,
        })
        .collect();
    plan_known(arena, start, goal, mind, &threats)
}
pub fn plan_known(
    arena: Arena,
    start: [f32; 2],
    goal: [f32; 2],
    mind: Mind,
    threats: &[Threat],
) -> Route {
    if arena.blocked(start) || arena.blocked(goal) {
        return Route::default();
    }
    let (Some(s), Some(g)) = (nearest(arena, start), nearest(arena, goal)) else {
        return Route::default();
    };
    let size = arena.half_size();
    let width = (size[0] * 2.0) as usize;
    let height = (size[1] * 2.0) as usize;
    let count = width * height;
    let mut costs = vec![f32::INFINITY; count];
    let mut previous = vec![usize::MAX; count];
    let mut heap = BinaryHeap::new();
    let threat: Vec<_> = (0..count)
        .map(|i| {
            if arena.blocked(point(arena, i)) {
                0.0
            } else {
                threat_at_known(arena, point(arena, i), threats)
            }
        })
        .collect();
    costs[s] = 0.0;
    heap.push(Entry { cost: 0.0, id: s });
    let graph = neighbors(arena);
    while let Some(Entry { cost, id }) = heap.pop() {
        if cost > costs[id] {
            continue;
        }
        if id == g {
            break;
        }
        for &next in &graph[id] {
            let candidate = cost
                + edge(
                    arena,
                    point(arena, id),
                    point(arena, next),
                    mind,
                    threat[next],
                )
                .total();
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
    let skip = usize::from(ids.len() > 1 && arena.walk_segment(start, point(arena, ids[1])));
    points.extend(ids.iter().skip(skip).map(|i| point(arena, *i)));
    points.push(goal);
    let mut cost = PathCost::default();
    for pair in points.windows(2) {
        cost.add(edge(
            arena,
            pair[0],
            pair[1],
            mind,
            threat_at_known(arena, pair[1], threats),
        ));
    }
    Route { points, cost }
}
