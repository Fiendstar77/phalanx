use phalanx::{
    Battle, FighterClass as Class, Mind,
    learned::Controller,
    progression::Campaign,
    terrain::Arena,
    theus::{Actor, Outcome, Sheet, Technique},
};

fn company(classes: [Class; 3]) -> Campaign {
    let mut c = Campaign::new();
    c.choose_founders("Theus company", classes).unwrap();
    for team in &mut c.teams {
        for f in &mut team.fighters {
            f.mind = Mind::disciplined();
            f.theus = Some(Sheet {
                attributes: [10, 14, 10, 10],
                ..Default::default()
            });
        }
    }
    c
}
fn melee() -> Battle {
    let mut c = company([Class::Guardian, Class::Warrior, Class::Healer]);
    let (_, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    for u in &mut b.units {
        u.health = 0.;
        u.cooldown = 999.;
    }
    for (id, position) in [(0, [-0.5, -0.5]), (1, [-0.5, 0.5]), (3, [0.6, 0.])] {
        b.units[id].health = 1000.;
        b.units[id].position = position;
    }
    b.units[0].cooldown = 0.;
    b.units[1].cooldown = 0.;
    b.units[3].theus = Some(Actor::new(Sheet {
        attributes: [6; 4],
        ..Default::default()
    }));
    b
}
#[test]
fn melee_has_a_real_windup_and_theus_resolution_is_reproducible() {
    let mut a = melee();
    let mut b = a.clone();
    a.step();
    b.step();
    assert!(
        a.theus_events.is_empty(),
        "a hit cannot land before its wind-up"
    );
    assert!(
        a.units[..2]
            .iter()
            .any(|u| u.theus.as_ref().unwrap().windup_target.is_some())
    );
    for _ in 0..180 {
        a.step();
        b.step();
    }
    assert!(!a.theus_events.is_empty());
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
    assert!(a.theus_events.iter().any(|e| e.resolution.damage > 0.));
    assert!(
        a.theus_events
            .iter()
            .all(|e| e.resolution.attack.dice.iter().all(|v| (1..=6).contains(v)))
    );
}
#[test]
fn an_actual_launch_can_be_followed_by_a_partner_and_costs_effort() {
    let mut b = melee();
    let mut launched = false;
    let mut relay = false;
    let mut height = 0.0_f32;
    let mut spent = false;
    for _ in 0..240 {
        b.step();
        launched |= b.theus_events.iter().any(|e| {
            e.technique == Technique::AegisLaunch && e.partner.is_some() && e.resolution.damage > 0.
        });
        relay |= b.theus_events.iter().any(|e| {
            e.technique == Technique::MeteorRelay && e.partner.is_some() && e.resolution.damage > 0.
        });
        spent |= b.units[..2].iter().any(|u| {
            let a = u.theus.as_ref().unwrap();
            a.stamina < a.sheet.endurance()
        });
        height = height.max(
            b.units
                .iter()
                .filter_map(|u| u.theus.as_ref())
                .map(|a| a.height(b.tick))
                .fold(0., f32::max),
        );
    }
    assert!(launched, "no real shield-led launch occurred");
    assert!(relay, "the ally never followed the opening");
    assert!(height > 3.);
    assert!(spent, "no effort was spent during the combination");
}
#[test]
fn combinations_need_a_living_partner_with_visible_shared_contact() {
    let mut b = melee();
    b.units[1].health = 0.;
    for _ in 0..100 {
        b.step();
    }
    assert!(!b.theus_events.iter().any(|e| e.technique.combination()));
    // An ally across an opaque wall cannot manufacture a formation bonus.
    let mut b = melee();
    b.arena = Arena::Choke;
    b.units[1].position = [0., 7.];
    for _ in 0..15 {
        b.step();
    }
    assert!(!b.theus_events.iter().any(|e| e.technique.combination()));
}
#[test]
fn ranged_partners_can_create_crossfire_from_real_projectile_hits() {
    let mut c = company([Class::Ranger, Class::Ranger, Class::Healer]);
    c.teams[1].fighters[0].class = Class::Ranger;
    let (_, mut b) = c
        .begin_battle(6, Arena::OpenField, Controller::Utility)
        .unwrap();
    for u in &mut b.units {
        u.health = 0.;
        u.cooldown = 999.;
    }
    for (id, p) in [(0, [-1., -2.]), (1, [-1., 2.]), (3, [5., 0.])] {
        b.units[id].health = 1000.;
        b.units[id].position = p;
    }
    b.units[0].cooldown = 0.;
    b.units[1].cooldown = 0.;
    b.units[3].theus = Some(Actor::new(Sheet {
        attributes: [6; 4],
        ..Default::default()
    }));
    for _ in 0..300 {
        b.step();
    }
    assert!(b.units[0].weapon_shots[3] > 0 && b.units[1].weapon_shots[3] > 0);
    assert!(
        b.theus_events
            .iter()
            .any(|e| e.technique == Technique::Crossfire && e.resolution.damage > 0.),
        "events {:?}; units {:?}",
        b.theus_events
            .iter()
            .map(|e| (
                e.tick,
                e.attacker,
                e.defender,
                e.technique,
                e.resolution.outcome
            ))
            .collect::<Vec<_>>(),
        b.units
            .iter()
            .map(|u| (
                u.id,
                u.position,
                u.enemy,
                u.is_ranged(),
                u.theus.as_ref().unwrap().stamina
            ))
            .collect::<Vec<_>>()
    );
}
#[test]
fn old_campaigns_migrate_and_real_practice_survives_settlement_and_reload() {
    let c = company([Class::Warrior, Class::RockThrower, Class::Paladin]);
    let mut value = serde_json::to_value(&c).unwrap();
    for team in value["teams"].as_array_mut().unwrap() {
        for f in team["fighters"].as_array_mut().unwrap() {
            f.as_object_mut().unwrap().remove("theus");
        }
    }
    let mut c: Campaign = serde_json::from_value(value).unwrap();
    c.validate().unwrap();
    let original = c.teams[0].fighters[0].fighter_id.clone();
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert!(b.units.iter().all(|u| u.theus.is_some()));
    while !b.finished {
        b.step();
    }
    assert!(
        b.theus_events
            .iter()
            .any(|e| matches!(e.resolution.outcome, Outcome::Hit | Outcome::Critical))
    );
    c.settle(id, &b).unwrap();
    let loaded: Campaign = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    loaded.validate().unwrap();
    assert_eq!(loaded.teams[0].fighters[0].fighter_id, original);
    for u in &b.units {
        let saved = loaded.teams[u.team].fighters[u.id % b.team_size()]
            .theus
            .unwrap();
        assert_eq!(saved, u.theus.as_ref().unwrap().sheet);
    }
    assert!(
        loaded.teams.iter().flat_map(|t| &t.fighters).any(|f| f
            .theus
            .unwrap()
            .practice
            .iter()
            .any(|v| *v > 0))
    );
    let lab = Battle::new(42, false);
    assert!(lab.units.iter().all(|u| u.theus.is_none()));
}
