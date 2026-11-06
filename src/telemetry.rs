//! Exact decision-time observations, captured before attacks and movement.
use crate::{ACTION_COUNT, ACTIONS, Mind, Unit, perception::Contact};
use serde::Serialize;
#[derive(Serialize)]
pub struct DecisionRecord {
    pub event: &'static str,
    pub phase: &'static str,
    pub tick: u32,
    pub unit: usize,
    pub team: usize,
    pub position: [f32; 2],
    pub health: f32,
    pub role: crate::Role,
    pub mind: Mind,
    pub contacts: Vec<Contact>,
    pub personal_memory: crate::social::Memory,
    pub companion_report: crate::kennel::Awareness,
    pub scores: [f32; ACTION_COUNT],
    pub action: &'static str,
    pub target: [f32; 2],
    pub enemy: Option<usize>,
    pub support_target: Option<usize>,
    pub reason: String,
    pub features: Vec<f32>,
    pub comparison: Option<crate::learned::Comparison>,
    pub action_mask: Vec<bool>,
}
impl DecisionRecord {
    pub fn capture(tick: u32, u: &Unit, features: Vec<f32>) -> Self {
        Self {
            event: "decision",
            phase: "decision_before_movement",
            tick,
            unit: u.id,
            team: u.team,
            position: u.position,
            health: u.health,
            role: u.role,
            mind: u.mind,
            contacts: u.contacts.clone(),
            personal_memory: u.social.clone(),
            companion_report: u.pack.clone(),
            scores: u.scores,
            action: ACTIONS[u.action],
            target: u.target,
            enemy: u.enemy,
            support_target: u.support_target,
            reason: u.decision_reason.clone(),
            features,
            comparison: u.comparison.clone(),
            action_mask: u.scores.iter().map(|s| *s >= 0.0).collect(),
        }
    }
}
pub const FEATURE_NAMES: &[&str] = &[
    "aggression",
    "caution",
    "courage",
    "teamwork",
    "patience",
    "self_preservation",
    "discipline",
    "health",
    "thrower",
    "x",
    "z",
    "elevation",
    "cover",
    "rough",
    "enemy_distance",
    "enemy_visible",
    "known_enemies",
    "enemy_health",
    "contact_confidence",
    "nearby_allies",
    "isolation",
    "local_threat",
    "support_urgency",
    "engaged_allies",
    "ridge_arrived",
    "hold_allowed",
    "flank_ready",
    "contact_exists",
    "flank_remaining",
    "trusted_support",
    "recognized_rival",
    "flank_habit",
    "regroup_habit",
    "effort",
    "bleeding",
    "unfitted_limbs",
    "healer_capability",
    "vault_capability",
    "melee_reach",
    "pack_available",
    "pack_report_current",
    "pack_health",
    "pack_effort",
    "pack_distance",
    "pack_scout",
    "pack_guard",
    "pack_harry",
    "pack_recall",
    "pack_reports",
    "pack_returns",
    "tactical_recovery",
    "tactical_scout",
    "tactical_assemble",
    "cloud_step_ready",
    "equipment_speed",
    "equipment_mitigation",
    "equipment_damage",
];

/// Internal memory and communicated state only; no hidden opponent or companion transforms.
pub fn context_features(u: &Unit, allies: &[&Unit], enemy: Option<&Contact>) -> [f32; 28] {
    let effort = u
        .theus
        .as_ref()
        .map_or(0., |a| a.stamina / a.sheet.endurance());
    let p = &u.pack;
    let known = p.known;
    [
        allies
            .iter()
            .map(|a| u.social.bond(a.id))
            .fold(0., f32::max),
        enemy.map_or(0., |e| u.social.caution(e.enemy)),
        (u.social.habits[0] as f32 / 8.).min(1.),
        (u.social.habits[1] as f32 / 8.).min(1.),
        effort,
        (u.bleeding_loss / 100.).min(1.),
        (u.lost_limbs & !u.prosthetics).count_ones() as f32 / 4.,
        f32::from(u.has_skill(crate::FighterClass::Healer)),
        f32::from(crate::ninjutsu::vault_body_allowed(u)),
        u.attack_range() / 10.,
        f32::from(p.available),
        f32::from(known),
        if known { p.health } else { 0. },
        if known { p.effort } else { 0. },
        if known { (p.distance / 8.).min(1.) } else { 0. },
        f32::from(known && p.order == crate::kennel::Order::Scout),
        f32::from(known && p.order == crate::kennel::Order::Guard),
        f32::from(known && p.order == crate::kennel::Order::Harass),
        f32::from(known && p.order == crate::kennel::Order::Recall),
        (p.reports as f32 / 4.).min(1.),
        (p.returns as f32 / 6.).min(1.),
        f32::from(crate::tactics::recovering(&u.tactics)),
        f32::from(u.tactics.scout == Some(u.id)),
        f32::from(u.tactics.phase == crate::tactics::Phase::Assemble),
        f32::from(
            crate::aerial::qualified(u)
                && u.theus.as_ref().is_some_and(|a| a.clock >= u.aerial_ready),
        ),
        u.equipment.speed(),
        u.equipment.incoming(),
        u.equipment.damage(),
    ]
}
