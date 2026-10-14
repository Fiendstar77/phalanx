use phalanx::{
    Battle, Mind, cognition,
    perception::{self, Contact, MEMORY_TICKS, Source},
    terrain::Arena,
};
fn sparse(arena: Arena, ids: &[usize]) -> Battle {
    let mut b = Battle::with_arena(42, false, arena);
    for u in &mut b.units {
        u.health = if ids.contains(&u.id) { 100.0 } else { 0.0 };
    }
    b
}
fn report_fixture() -> Battle {
    let mut b = sparse(Arena::Choke, &[0, 1, 5]);
    b.units[0].position = [-3.0, 4.0];
    b.units[1].position = [-1.5, 0.0];
    b.units[5].position = [3.0, 0.0];
    b.units[0].mind = Mind::disciplined();
    b.units[1].mind = Mind::disciplined();
    b
}
#[test]
fn walls_and_range_limit_direct_perception() {
    let mut b = report_fixture();
    b.units[0].mind.teamwork = 0.0;
    b.units[0].mind.discipline = 0.0;
    perception::update(&mut b.units, b.arena, 0);
    assert_eq!(b.units[0].visible_enemies, 0);
    assert!(b.units[0].contacts.is_empty());
    assert_eq!(b.units[1].visible_enemies, 1);
    assert!(!perception::can_see(
        Arena::OpenField,
        [-8.0, 0.0],
        [8.0, 0.0]
    ));
    assert!(!perception::can_see(Arena::Ridge, [-8.0, 0.0], [8.0, 0.0]));
}
#[test]
fn cooperative_allies_share_sightings_without_granting_direct_sight() {
    let mut b = report_fixture();
    perception::update(&mut b.units, b.arena, 0);
    let receiver = &b.units[0];
    assert_eq!(receiver.visible_enemies, 0);
    assert_eq!(receiver.contacts.len(), 1);
    assert_eq!(receiver.contacts[0].source, Source::Ally(1));
    assert!(!receiver.contacts[0].visible);
    assert_eq!(receiver.contacts[0].position, [3.0, 0.0]);
    assert_eq!(receiver.shared_reports, 1);
    let mut isolated = report_fixture();
    isolated.units[1].mind.teamwork = 0.0;
    isolated.units[1].mind.discipline = 0.0;
    perception::update(&mut isolated.units, isolated.arena, 0);
    assert!(isolated.units[0].contacts.is_empty());
}
#[test]
fn memory_keeps_last_observed_position_and_expires_without_echoes() {
    let mut b = report_fixture();
    perception::update(&mut b.units, b.arena, 0);
    b.units[5].position = [9.0, 6.0];
    perception::update(&mut b.units, b.arena, 1);
    assert_eq!(b.units[0].contacts[0].position, [3.0, 0.0]);
    assert_eq!(b.units[1].contacts[0].position, [3.0, 0.0]);
    assert!(!b.units[1].contacts[0].visible);
    assert_eq!(b.units[0].contacts[0].seen_tick, 0);
    let first = b.units[0].contacts[0].confidence(1);
    for tick in 2..=MEMORY_TICKS {
        perception::update(&mut b.units, b.arena, tick);
    }
    assert!(b.units[0].contacts[0].confidence(MEMORY_TICKS) < first);
    perception::update(&mut b.units, b.arena, MEMORY_TICKS + 1);
    assert!(b.units[0].contacts.is_empty());
    assert!(b.units[1].contacts.is_empty());
}
#[test]
fn moving_an_unseen_enemy_does_not_change_an_agents_decision_or_route() {
    let mut a = sparse(Arena::Choke, &[0, 5]);
    a.units[0].position = [-4.0, 4.0];
    a.units[5].position = [4.0, 4.0];
    let mut b = a.clone();
    b.units[5].position = [7.0, 5.0];
    a.step();
    b.step();
    assert!(a.units[0].contacts.is_empty());
    assert!(b.units[0].contacts.is_empty());
    assert_eq!(a.units[0].enemy, None);
    assert_eq!(a.units[0].action, b.units[0].action);
    assert_eq!(a.units[0].scores, b.units[0].scores);
    assert_eq!(a.units[0].target, b.units[0].target);
    assert_eq!(a.units[0].position, b.units[0].position);
    assert_eq!(
        serde_json::to_string(&a.units[0].route).unwrap(),
        serde_json::to_string(&b.units[0].route).unwrap()
    );
}
#[test]
fn teamwork_changes_support_for_a_known_wounded_ally() {
    let mut high = sparse(Arena::OpenField, &[0, 1, 5]);
    high.units[0].position = [-4.0, 0.0];
    high.units[0].mind = Mind::disciplined();
    high.units[1].position = [0.0, 0.0];
    high.units[1].health = 30.0;
    high.units[5].position = [5.0, 0.0];
    let mut low = high.clone();
    low.units[0].mind.teamwork = 0.0;
    high.step();
    low.step();
    assert_eq!(high.units[0].action, 8);
    assert_eq!(high.units[0].support_target, Some(1));
    assert_ne!(low.units[0].action, 8);
}
#[test]
fn cooperative_target_selection_can_follow_an_allied_focus() {
    let mut b = sparse(Arena::OpenField, &[0, 1, 5, 6]);
    b.units[0].position = [0.0, 0.0];
    b.units[0].mind = Mind::disciplined();
    b.units[1].mind = Mind::disciplined();
    b.units[1].enemy = Some(5);
    b.units[0].contacts = vec![5, 6]
        .into_iter()
        .map(|id| Contact {
            enemy: id,
            position: [if id == 5 { 4.0 } else { 3.5 }, 0.0],
            health: 100.0,
            role: b.units[id].role,
            seen_tick: 0,
            source: Source::OwnSight,
            visible: true,
        })
        .collect();
    assert_eq!(
        cognition::focus_target(&b.units[0], &[&b.units[1]], 0)
            .unwrap()
            .enemy,
        5
    );
    b.units[0].mind.teamwork = 0.0;
    b.units[0].mind.discipline = 0.0;
    assert_eq!(
        cognition::focus_target(&b.units[0], &[&b.units[1]], 0)
            .unwrap()
            .enemy,
        6
    );
}
#[test]
fn an_engaged_ally_can_make_a_disciplined_fighter_choose_a_flank() {
    let mut b = sparse(Arena::OpenField, &[0, 1, 5]);
    b.units[0].mind = Mind::disciplined();
    b.units[0].position = [-3.6, 0.0];
    b.units[1].position = [-1.0, 0.0];
    b.units[1].enemy = Some(5);
    b.units[1].action = 1;
    b.units[5].position = [0.4, 0.0];
    b.step();
    assert_eq!(b.units[0].action, 3);
    assert_ne!(b.units[0].target, b.units[5].position);
    assert!(b.units[0].flank_anchor.is_some());
    assert!(b.units[0].target[1].abs() > 1.0);
}

