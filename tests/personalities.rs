use phalanx::{
    Battle, FighterClass as C,
    learned::Controller,
    perception::{Contact, Source},
    progression::Campaign,
    skills::Skill,
    social::{self, Kind},
    terrain::Arena,
};
fn campaign() -> Campaign {
    let mut c = Campaign::new();
    c.teams[0].fighters[0].class = C::Houndwarden;
    c.teams[1].fighters[0].class = C::Houndwarden;
    c
}
#[test]
fn actual_help_teaches_directional_memory_and_survives_save_and_reload_without_temperament_drift() {
    let mut c = Campaign::new();
    let mind = c.teams[0].fighters[0].mind;
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    b.heals.push(phalanx::HealEvent {
        tick: 0,
        healer: 2,
        target: 0,
        amount: 15.,
    });
    social::observe(&mut b);
    assert_eq!(b.units[0].social.links[0].aid, 1);
    assert!(b.units[2].social.links.is_empty());
    social::observe(&mut b);
    assert_eq!(b.units[0].social.links[0].aid, 1);
    b.finished = true;
    c.settle(id, &b).unwrap();
    let restored: Campaign = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.teams[0].fighters[0].mind, mind);
    assert_eq!(
        restored.teams[0].fighters[0].social.receipts[0].kind,
        Kind::HealedMe
    );
    assert_eq!(
        restored.teams[0].fighters[0].social.receipts[0].battle_id,
        Some(id)
    );
}
#[test]
fn bonded_introvert_supports_a_known_ally_but_a_hidden_rival_adds_no_features() {
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    b.units[0].mind.teamwork = 0.2;
    b.units[0].social.links.push(social::Link {
        identity: "ally".into(),
        aid: 3,
        actor: Some(1),
        ..Default::default()
    });
    b.units[0].position = [-4., 0.];
    b.units[1].position = [0., 0.];
    b.units[1].health = 25.;
    assert!(phalanx::cognition::support_ally(&b.units[0], &[&b.units[1]], 0).is_some());
    let features = phalanx::telemetry::context_features(&b.units[0], &[], None);
    b.units[3].position = [30., 30.];
    b.units[3].health = 0.;
    assert_eq!(
        features,
        phalanx::telemetry::context_features(&b.units[0], &[], None)
    );
}
#[test]
fn hounds_are_separate_actors_with_real_paths_opposed_bites_and_recall_receipts() {
    let mut c = campaign();
    let (_, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(b.kennel.actors.len(), 2);
    assert_eq!(b.units.len(), 6);
    for u in &mut b.units {
        u.tactics.phase = phalanx::tactics::Phase::Engage;
        u.position = [if u.team == 0 { -2. } else { 2. }, (u.id % 3) as f32 * 2.];
    }
    b.kennel.actors[0].position = [0.5, 0.];
    b.kennel.actors[1].position = [-0.5, 0.];
    for _ in 0..100 {
        b.step();
        if b.finished {
            break;
        }
    }
    assert!(
        b.kennel
            .events
            .iter()
            .any(|e| e.kind == phalanx::kennel::Kind::BiteResolved
                || e.kind == phalanx::kennel::Kind::CounterResolved)
    );
    assert!(
        b.kennel
            .events
            .iter()
            .filter_map(|e| e.resolution.as_ref())
            .all(|r| r.damage >= 0. && r.damage.is_finite())
    );
    assert!(b.kennel.actors.iter().all(|a| !b.arena.blocked(a.position)));
    b.finished = false;
    b.units[0].health = 40.;
    b.kennel.actors[0].health = 30.;
    b.kennel.actors[0].position = [b.units[0].position[0], b.units[0].position[1] + 3.];
    for _ in 0..80 {
        phalanx::kennel::step(&mut b);
        b.tick += 1;
    }
    assert!(
        b.kennel
            .events
            .iter()
            .any(|e| e.kind == phalanx::kennel::Kind::RecallReached)
    );
}
#[test]
fn distant_handler_changes_do_not_telepathically_change_a_hound() {
    let mut c = campaign();
    let (_, mut a) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    a.kennel.actors[0].position = [0., 7.];
    a.units[0].position = [-10., -7.];
    let mut b = a.clone();
    b.units[0].health = 1.;
    b.units[0].contacts.push(Contact {
        enemy: 3,
        position: [0., 0.],
        health: 300.,
        role: phalanx::Role::Fighter,
        range: 2.,
        seen_tick: 0,
        source: Source::OwnSight,
        visible: true,
    });
    phalanx::kennel::step(&mut a);
    phalanx::kennel::step(&mut b);
    assert_eq!(a.kennel.actors[0].position, b.kennel.actors[0].position);
    assert_eq!(a.kennel.actors[0].order, b.kennel.actors[0].order);
}
#[test]
fn retained_pack_skill_keeps_the_same_hound_and_injuries_persist() {
    let mut c = campaign();
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    let name = b.kennel.actors[0].hound.identity.clone();
    b.kennel.actors[0].health = 0.;
    b.finished = true;
    c.settle(id, &b).unwrap();
    let f = &mut c.teams[0].fighters[0];
    assert_eq!(f.hound.as_ref().unwrap().recovery_bouts, 2);
    f.mastery.train(C::Houndwarden, 100);
    f.class = C::Warrior;
    f.skills = Some(phalanx::skills::SkillLoadout::defaults(
        C::Houndwarden,
        f.mastery,
    ));
    let path = std::env::temp_dir().join(format!("phalanx-hound-{}.json", std::process::id()));
    c.save(&path).unwrap();
    let mut c = Campaign::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(
        c.teams[0].fighters[0].hound.as_ref().unwrap().identity,
        name
    );
    let (_, b) = c
        .begin_battle(43, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert!(b.units[0].equipped_skill(Skill::PackCall));
    assert!(b.kennel.actors.iter().all(|a| a.owner != 0));
}

#[test]
fn cloud_step_spends_effort_follows_its_real_arc_and_can_be_interrupted() {
    let mut c = Campaign::new();
    c.teams[0].fighters[0].class = C::Acrobat;
    c.teams[0].fighters[0].mind.courage = 0.9;
    c.teams[0].fighters[0].mind.caution = 0.2;
    c.teams[0].fighters[0].theus = Some(phalanx::theus::Sheet {
        attributes: [10, 16, 11, 11],
        practice: [0, 0, 100, 0, 0, 0],
        ..Default::default()
    });
    let (_, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    b.units[0].position = [-3., 0.];
    b.units[3].position = [3., 0.];
    b.units[0].tactics.phase = phalanx::tactics::Phase::Engage;
    b.units[0].contacts = vec![Contact {
        enemy: 3,
        position: [3., 0.],
        health: 300.,
        role: phalanx::Role::Fighter,
        range: 2.,
        seen_tick: 0,
        source: Source::OwnSight,
        visible: true,
    }];
    let effort = b.units[0].theus.as_ref().unwrap().stamina;
    phalanx::aerial::choose(&mut b);
    assert!(
        b.aerial_events
            .iter()
            .any(|e| e.kind == phalanx::aerial::Kind::Proposed)
    );
    assert!((b.units[0].theus.as_ref().unwrap().stamina - (effort - 9.)).abs() < 0.01);
    let flight = b.units[0]
        .aerial
        .clone()
        .expect("staged high-skill seed should launch");
    b.tick = flight.start + 9;
    phalanx::aerial::advance(&mut b);
    assert!(phalanx::ninjutsu::physical_height(&b.units[0], b.arena, b.tick as f32) > 2.9);
    assert_eq!(
        b.units[0].position,
        [
            flight.sample(b.tick as f32)[0],
            flight.sample(b.tick as f32)[2]
        ]
    );
    b.units[0].theus.as_mut().unwrap().stagger_until = b.tick + 10;
    phalanx::aerial::advance(&mut b);
    assert!(b.units[0].aerial.as_ref().unwrap().falling);
    assert!(
        b.aerial_events
            .iter()
            .any(|e| e.kind == phalanx::aerial::Kind::Interrupted)
    );
    b.tick += 5;
    phalanx::aerial::advance(&mut b);
    assert!(b.units[0].aerial.is_none());
    assert!(!b.arena.blocked(b.units[0].position));
}
#[test]
fn new_equipment_preserves_its_slots_and_has_real_tradeoffs() {
    use phalanx::equipment::Gear as G;
    let mut c = Campaign::new();
    c.teams[0].gold = 1000;
    c.teams[0].salvage = 1000;
    c.workshop(0, 0, G::BoneGlaive).unwrap();
    c.workshop(0, 0, G::PorcelainHarness).unwrap();
    let (_, b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(b.units[0].attack_range(), 2.8);
    assert!(b.units[0].equipment.incoming() < 1.);
    assert!(b.units[0].equipment.speed() < 1.);
    assert_eq!(
        phalanx::telemetry::context_features(&b.units[0], &[], None).len() + 29,
        phalanx::telemetry::FEATURE_NAMES.len()
    );
}

#[test]
fn pug_sized_hound_must_close_to_bite_and_cannot_hit_a_retreating_target() {
    let mut c = campaign();
    let (_, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    b.kennel.actors.truncate(1);
    for u in &mut b.units {
        u.position = [20., 20.];
        u.cooldown = 100.;
    }
    b.units[0].position = [-1., 0.];
    b.units[0].tactics.phase = phalanx::tactics::Phase::Engage;
    b.units[3].position = [1.3, 0.];
    b.kennel.actors[0].position = [0., 0.];
    phalanx::kennel::step(&mut b);
    assert_eq!(b.kennel.actors[0].order, phalanx::kennel::Order::Harass);
    assert!(b.kennel.actors[0].winding.is_none());
    b.units[3].position = [0.45, 0.];
    b.tick = 10;
    phalanx::kennel::step(&mut b);
    let (target, end) = b.kennel.actors[0]
        .winding
        .expect("close target is reachable");
    assert_eq!(target, 3);
    let health = b.units[3].health;
    b.units[3].position = [1.4, 0.];
    b.tick = end;
    phalanx::kennel::step(&mut b);
    assert_eq!(b.units[3].health, health);
    assert!(b.kennel.actors[0].winding.is_none());
    assert!(
        !b.kennel
            .events
            .iter()
            .any(|e| e.kind == phalanx::kennel::Kind::BiteResolved)
    );
}
