use phalanx::{
    Battle, FighterClass,
    learned::Controller,
    progression::{Campaign, PALETTE},
    terrain::Arena,
};
#[test]
fn paladin_aura_changes_real_damage_and_stops_on_death() {
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[0].health = 110.0;
    b.units[0].position = [0.0, 0.0];
    b.units[2].health = 100.0;
    b.units[2].position = [0.0, 2.0];
    b.units[3].health = 110.0;
    b.units[3].position = [1.0, 0.0];
    b.step();
    assert!(b.units[0].blessed);
    assert!(!b.units[2].blessed);
    assert!(!b.units[3].blessed);
    assert!((b.units[0].health - 100.1).abs() < 0.0001);
    assert!((b.units[3].health - 98.45).abs() < 0.0001);
    b.units[2].health = 0.0;
    b.step();
    assert!(!b.units[0].blessed);
}
#[test]
fn aura_obeys_radius_and_walls() {
    let mut b = Battle::with_founders(42, false, Arena::Choke);
    b.units[0].position = [-2.0, 5.0];
    b.units[2].position = [2.0, 5.0];
    b.step();
    assert!(!b.units[0].blessed);
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    b.units[0].position = [-5.0, 0.0];
    b.units[2].position = [0.0, 0.0];
    b.step();
    assert!(!b.units[0].blessed);
}
#[test]
fn persistent_squads_gain_experience_once_and_recover_as_the_same_veterans() {
    let mut c = Campaign::new();
    let ids: Vec<_> = c
        .teams
        .iter()
        .flat_map(|t| t.fighters.iter().map(|f| f.fighter_id.clone()))
        .collect();
    for seed in 42..46 {
        let (id, mut b) = c
            .begin_battle(seed, Arena::Ridge, Controller::Utility)
            .unwrap();
        assert!(c.settle(id, &b).is_err());
        while !b.finished {
            b.step();
        }
        c.settle(id, &b).unwrap();
        let settled = serde_json::to_string(&c).unwrap();
        assert!(c.settle(id, &b).is_err());
        assert_eq!(settled, serde_json::to_string(&c).unwrap());
    }
    assert!(c.teams[0].fighters[0].experience >= 100);
    let (_, b) = c
        .begin_battle(46, Arena::Ridge, Controller::Utility)
        .unwrap();
    assert_eq!(
        b.units
            .iter()
            .map(|u| u.fighter_id.clone().unwrap())
            .collect::<Vec<_>>(),
        ids
    );
    assert!(
        b.units
            .iter()
            .all(|u| u.health == u.max_health() && u.level() >= 2 && u.battle_count == 4)
    );
    assert_eq!(b.units[2].class, Some(FighterClass::Paladin));
    assert!(b.units[0].max_health() > 110.0);
    let mut restored: Campaign = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    let mut original = c.clone();
    let (_, a) = original
        .begin_battle(77, Arena::Choke, Controller::Learned)
        .unwrap();
    let (_, b) = restored
        .begin_battle(77, Arena::Choke, Controller::Learned)
        .unwrap();
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
}
#[test]
fn saves_preserve_colors_and_ids_and_reject_invalid_data_without_mutating_rewards() {
    let mut c = Campaign::new();
    c.teams[0].color = PALETTE[2];
    let root = std::env::temp_dir().join(format!(
        "phalanx-save-test-{}-{}",
        std::process::id(),
        c.teams[0].fighters[0].fighter_id
    ));
    let path = root.join("save.json");
    c.save(&path).unwrap();
    let loaded = Campaign::load(&path).unwrap();
    assert_eq!(
        serde_json::to_string(&c).unwrap(),
        serde_json::to_string(&loaded).unwrap()
    );
    c.teams[0].color = PALETTE[5];
    c.save(&path).unwrap();
    assert!(path.with_extension("json.bak").exists());
    let mut bad = c.clone();
    bad.teams[0].color[0] = 2.0;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.teams[0].fighters[1].fighter_id = bad.teams[0].fighters[0].fighter_id.clone();
    assert!(bad.validate().is_err());
    let (id, mut b) = c
        .begin_battle(42, Arena::Ridge, Controller::Utility)
        .unwrap();
    while !b.finished {
        b.step();
    }
    b.units[0].mind.courage = f32::NAN;
    let before = serde_json::to_string(&c).unwrap();
    assert!(c.settle(id, &b).is_err());
    assert_eq!(before, serde_json::to_string(&c).unwrap());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn veteran_power_and_battle_conditioned_habits_are_bounded() {
    let mut c = Campaign::new();
    c.teams[0].fighters[0].experience = 1_000_000;
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(b.units[0].level(), 10);
    assert_eq!(b.units[0].max_health(), 128.0);
    let old = c.teams[0].fighters[0].mind.self_preservation;
    while !b.finished {
        b.step();
    }
    b.units[0].health = 0.0;
    c.settle(id, &b).unwrap();
    assert_eq!(
        c.teams[0].fighters[0].mind.self_preservation,
        (old + 0.01).min(1.0)
    );
    c.validate().unwrap();
}
