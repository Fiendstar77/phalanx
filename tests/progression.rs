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

#[test]
fn appearance_is_backward_compatible_persistent_and_cannot_change_combat() {
    use phalanx::progression::Appearance;
    let campaign = Campaign::new();
    let mut old_save = serde_json::to_value(&campaign).unwrap();
    for team in old_save["teams"].as_array_mut().unwrap() {
        team.as_object_mut().unwrap().remove("salvage");
        for fighter in team["fighters"].as_array_mut().unwrap() {
            for field in ["appearance", "lost_limbs", "prosthetics", "wounds"] {
                fighter.as_object_mut().unwrap().remove(field);
            }
        }
    }
    let mut restored: Campaign = serde_json::from_value(old_save).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.teams[0].fighters[0].appearance.hair, 0);
    restored.teams[0].fighters[0].appearance = Appearance {
        female: Some(true),
        hair: 6,
        face: 16,
        eyes: 15,
        nose: 16,
    };
    let mut copy: Campaign =
        serde_json::from_str(&serde_json::to_string(&restored).unwrap()).unwrap();
    copy.validate().unwrap();
    assert_eq!(copy.teams[0].fighters[0].appearance.female, Some(true));
    let (_, mut original_battle) = campaign
        .clone()
        .begin_battle(42, Arena::Citadel, Controller::Utility)
        .unwrap();
    let (_, mut customized_battle) = copy
        .begin_battle(42, Arena::Citadel, Controller::Utility)
        .unwrap();
    for _ in 0..40 {
        original_battle.step();
        customized_battle.step();
    }
    for (original, customized) in original_battle.units.iter().zip(&customized_battle.units) {
        assert_eq!(original.position, customized.position);
        assert_eq!(original.health, customized.health);
        assert_eq!(original.action, customized.action);
    }
    copy.teams[0].fighters[0].appearance.eyes = 16;
    assert!(copy.validate().is_err());
}

#[test]
fn severe_wounds_are_nonfatal_and_prosthetics_are_a_real_tradeoff() {
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[0].health = 56.0;
    b.units[0].position = [0.0, 0.0];
    b.units[3].health = 110.0;
    b.units[3].position = [1.0, 0.0];
    b.step();
    assert!(b.units[0].health > 0.0);
    assert_eq!(b.units[0].lost_limbs, 1);
    assert!(
        b.units[0].damage_dealt > 0.0,
        "The wounded fighter still fights"
    );
    let natural = 1.1;
    assert!((b.units[0].damage_multiplier() - natural * 0.8).abs() < 0.0001);
    b.units[0].health = 28.0;
    b.units[3].cooldown = 0.0;
    b.step();
    assert!(b.units[0].health > 0.0);
    assert_eq!(b.units[0].lost_limbs, 5);
    assert_eq!(b.units[0].movement_multiplier(), 0.65);
    b.units[0].prosthetics = 5;
    assert!((b.units[0].damage_multiplier() - natural * 3.0).abs() < 0.0001);
    assert_eq!(b.units[0].movement_multiplier(), 0.85);
    let mut lab = Battle::with_arena(42, false, Arena::OpenField);
    for u in &mut lab.units {
        u.health = 0.0;
    }
    lab.units[0].health = 51.0;
    lab.units[0].position = [0.0, 0.0];
    lab.units[lab.arena.team_size()].health = 100.0;
    lab.units[lab.arena.team_size()].position = [1.0, 0.0];
    lab.step();
    assert_eq!(lab.units[0].lost_limbs, 0);
}

#[test]
fn battle_salvage_crafting_and_wounds_survive_the_next_battle() {
    let mut c = Campaign::new();
    c.teams[0].fighters[0].lost_limbs = 1;
    let before = serde_json::to_string(&c).unwrap();
    assert!(c.craft_prosthetic(0, 0).is_err());
    assert_eq!(before, serde_json::to_string(&c).unwrap());
    let fighter_id = c.teams[0].fighters[0].fighter_id.clone();
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(b.units[0].lost_limbs, 1);
    for u in &mut b.units {
        if u.team == 1 {
            u.health = 0.0;
        }
    }
    b.step();
    assert!(b.finished);
    c.settle(id, &b).unwrap();
    assert_eq!(c.teams[0].salvage, 10);
    assert_eq!(c.teams[0].fighters[0].wounds[0].battle_id, id);
    assert_eq!(c.teams[0].fighters[0].wounds[0].limb, 1);
    assert_eq!(c.craft_prosthetic(0, 0).unwrap(), "Left arm");
    assert_eq!(c.teams[0].salvage, 5);
    assert!(c.craft_prosthetic(0, 0).is_err());
    assert_eq!(c.teams[0].salvage, 5);
    let mut restored: Campaign = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    let (_, next) = restored
        .begin_battle(43, Arena::Citadel, Controller::Utility)
        .unwrap();
    assert_eq!(
        next.units[0].fighter_id.as_deref(),
        Some(fighter_id.as_str())
    );
    assert_eq!(next.units[0].lost_limbs, 1);
    assert_eq!(next.units[0].prosthetics, 1);
    assert!((next.units[0].damage_multiplier() - 3.3).abs() < 0.0001);
}
