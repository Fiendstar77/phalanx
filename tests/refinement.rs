use phalanx::{pressure, procedural, progression::Campaign, terrain::Arena};

#[test]
fn seeded_tactical_zones_connect_and_destroyed_cover_opens_space() {
    let c = Campaign::new();
    let mut layouts = std::collections::BTreeSet::new();
    for seed in 0..32 {
        let Arena::Pressure(field) = pressure::generate(&c.teams, seed) else {
            panic!()
        };
        assert_eq!(field.seed, seed);
        assert!(procedural::connected(field), "disconnected seed {seed}");
        layouts.insert(field.layout);
        let cleared = Arena::Pressure(pressure::PressureField {
            destroyed: 15,
            ..field
        });
        assert!(cleared.walls().is_empty());
        for (center, _) in procedural::original_walls(field) {
            assert!(Arena::Pressure(field).blocked(*center));
            assert!(!cleared.blocked(*center));
        }
        let encoded = serde_json::to_string(&Arena::Pressure(field)).unwrap();
        assert_eq!(
            serde_json::from_str::<Arena>(&encoded).unwrap(),
            Arena::Pressure(field)
        );
    }
    assert!(layouts.len() >= 4, "layouts: {layouts:?}");
}

#[test]
fn all_five_rivals_keep_independent_progress_through_swaps_and_serialization() {
    let mut c = Campaign::new();
    assert_eq!(c.rivals.len(), 5);
    c.validate().unwrap();
    for index in 0..5 {
        let name = c.rivals[index].name.clone();
        c.select_rival(index).unwrap();
        assert_eq!(c.teams[1].name, name);
        c.teams[1].fighters[0].experience += 50;
        c.select_rival(index).unwrap();
        assert_eq!(c.rivals[index].fighters[0].experience, 50);
    }
    let encoded = serde_json::to_string(&c).unwrap();
    let loaded: Campaign = serde_json::from_str(&encoded).unwrap();
    loaded.validate().unwrap();
    assert_eq!(serde_json::to_string(&loaded).unwrap(), encoded);
}

#[test]
fn rival_match_earns_progress_once_and_seeded_battle_replays() {
    let mut c = Campaign::new();
    c.select_rival(2).unwrap();
    let field = pressure::generate(&c.teams, 123);
    let (id, mut a) = c
        .begin_battle(123, field, phalanx::learned::Controller::Utility)
        .unwrap();
    let mut b = a.clone();
    while !a.finished {
        a.step();
        b.step();
    }
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
    assert!(!a.damage_events.is_empty());
    assert!(
        a.damage_events
            .iter()
            .all(|e| e.amount > 0. && e.velocity.iter().all(|v| v.is_finite()))
    );
    let reserve = serde_json::to_string(&c.rivals).unwrap();
    c.settle(id, &a).unwrap();
    assert!(
        c.teams[1]
            .fighters
            .iter()
            .all(|f| f.battles == 1 && f.experience > 0)
    );
    assert_eq!(serde_json::to_string(&c.rivals).unwrap(), reserve);
    let settled = serde_json::to_string(&c).unwrap();
    let _ = c.settle(id, &a);
    assert_eq!(serde_json::to_string(&c).unwrap(), settled);
}

#[test]
fn remembered_enemy_can_trigger_real_barricade_destruction_and_open_navigation() {
    let field = pressure::PressureField {
        layout: 1,
        reach: 1,
        ..Default::default()
    };
    let mut b = phalanx::Battle::with_founders(42, false, Arena::Pressure(field));
    for u in &mut b.units {
        u.health = 0.;
    }
    b.units[0].health = 100.;
    b.units[0].position = [-5.2, -2.5];
    b.units[3].health = 100.;
    b.units[3].position = [-2.5, -2.5];
    let enemy = b.units[3].clone();
    b.units[0].contacts.push(phalanx::perception::Contact {
        enemy: 3,
        position: enemy.position,
        health: 100.,
        role: enemy.role,
        range: enemy.attack_range(),
        seen_tick: 0,
        source: phalanx::perception::Source::OwnSight,
        visible: false,
    });
    b.barricade_health[0] = 1.;
    assert!(!b.arena.walk_segment([-5.2, -2.5], [-2.5, -2.5]));
    b.step();
    let Arena::Pressure(after) = b.arena else {
        panic!()
    };
    assert_ne!(after.destroyed & 1, 0);
    assert!(b.arena.walk_segment([-5.2, -2.5], [-2.5, -2.5]));
    assert_eq!(b.initial_arena, Arena::Pressure(field));
}
