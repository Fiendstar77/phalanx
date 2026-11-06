//! Directional personal memories. Only experienced or witnessed events teach them.
//! Temperament and match outcomes never train these records or grant combat stats.
use crate::{Battle, Unit, distance, navigation::Route, perception::Contact};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    HealedMe,
    StoodBetween,
    RecognizedHarm,
    FlankArrived,
    RegroupArrived,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::HealedMe => "treated my wounds",
            Self::StoodBetween => "stood between me and a known threat",
            Self::RecognizedHarm => "hurt me while recognized",
            Self::FlankArrived => "reached a flank",
            Self::RegroupArrived => "reached a recovery rally",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    pub battle_id: Option<u64>,
    pub seed: u64,
    pub tick: u32,
    pub kind: Kind,
    pub other: String,
    pub source: u64,
    pub amount: f32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Link {
    pub identity: String,
    pub aid: u16,
    pub screens: u16,
    pub hurts: u16,
    #[serde(skip)]
    pub actor: Option<usize>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Memory {
    pub links: Vec<Link>,
    pub habits: [u16; 2],
    pub receipts: Vec<Receipt>,
}
pub fn blank(m: &Memory) -> bool {
    m.links.is_empty() && m.habits == [0; 2] && m.receipts.is_empty()
}
impl Memory {
    pub fn valid(&self, next: u64) -> bool {
        self.links.len() <= 24
            && self.receipts.len() <= 64
            && self.habits.iter().all(|v| *v <= 600)
            && self.links.iter().all(|l| {
                !l.identity.is_empty()
                    && l.identity.len() <= 128
                    && !l.identity.chars().any(char::is_control)
                    && l.aid <= 600
                    && l.screens <= 600
                    && l.hurts <= 600
            })
            && self
                .links
                .iter()
                .enumerate()
                .all(|(i, l)| self.links[..i].iter().all(|p| p.identity != l.identity))
            && self.receipts.iter().all(|r| {
                r.battle_id.is_some_and(|id| id < next)
                    && r.tick <= crate::BATTLE_TICK_LIMIT
                    && r.other.len() <= 128
                    && !r.other.chars().any(char::is_control)
                    && r.amount.is_finite()
                    && (0.0..=1000.).contains(&r.amount)
            })
    }
    pub fn bond(&self, actor: usize) -> f32 {
        self.links
            .iter()
            .find(|l| l.actor == Some(actor))
            .map_or(0., |l| {
                ((l.aid as f32 + l.screens as f32 * 0.75) / 6.).min(1.)
            })
    }
    pub fn caution(&self, actor: usize) -> f32 {
        self.links
            .iter()
            .find(|l| l.actor == Some(actor))
            .map_or(0., |l| (l.hurts.saturating_sub(1) as f32 / 5.).min(1.))
    }
    pub fn settle(&mut self, id: u64) {
        for r in &mut self.receipts {
            if r.battle_id.is_none() {
                r.battle_id = Some(id);
            }
        }
    }
    pub fn summary_named(&self, teams: &[crate::progression::Phalanx]) -> String {
        let mut text = format!(
            "PERSONAL MEMORY | {} known individuals\nHabits: {} reached flanks / {} recovery rallies",
            self.links.len(),
            self.habits[0],
            self.habits[1]
        );
        for r in self.receipts.iter().rev().take(5) {
            let name = teams
                .iter()
                .flat_map(|t| &t.fighters)
                .find(|f| f.fighter_id == r.other || f.source_identity() == r.other)
                .map(|f| f.name.as_str())
                .unwrap_or(if r.other.is_empty() {
                    "Self"
                } else {
                    "Remembered veteran"
                });
            text.push_str(&format!(
                "\n{name}: {} | bout {}, tick {}",
                r.kind.label(),
                r.battle_id.unwrap_or(0),
                r.tick
            ));
        }
        text
    }
    pub fn summary(&self) -> String {
        let mut s = format!(
            "PERSONAL MEMORY | {} known individuals\nHabits: {} reached flanks / {} reached recovery rallies",
            self.links.len(),
            self.habits[0],
            self.habits[1]
        );
        for r in self.receipts.iter().rev().take(5) {
            s.push_str(&format!(
                "\n{}: {} | bout {}, tick {}",
                r.other,
                r.kind.label(),
                r.battle_id.unwrap_or(0),
                r.tick
            ));
        }
        s
    }
    fn record(&mut self, receipt: Receipt, actor: Option<usize>) -> bool {
        // A repeated heal/attack animation is not a new lifetime bond every frame.
        if self
            .receipts
            .iter()
            .any(|r| r.battle_id.is_none() && r.kind == receipt.kind && r.other == receipt.other)
        {
            return false;
        }
        match receipt.kind {
            Kind::FlankArrived => self.habits[0] = (self.habits[0] + 1).min(600),
            Kind::RegroupArrived => self.habits[1] = (self.habits[1] + 1).min(600),
            _ => {
                if !self.links.iter().any(|l| l.identity == receipt.other) {
                    if self.links.len() >= 24 {
                        return false;
                    }
                    self.links.push(Link {
                        identity: receipt.other.clone(),
                        actor,
                        ..Default::default()
                    });
                }
                let l = self
                    .links
                    .iter_mut()
                    .find(|l| l.identity == receipt.other)
                    .unwrap();
                l.actor = actor;
                let count = match receipt.kind {
                    Kind::HealedMe => &mut l.aid,
                    Kind::StoodBetween => &mut l.screens,
                    _ => &mut l.hurts,
                };
                *count = count.saturating_add(1).min(600);
            }
        }
        self.receipts.push(receipt);
        if self.receipts.len() > 64 {
            self.receipts.remove(0);
        }
        true
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub sequence: u64,
    pub tick: u32,
    pub observer: usize,
    pub other: Option<usize>,
    pub kind: Kind,
    pub source: u64,
    pub amount: f32,
}
pub fn bind(units: &mut [Unit]) {
    let ids: Vec<_> = units.iter().map(|u| u.fighter_id.clone()).collect();
    for u in units {
        for l in &mut u.social.links {
            l.actor = ids
                .iter()
                .position(|id| id.as_deref() == Some(l.identity.as_str()));
        }
    }
}
pub fn target_bias(unit: &Unit, c: &Contact) -> f32 {
    // Familiarity can create apprehension or fixation, never extra sight or power.
    unit.social.caution(c.enemy) * (unit.mind.caution * 1.5 - unit.mind.aggression * 0.8)
}
pub fn habit_bias(unit: &mut Unit) {
    if unit.scores[3] >= 0. {
        unit.scores[3] += (unit.social.habits[0] as f32 / 8.).min(0.12);
    }
    if unit.scores[5] >= 0. {
        unit.scores[5] += (unit.social.habits[1] as f32 / 8.).min(0.12);
    }
}
fn reached(route: &Route, position: [f32; 2]) -> bool {
    route
        .points
        .last()
        .is_some_and(|p| distance(*p, position) < 0.85)
}
pub fn observe(b: &mut Battle) {
    if !b.founders {
        return;
    }
    let mut events = Vec::new();
    for h in b
        .heals
        .iter()
        .filter(|h| h.tick == b.tick && h.amount > 0. && h.healer != h.target)
    {
        events.push((
            h.target,
            Some(h.healer),
            Kind::HealedMe,
            h.healer as u64,
            h.amount.min(1000.),
        ));
    }
    for e in b
        .damage_events
        .iter()
        .filter(|e| e.tick == b.tick && e.amount > 0. && e.source_actor.is_none())
    {
        if b.units[e.defender]
            .contacts
            .iter()
            .any(|c| c.enemy == e.attacker && c.confidence(b.tick) > 0.3)
        {
            events.push((
                e.defender,
                Some(e.attacker),
                Kind::RecognizedHarm,
                e.sequence,
                e.amount.min(1000.),
            ));
        }
    }
    for e in b.tactical_events.iter().filter(|e| e.tick == b.tick) {
        if e.kind == crate::tactics::Kind::ScreenPositionReached
            && let Some(patient) = e.subject.filter(|p| *p != e.unit)
        {
            let a = &b.units[patient];
            let guard = &b.units[e.unit];
            if distance(a.position, guard.position) <= 8.
                && crate::perception::can_see(b.arena, a.position, guard.position)
                && a.contacts.iter().any(|c| c.confidence(b.tick) > 0.3)
            {
                events.push((patient, Some(e.unit), Kind::StoodBetween, e.sequence, 0.));
            }
        }
        if e.kind == crate::tactics::Kind::RallyReached && e.subject == Some(e.unit) {
            events.push((e.unit, None, Kind::RegroupArrived, e.sequence, 0.));
        }
    }
    for u in &b.units {
        if u.action == 3
            && u.health > 0.
            && u.distance_moved > 2.
            && reached(&u.route, u.position)
            && u.contacts.iter().any(|c| c.confidence(b.tick) > 0.3)
        {
            events.push((u.id, None, Kind::FlankArrived, 0, 0.));
        }
    }
    for (observer, other, kind, source, amount) in events {
        let identity = other
            .and_then(|id| b.units.get(id))
            .and_then(|u| u.fighter_id.clone())
            .unwrap_or_default();
        if other.is_some() && identity.is_empty() {
            continue;
        }
        let receipt = Receipt {
            battle_id: None,
            seed: b.seed,
            tick: b.tick,
            kind,
            other: identity,
            source,
            amount,
        };
        if b.units[observer].social.record(receipt, other) {
            let sequence = b.social_events.last().map_or(0, |e| e.sequence + 1);
            b.social_events.push(Event {
                sequence,
                tick: b.tick,
                observer,
                other,
                kind,
                source,
                amount,
            });
            if b.social_events.len() > 256 {
                b.social_events.remove(0);
            }
        }
    }
}

/// Exhibition runtime IDs are namespaced; memories retain original evidence IDs.
pub fn bind_aliases(units: &mut [Unit], teams: &[crate::progression::Phalanx; 2]) {
    let n = teams[0].fighters.len();
    for u in units {
        for l in &mut u.social.links {
            if l.actor.is_none() {
                let ally = l.aid > 0 || l.screens > 0;
                let side = if ally { u.team } else { 1 - u.team };
                l.actor = teams[side]
                    .fighters
                    .iter()
                    .position(|f| f.source_identity() == l.identity)
                    .map(|i| side * n + i);
            }
        }
    }
}
