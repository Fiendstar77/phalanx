//! Persistent war-hound prototype. Separate actors, local perception, legal ground
//! paths, effort and opposed Theus checks. Companion art can be replaced independently.
use crate::{
    Battle, DT, Unit, distance,
    navigation::{self, Route},
    perception::{Contact, Source},
    skills::Skill,
    theus::{self, Resolution, Sheet, Strike},
};
use serde::{Deserialize, Serialize};
/// Pug-sized physical profile, matching the rig's quarter-scale source geometry.
pub const EYE_HEIGHT: f32 = 0.34;
pub const BITE_REACH: f32 = 0.70;
const BITE_RESOLVE_REACH: f32 = 0.80;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Hound {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_identity: Option<String>,
    pub identity: String,
    pub name: String,
    pub sheet: Sheet,
    pub recalls: u16,
    pub scars: u16,
    pub recovery_bouts: u8,
}
impl Hound {
    pub fn new(owner: &str) -> Self {
        Self {
            source_identity: None,
            identity: format!("{owner}/hound"),
            name: ["Morrow", "Pith", "Thimble", "Rusk", "Moth", "Suture"][owner
                .bytes()
                .fold(0usize, |h, b| h.wrapping_mul(31).wrapping_add(b as usize))
                % 6]
            .into(),
            sheet: Sheet {
                attributes: [11, 13, 8, 9],
                ..Default::default()
            },
            recalls: 0,
            scars: 0,
            recovery_bouts: 0,
        }
    }
    pub fn valid(&self, owner: &str) -> bool {
        self.identity == format!("{owner}/hound")
            && !self.name.is_empty()
            && self.name.chars().count() <= 32
            && !self.name.chars().any(char::is_control)
            && self.sheet.valid()
            && self.recalls <= 600
            && self.scars <= 600
            && self.recovery_bouts <= 3
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Order {
    #[default]
    Heel,
    Scout,
    Guard,
    Harass,
    Recall,
    Down,
}
impl Order {
    pub fn label(self) -> &'static str {
        match self {
            Self::Heel => "HEEL",
            Self::Scout => "SCOUT",
            Self::Guard => "GUARD",
            Self::Harass => "HARRY",
            Self::Recall => "RECALL",
            Self::Down => "DOWN",
        }
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Awareness {
    pub available: bool,
    pub known: bool,
    pub health: f32,
    pub effort: f32,
    pub distance: f32,
    pub order: Order,
    pub reports: u16,
    pub returns: u16,
}
pub fn unaware(a: &Awareness) -> bool {
    !a.available
}
#[derive(Clone, Debug, Serialize)]
pub struct Actor {
    pub owner: usize,
    pub hound: Hound,
    pub position: [f32; 2],
    pub health: f32,
    pub effort: f32,
    pub order: Order,
    pub goal: [f32; 2],
    pub target: Option<usize>,
    pub winding: Option<(usize, u32)>,
    pub cooldown: u32,
    pub route: Route,
    pub cursor: usize,
    pub last_owner: [f32; 2],
    pub reports: u16,
    pub returned: bool,
    pub contacted: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    OrderChosen,
    ContactReported,
    RecallReached,
    BiteResolved,
    CounterResolved,
    CareReceived,
    Downed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub sequence: u64,
    pub tick: u32,
    pub companion: String,
    pub owner: usize,
    pub other: Option<usize>,
    pub kind: Kind,
    pub order: Order,
    pub position: [f32; 2],
    pub amount: f32,
    pub resolution: Option<Resolution>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Field {
    pub actors: Vec<Actor>,
    pub events: Vec<Event>,
}
impl Field {
    pub fn empty(&self) -> bool {
        self.actors.is_empty() && self.events.is_empty()
    }
    fn record(
        &mut self,
        a: &Actor,
        tick: u32,
        kind: Kind,
        other: Option<usize>,
        amount: f32,
        resolution: Option<Resolution>,
    ) {
        let sequence = self.events.last().map_or(0, |e| e.sequence + 1);
        self.events.push(Event {
            sequence,
            tick,
            companion: a.hound.identity.clone(),
            owner: a.owner,
            other,
            kind,
            order: a.order,
            position: a.position,
            amount,
            resolution,
        });
        if self.events.len() > 256 {
            self.events.remove(0);
        }
    }
}
pub fn begin(b: &mut Battle, teams: &mut [crate::progression::Phalanx; 2]) {
    let n = b.team_size();
    for u in &b.units {
        if !u.equipped_skill(Skill::PackCall) {
            continue;
        }
        let f = &mut teams[u.team].fighters[u.id % n];
        let h = f.hound.get_or_insert_with(|| Hound::new(&f.fighter_id));
        if h.recovery_bouts > 0 {
            continue;
        }
        let p = [
            u.position[0] + if u.team == 0 { 0.8 } else { -0.8 },
            u.position[1] + 0.8,
        ];
        let p = if b.arena.blocked(p) { u.position } else { p };
        b.kennel.actors.push(Actor {
            owner: u.id,
            hound: h.clone(),
            position: p,
            health: 55.,
            effort: h.sheet.endurance(),
            order: Order::Heel,
            goal: p,
            target: None,
            winding: None,
            cooldown: 0,
            route: Route::default(),
            cursor: 0,
            last_owner: u.position,
            reports: 0,
            returned: false,
            contacted: false,
        });
    }
}
pub fn settle(teams: &mut [crate::progression::Phalanx; 2], b: &Battle) {
    for team in teams.iter_mut() {
        for f in &mut team.fighters {
            if let Some(h) = &mut f.hound {
                if let Some(a) = b
                    .kennel
                    .actors
                    .iter()
                    .find(|a| a.hound.identity == h.identity)
                {
                    *h = a.hound.clone();
                    if a.health <= 0. {
                        h.scars = h.scars.saturating_add(1).min(600);
                        h.recovery_bouts = 2;
                    }
                    if a.returned {
                        h.recalls = h.recalls.saturating_add(1).min(600);
                    }
                } else {
                    h.recovery_bouts = h.recovery_bouts.saturating_sub(1);
                }
            }
        }
    }
}
fn sensed(b: &Battle, a: &Actor, u: &Unit) -> bool {
    crate::perception::can_see_from(b.arena, a.position, u.position, EYE_HEIGHT, 0.95)
        && b.ninjutsu.clear_sight(
            [
                a.position[0],
                b.arena.elevation(a.position) + EYE_HEIGHT,
                a.position[1],
            ],
            [
                u.position[0],
                b.arena.elevation(u.position) + 0.95,
                u.position[1],
            ],
            b.tick as f32,
        )
}
fn move_ground(b: &Battle, a: &mut Actor, owner: &Unit, contacts: &[Contact]) {
    if b.tick.is_multiple_of(10)
        || distance(a.goal, *a.route.points.last().unwrap_or(&a.position)) > 1.2
    {
        let threats: Vec<_> = contacts
            .iter()
            .filter(|c| c.confidence(b.tick) > 0.3)
            .map(|c| navigation::Threat {
                position: c.position,
                range: c.range,
                confidence: c.confidence(b.tick),
            })
            .collect();
        a.route = navigation::plan_known(b.arena, a.position, a.goal, owner.mind, &threats);
        a.cursor = 1;
    }
    let speed = match a.order {
        Order::Recall => 2.8,
        Order::Scout | Order::Harass => 2.2,
        _ if distance(a.position, a.goal) > 2. => 1.8,
        _ => 0.65,
    };
    let mut remaining = DT * speed * (0.6 + 0.4 * a.effort / a.hound.sheet.endurance());
    while remaining > 0. && a.cursor < a.route.points.len() {
        let q = a.route.points[a.cursor];
        let d = distance(a.position, q);
        if d < 0.01 {
            a.cursor += 1;
            continue;
        }
        let step = remaining.min(d) / (1. + b.arena.roughness(q) * 0.9);
        let t = step / d;
        let p = [
            a.position[0] + (q[0] - a.position[0]) * t,
            a.position[1] + (q[1] - a.position[1]) * t,
        ];
        if !b.arena.walk_segment(a.position, p) {
            a.route = Route::default();
            break;
        }
        a.position = p;
        remaining -= step;
        if step >= d - 0.001 {
            a.cursor += 1;
        }
    }
}
pub fn step(b: &mut Battle) {
    if !b.founders || b.kennel.actors.is_empty() {
        return;
    }
    let mut field = std::mem::take(&mut b.kennel);
    for i in 0..field.actors.len() {
        let mut a = field.actors[i].clone();
        let owner = b.units[a.owner].clone();
        a.effort = (a.effort + DT * 2.).min(a.hound.sheet.endurance());
        if a.health <= 0. {
            a.order = Order::Down;
            field.actors[i] = a;
            continue;
        }
        let communicated =
            distance(owner.position, a.position) <= crate::perception::COMMUNICATION_RANGE;
        if communicated {
            a.last_owner = owner.position;
        }
        let visible: Vec<_> = b
            .units
            .iter()
            .filter(|u| u.team != owner.team && u.health > 0. && sensed(b, &a, u))
            .map(|u| Contact {
                enemy: u.id,
                position: u.position,
                health: u.health,
                role: u.role,
                range: u.attack_range(),
                seen_tick: b.tick,
                source: Source::Companion(i),
                visible: false,
            })
            .collect();
        if communicated
            && let Some(c) = visible.iter().min_by(|x, y| {
                distance(a.position, x.position).total_cmp(&distance(a.position, y.position))
            })
        {
            if !b.units[a.owner]
                .contacts
                .iter()
                .any(|old| old.enemy == c.enemy && old.seen_tick >= c.seen_tick)
            {
                b.units[a.owner].contacts.push(c.clone());
            }
            if !a.contacted {
                a.reports = a.reports.saturating_add(1);
                a.contacted = true;
                field.record(&a, b.tick, Kind::ContactReported, Some(c.enemy), 0., None);
            }
        }
        let target = visible.iter().min_by(|x, y| {
            distance(a.position, x.position).total_cmp(&distance(a.position, y.position))
        });
        let recall = (communicated
            && (owner.health <= 0.
                || owner.health_fraction() < 0.35
                || crate::tactics::recovering(&owner.tactics)
                || !owner.equipped_skill(Skill::PackCall)))
            || a.health < 18. + a.hound.scars.min(4) as f32 * 2.
            || a.effort < 6.;
        let desired = if recall {
            Order::Recall
        } else if !communicated {
            Order::Heel
        } else if target.is_some() && owner.tactics.phase == crate::tactics::Phase::Engage {
            Order::Harass
        } else if owner.tactics.scout == Some(owner.id)
            && !owner.contacts.iter().any(|c| c.confidence(b.tick) > 0.3)
        {
            Order::Scout
        } else {
            Order::Guard
        };
        if desired != a.order {
            let cost = if owner.equipped_skill(Skill::PackRecall) {
                1.5
            } else {
                3.
            };
            if desired == Order::Recall
                || !communicated
                || b.units[a.owner]
                    .theus
                    .as_mut()
                    .is_some_and(|actor| actor.spend(cost))
            {
                a.order = desired;
                a.winding = None;
                field.record(
                    &a,
                    b.tick,
                    Kind::OrderChosen,
                    target.map(|t| t.enemy),
                    0.,
                    None,
                );
            }
        }
        a.target = if a.order == Order::Harass {
            target.map(|t| t.enemy)
        } else {
            None
        };
        let ahead = if owner.team == 0 { 1. } else { -1. };
        a.goal = match a.order {
            Order::Recall | Order::Heel => a.last_owner,
            Order::Scout => [owner.position[0] + ahead * 4., owner.position[1] + 1.3],
            Order::Harass => target.map_or(a.last_owner, |c| {
                [
                    c.position[0] - ahead * 0.30,
                    c.position[1] + if i % 2 == 0 { 0.42 } else { -0.42 },
                ]
            }),
            _ => [
                owner.position[0] + ahead * (1.1 - (a.hound.recalls as f32 * 0.04).min(0.4)),
                owner.position[1] + 1.1,
            ],
        };
        if b.arena.blocked(a.goal) {
            a.goal = a.last_owner;
        }
        if a.winding.is_none() {
            move_ground(b, &mut a, &owner, &visible);
        }
        if a.order == Order::Recall
            && communicated
            && distance(a.position, owner.position) < 1.5
            && !a.returned
        {
            a.returned = true;
            field.record(&a, b.tick, Kind::RecallReached, Some(owner.id), 0., None);
            a.hound.sheet.train(4, 1);
        }
        if a.order == Order::Harass
            && let Some(c) = target
            && b.tick >= a.cooldown
            && distance(a.position, c.position) < BITE_REACH
            && a.effort >= 4.
        {
            a.winding.get_or_insert((c.enemy, b.tick + 6));
        }
        if let Some((enemy, end)) = a.winding {
            // A threatened fighter can spend their own attack and effort repelling
            // the hound. This consumes cooldown; there is no free counter attack.
            if b.units[enemy].health > 0.
                && distance(a.position, b.units[enemy].position) < 1.8
                && sensed(b, &a, &b.units[enemy])
                && b.units[enemy].cooldown <= 0.
                && b.units[enemy].windup.is_none()
            {
                let defender = &mut b.units[enemy];
                if defender.theus.as_mut().is_some_and(|actor| {
                    actor.stagger_until <= b.tick
                        && actor.airborne_until <= b.tick
                        && actor.spend(3.)
                }) {
                    defender.cooldown = 1.1;
                    defender.decision_reason = format!("Repel {} before its bite", a.hound.name);
                    let sheet = defender.theus.as_ref().unwrap().sheet;
                    let r = theus::resolve(
                        b.seed ^ u64::from(b.tick).rotate_left(11) ^ i as u64 ^ 0xd093,
                        Strike {
                            attack: sheet.rating(0, 0),
                            defence: a.hound.sheet.rating(2, 0),
                            damage: 9.,
                            armour: 0.,
                            parry: false,
                            exposed: a.order == Order::Harass,
                            critical_bonus: 0,
                        },
                    );
                    a.health = (a.health - r.damage).max(0.);
                    field.record(
                        &a,
                        b.tick,
                        Kind::CounterResolved,
                        Some(enemy),
                        r.damage,
                        Some(r),
                    );
                }
            }
            if b.tick >= end {
                a.winding = None;
                a.cooldown = b.tick + 22;
                let legal = a.health > 0.
                    && b.units[enemy].health > 0.
                    && distance(a.position, b.units[enemy].position) < BITE_RESOLVE_REACH
                    && sensed(b, &a, &b.units[enemy])
                    && crate::ninjutsu::physical_height(&b.units[enemy], b.arena, b.tick as f32)
                        < 0.65;
                if legal {
                    a.effort -= 4.;
                    a.hound.sheet.train(0, 1);
                    let target = &b.units[enemy];
                    let sheet = crate::theus_combat::sheet(target);
                    let r = theus::resolve(
                        b.seed ^ u64::from(b.tick).rotate_left(19) ^ i as u64 ^ 0xb17e,
                        Strike {
                            attack: a.hound.sheet.rating(0, 0),
                            defence: sheet.rating(2, 0),
                            damage: 5.,
                            armour: if target.equipment.has(crate::equipment::Gear::IronArmor) {
                                2.
                            } else {
                                0.
                            },
                            parry: false,
                            exposed: false,
                            critical_bonus: -20,
                        },
                    );
                    let damage = r.damage.min(b.units[enemy].health);
                    if damage > 0. {
                        b.units[enemy].health = (b.units[enemy].health - damage).max(0.);
                        if let Some(actor) = &mut b.units[enemy].theus {
                            actor.stagger_until = actor.stagger_until.max(b.tick + 3);
                            match r.critical {
                                theus::Critical::OffBalance => actor.stagger_until = b.tick + 8,
                                theus::Critical::GuardSplit => actor.exposed_until = b.tick + 12,
                                theus::Critical::Launch => {
                                    actor.airborne_until = b.tick + 8;
                                    actor.stagger_until = b.tick + 10;
                                }
                                _ => {}
                            }
                        }
                        let region = crate::injuries::lacerate(
                            b.seed,
                            b.next_damage_event,
                            &mut b.units[enemy],
                            damage,
                            true,
                        );
                        b.damage_events.push(crate::DamageEvent {
                            source_actor: Some(a.hound.identity.clone()),
                            sequence: b.next_damage_event,
                            tick: b.tick,
                            attacker: a.owner,
                            defender: enemy,
                            amount: damage,
                            velocity: [0., 0.],
                            position: b.units[enemy].position,
                            severed: 0,
                            blunt: false,
                            flesh_region: region,
                        });
                        b.next_damage_event += 1;
                    }
                    field.record(&a, b.tick, Kind::BiteResolved, Some(enemy), damage, Some(r));
                }
            }
        }
        if a.health <= 0. {
            a.order = Order::Down;
            a.winding = None;
            field.record(&a, b.tick, Kind::Downed, None, 0., None);
        }
        if communicated {
            b.units[a.owner].pack = Awareness {
                available: true,
                known: true,
                health: a.health / 55.,
                effort: a.effort / a.hound.sheet.endurance(),
                distance: distance(a.position, owner.position),
                order: a.order,
                reports: a.reports,
                returns: a
                    .hound
                    .recalls
                    .saturating_add(u16::from(a.returned))
                    .min(600),
            };
        } else {
            b.units[a.owner].pack.known = false;
        }
        field.actors[i] = a;
    }
    b.kennel = field;
}
