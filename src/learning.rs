//! Receipts for experience-driven adaptation. These describe persisted rule changes,
//! not updates to the frozen PyTorch policy or Bonsai weights.
use crate::{Mind, psyche::Psyche};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Learning {
    pub fighter_id: String,
    pub experience: [u32; 2],
    pub class_practice: [u32; 2],
    pub mind_before: Mind,
    pub mind_after: Mind,
    pub psyche_before: Psyche,
    pub psyche_after: Psyche,
}

fn traits(m: Mind) -> [f32; 7] {
    [
        m.aggression,
        m.caution,
        m.courage,
        m.teamwork,
        m.patience,
        m.self_preservation,
        m.discipline,
    ]
}

impl Learning {
    pub fn valid(&self) -> bool {
        !self.fighter_id.is_empty()
            && self.fighter_id.len() <= 128
            && !self.fighter_id.chars().any(char::is_control)
            && self.experience[0] <= self.experience[1]
            && self.class_practice[0] <= self.class_practice[1]
            && self
                .experience
                .iter()
                .chain(&self.class_practice)
                .all(|v| *v <= 1_000_000)
            && traits(self.mind_before)
                .into_iter()
                .chain(traits(self.mind_after))
                .all(|v| v.is_finite() && (0.0..=1.0).contains(&v))
            && self.psyche_before.valid()
            && self.psyche_after.valid()
    }

    /// Player-facing descriptions of actual deltas, with their existing rule triggers.
    pub fn changes(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let old_level = 1 + (self.experience[0] / 100).min(9);
        let new_level = 1 + (self.experience[1] / 100).min(9);
        if new_level > old_level {
            lines.push(format!(
                "LEVEL UP! {old_level} -> {new_level}. Experience {} -> {}.",
                self.experience[0], self.experience[1]
            ));
        }
        let before = traits(self.mind_before);
        let after = traits(self.mind_after);
        for (index, label, reason) in [
            (
                2,
                "Courage",
                "historical temperament adjustment; automatic outcome drift is retired",
            ),
            (
                3,
                "Teamwork",
                "historical temperament adjustment; event memories now remain separate",
            ),
            (
                5,
                "Self-preservation",
                "historical temperament adjustment; event memories now remain separate",
            ),
            (
                6,
                "Discipline",
                "historical temperament adjustment; automatic outcome drift is retired",
            ),
        ] {
            if after[index] != before[index] {
                lines.push(format!(
                    "{label} {:.1}% -> {:.1}%: {reason}.",
                    before[index] * 100.,
                    after[index] * 100.
                ));
            }
        }
        for (trigger, old, new) in [
            (
                "Isolation",
                self.psyche_before.isolation,
                self.psyche_after.isolation,
            ),
            (
                "Encirclement",
                self.psyche_before.encirclement,
                self.psyche_after.encirclement,
            ),
            (
                "Ally loss",
                self.psyche_before.ally_loss,
                self.psyche_after.ally_loss,
            ),
            (
                "Failed charge",
                self.psyche_before.failed_charge,
                self.psyche_after.failed_charge,
            ),
        ] {
            if old != new {
                lines.push(format!(
                    "{trigger} memory: {} {:.0}% -> {} {:.0}%. Reacts when that situation returns.",
                    old.reaction.label(),
                    old.intensity * 100.,
                    new.reaction.label(),
                    new.intensity * 100.
                ));
            }
        }
        if self.psyche_before.confidence != self.psyche_after.confidence {
            lines.push(format!("Legacy pressure confidence {:.1}% -> {:.1}%: historical value; inactive in current decisions.", self.psyche_before.confidence * 100., self.psyche_after.confidence * 100.));
        }
        let rank = |points| match points {
            600.. => 3,
            300.. => 2,
            100.. => 1,
            _ => 0,
        };
        let old_rank = rank(self.class_practice[0]);
        let new_rank = rank(self.class_practice[1]);
        if new_rank > old_rank {
            lines.push(format!(
                "CLASS RANK UP! {old_rank} -> {new_rank}. Practice is retained across retraining."
            ));
        }
        let points = self.class_practice[1];
        let (base, next) = match points {
            0..100 => (0, 100),
            100..300 => (100, 300),
            _ => (300, 600),
        };
        let filled = ((points.saturating_sub(base).min(next - base) * 12) / (next - base)) as usize;
        lines.push(format!(
            "Class practice {} -> {} [{}{}] {}.",
            self.class_practice[0],
            points,
            "#".repeat(filled),
            "-".repeat(12 - filled),
            if new_rank == 3 {
                "top rank".into()
            } else {
                format!("next rank at {next}")
            }
        ));
        lines
    }
}
