use phalanx::{
    equipment::{Equipment, Gear},
    learned::Controller,
    progression::Campaign,
    terrain::Arena,
};

#[test]
fn crafting_is_atomic_and_owned_gear_switches_without_payment() {
    let mut c = Campaign::new();
    let before = serde_json::to_value(&c).unwrap();
    assert!(c.workshop(0, 0, Gear::IronArmor).is_err());
    assert_eq!(before, serde_json::to_value(&c).unwrap());
    c.teams[0].salvage = 100;
    c.workshop(0, 0, Gear::IronArmor).unwrap();
    assert_eq!((c.teams[0].gold, c.teams[0].salvage), (70, 90));
    c.workshop(0, 0, Gear::TrailArmor).unwrap();
    c.workshop(0, 0, Gear::IronArmor).unwrap();
    assert!(c.teams[0].fighters[0].equipment.has(Gear::IronArmor));
    c.workshop(0, 0, Gear::IronArmor).unwrap();
    assert_eq!(c.teams[0].fighters[0].equipment.slots[1], None);
    assert_eq!((c.teams[0].gold, c.teams[0].salvage), (50, 84));
}

#[test]
fn persistent_loadouts_change_actual_combat_stats() {
    let mut c = Campaign::new();
    c.teams[0].salvage = 100;
    let (_, base) = c
        .begin_battle(42, Arena::Ridge, Controller::Utility)
        .unwrap();
    for gear in [Gear::TemperedWeapon, Gear::IronArmor, Gear::VitalityCharm] {
        c.workshop(0, 0, gear).unwrap();
    }
    let json = serde_json::to_string(&c).unwrap();
    let mut restored: Campaign = serde_json::from_str(&json).unwrap();
    restored.validate().unwrap();
    let (_, battle) = restored
        .begin_battle(42, Arena::Ridge, Controller::Utility)
        .unwrap();
    let (a, b) = (&base.units[0], &battle.units[0]);
    assert!((b.damage_multiplier() / a.damage_multiplier() - 1.20).abs() < 0.001);
    assert!((b.movement_multiplier() / a.movement_multiplier() - 0.92 * 0.85).abs() < 0.001);
    assert!((b.incoming_multiplier() / a.incoming_multiplier() - 0.80).abs() < 0.001);
    assert_eq!(b.health, a.health + 20.0);
    assert_eq!(b.health, b.max_health());
}

#[test]
fn legacy_saves_default_and_invalid_loadouts_are_rejected() {
    let c = Campaign::new();
    let mut json = serde_json::to_value(&c).unwrap();
    for team in json["teams"].as_array_mut().unwrap() {
        for fighter in team["fighters"].as_array_mut().unwrap() {
            fighter.as_object_mut().unwrap().remove("equipment");
        }
    }
    let mut restored: Campaign = serde_json::from_value(json).unwrap();
    restored.validate().unwrap();
    assert_eq!(
        restored.teams[0].fighters[0].equipment,
        Equipment::default()
    );
    restored.teams[0].fighters[0].equipment.slots[0] = Some(Gear::IronArmor);
    assert!(restored.validate().is_err());
    restored.teams[0].fighters[0].equipment = Equipment {
        owned: 128,
        slots: [None; 3],
    };
    assert!(restored.validate().is_err());
}

#[test]
fn workshop_survives_disk_save_and_restart() {
    let mut c = Campaign::new();
    let (id, mut earned) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    while !earned.finished {
        earned.step();
    }
    c.settle(id, &earned).unwrap();
    let purse = (c.teams[0].gold, c.teams[0].salvage);
    assert!(purse.1 >= 6);
    c.workshop(0, 1, Gear::BalancedWeapon).unwrap();
    let path = std::env::temp_dir().join(format!("phalanx-equipment-{}.json", std::process::id()));
    c.save(&path).unwrap();
    let mut restored = Campaign::load(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let (_, b) = restored
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert!(b.units[1].equipment.has(Gear::BalancedWeapon));
    assert_eq!(b.units[1].health, b.units[1].max_health());
    assert_eq!(
        (restored.teams[0].gold, restored.teams[0].salvage),
        (purse.0 - 20, purse.1 - 6)
    );
}
