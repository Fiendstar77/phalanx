//! Coordination operates on the recipient's contacts and nearby allied information.
use crate::{
    Unit, distance,
    perception::{Contact, cooperative},
    terrain::Arena,
};
pub fn focus_target(unit: &Unit, allies: &[&Unit], tick: u32) -> Option<Contact> {
    let cost = |c: &Contact| {
        let shared_focus = if cooperative(unit.mind) {
            allies
                .iter()
                .filter(|a| cooperative(a.mind) && a.enemy == Some(c.enemy))
                .count() as f32
        } else {
            0.0
        };
        distance(unit.position, c.position)
            + if c.visible { 0.0 } else { 2.0 }
            + (1.0 - c.confidence(tick)) * 3.0
            - shared_focus * unit.mind.teamwork * 1.2
            - (1.0 - c.health / 100.0) * unit.mind.aggression * 0.7
    };
    unit.contacts
        .iter()
        .min_by(|a, b| cost(a).total_cmp(&cost(b)).then(a.enemy.cmp(&b.enemy)))
        .cloned()
}
pub fn support_ally<'a>(unit: &Unit, allies: &[&'a Unit], tick: u32) -> Option<(&'a Unit, f32)> {
    if unit.mind.teamwork < 0.5 {
        return None;
    }
    allies
        .iter()
        .filter_map(|ally| {
            let d = distance(unit.position, ally.position);
            if d < 2.3 {
                return None;
            }
            let injury = 1.0 - ally.health_fraction();
            let enemies = unit
                .contacts
                .iter()
                .filter(|c| c.confidence(tick) > 0.4 && distance(c.position, ally.position) < 3.5)
                .count() as f32;
            let nearby = allies
                .iter()
                .filter(|a| a.id != ally.id && distance(a.position, ally.position) < 2.8)
                .count()
                + usize::from(d < 2.8);
            if injury < 0.35 && (enemies < 1.0 || nearby >= 2) {
                return None;
            }
            let urgency = injury + (enemies * 0.25).min(0.75) + if nearby == 0 { 0.3 } else { 0.0 };
            Some((*ally, urgency))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.id.cmp(&a.0.id)))
}
pub fn support_position(arena: Arena, unit: &Unit, ally: &Unit) -> [f32; 2] {
    let back = if unit.is_ranged() { 1.8 } else { 0.8 };
    let p = [
        (ally.position[0] + if unit.team == 0 { -back } else { back })
            .clamp(-arena.half_size()[0] + 0.5, arena.half_size()[0] - 0.5),
        (ally.position[1] + if unit.id.is_multiple_of(2) { 0.8 } else { -0.8 })
            .clamp(-arena.half_size()[1] + 1.0, arena.half_size()[1] - 1.0),
    ];
    if arena.blocked(p) { ally.position } else { p }
}
pub fn flank_position(arena: Arena, unit: &Unit, enemy: &Contact) -> [f32; 2] {
    let side = if unit.id.is_multiple_of(2) { 1.0 } else { -1.0 };
    let own = if unit.team == 0 { -1.0 } else { 1.0 };
    for offset in [
        [own * 0.7, side * 2.8],
        [own * 2.5, side * 1.5],
        [own * 2.5, -side * 1.5],
    ] {
        let p = [
            (enemy.position[0] + offset[0])
                .clamp(-arena.half_size()[0] + 0.5, arena.half_size()[0] - 0.5),
            (enemy.position[1] + offset[1])
                .clamp(-arena.half_size()[1] + 1.0, arena.half_size()[1] - 1.0),
        ];
        if !arena.blocked(p) {
            return p;
        }
    }
    unit.position
}
