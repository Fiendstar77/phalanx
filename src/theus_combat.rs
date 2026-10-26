//! Theus applied to Phalanx contacts, roles, equipment and simultaneous impacts.
use crate::{
    Battle, DT, FighterClass as Class, Unit, distance,
    perception::Contact,
    terrain::Arena,
    theus::{self, Actor, Critical, Event, Outcome, Sheet, Strike, Technique},
};
pub(crate) type Impact = (usize, usize, f32, bool, f32, Option<([f32; 2], [f32; 2])>);
pub fn description(unit: &Unit) -> String {
    let Some(actor) = &unit.theus else {
        return String::new();
    };
    let [might, agility, insight, resolve] = actor.sheet.attributes;
    format!(
        "THEUS · Might {might} / Agility {agility} / Insight {insight} / Resolve {resolve}\nEffort {:.0}/{:.0} · Melee {} / Aim {} / Acrobatics {}",
        actor.stamina,
        actor.sheet.endurance(),
        actor.sheet.rating(0, 2),
        actor.sheet.rating(1, 2),
        actor.sheet.rating(2, 0)
    )
}
pub fn sheet(unit: &Unit) -> Sheet {
    Sheet::from_temperament([
        unit.mind.aggression,
        unit.mind.self_preservation,
        unit.mind.patience,
        unit.mind.courage,
    ])
}
pub fn enable(battle: &mut Battle) {
    for unit in &mut battle.units {
        if unit.theus.is_none() {
            unit.theus = Some(Actor::new(sheet(unit)));
        }
    }
}
pub fn recover(battle: &mut Battle) {
    for unit in &mut battle.units {
        if let Some(actor) = &mut unit.theus {
            actor.clock = battle.tick;
            let recovery = if unit.action == 6 { 4.5 } else { 2.8 };
            actor.stamina = (actor.stamina + recovery * DT).min(actor.sheet.endurance());
        }
    }
}
fn willing(unit: &Unit) -> bool {
    unit.mind.teamwork >= 0.45 || unit.blessed || unit.has_skill(Class::Paladin)
}
fn knows(unit: &Unit, id: usize) -> bool {
    unit.contacts.iter().any(|c| c.enemy == id && c.visible)
}
fn partner<'a>(unit: &Unit, target: &Unit, units: &'a [Unit]) -> Option<&'a Unit> {
    units
        .iter()
        .filter(|ally| {
            ally.id != unit.id
                && ally.team == unit.team
                && ally.health > 0.
                && willing(ally)
                && distance(unit.position, ally.position) < 10.
                && distance(ally.position, target.position)
                    < if unit.is_ranged() && ally.is_ranged() {
                        12.
                    } else {
                        3.5
                    }
                && (!unit.is_ranged() || ally.is_ranged())
                && knows(ally, target.id)
                && ally.enemy == Some(target.id)
        })
        .min_by(|a, b| {
            let opener = target.theus.as_ref().and_then(|a| {
                if a.airborne_until > a.clock {
                    a.launcher
                } else {
                    a.opener
                }
            });
            (opener != Some(a.id))
                .cmp(&(opener != Some(b.id)))
                .then(
                    distance(a.position, target.position)
                        .total_cmp(&distance(b.position, target.position)),
                )
                .then(a.id.cmp(&b.id))
        })
}
fn opportunity(
    unit: &Unit,
    target: &Unit,
    units: &[Unit],
    tick: u32,
) -> (Technique, Option<usize>) {
    let Some(actor) = &unit.theus else {
        return (Technique::None, None);
    };
    if !willing(unit) || actor.combo_ready > tick || actor.stamina < 8. || !knows(unit, target.id) {
        return (Technique::None, None);
    }
    let Some(other) = partner(unit, target, units) else {
        return (Technique::None, None);
    };
    if let Some(victim) = &target.theus {
        if victim.airborne_until > tick
            && victim.launcher == Some(other.id)
            && unit.lost_limbs & !unit.prosthetics & 12 == 0
            && actor.sheet.rating(2, 0) >= 12
            && !unit.is_ranged()
        {
            return (Technique::MeteorRelay, Some(other.id));
        }
        if victim.exposed_until > tick
            && unit.is_ranged()
            && other.is_ranged()
            && victim.opener == Some(other.id)
        {
            return (Technique::Crossfire, Some(other.id));
        }
    }
    if !unit.is_ranged()
        && !other.is_ranged()
        && !unit.has_skill(Class::Guardian)
        && !unit.has_skill(Class::Paladin)
    {
        let a = [
            unit.position[0] - target.position[0],
            unit.position[1] - target.position[1],
        ];
        let b = [
            other.position[0] - target.position[0],
            other.position[1] - target.position[1],
        ];
        let cosine = (a[0] * b[0] + a[1] * b[1])
            / (distance(unit.position, target.position)
                * distance(other.position, target.position))
            .max(0.01);
        if cosine < -0.15 {
            return (Technique::Pincer, Some(other.id));
        }
    }
    if !unit.is_ranged()
        && (unit.has_skill(Class::Guardian) || unit.has_skill(Class::Paladin))
        && !other.is_ranged()
    {
        return (Technique::AegisLaunch, Some(other.id));
    }
    (Technique::None, None)
}
pub fn priority_contact(unit: &Unit, units: &[Unit], tick: u32) -> Option<Contact> {
    if unit.theus.is_none() || !willing(unit) {
        return None;
    }
    unit.contacts
        .iter()
        .filter(|c| c.visible && distance(unit.position, c.position) < 5.)
        .filter(|c| {
            units.get(c.enemy).is_some_and(|target| {
                target
                    .theus
                    .as_ref()
                    .is_some_and(|a| a.exposed_until > tick || a.airborne_until > tick)
                    && opportunity(unit, target, units, tick).0 != Technique::None
            })
        })
        .min_by_key(|c| c.enemy)
        .cloned()
}
pub fn mobility(unit: &mut Unit, enemy: Option<&Contact>, arena: Arena, tick: u32) {
    let Some(enemy) = enemy.filter(|c| c.visible) else {
        return;
    };
    if unit.is_ranged() || unit.lost_limbs & !unit.prosthetics & 12 != 0 {
        return;
    }
    let d = distance(unit.position, enemy.position);
    let Some(actor) = &mut unit.theus else { return };
    if !(2.2..8.5).contains(&d)
        || actor.burst_ready > tick
        || actor.sheet.rating(2, 0) < 12
        || !(tick + unit.id as u32 * 7).is_multiple_of(17)
        || actor.stagger_until > tick
        || !actor.spend(6.)
    {
        return;
    }
    let near_wall = arena.walls().iter().any(|(c, s)| {
        let dx = ((unit.position[0] - c[0]).abs() - s[0] * 0.5).max(0.);
        let dz = ((unit.position[1] - c[1]).abs() - s[1] * 0.5).max(0.);
        dx.hypot(dz) < 1.1
    });
    actor.burst_until = tick + 6;
    actor.burst_ready = tick + 38;
    actor.perform(
        if near_wall {
            Technique::WallKick
        } else {
            Technique::BurstStep
        },
        tick,
        10,
    );
    actor.sheet.train(2, 1);
}
pub fn cancel_windup(unit: &mut Unit, enemy: Option<&Contact>, d: f32, clear: bool) {
    let range = unit.attack_range();
    if let Some(actor) = &mut unit.theus
        && actor.windup_target.is_some()
        && (!clear
            || d > range
            || enemy.is_none_or(|e| !e.visible || Some(e.enemy) != actor.windup_target))
    {
        actor.windup_target = None;
    }
}
pub fn melee_ready(unit: &mut Unit, target: usize, units: &[Unit], tick: u32) -> bool {
    let (technique, _) = opportunity(unit, &units[target], units, tick);
    let Some(actor) = &mut unit.theus else {
        return true;
    };
    if actor.stagger_until > tick {
        return false;
    }
    if actor.windup_target != Some(target) {
        actor.windup_target = Some(target);
        actor.strike_at = tick + 2;
        if technique != Technique::None {
            actor.perform(technique, tick, 12);
        } else if actor.technique != Technique::Counter || actor.technique_until <= tick {
            actor.perform(Technique::None, tick, 8);
        }
        return false;
    }
    if tick < actor.strike_at {
        return false;
    }
    actor.windup_target = None;
    true
}
pub fn exchanges(
    battle: &mut Battle,
    snapshot: &[Unit],
    damage: &mut [f32],
    impacts: &mut Vec<Impact>,
) {
    if !battle.units.iter().any(|u| u.theus.is_some()) {
        return;
    }
    damage.fill(0.);
    for (unit, before) in battle.units.iter_mut().zip(snapshot) {
        unit.damage_dealt = before.damage_dealt;
    }
    let mut resolved = Vec::with_capacity(impacts.len());
    for (ordinal, (attacker, defender, amount, blunt, speed, contact)) in
        std::mem::take(impacts).into_iter().enumerate()
    {
        let Some(a) = snapshot[attacker].theus.as_ref() else {
            damage[defender] += amount;
            battle.units[attacker].damage_dealt += amount;
            resolved.push((attacker, defender, amount, blunt, speed, contact));
            continue;
        };
        let Some(b) = snapshot[defender].theus.as_ref() else {
            continue;
        };
        // Charging body contacts already resolved by the collision solver.
        if contact.is_some() && blunt && speed < 9. {
            damage[defender] += amount;
            battle.units[attacker].damage_dealt += amount;
            resolved.push((attacker, defender, amount, blunt, speed, contact));
            continue;
        }
        let source = &snapshot[attacker];
        let target = &snapshot[defender];
        let mut available = source.clone();
        available.theus = battle.units[attacker].theus.clone();
        let (mut technique, partner) = opportunity(&available, target, snapshot, battle.tick);
        if technique == Technique::None
            && a.technique == Technique::Counter
            && a.technique_until > battle.tick
        {
            technique = Technique::Counter;
        }
        let ranged = speed >= 9.;
        let skill = usize::from(ranged);
        let training = if ranged {
            i16::from(source.is_ranged()) * 2
        } else {
            i16::from(!source.is_ranged()) * 2
        };
        let tired = if a.tired() { 3 } else { 0 };
        let defence = b.sheet.attributes[1] as i16 / 2
            + b.sheet.attributes[3] as i16 / 4
            + 2
            + b.sheet.rank(2)
            + i16::from(target.has_skill(Class::Guardian))
            - if b.tired() { 2 } else { 0 };
        let multiplier = match technique {
            Technique::MeteorRelay => 2.1,
            Technique::AegisLaunch => 1.35,
            Technique::Crossfire | Technique::Pincer => 1.65,
            Technique::Counter => 1.25,
            _ => 1.,
        };
        let seed = battle.seed
            ^ (battle.tick as u64).wrapping_mul(0x9e3779b97f4a7c15)
            ^ (attacker as u64).wrapping_mul(131)
            ^ (defender as u64).wrapping_mul(7919)
            ^ ordinal as u64;
        let result = theus::resolve(
            seed,
            Strike {
                attack: a.sheet.rating(skill, training) - tired
                    + if technique.combination() { 2 } else { 0 },
                defence,
                damage: amount * 1.35 * (0.85 + a.sheet.attributes[0] as f32 * 0.015) * multiplier,
                armour: (1. - target.incoming_multiplier()).max(0.) * 5.,
                parry: !target.is_ranged() && target.lost_limbs & !target.prosthetics & 3 != 3,
                exposed: b.exposed_until > battle.tick
                    || b.stagger_until > battle.tick
                    || b.airborne_until > battle.tick,
                critical_bonus: if technique.combination() { 12 } else { 0 },
            },
        );
        if let Some(actor) = &mut battle.units[attacker].theus {
            actor.spend(if technique.combination() { 8. } else { 1.5 });
            if technique != Technique::None {
                actor.perform(technique, battle.tick, 12);
                if technique.combination() {
                    actor.combo_ready = battle.tick + (80 - 2 * actor.sheet.rating(4, 0)) as u32;
                    if result.damage > 0. {
                        actor.sheet.train(4, 3);
                    }
                }
            }
            actor.sheet.train(skill, 1);
        }
        if let Some(actor) = &mut battle.units[defender].theus {
            match result.outcome {
                Outcome::Dodge => {
                    actor.spend(1.);
                    actor.perform(Technique::Cartwheel, battle.tick, 7);
                    actor.sheet.train(2, 1);
                }
                Outcome::Parry => {
                    actor.spend(1.);
                    actor.perform(Technique::Counter, battle.tick, 8);
                    actor.sheet.train(0, 1);
                }
                Outcome::Hit | Outcome::Critical => {
                    actor.exposed_until = battle.tick + if ranged { 20 } else { 8 };
                    actor.opener = Some(attacker);
                    if matches!(result.critical, Critical::OffBalance | Critical::GuardSplit) {
                        actor.stagger_until = battle.tick + 3;
                    }
                    if result.critical == Critical::Launch || technique == Technique::AegisLaunch {
                        actor.airborne_since = battle.tick;
                        actor.airborne_until = battle.tick + 12;
                        actor.launcher = Some(attacker);
                    }
                    if technique == Technique::MeteorRelay {
                        actor.airborne_until = battle.tick + 8;
                        actor.exposed_until = battle.tick + 15;
                    }
                }
                _ => {}
            }
        }
        if result.damage > 0. {
            damage[defender] += result.damage;
            battle.units[attacker].damage_dealt += result.damage;
            resolved.push((attacker, defender, result.damage, blunt, speed, contact));
        } else if ranged {
            battle.units[attacker].rocks_hit = battle.units[attacker].rocks_hit.saturating_sub(1);
        }
        let sequence = battle.theus_events.last().map_or(0, |e| e.sequence + 1);
        battle.theus_events.push(Event {
            sequence,
            tick: battle.tick,
            attacker,
            defender,
            technique,
            partner,
            resolution: result,
        });
    }
    if battle.theus_events.len() > 256 {
        battle.theus_events.drain(..battle.theus_events.len() - 256);
    }
    *impacts = resolved;
}
