//! First open-ground wuxia traversal: a real trajectory, checked terrain, effort,
//! opposed-environment skill check and separate proposal/launch/landing receipts.
use crate::{Battle, Unit, distance, ninjutsu, skills::Skill, theus};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize)]
pub struct Flight {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub start: u32,
    pub end: u32,
    pub apex: f32,
    pub curve: f32,
    pub falling: bool,
}
impl Flight {
    pub fn sample(&self, tick: f32) -> [f32; 3] {
        let t = ((tick - self.start as f32) / (self.end - self.start).max(1) as f32).clamp(0., 1.);
        let d = [self.to[0] - self.from[0], self.to[2] - self.from[2]];
        let len = d[0].hypot(d[1]).max(0.01);
        let wave = (std::f32::consts::PI * t).sin();
        [
            self.from[0] + d[0] * t - d[1] / len * self.curve * wave,
            self.from[1] + (self.to[1] - self.from[1]) * t + self.apex * wave,
            self.from[2] + d[1] * t + d[0] / len * self.curve * wave,
        ]
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Kind {
    Proposed,
    Rejected,
    Launched,
    Interrupted,
    Landed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub sequence: u64,
    pub tick: u32,
    pub unit: usize,
    pub target: Option<usize>,
    pub kind: Kind,
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub effort: f32,
    pub check: Option<theus::Check>,
}
#[allow(clippy::too_many_arguments)]
fn record(
    events: &mut Vec<Event>,
    unit: usize,
    tick: u32,
    target: Option<usize>,
    kind: Kind,
    f: &Flight,
    effort: f32,
    check: Option<theus::Check>,
) {
    let sequence = events.last().map_or(0, |e| e.sequence + 1);
    events.push(Event {
        sequence,
        tick,
        unit,
        target,
        kind,
        from: f.from,
        to: f.to,
        effort,
        check,
    });
    if events.len() > 128 {
        events.remove(0);
    }
}
pub fn qualified(u: &Unit) -> bool {
    u.health > 0.
        && u.health_fraction() > 0.3
        && u.equipped_skill(Skill::CloudStep)
        && ninjutsu::vault_body_allowed(u)
        && u.theus
            .as_ref()
            .is_some_and(|a| a.sheet.attributes[1] >= 12 && a.sheet.rank(2) >= 1)
}
pub fn choose(b: &mut Battle) {
    if !b.founders {
        return;
    }
    for u in &mut b.units {
        if !qualified(u)
            || ninjutsu::in_transit(u)
            || b.opportunities.owns_mobility(u.id)
            || b.tick < u.aerial_ready
            || !b.tick.is_multiple_of(10)
            || u.tactics.phase != crate::tactics::Phase::Engage
        {
            continue;
        }
        if u.ninja.as_ref().is_none_or(|n| n.bound_until > b.tick)
            || u.theus
                .as_ref()
                .is_none_or(|a| a.stagger_until > b.tick || a.stamina < 9.)
        {
            continue;
        }
        let courage = u.mind.courage + u.support_target.map_or(0., |id| u.social.bond(id) * 0.2);
        if courage < 0.55 || u.mind.caution > 0.85 {
            continue;
        }
        let Some(c) = u
            .contacts
            .iter()
            .filter(|c| c.visible && c.confidence(b.tick) > 0.5)
            .min_by(|a, b| {
                distance(u.position, a.position).total_cmp(&distance(u.position, b.position))
            })
        else {
            continue;
        };
        let d = distance(u.position, c.position);
        if !(3.5..=8.).contains(&d) {
            continue;
        }
        let direction = [
            (c.position[0] - u.position[0]) / d,
            (c.position[1] - u.position[1]) / d,
        ];
        let side = if u.id % 2 == 0 { 1. } else { -1. };
        let to = [
            c.position[0] - direction[0] * 1.3 - direction[1] * side * 1.1,
            c.position[1] - direction[1] * 1.3 + direction[0] * side * 1.1,
        ];
        let flight = Flight {
            from: [
                u.position[0],
                b.arena.elevation(u.position) + 0.08,
                u.position[1],
            ],
            to: [to[0], b.arena.elevation(to) + 0.08, to[1]],
            start: b.tick,
            end: b.tick + 19,
            apex: 3.2,
            curve: side * 0.6,
            falling: false,
        };
        // Open-ground arcs are distinct from wall vaults. Every projected foothold
        // remains legal, so an interruption can descend without wall phasing.
        if !(0..=76).all(|i| {
            let p = flight.sample(b.tick as f32 + i as f32 / 4.);
            !b.arena.blocked([p[0], p[2]]) && !ninjutsu::solid(b.arena, p, 0.3)
        }) {
            continue;
        }
        let target = c.enemy;
        record(
            &mut b.aerial_events,
            u.id,
            b.tick,
            Some(target),
            Kind::Proposed,
            &flight,
            9.,
            None,
        );
        let actor = u.theus.as_mut().unwrap();
        actor.spend(9.);
        actor.sheet.train(2, 1);
        u.aerial_ready = b.tick + 65;
        let check = theus::check(
            b.seed ^ u64::from(b.tick).rotate_left(23) ^ u.id as u64 ^ 0xc10d,
            actor.sheet.rating(2, 0),
        );
        if check.margin < 0 {
            actor.stagger_until = b.tick + 6;
            record(
                &mut b.aerial_events,
                u.id,
                b.tick,
                Some(target),
                Kind::Rejected,
                &flight,
                9.,
                Some(check),
            );
            continue;
        }
        u.windup = None;
        u.route = Default::default();
        u.aerial = Some(flight.clone());
        u.decision_reason = "Cloud step: checked lateral approach to a visible contact".into();
        record(
            &mut b.aerial_events,
            u.id,
            b.tick,
            Some(target),
            Kind::Launched,
            &flight,
            9.,
            Some(check),
        );
    }
}
pub fn advance(b: &mut Battle) {
    for u in &mut b.units {
        let Some(mut f) = u.aerial.take() else {
            continue;
        };
        let p = f.sample(b.tick as f32);
        let old = u.position;
        u.position = [p[0], p[2]];
        u.distance_moved += distance(old, u.position);
        if !f.falling
            && (u.health <= 0.
                || !ninjutsu::vault_body_allowed(u)
                || u.ninja.as_ref().is_some_and(|n| n.bound_until > b.tick)
                || u.theus.as_ref().is_some_and(|a| a.stagger_until > b.tick))
        {
            record(
                &mut b.aerial_events,
                u.id,
                b.tick,
                u.enemy,
                Kind::Interrupted,
                &f,
                0.,
                None,
            );
            f = Flight {
                from: p,
                to: [p[0], b.arena.elevation(u.position) + 0.08, p[2]],
                start: b.tick,
                end: b.tick + 5,
                apex: 0.,
                curve: 0.,
                falling: true,
            };
        }
        if b.tick >= f.end {
            u.position = [f.to[0], f.to[2]];
            u.route = Default::default();
            record(
                &mut b.aerial_events,
                u.id,
                b.tick,
                u.enemy,
                Kind::Landed,
                &f,
                0.,
                None,
            );
        } else {
            u.aerial = Some(f);
        }
    }
}
