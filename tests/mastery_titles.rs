use phalanx::{
    Battle, CommandTitle as Title, FighterClass as Class, Role, equipment::Gear,
    learned::Controller, progression::Campaign, terrain::Arena,
};

#[test]
fn legacy_class_practice_keeps_its_discipline_when_new_tracks_are_added() {
    use phalanx::mastery::Mastery;
    let legacy = serde_json::json!({"practice": [10,20,30,40,50,60,70,80,90,100]});
    let mut restored: Mastery = serde_json::from_value(legacy).unwrap();
    for (index, class) in Class::ALL[..10].iter().enumerate() {
        assert_eq!(restored.points(*class), (index as u32 + 1) * 10);
    }
    for class in &Class::ALL[10..] {
        assert_eq!(restored.points(*class), 0);
    }
    restored.train(Class::Warlock, 101);
    let reloaded: Mastery =
        serde_json::from_str(&serde_json::to_string(&restored).unwrap()).unwrap();
    assert_eq!(restored, reloaded);
    assert!(reloaded.retained(Class::Warlock));
    for length in [0, 9, 11, 17, 19] {
        assert!(
            serde_json::from_value::<Mastery>(serde_json::json!({"practice": vec![0; length]}))
                .is_err()
        );
    }
}

#[test]
fn titles_swap_without_changing_identity_class_gear_mastery_or_appearance() {
    let mut c = Campaign::new();
    c.teams[0].salvage = 100;
    c.set_class(0, 2, Class::Healer).unwrap();
    c.teams[0].fighters[1].mastery.train(Class::Healer, 100);
    c.workshop(0, 1, Gear::BalancedWeapon).unwrap();
    let queen = c.teams[0].fighters[1].clone();
    let general = c.teams[0].fighters[2].clone();
    c.assign_title(0, 2, Title::Queen).unwrap();
    assert_eq!(c.teams[0].fighters[1].title, Title::General);
    assert_eq!(c.teams[0].fighters[2].title, Title::Queen);
    assert_eq!(c.teams[0].fighters[1].fighter_id, queen.fighter_id);
    assert_eq!(c.teams[0].fighters[1].class, queen.class);
    assert_eq!(c.teams[0].fighters[1].mastery, queen.mastery);
    assert_eq!(c.teams[0].fighters[1].equipment, queen.equipment);
    assert_eq!(
        c.teams[0].fighters[1].appearance.hair,
        queen.appearance.resolved(0, 1).hair
    );
    assert_eq!(
        c.teams[0].fighters[1].appearance.face,
        queen.appearance.resolved(0, 1).face
    );
    assert_eq!(c.teams[0].fighters[1].appearance.female, Some(true));
    assert_eq!(c.teams[0].fighters[2].appearance.female, Some(false));
    assert_eq!(c.teams[0].fighters[2].class, general.class);
    c.assign_title(0, 2, Title::Member).unwrap();
    c.assign_title(0, 0, Title::Member).unwrap();
    c.assign_title(0, 1, Title::Captain).unwrap();
    c.validate().unwrap();
    let (_, b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(b.units[1].command_title, Some(Title::Captain));
    assert!((b.units[1].movement_multiplier() - 1.05 * 1.10).abs() < 1e-5);
}

#[test]
fn queen_retains_healing_when_retrained_and_can_heal_and_throw_together() {
    let mut c = Campaign::new();
    c.teams[0].fighters[1].mastery.train(Class::Healer, 100);
    let (_, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    for u in &mut b.units {
        u.position = if u.team == 0 { [-4., 0.] } else { [0., 0.] };
    }
    b.units[0].health = 40.0;
    b.units[1].position = [-4., 1.];
    assert_eq!(b.units[1].class, Some(Class::RockThrower));
    assert_eq!(b.units[1].role, Role::RockThrower);
    b.step();
    assert!(
        b.heals
            .iter()
            .any(|e| e.healer == 1 && e.target == 0 && e.amount == 15.0)
    );
    assert!(
        b.rocks.iter().any(|rock| rock.shooter == 1),
        "retained healing must not consume the ranged attack cooldown"
    );
}

#[test]
fn earned_practice_persists_and_only_rewards_once() {
    let mut c = Campaign::new();
    c.set_class(0, 1, Class::Healer).unwrap();
    for seed in 0..4 {
        let (id, mut b) = c
            .begin_battle(seed, Arena::OpenField, Controller::Utility)
            .unwrap();
        while !b.finished {
            b.step();
        }
        c.settle(id, &b).unwrap();
        let points = c.teams[0].fighters[1].mastery.points(Class::Healer);
        assert!(c.settle(id, &b).is_err());
        assert_eq!(points, c.teams[0].fighters[1].mastery.points(Class::Healer));
    }
    assert!(c.teams[0].fighters[1].mastery.retained(Class::Healer));
    c.set_class(0, 1, Class::RockThrower).unwrap();
    c.assign_title(0, 1, Title::Champion).unwrap();
    let path = std::env::temp_dir().join(format!("phalanx-mastery-{}.json", std::process::id()));
    c.save(&path).unwrap();
    let mut restored = Campaign::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    let (_, b) = restored
        .begin_battle(43, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert!(b.units[1].has_skill(Class::Healer));
    assert_eq!(b.units[1].command_title, Some(Title::Champion));
}

#[test]
fn banner_skill_requires_banner_equipment_after_retraining() {
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    let u = &mut b.units[1];
    u.mastery.train(Class::BannerBearer, 100);
    assert!(!u.banner_active());
    u.equipment.owned |= 1 << Gear::WarBanner.index();
    u.equipment.slots[2] = Some(Gear::WarBanner);
    assert!(u.banner_active());
    u.equipment.slots[2] = Some(Gear::VitalityCharm);
    assert!(!u.banner_active());
}

#[test]
fn old_saves_default_to_no_mastery_and_duplicate_titles_are_rejected() {
    let mut json = serde_json::to_value(Campaign::new()).unwrap();
    for team in json["teams"].as_array_mut().unwrap() {
        for f in team["fighters"].as_array_mut().unwrap() {
            f.as_object_mut().unwrap().remove("mastery");
        }
    }
    let mut c: Campaign = serde_json::from_value(json).unwrap();
    c.validate().unwrap();
    assert_eq!(c.teams[0].fighters[0].mastery.points(Class::Healer), 0);
    c.teams[0].fighters[1].title = Title::King;
    assert!(c.validate().is_err());
}

#[test]
fn retained_banner_changes_real_ally_damage_only_when_equipped() {
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[0].health = 110.0;
    b.units[0].position = [0.0, 0.0];
    b.units[1].health = 110.0;
    b.units[1].position = [-1.0, 0.0];
    b.units[1].mastery.train(Class::BannerBearer, 100);
    b.units[5].health = 100.0;
    b.units[5].position = [1.0, 0.0];
    b.units[5].cooldown = 100.0;
    let mut with_banner = b.clone();
    with_banner.units[1].equipment.owned |= 1 << Gear::WarBanner.index();
    with_banner.units[1].equipment.slots[2] = Some(Gear::WarBanner);
    b.step();
    with_banner.step();
    assert!((with_banner.units[0].damage_dealt / b.units[0].damage_dealt - 1.15).abs() < 0.001);
}

#[test]
fn all_disciplines_are_available_to_the_general_and_healers_seek_wounded_allies() {
    let mut c = Campaign::new();
    c.teams[0].gold = 10 * Class::ALL.len() as u32;
    for class in Class::ALL {
        c.set_class(0, 2, class).unwrap();
    }
    assert_eq!(c.teams[0].fighters[2].title, Title::General);
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    b.units[1].class = Some(Class::Healer);
    b.units[1].role = Role::Fighter;
    b.units[1].position = [-8., 0.];
    b.units[0].position = [-2., 0.];
    b.units[0].health = 30.;
    b.units[2].position = [-8., 2.];
    for u in b.units.iter_mut().filter(|u| u.team == 1) {
        u.position = [8., 6.];
    }
    b.step();
    assert_eq!(b.units[1].action, 8);
    assert_eq!(b.units[1].support_target, Some(0));
    assert!(
        b.units[1].target[0] < b.units[0].position[0],
        "support goal should be behind the patient"
    );
}
