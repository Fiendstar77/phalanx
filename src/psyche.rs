//! Personality-specific scars. The same event can mark two fighters differently.
//! This is not a sanity meter, and it does not overwrite the seven mind traits.
use crate::{Mind, distance};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Reaction {
    #[default]
    None,
    Fear,
    Hesitation,
    OverAggression,
    Attachment,
}
impl Reaction {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Fear => "fear",
            Self::Hesitation => "hesitation",
            Self::OverAggression => "recklessness",
            Self::Attachment => "attachment",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub reaction: Reaction,
    pub intensity: f32,
}
impl Default for Response {
    fn default() -> Self {
        Self {
            reaction: Reaction::None,
            intensity: 0.0,
        }
    }
}
impl Response {
    pub fn valid(self) -> bool {
        self.intensity.is_finite()
            && (0.0..=1.0).contains(&self.intensity)
            && ((self.reaction == Reaction::None) == (self.intensity == 0.0))
    }
    fn reinforce(&mut self, reaction: Reaction, gain: f32) {
        if gain <= 0.0 {
            return;
        }
        if self.reaction == Reaction::None || self.reaction == reaction {
            self.reaction = reaction;
            self.intensity = (self.intensity + gain).min(1.0);
        } else {
            // A contradicting personality wears the old scar away before a new one forms.
            self.intensity = (self.intensity - gain).max(0.0);
            if self.intensity == 0.0 {
                self.reaction = Reaction::None;
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Psyche {
    pub isolation: Response,
    pub encirclement: Response,
    pub ally_loss: Response,
    pub failed_charge: Response,
    pub confidence: f32,
}
pub fn is_blank_field(psyche: &Psyche) -> bool {
    psyche.is_blank()
}
impl Psyche {
    pub fn is_blank(self) -> bool {
        self.isolation.reaction == Reaction::None
            && self.encirclement.reaction == Reaction::None
            && self.ally_loss.reaction == Reaction::None
            && self.failed_charge.reaction == Reaction::None
            && self.confidence == 0.0
    }
    pub fn valid(self) -> bool {
        self.confidence.is_finite()
            && (0.0..=1.0).contains(&self.confidence)
            && self.isolation.valid()
            && self.encirclement.valid()
            && self.ally_loss.valid()
            && self.failed_charge.valid()
    }
    pub fn summary(self) -> String {
        if self.is_blank() {
            return "Mind: no conditioned reactions yet".into();
        }
        let mut parts = Vec::new();
        for (name, slot) in [
            ("isolation", self.isolation),
            ("encirclement", self.encirclement),
            ("ally loss", self.ally_loss),
            ("failed charge", self.failed_charge),
        ] {
            if slot.reaction != Reaction::None {
                parts.push(format!(
                    "{name} {} {:.0}%",
                    slot.reaction.label(),
                    slot.intensity * 100.0
                ));
            }
        }
        if self.confidence > 0.0 {
            parts.push(format!("confidence {:.0}%", self.confidence * 100.0));
        }
        format!("Mind: {}", parts.join(" · "))
    }
    /// One battle plants at most 0.55 intensity. A second deep exposure can change the call.
    pub fn absorb(&mut self, mind: Mind, exposure: Exposure) {
        self.isolation.reinforce(
            reaction_to(mind, Trigger::Isolation),
            tick_gain(exposure.isolation_ticks, 40),
        );
        self.encirclement.reinforce(
            reaction_to(mind, Trigger::Encirclement),
            tick_gain(exposure.encirclement_ticks, 15),
        );
        self.ally_loss.reinforce(
            reaction_to(mind, Trigger::AllyLoss),
            count_gain(exposure.ally_losses, 0.25, 0.1),
        );
        self.failed_charge.reinforce(
            reaction_to(mind, Trigger::FailedCharge),
            count_gain(exposure.failed_charges, 0.2, 0.1),
        );
        if exposure.survived_pressure {
            let gain = 0.08 * (0.4 + 0.6 * mind.courage) * (1.2 - 0.5 * mind.self_preservation);
            self.confidence = (self.confidence + gain).min(1.0);
        } else if exposure.broke {
            self.confidence = (self.confidence - 0.05).max(0.0);
        }
    }
}
fn tick_gain(ticks: u32, ignore: u32) -> f32 {
    if ticks <= ignore {
        0.0
    } else {
        ((ticks - ignore) as f32 / 120.0).min(0.55)
    }
}
fn count_gain(count: u32, first: f32, extra: f32) -> f32 {
    if count == 0 {
        0.0
    } else {
        (first + extra * (count - 1) as f32).min(0.55)
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Exposure {
    pub isolation_ticks: u32,
    pub encirclement_ticks: u32,
    pub ally_losses: u32,
    pub failed_charges: u32,
    pub survived_pressure: bool,
    pub broke: bool,
}
#[derive(Clone, Copy, Debug)]
enum Trigger {
    Isolation,
    Encirclement,
    AllyLoss,
    FailedCharge,
}
fn reaction_to(mind: Mind, trigger: Trigger) -> Reaction {
    match trigger {
        Trigger::Isolation => {
            if mind.teamwork >= 0.55 && mind.teamwork >= mind.aggression {
                Reaction::Attachment
            } else if mind.caution + mind.self_preservation >= mind.courage + mind.aggression {
                Reaction::Fear
            } else if mind.aggression >= 0.7 && mind.courage >= 0.6 {
                Reaction::OverAggression
            } else {
                Reaction::Hesitation
            }
        }
        Trigger::Encirclement => {
            if mind.caution >= 0.65 || mind.self_preservation >= 0.75 {
                Reaction::Fear
            } else if mind.discipline >= 0.7 {
                Reaction::Hesitation
            } else if mind.aggression >= 0.7 {
                Reaction::OverAggression
            } else {
                Reaction::Hesitation
            }
        }
        Trigger::AllyLoss => {
            if mind.teamwork >= 0.6 && mind.teamwork >= mind.aggression {
                Reaction::Attachment
            } else if mind.self_preservation >= 0.6 && mind.caution >= mind.aggression {
                Reaction::Fear
            } else if mind.aggression >= 0.7 {
                Reaction::OverAggression
            } else {
                Reaction::Hesitation
            }
        }
        Trigger::FailedCharge => {
            if mind.aggression >= 0.7 && mind.discipline < 0.5 {
                Reaction::OverAggression
            } else {
                Reaction::Hesitation
            }
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Moment {
    pub isolated: bool,
    pub encircled: bool,
    pub ally_hurt: bool,
    pub closing: bool,
}
pub struct ScarLine {
    pub action: usize,
    pub text: &'static str,
}
/// No living ally within 5 units. An empty squad counts as isolated.
pub fn isolated(origin: [f32; 2], allies: &[[f32; 2]]) -> bool {
    allies.iter().all(|ally| distance(origin, *ally) > 5.0)
}
/// Two close enemies on opposite sides, or three enemies within 4.5 units.
pub fn encircled(origin: [f32; 2], enemies: &[[f32; 2]]) -> bool {
    let near: Vec<[f32; 2]> = enemies
        .iter()
        .copied()
        .filter(|enemy| distance(origin, *enemy) <= 4.5)
        .collect();
    if near.len() >= 3 {
        return true;
    }
    near.iter().enumerate().any(|(i, a)| {
        near.iter().skip(i + 1).any(|b| {
            let ax = a[0] - origin[0];
            let ay = a[1] - origin[1];
            let bx = b[0] - origin[0];
            let by = b[1] - origin[1];
            ax * bx + ay * by < 0.0
        })
    })
}
/// Adjust utility scores already computed for this tick. Illegal attacks stay illegal.
/// Lab battles pass a blank psyche and leave scores untouched.
pub fn bias_scores(psyche: Psyche, moment: Moment, scores: &mut [f32; 9]) -> Option<ScarLine> {
    if psyche.is_blank() {
        return None;
    }
    let mut best: Option<(usize, f32, &'static str)> = None;
    let mut bump = |action: usize, delta: f32, open: bool, text: &'static str| {
        if delta.abs() < 1e-6 {
            return;
        }
        if scores[action] < 0.0 {
            if !(open && delta > 0.0) {
                return;
            }
            scores[action] = delta;
        } else {
            scores[action] += delta;
            if scores[action] < 0.0 {
                scores[action] = -1.0;
            }
        }
        if delta > 0.0 && best.as_ref().is_none_or(|(_, amount, _)| delta > *amount) {
            best = Some((action, delta, text));
        }
    };
    let intensity = psyche.isolation.intensity;
    match psyche.isolation.reaction {
        Reaction::Fear if moment.isolated => {
            bump(
                4,
                0.85 * intensity,
                true,
                "Isolation fear pulls them back to the squad",
            );
            bump(5, 0.50 * intensity, true, "Isolation fear pulls them back");
            bump(0, -0.50 * intensity, false, "");
        }
        Reaction::Attachment if moment.isolated => {
            bump(
                4,
                1.10 * intensity,
                true,
                "Attachment pulls them back to the squad",
            );
            bump(0, -0.30 * intensity, false, "");
        }
        Reaction::OverAggression if moment.isolated => {
            bump(0, 0.50 * intensity, false, "Isolation drives a charge");
            bump(4, -0.35 * intensity, false, "");
        }
        Reaction::Hesitation if moment.isolated => {
            bump(6, 0.90 * intensity, true, "Isolation makes them hesitate");
            bump(0, -0.55 * intensity, false, "");
        }
        _ => {}
    }
    let intensity = psyche.encirclement.intensity;
    match psyche.encirclement.reaction {
        Reaction::Fear if moment.encircled => {
            bump(
                5,
                0.95 * intensity,
                true,
                "Encirclement fear: they break out",
            );
            bump(1, -0.70 * intensity, false, "");
            bump(0, -0.55 * intensity, false, "");
        }
        Reaction::Hesitation if moment.encircled => {
            bump(6, 0.80 * intensity, true, "Encirclement makes them hold");
            bump(0, -0.45 * intensity, false, "");
            bump(1, -0.45 * intensity, false, "");
        }
        Reaction::OverAggression if moment.encircled => {
            bump(0, 0.45 * intensity, false, "Encirclement makes them charge");
            bump(1, 0.25 * intensity, false, "Encirclement makes them strike");
        }
        _ => {}
    }
    let intensity = psyche.ally_loss.intensity;
    match psyche.ally_loss.reaction {
        Reaction::Attachment if moment.ally_hurt => {
            bump(
                8,
                0.55 * intensity,
                false,
                "Ally loss: they stay with the wounded",
            );
            bump(
                4,
                0.40 * intensity,
                true,
                "Ally loss pulls them back to the squad",
            );
        }
        Reaction::Fear if moment.ally_hurt => {
            bump(5, 0.50 * intensity, true, "Ally loss frightens them");
            bump(0, -0.30 * intensity, false, "");
        }
        Reaction::OverAggression if moment.ally_hurt => {
            bump(0, 0.50 * intensity, false, "Ally loss drives a charge");
        }
        Reaction::Hesitation if moment.ally_hurt => {
            bump(6, 0.40 * intensity, true, "Ally loss makes them hesitate");
        }
        _ => {}
    }
    let intensity = psyche.failed_charge.intensity;
    match psyche.failed_charge.reaction {
        Reaction::Hesitation if moment.closing => {
            bump(0, -0.60 * intensity, false, "");
            bump(
                6,
                0.80 * intensity,
                true,
                "The failed charge makes them hesitate",
            );
            bump(
                7,
                0.35 * intensity,
                false,
                "The failed charge sends them to cover",
            );
        }
        Reaction::OverAggression if moment.closing => {
            bump(
                0,
                0.45 * intensity,
                false,
                "The failed charge makes them try harder",
            );
        }
        _ => {}
    }
    if psyche.confidence >= 0.25 {
        let confidence = psyche.confidence;
        bump(
            1,
            0.12 * confidence,
            false,
            "Confidence keeps them in the fight",
        );
        bump(
            0,
            0.08 * confidence,
            false,
            "Confidence keeps them advancing",
        );
        bump(5, -0.15 * confidence, false, "");
    }
    best.filter(|(_, amount, _)| *amount >= 0.2)
        .map(|(action, _, text)| ScarLine { action, text })
}
pub fn bark(psyche: Psyche, moment: Moment) -> Option<&'static str> {
    if moment.encircled && psyche.encirclement.intensity > 0.0 {
        return match psyche.encirclement.reaction {
            Reaction::Fear => Some("THEY'RE ALL AROUND!"),
            Reaction::OverAggression => Some("COME ON THEN!"),
            Reaction::Hesitation => Some("HOLD!"),
            Reaction::Attachment => Some("TOGETHER!"),
            Reaction::None => None,
        };
    }
    if moment.isolated && psyche.isolation.intensity > 0.0 {
        return match psyche.isolation.reaction {
            Reaction::Fear => Some("DON'T LEAVE ME!"),
            Reaction::Attachment => Some("WAIT FOR ME!"),
            Reaction::OverAggression => Some("THEN I GO ALONE!"),
            Reaction::Hesitation => Some("NOT AGAIN..."),
            Reaction::None => None,
        };
    }
    if moment.ally_hurt && psyche.ally_loss.intensity > 0.0 {
        return match psyche.ally_loss.reaction {
            Reaction::Attachment => Some("FOR THEM!"),
            Reaction::Fear => Some("WE'RE FALLING!"),
            Reaction::OverAggression => Some("YOU'LL PAY!"),
            Reaction::Hesitation => Some("NOT AGAIN..."),
            Reaction::None => None,
        };
    }
    if moment.closing && psyche.failed_charge.intensity > 0.0 {
        return match psyche.failed_charge.reaction {
            Reaction::Hesitation => Some("NOT LIKE THAT!"),
            Reaction::OverAggression => Some("AGAIN!"),
            _ => None,
        };
    }
    None
}
