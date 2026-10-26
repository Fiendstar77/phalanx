//! Authoritative occult martial arts: solid-wall traversal, obscurants and weapons.
//! Decisions use contacts; rendering reads these states and receipts.
use crate::{
    Battle, Unit, distance,
    equipment::Gear,
    skills::Skill,
    terrain::Arena,
    theus::{self, Check},
    theus_combat::Impact,
};
use serde::Serialize;
pub const SMOKE_CAPACITY: usize = 8;
pub const WHEEL_CAPACITY: usize = 8;
#[derive(Clone, Debug, Serialize)]
pub struct Leap {
    pub from: [f32; 3],
    pub touch: [f32; 3],
    pub to: [f32; 3],
    pub top: f32,
    pub start: u32,
    pub end: u32,
    pub wall: usize,
}
impl Leap {
    pub fn sample(&self, tick: f32) -> [f32; 3] {
        let t = ((tick - self.start as f32) / (self.end - self.start).max(1) as f32).clamp(0., 1.);
        let ease = |u: f32| u * u * (3. - 2. * u);
        let roof = [self.touch[0], self.top, self.touch[2]];
        let landing = [self.to[0], self.top, self.to[2]];
        if t < 0.20 {
            lerp(self.from, self.touch, ease(t / 0.20))
        } else if t < 0.40 {
            lerp(self.touch, roof, ease((t - 0.20) / 0.20))
        } else if t < 0.75 {
            lerp(roof, landing, ease((t - 0.40) / 0.35))
        } else {
            lerp(landing, self.to, ease((t - 0.75) / 0.25))
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Fighter {
    pub leap: Option<Leap>,
    pub wall_ready: u32,
    pub smoke_charges: u8,
    pub smoke_ready: u32,
    pub seal_ready: u32,
    pub bound_until: u32,
    pub hook_ready: u32,
    pub pull: Option<[f32; 2]>,
    pub last_position: Option<[f32; 2]>,
    pub stuck_ticks: u16,
}
impl Default for Fighter {
    fn default() -> Self {
        Self {
            leap: None,
            wall_ready: 0,
            smoke_charges: 2,
            smoke_ready: 0,
            seal_ready: 0,
            bound_until: 0,
            hook_ready: 0,
            pull: None,
            last_position: None,
            stuck_ticks: 0,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Smoke {
    pub owner: usize,
    pub position: [f32; 3],
    pub start: u32,
    pub end: u32,
}
impl Smoke {
    pub fn radius(&self, tick: f32) -> f32 {
        let age = tick - self.start as f32;
        let remaining = self.end as f32 - tick;
        2.6 * (age / 3.).clamp(0., 1.).min((remaining / 8.).clamp(0., 1.))
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Wheel {
    pub owner: usize,
    pub previous: [f32; 3],
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub returning: bool,
    pub age: u32,
    pub hit: Vec<(usize, bool)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Art {
    WallVault,
    SmokeBomb,
    GraveBind,
    ChainSnare,
    WheelThrow,
}
impl Art {
    pub fn name(self) -> &'static str {
        match self {
            Self::WallVault => "Wall dance",
            Self::SmokeBomb => "Smoke escape",
            Self::GraveBind => "Grave binding",
            Self::ChainSnare => "Reaper's hook",
            Self::WheelThrow => "Demon wheel",
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub sequence: u64,
    pub tick: u32,
    pub caster: usize,
    pub target: Option<usize>,
    pub art: Art,
    pub success: bool,
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub attack: Option<Check>,
    pub defence: Option<Check>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Field {
    pub smoke: Vec<Smoke>,
    pub wheels: Vec<Wheel>,
    pub events: Vec<Event>,
    hook_event: Option<u64>,
}
impl Field {
    pub fn empty(&self) -> bool {
        self.smoke.is_empty() && self.wheels.is_empty() && self.events.is_empty()
    }
    fn record(&mut self, mut event: Event) {
        event.sequence = self.events.last().map_or(0, |e| e.sequence + 1);
        self.events.push(event);
        if self.events.len() > 128 {
            self.events.remove(0);
        }
    }
    pub fn clear_sight(&self, from: [f32; 3], to: [f32; 3], tick: f32) -> bool {
        // Touching distance remains perceptible; smoke never grants wall sight.
        if len(sub(from, to)) < 1.1 {
            return true;
        }
        !self.smoke.iter().any(|s| {
            let r = s.radius(tick);
            if r < 0.15 {
                return false;
            }
            let scale = |p: [f32; 3]| {
                [
                    (p[0] - s.position[0]) / r,
                    (p[1] - s.position[1] - 1.) / (r * 0.75),
                    (p[2] - s.position[2]) / r,
                ]
            };
            segment_distance(scale(from), scale(to), [0.; 3]) < 1.
        })
    }
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn len(v: [f32; 3]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}
fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}
fn segment_distance(a: [f32; 3], b: [f32; 3], p: [f32; 3]) -> f32 {
    let d = sub(b, a);
    let v = sub(p, a);
    let t = (d.iter().zip(v).map(|(a, b)| a * b).sum::<f32>()
        / d.iter().map(|x| x * x).sum::<f32>().max(1e-8))
    .clamp(0., 1.);
    len(sub(p, lerp(a, b, t)))
}
pub fn in_transit(u: &Unit) -> bool {
    u.ninja.as_ref().is_some_and(|n| n.leap.is_some())
}
pub fn physical_height(u: &Unit, arena: Arena, tick: f32) -> f32 {
    u.ninja
        .as_ref()
        .and_then(|n| n.leap.as_ref())
        .map_or(0., |l| {
            (l.sample(tick)[1] - arena.elevation(u.position)).max(0.)
        })
}
pub fn visual_height(u: &Unit, arena: Arena, tick: f32) -> f32 {
    physical_height(u, arena, tick).max(u.theus.as_ref().map_or(0., |a| a.height_at(tick)))
}
pub fn body(u: &Unit, arena: Arena, tick: f32) -> [f32; 3] {
    [
        u.position[0],
        arena.elevation(u.position) + physical_height(u, arena, tick) + 0.85,
        u.position[1],
    ]
}
pub fn solid(arena: Arena, p: [f32; 3], margin: f32) -> bool {
    let q = [p[0], p[2]];
    let size = arena.half_size();
    p[0].abs() > size[0] - crate::collision::BODY_RADIUS
        || p[2].abs() > size[1] - crate::collision::BODY_RADIUS
        || p[1] < arena.elevation(q) - 0.02
        || arena.walls().iter().any(|(c, s)| {
            (p[0] - c[0]).abs() < s[0] * 0.5 + margin
                && (p[2] - c[1]).abs() < s[1] * 0.5 + margin
                && p[1] < arena.elevation(*c) + 2.4 + margin
        })
}
pub fn clear_line(arena: Arena, from: [f32; 3], to: [f32; 3], margin: f32) -> bool {
    let steps = (len(sub(to, from)) / 0.12).ceil().max(1.) as usize;
    (0..=steps).all(|i| !solid(arena, lerp(from, to, i as f32 / steps as f32), margin))
}
/// A staged climb/cross/descent has explicit solid clearance, not wall phasing.
pub fn wall_plan(unit: &Unit, arena: Arena, goal: [f32; 2], tick: u32) -> Option<Leap> {
    let p = unit.position;
    arena
        .walls()
        .iter()
        .enumerate()
        .filter_map(|(wall, (c, s))| {
            let axis = usize::from(s[1] < s[0]);
            let other = 1 - axis;
            if s[axis] > 5. || (p[other] - c[other]).abs() > s[other] * 0.5 + 0.2 {
                return None;
            }
            let sign = if p[axis] < c[axis] { -1. } else { 1. };
            let gap = (p[axis] - c[axis]).abs() - s[axis] * 0.5;
            if !(0.46..1.8).contains(&gap) {
                return None;
            }
            let mut landing = p;
            landing[axis] = c[axis] - sign * (s[axis] * 0.5 + 0.72);
            if arena.blocked(landing) || distance(landing, goal) + 0.7 >= distance(p, goal) {
                return None;
            }
            let from = [p[0], arena.elevation(p), p[1]];
            let to = [landing[0], arena.elevation(landing), landing[1]];
            let mut contact = p;
            contact[axis] = c[axis] + sign * (s[axis] * 0.5 + 0.50);
            let leap = Leap {
                from,
                touch: [contact[0], arena.elevation(contact) + 1.1, contact[1]],
                to,
                top: (arena.elevation(*c) + 3.15)
                    .max(from[1] + 1.)
                    .max(to[1] + 1.),
                start: tick,
                end: tick + 14,
                wall,
            };
            (0..=56)
                .all(|i| !solid(arena, leap.sample(tick as f32 + i as f32 * 0.25), 0.45))
                .then_some(leap)
        })
        .min_by(|a, b| {
            distance([a.to[0], a.to[2]], goal).total_cmp(&distance([b.to[0], b.to[2]], goal))
        })
}
fn receipt(
    art: Art,
    caster: usize,
    target: Option<usize>,
    tick: u32,
    from: [f32; 3],
    to: [f32; 3],
    success: bool,
) -> Event {
    Event {
        sequence: 0,
        tick,
        caster,
        target,
        art,
        success,
        from,
        to,
        attack: None,
        defence: None,
    }
}
pub fn prepare(battle: &mut Battle) {
    let tick = battle.tick;
    let arena = battle.arena;
    battle.ninjutsu.smoke.retain(|s| s.end > tick);
    for u in &mut battle.units {
        if let Some(n) = &mut u.ninja {
            if n.last_position
                .is_some_and(|p| distance(p, u.position) < 0.04)
            {
                n.stuck_ticks = n.stuck_ticks.saturating_add(1);
            } else {
                n.stuck_ticks = 0;
            }
            n.last_position = Some(u.position);
            if u.health > 0. && u.action == 0 && u.contacts.is_empty() && n.stuck_ticks >= 30 {
                // An unproductive search advances to another authored patrol
                // objective. It does not consult an unseen opponent's position.
                u.search_waypoint += 1;
                u.route = Default::default();
                n.stuck_ticks = 0;
            }
        }
        if let Some(n) = &mut u.ninja
            && let Some(l) = &n.leap
        {
            let p = l.sample(tick as f32);
            let old = u.position;
            u.position = [p[0], p[2]];
            u.distance_moved += distance(old, u.position);
            if tick >= l.end {
                n.leap = None;
                u.route = Default::default();
            }
        }
    }
    let snapshot = battle.units.clone();
    let mut reserved: Vec<_> = snapshot
        .iter()
        .filter_map(|u| u.ninja.as_ref()?.leap.clone())
        .collect();
    for u in &mut battle.units {
        if u.health <= 0. || u.theus.is_none() || u.ninja.is_none() || in_transit(u) {
            continue;
        }
        let contact = u.contacts.iter().min_by(|a, b| {
            distance(u.position, a.position).total_cmp(&distance(u.position, b.position))
        });
        let goal = contact.map_or(u.target, |c| c.position);
        let known_target = contact.map(|c| c.enemy);
        let hands = u.lost_limbs & !u.prosthetics & 3 != 3;
        let legs = u.lost_limbs & !u.prosthetics & 12 == 0;
        if u.equipped_skill(Skill::WallDance)
            && legs
            && u.ninja.as_ref().unwrap().wall_ready <= tick
            && u.ninja.as_ref().unwrap().bound_until <= tick
            && u.theus.as_ref().unwrap().sheet.rating(2, 0) >= 12
            && let Some(leap) = wall_plan(u, arena, goal, tick)
            && reserved.iter().all(|other| {
                (0..=28).all(|i| {
                    len(sub(
                        leap.sample(tick as f32 + i as f32 * 0.5),
                        other.sample(tick as f32 + i as f32 * 0.5),
                    )) > 1.05
                })
            })
            && u.theus.as_mut().unwrap().spend(7.)
        {
            let n = u.ninja.as_mut().unwrap();
            n.wall_ready = tick + 55;
            n.leap = Some(leap.clone());
            reserved.push(leap.clone());
            u.windup = None;
            u.theus.as_mut().unwrap().sheet.train(2, 2);
            battle.ninjutsu.record(receipt(
                Art::WallVault,
                u.id,
                known_target,
                tick,
                leap.from,
                leap.to,
                true,
            ));
            continue;
        }
        let Some(target) = known_target else {
            continue;
        };
        let visible = contact.is_some_and(|c| c.visible);
        let d = distance(u.position, goal);
        let from = body(u, arena, tick as f32);
        let to = body(&snapshot[target], arena, tick as f32);
        if hands
            && u.equipped_skill(Skill::SmokeBomb)
            && visible
            && d < 4.8
            && tick >= 8 + (u.id % 3) as u32 * 6
            && (u.health_fraction() < 0.68
                || u.contacts
                    .iter()
                    .filter(|c| c.visible && distance(u.position, c.position) < 5.)
                    .count()
                    > 1)
            && u.ninja.as_ref().unwrap().smoke_charges > 0
            && u.ninja.as_ref().unwrap().smoke_ready <= tick
            && battle.ninjutsu.smoke.len() < SMOKE_CAPACITY
            && !battle
                .ninjutsu
                .smoke
                .iter()
                .any(|s| distance([s.position[0], s.position[2]], u.position) < 3.)
            && u.theus.as_mut().unwrap().spend(3.)
        {
            let n = u.ninja.as_mut().unwrap();
            n.smoke_charges -= 1;
            n.smoke_ready = tick + 65;
            let position = [u.position[0], arena.elevation(u.position), u.position[1]];
            battle.ninjutsu.smoke.push(Smoke {
                owner: u.id,
                position,
                start: tick,
                end: tick + 42,
            });
            battle.ninjutsu.record(receipt(
                Art::SmokeBomb,
                u.id,
                None,
                tick,
                from,
                position,
                true,
            ));
            u.windup = None;
            continue;
        }
        let clear = visible
            && snapshot[target].health > 0.
            && clear_line(arena, from, to, 0.)
            && battle.ninjutsu.clear_sight(from, to, tick as f32);
        if hands
            && u.equipped_skill(Skill::ShadowSeal)
            && clear
            && (1.3..6.5).contains(&d)
            && !in_transit(&snapshot[target])
            && u.ninja.as_ref().unwrap().seal_ready <= tick
            && (tick + u.id as u32 * 11).is_multiple_of(23)
            && u.theus.as_mut().unwrap().spend(8.)
        {
            u.ninja.as_mut().unwrap().seal_ready = tick + 90;
            let seed = battle.seed ^ (tick as u64).wrapping_mul(7919) ^ (u.id as u64 * 131);
            let attack = theus::check(seed, u.theus.as_ref().unwrap().sheet.rating(4, 0));
            let defence = theus::check(
                seed ^ 0x8b17,
                snapshot[target]
                    .theus
                    .as_ref()
                    .map_or(11, |a| a.sheet.attributes[3] as i16),
            );
            let success = attack.margin >= 0 && attack.margin + 2 >= defence.margin;
            let mut event = receipt(Art::GraveBind, u.id, Some(target), tick, from, to, success);
            event.attack = Some(attack);
            event.defence = Some(defence);
            battle.ninjutsu.record(event);
            u.theus.as_mut().unwrap().sheet.train(4, 1);
        }
        if hands
            && u.equipment.has(Gear::RazorWheel)
            && clear
            && d < 8.
            && u.cooldown <= 0.
            && battle.ninjutsu.wheels.len() < WHEEL_CAPACITY
            && u.theus.as_mut().unwrap().spend(4.)
        {
            battle.ninjutsu.wheels.push(Wheel {
                owner: u.id,
                previous: from,
                position: from,
                target: to,
                returning: false,
                age: 0,
                hit: Vec::new(),
            });
            battle.ninjutsu.record(receipt(
                Art::WheelThrow,
                u.id,
                Some(target),
                tick,
                from,
                to,
                true,
            ));
            u.cooldown = 2.2;
        }
    }
    let binds: Vec<_> = battle
        .ninjutsu
        .events
        .iter()
        .filter(|e| e.tick == tick && e.art == Art::GraveBind && e.success)
        .filter_map(|e| e.target.map(|t| (t, e.caster)))
        .collect();
    for (id, caster) in binds {
        if let Some(n) = &mut battle.units[id].ninja {
            n.bound_until = tick + 22;
        }
        if let Some(a) = &mut battle.units[id].theus {
            a.exposed_until = tick + 22;
            a.opener = Some(caster);
        }
    }
}
pub fn smoke_exit(u: &Unit, arena: Arena, field: &Field, tick: u32) -> Option<[f32; 2]> {
    let smoke = field.smoke.iter().find(|s| {
        distance(u.position, [s.position[0], s.position[2]]) < s.radius(tick as f32) + 0.25
    })?;
    let goal = u.contacts.first()?.position;
    let angle = (goal[1] - smoke.position[2]).atan2(goal[0] - smoke.position[0])
        + if u.id.is_multiple_of(2) { 1.65 } else { -1.65 };
    [0., 0.5, -0.5, 1., -1.]
        .into_iter()
        .map(|offset| {
            [
                smoke.position[0] + (angle + offset).cos() * 3.4,
                smoke.position[2] + (angle + offset).sin() * 3.4,
            ]
        })
        .find(|p| {
            !arena.blocked(*p)
                && p[0].abs() < arena.half_size()[0] - 0.5
                && p[1].abs() < arena.half_size()[1] - 0.5
        })
}
pub fn pulls(battle: &mut Battle) {
    for u in &mut battle.units {
        if let Some(towards) = u.ninja.as_mut().and_then(|n| n.pull.take())
            && u.health > 0.
            && !in_transit(u)
        {
            let d = distance(u.position, towards);
            let amount = (d - 0.95).clamp(0., 0.7) / d.max(0.001);
            let to = std::array::from_fn(|i| u.position[i] + (towards[i] - u.position[i]) * amount);
            if battle.arena.walk_segment(u.position, to) {
                u.position = to;
            }
        }
    }
}
pub fn wheels(battle: &mut Battle, impacts: &mut Vec<Impact>) {
    let mut alive = Vec::new();
    let tick = battle.tick;
    let arena = battle.arena;
    for mut wheel in std::mem::take(&mut battle.ninjutsu.wheels) {
        wheel.age += 1;
        let caster = &battle.units[wheel.owner];
        let from = wheel.position;
        wheel.previous = from;
        let target = if wheel.returning {
            body(caster, arena, tick as f32)
        } else {
            wheel.target
        };
        let d = len(sub(target, from));
        let to = lerp(from, target, (1.25 / d.max(0.001)).min(1.));
        if wheel.age > 32 || (wheel.returning && d < 0.65) {
            continue;
        }
        if !clear_line(arena, from, to, 0.1) {
            if wheel.returning {
                continue;
            }
            wheel.returning = true;
            alive.push(wheel);
            continue;
        }
        let hit = battle
            .units
            .iter()
            .filter(|u| {
                u.team != caster.team
                    && u.health > 0.
                    && !wheel.hit.contains(&(u.id, wheel.returning))
            })
            .filter(|u| segment_distance(from, to, body(u, arena, tick as f32)) < 0.65)
            .min_by(|a, b| {
                len(sub(body(a, arena, tick as f32), from))
                    .total_cmp(&len(sub(body(b, arena, tick as f32), from)))
            });
        if let Some(u) = hit {
            let v = sub(to, from);
            impacts.push((
                caster.id,
                u.id,
                12. * caster.damage_multiplier() * u.incoming_multiplier(),
                false,
                12.5,
                Some(([to[0], to[2]], [v[0] / crate::DT, v[2] / crate::DT])),
            ));
            wheel.hit.push((u.id, wheel.returning));
        }
        wheel.position = to;
        if !wheel.returning && d < 1.3 {
            wheel.returning = true;
        }
        alive.push(wheel);
    }
    battle.ninjutsu.wheels = alive;
}
pub fn hooks(battle: &mut Battle) {
    let mut hits = Vec::new();
    for e in &battle.theus_events {
        if battle
            .ninjutsu
            .hook_event
            .is_some_and(|old| e.sequence <= old)
        {
            continue;
        }
        battle.ninjutsu.hook_event = Some(e.sequence);
        let a = &battle.units[e.attacker];
        if a.equipment.has(Gear::Kusarigama)
            && e.resolution.damage > 0.
            && a.ninja
                .as_ref()
                .is_some_and(|n| n.hook_ready <= battle.tick)
        {
            hits.push((e.attacker, e.defender));
        }
    }
    for (a, b) in hits {
        if in_transit(&battle.units[b]) {
            continue;
        }
        let position = battle.units[a].position;
        if let Some(n) = &mut battle.units[a].ninja {
            n.hook_ready = battle.tick + 28;
        }
        if let Some(n) = &mut battle.units[b].ninja {
            n.bound_until = n.bound_until.max(battle.tick + 8);
            n.pull = Some(position);
        }
        if let Some(t) = &mut battle.units[b].theus {
            t.exposed_until = t.exposed_until.max(battle.tick + 10);
            t.opener = Some(a);
        }
        battle.ninjutsu.record(receipt(
            Art::ChainSnare,
            a,
            Some(b),
            battle.tick,
            body(&battle.units[a], battle.arena, battle.tick as f32),
            body(&battle.units[b], battle.arena, battle.tick as f32),
            true,
        ));
    }
}