#[test]
fn ridge_occludes_low_observers_and_high_ground_improves_sight() {
    assert!(!perception::can_see(Arena::Ridge, [-4.0, 0.0], [4.0, 0.0]));
    assert!(perception::can_see(Arena::Ridge, [0.0, 0.0], [8.0, 0.0]));
    assert!(perception::can_see(
        Arena::OpenField,
        [-4.5, -4.5],
        [-4.5, 5.0]
    ));
    assert!(!perception::can_see(
        Arena::Ridge,
        [-4.5, -4.5],
        [-4.5, 5.0]
    ));
}
#[test]
fn a_remembered_position_inside_weapon_range_does_not_authorize_a_throw() {
    let mut b = sparse(Arena::OpenField, &[0, 5]);
    b.units[0].role = phalanx::Role::RockThrower;
    b.units[0].position = [0.0, 0.0];
    b.units[5].position = [10.5, 7.5];
    let enemy_role = b.units[5].role;
    b.units[0].contacts.push(Contact {
        enemy: 5,
        position: [2.0, 0.0],
        health: 100.0,
        role: enemy_role,
        seen_tick: 0,
        source: Source::OwnSight,
        visible: false,
    });
    b.step();
    assert_eq!(b.units[0].visible_enemies, 0);
    assert_eq!(b.units[0].enemy, Some(5));
    assert!(b.rocks.is_empty());
    assert_eq!(b.units[5].health, 100.0);
}
#[test]
fn reports_do_not_reach_teammates_outside_communication_range() {
    let mut b = report_fixture();
    b.units[0].position = [-8.0, 5.0];
    perception::update(&mut b.units, b.arena, 0);
    assert_eq!(b.units[1].visible_enemies, 1);
    assert!(b.units[0].contacts.is_empty());
}
