//! Deliberate campaign opening from each fighter's own contacts and nearby allies.
//! No enemy world transforms or unseen health are consulted.
use crate::{FighterClass, Unit, distance, terrain::Arena};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Phase {
    #[default]
    Reconnaissance,
    Assemble,
    Engage,
    Withdraw,
    Regroup,
    Reapproach,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct State {
    pub scout: Option<usize>,
    pub first_report: Option<u32>,
    pub phase: Phase,
    pub approach: u8,
    pub since: u32,
    pub lane: i8,
    pub rally: Option<[f32; 2]>,
    pub patient: Option<usize>,
    pub screen: Option<usize>,
    pub goal: Option<[f32; 2]>,
    pub treated: bool,
    pub screened: bool,
    pub reached: bool,
    pub retry_after: u32,
    pub committed: bool,
    pub window_tick: u32,
    pub window_health: f32,
    pub window_damage: f32,
    pub window_position: [f32; 2],
}
pub fn is_blank(state: &State) -> bool {
    state.scout.is_none() && !state.committed && !manoeuvring(state)
}
pub struct Intent {
    pub goal: [f32; 2],
    pub reason: String,
    pub screen: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    ApproachChosen,
    ApproachAbandoned,
    WithdrawalJoined,
    RallyReached,
    ScreenPositionReached,
    TreatmentReceived,
    ReapproachChosen,
    Reengaged,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cause {
    #[default]
    Contact,
    LosingExchange,
    Bleeding,
    StalledApproach,
    AllyWithdrawal,
    Treatment,
    Recovered,
    RegroupDeadline,
    RouteReached,
    RouteDeadline,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub sequence: u64,
    pub tick: u32,
    pub unit: usize,
    pub approach: u8,
    pub kind: Kind,
    pub cause: Cause,
    pub subject: Option<usize>,
    pub position: [f32; 2],
    pub goal: [f32; 2],
    pub amount: f32,
}
impl Event {
    pub fn valid(&self, count: usize) -> bool {
        self.unit < count
            && self.subject.is_none_or(|s| s < count)
            && self.approach <= 2
            && self.tick <= crate::BATTLE_TICK_LIMIT
            && self.amount.is_finite()
            && (0.0..=1000.).contains(&self.amount)
            && self
                .position
                .iter()
                .chain(&self.goal)
                .all(|x| x.is_finite() && x.abs() <= 1000.)
    }
    pub fn description(&self) -> &'static str {
        match self.kind {
            Kind::ApproachChosen => "Approach committed from known contacts",
            Kind::ApproachAbandoned => match self.cause {
                Cause::Bleeding => "Approach abandoned: bleeding",
                Cause::LosingExchange => "Approach abandoned: taking damage without useful return",
                _ => "Approach abandoned: progress stalled",
            },
            Kind::WithdrawalJoined => "Joined a nearby ally's withdrawal plan",
            Kind::RallyReached => "Reached the rally position",
            Kind::ScreenPositionReached => {
                "Reached a screen position between patient and known threat"
            }
            Kind::TreatmentReceived => "Received actual healing during regrouping",
            Kind::ReapproachChosen => {
                if self.cause == Cause::RegroupDeadline {
                    "Regroup deadline: try a different approach"
                } else {
                    "Recovery observed: try a different approach"
                }
            }
            Kind::Reengaged => {
                if self.cause == Cause::RouteDeadline {
                    "Flank route deadline: resume local combat"
                } else {
                    "Reached the alternate approach; resume local combat"
                }
            }
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn record(
    events: &mut Vec<Event>,
    unit: &Unit,
    tick: u32,
    kind: Kind,
    cause: Cause,
    subject: Option<usize>,
    goal: [f32; 2],
    amount: f32,
) {
    let sequence = events.last().map_or(0, |e| e.sequence + 1);
    events.push(Event {
        sequence,
        tick,
        unit: unit.id,
        approach: unit.tactics.approach,
        kind,
        cause,
        subject,
        position: unit.position,
        goal,
        amount,
    });
    if events.len() > 128 {
        events.drain(..events.len() - 128);
    }
}
pub fn recovering(state: &State) -> bool {
    matches!(state.phase, Phase::Withdraw | Phase::Regroup)
}
pub fn manoeuvring(state: &State) -> bool {
    recovering(state) || state.phase == Phase::Reapproach
}
pub fn label(unit: &Unit) -> &'static str {
    match unit.tactics.phase {
        Phase::Reconnaissance => {
            if unit.tactics.scout == Some(unit.id) {
                "SCOUTING"
            } else {
                "STAGING"
            }
        }
        Phase::Assemble => "ASSEMBLING",
        Phase::Engage => "",
        Phase::Withdraw | Phase::Regroup => {
            if unit.tactics.patient == Some(unit.id) {
                "WITHDRAWING"
            } else if unit.tactics.screen == Some(unit.id) {
                "SCREENING"
            } else if unit.has_skill(FighterClass::Healer) {
                "MEDIC"
            } else {
                "REGROUPING"
            }
        }
        Phase::Reapproach => "NEW APPROACH",
    }
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

fn opening(
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
        screen: false,
    })
}

fn watch(unit: &mut Unit, tick: u32) {
    unit.tactics.window_tick = tick;
    unit.tactics.window_health = unit.health;
    unit.tactics.window_damage = unit.damage_dealt;
    unit.tactics.window_position = unit.position;
}
fn contact(unit: &Unit, tick: u32) -> Option<[f32; 2]> {
    unit.contacts
        .iter()
        .filter(|c| c.confidence(tick) > 0.15 && c.health > 0.)
        .min_by(|a, b| {
            distance(unit.position, a.position).total_cmp(&distance(unit.position, b.position))
        })
        .map(|c| c.position)
}
fn away(from: [f32; 2], enemy: Option<[f32; 2]>, side: f32, metres: f32) -> [f32; 2] {
    let enemy = enemy.unwrap_or([from[0] - side, from[1]]);
    let d = distance(from, enemy).max(0.01);
    [
        from[0] + (from[0] - enemy[0]) / d * metres,
        from[1] + (from[1] - enemy[1]) / d * metres,
    ]
}
pub(crate) fn intent(
    unit: &mut Unit,
    allies: &[&Unit],
    arena: Arena,
    tick: u32,
    team_size: usize,
    events: &mut Vec<Event>,
) -> Option<Intent> {
    unit.theus.as_ref()?;
    if matches!(unit.tactics.phase, Phase::Reconnaissance | Phase::Assemble) {
        if let Some(intent) = opening(unit, allies, arena, tick, team_size) {
            return Some(intent);
        }
        unit.tactics.phase = Phase::Engage;
    }
    let side = if unit.team == 0 { -1. } else { 1. };
    let threat = contact(unit, tick);
    if unit.tactics.phase == Phase::Engage
        && !unit.tactics.committed
        && let Some(known) = threat
    {
        unit.tactics.committed = true;
        unit.tactics.since = tick;
        watch(unit, tick);
        record(
            events,
            unit,
            tick,
            Kind::ApproachChosen,
            Cause::Contact,
            None,
            known,
            0.,
        );
    }
    // A plan is a report from a nearby known ally, not a team-wide telepathic order.
    if unit.tactics.phase == Phase::Engage
        && tick >= unit.tactics.retry_after
        && unit.tactics.approach < 2
        && unit.mind.teamwork >= 0.30
        && let Some(ally) = allies
            .iter()
            .filter(|a| recovering(&a.tactics) && a.tactics.rally.is_some())
            .min_by_key(|a| (a.tactics.patient.unwrap_or(a.id), a.id))
    {
        unit.tactics.phase = Phase::Withdraw;
        unit.tactics.since = ally.tactics.since;
        unit.tactics.approach += 1;
        unit.tactics.lane = ally.tactics.lane;
        unit.tactics.rally = ally.tactics.rally;
        unit.tactics.patient = ally.tactics.patient;
        unit.tactics.screen = ally.tactics.screen;
        unit.tactics.treated = false;
        unit.tactics.reached = false;
        unit.tactics.screened = false;
        record(
            events,
            unit,
            tick,
            Kind::WithdrawalJoined,
            Cause::AllyWithdrawal,
            unit.tactics.patient,
            unit.tactics.rally.unwrap(),
            0.,
        );
    }
    if unit.tactics.phase == Phase::Engage
        && unit.tactics.committed
        && unit.tactics.approach < 2
        && tick >= unit.tactics.retry_after
        && let Some(known) = threat
    {
        let elapsed = tick.saturating_sub(unit.tactics.window_tick);
        let loss = unit.tactics.window_health - unit.health;
        let gain = unit.damage_dealt - unit.tactics.window_damage;
        let risk =
            (0.085 + unit.mind.aggression * 0.025 - unit.mind.caution * 0.02) * unit.max_health();
        let cause = if unit.flesh.bleeding() && unit.health_fraction() < 0.85 {
            Some(Cause::Bleeding)
        } else if elapsed >= 12 && loss >= risk && gain < loss * 0.55 {
            Some(Cause::LosingExchange)
        } else if elapsed >= 60
            && distance(unit.position, unit.tactics.window_position) < 0.7
            && gain < 1.
            && distance(unit.position, known) > unit.attack_range() + 0.8
        {
            Some(Cause::StalledApproach)
        } else {
            None
        };
        if let Some(cause) = cause {
            let medic = allies
                .iter()
                .filter(|a| a.has_skill(FighterClass::Healer))
                .min_by(|a, b| {
                    distance(unit.position, a.position)
                        .total_cmp(&distance(unit.position, b.position))
                });
            let base = medic.map_or(unit.position, |m| m.position);
            let rally = legal_goal(
                arena,
                away(base, threat, side, if medic.is_some() { 1.5 } else { 4.5 }),
            );
            let guard = allies
                .iter()
                .filter(|a| !a.has_skill(FighterClass::Healer) && a.health_fraction() > 0.55)
                .max_by(|a, b| {
                    screen_value(a)
                        .total_cmp(&screen_value(b))
                        .then(b.id.cmp(&a.id))
                })
                .map(|a| a.id);
            unit.tactics.approach += 1;
            unit.tactics.lane = if unit.tactics.lane == 0 {
                if unit.position[1] <= 0. { 1 } else { -1 }
            } else {
                -unit.tactics.lane
            };
            unit.tactics.phase = Phase::Withdraw;
            unit.tactics.since = tick;
            unit.tactics.rally = Some(rally);
            unit.tactics.patient = Some(unit.id);
            unit.tactics.screen = guard;
            unit.tactics.treated = false;
            unit.tactics.reached = false;
            unit.tactics.screened = false;
            record(
                events,
                unit,
                tick,
                Kind::ApproachAbandoned,
                cause,
                Some(unit.id),
                rally,
                loss.max(0.),
            );
        } else if elapsed >= 60 {
            watch(unit, tick);
        }
    }
    if recovering(&unit.tactics) {
        let rally = unit.tactics.rally.unwrap_or(unit.position);
        let patient = if unit.tactics.patient == Some(unit.id) {
            Some(&*unit)
        } else {
            allies
                .iter()
                .copied()
                .find(|a| Some(a.id) == unit.tactics.patient)
        };
        let recovered = patient.is_some_and(|p| {
            !p.flesh.bleeding()
                && (p.health_fraction() >= 0.62 || p.tactics.treated && p.health_fraction() >= 0.40)
        });
        let elapsed = tick.saturating_sub(unit.tactics.since);
        if elapsed >= 140 || elapsed >= 35 && recovered {
            unit.tactics.phase = Phase::Reapproach;
            unit.tactics.since = tick;
            let enemy = threat.unwrap_or([0., 0.]);
            let goal = legal_goal(
                arena,
                [
                    enemy[0] + side * 3.,
                    unit.tactics.lane as f32 * arena.half_size()[1] * 0.42,
                ],
            );
            unit.tactics.goal = Some(goal);
            record(
                events,
                unit,
                tick,
                Kind::ReapproachChosen,
                if elapsed >= 140 {
                    Cause::RegroupDeadline
                } else {
                    Cause::Recovered
                },
                unit.tactics.patient,
                goal,
                0.,
            );
        } else {
            let patient_known = patient.is_some();
            let patient_position = patient.map_or(rally, |p| p.position);
            let screening = unit.tactics.screen == Some(unit.id);
            let goal = if unit.tactics.patient == Some(unit.id) {
                rally
            } else if screening {
                legal_goal(arena, away(patient_position, threat, side, -1.5))
            } else if unit.has_skill(FighterClass::Healer) {
                legal_goal(arena, away(patient_position, threat, side, 2.0))
            } else {
                rally
            };
            if unit.tactics.patient == Some(unit.id)
                && distance(unit.position, rally) < 1.4
                && !unit.tactics.reached
            {
                unit.tactics.reached = true;
                unit.tactics.phase = Phase::Regroup;
                record(
                    events,
                    unit,
                    tick,
                    Kind::RallyReached,
                    Cause::RouteReached,
                    Some(unit.id),
                    rally,
                    0.,
                );
            }
            if screening
                && patient_known
                && !unit.tactics.screened
                && distance(unit.position, goal) < 1.2
                && arena.clear_segment(unit.position, patient_position)
                && threat
                    .is_some_and(|e| distance(unit.position, e) < distance(patient_position, e))
            {
                unit.tactics.screened = true;
                record(
                    events,
                    unit,
                    tick,
                    Kind::ScreenPositionReached,
                    Cause::RouteReached,
                    unit.tactics.patient,
                    goal,
                    0.,
                );
            }
            return Some(Intent {
                goal,
                screen: screening,
                reason: if screening {
                    "Protect: occupy the route between the injured ally and known threat".into()
                } else if unit.has_skill(FighterClass::Healer)
                    && unit.tactics.patient != Some(unit.id)
                {
                    "Regroup: medic moves behind the patient; healing still needs real range and sight".into()
                } else {
                    "Withdraw: reach the rally point while allies screen and treat".into()
                },
            });
        }
    }
    if unit.tactics.phase == Phase::Reapproach {
        let goal = unit.tactics.goal.unwrap_or(unit.position);
        let arrived = distance(unit.position, goal) < 1.2;
        if arrived || tick.saturating_sub(unit.tactics.since) >= 120 {
            unit.tactics.phase = Phase::Engage;
            unit.tactics.retry_after = tick + 70;
            watch(unit, tick);
            record(
                events,
                unit,
                tick,
                Kind::Reengaged,
                if arrived {
                    Cause::RouteReached
                } else {
                    Cause::RouteDeadline
                },
                None,
                goal,
                0.,
            );
            return None;
        }
        return Some(Intent {
            goal,
            screen: false,
            reason: format!(
                "Reapproach: use the {} lane after abandoning the previous approach",
                if unit.tactics.lane > 0 {
                    "upper"
                } else {
                    "lower"
                }
            ),
        });
    }
    None
}
fn screen_value(u: &Unit) -> f32 {
    u.health_fraction()
        + u.mind.teamwork
        + u.mind.courage * 0.5
        + if matches!(
            u.class,
            Some(FighterClass::Guardian | FighterClass::Paladin)
        ) {
            2.
        } else {
            0.
        }
}
pub(crate) fn steer(unit: &mut Unit, intent: &Intent) {
    if !manoeuvring(&unit.tactics) {
        unit.scores[0] = unit.scores[0].max(2.25);
        return;
    }
    // Preserve valid attack/defence opportunities for the physical screen.
    if intent.screen && distance(unit.position, intent.goal) < 2. && unit.scores[1] >= 0. {
        return;
    }
    let best = unit.scores.iter().copied().fold(0., f32::max);
    unit.scores[0] = best + 0.5;
}
pub(crate) fn observe(battle: &mut crate::Battle) {
    for heal in battle
        .heals
        .iter()
        .filter(|h| h.tick == battle.tick && h.amount > 0.)
    {
        let unit = &mut battle.units[heal.target];
        if recovering(&unit.tactics) && !unit.tactics.treated {
            unit.tactics.treated = true;
            record(
                &mut battle.tactical_events,
                unit,
                battle.tick,
                Kind::TreatmentReceived,
                Cause::Treatment,
                Some(heal.healer),
                unit.position,
                heal.amount,
            );
        }
    }
}
