use phalanx::{
    FighterClass as C,
    equipment::{Gear, Item},
    inventory::{Destination as D, Location as L},
    learned::Controller,
    progression::Campaign,
    skills::{Skill as S, SkillLoadout},
    terrain::Arena,
};
fn funded() -> Campaign {
    let mut c = Campaign::new();
    c.teams[0].gold = 10000;
    c.teams[0].salvage = 10000;
    c
}
#[test]
fn physical_items_keep_their_progress_across_fighters_and_disk() {
    let mut c = funded();
    c.craft_item(0, 1, Gear::TemperedWeapon).unwrap();
    c.forge_item(0, 1, L::Personal(0)).unwrap();
    let item = c.inventory_item(0, 1, L::Personal(0)).unwrap();
    c.transfer_item(0, 1, L::Personal(0), D::Equipped(0))
        .unwrap();
    c.transfer_item(0, 1, L::Equipped(0), D::Shared).unwrap();
    assert!(
        c.teams[0].fighters[1]
            .equipment
            .items
            .iter()
            .all(Option::is_none)
    );
    c.transfer_item(0, 0, L::Shared(0), D::Equipped(0)).unwrap();
    assert_eq!(c.inventory_item(0, 0, L::Equipped(0)).unwrap(), item);
    assert!(c.teams[0].inventory.is_empty());
    let path =
        std::env::temp_dir().join(format!("phalanx-rpg-transfer-{}.json", std::process::id()));
    c.save(&path).unwrap();
    let mut loaded = Campaign::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    let (_, battle) = loaded
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(battle.units[0].equipment.worn(0), Some(item));
    assert!(battle.units[0].damage_multiplier() > 1.2 * 1.1);
}
#[test]
fn reforging_consumes_the_max_level_item_and_increases_power() {
    let mut c = funded();
    c.craft_item(0, 0, Gear::IronArmor).unwrap();
    for _ in 0..4 {
        c.forge_item(0, 0, L::Personal(0)).unwrap();
    }
    let old = c.inventory_item(0, 0, L::Personal(0)).unwrap();
    assert_eq!(old.level, 5);
    c.transfer_item(0, 0, L::Personal(0), D::Equipped(1))
        .unwrap();
    c.forge_item(0, 0, L::Equipped(1)).unwrap();
    let new = c.inventory_item(0, 0, L::Equipped(1)).unwrap();
    assert_eq!((new.alloy, new.level), (1, 1));
    assert!(new.power() > old.power());
    assert_eq!(
        c.teams[0].fighters[0]
            .equipment
            .items
            .iter()
            .flatten()
            .count(),
        1
    );
    assert!(c.teams[0].fighters[0].equipment.incoming() < 0.8);
}
#[test]
fn invalid_transactions_and_full_inventories_never_consume_items_or_currency() {
    let mut c = funded();
    c.craft_item(0, 0, Gear::IronArmor).unwrap();
    let before = serde_json::to_value(&c).unwrap();
    assert!(
        c.transfer_item(0, 0, L::Personal(0), D::Equipped(0))
            .is_err()
    );
    assert_eq!(before, serde_json::to_value(&c).unwrap());
    c.teams[0].gold = 0;
    let before = serde_json::to_value(&c).unwrap();
    assert!(c.forge_item(0, 0, L::Personal(0)).is_err());
    assert_eq!(before, serde_json::to_value(&c).unwrap());
    let mut c = funded();
    for _ in 0..24 {
        c.craft_item(0, 0, Gear::WarBanner).unwrap();
    }
    let before = serde_json::to_value(&c).unwrap();
    assert!(c.craft_item(0, 0, Gear::WarBanner).is_err());
    assert_eq!(before, serde_json::to_value(&c).unwrap());
    c.teams[0].inventory.push(Item::new(Gear::IronArmor));
    let before = serde_json::to_value(&c).unwrap();
    assert!(c.transfer_item(0, 0, L::Shared(0), D::Personal).is_err());
    assert_eq!(before, serde_json::to_value(&c).unwrap());
}
#[test]
fn legacy_owned_equipment_materializes_once_with_worn_slots_preserved() {
    let mut c = funded();
    let e = &mut c.teams[0].fighters[0].equipment;
    e.owned = (1 << Gear::IronArmor.index()) | (1 << Gear::TemperedWeapon.index());
    e.slots[1] = Some(Gear::IronArmor);
    c.validate().unwrap();
    e_migrate(&mut c);
    e_migrate(&mut c);
    let e = c.teams[0].fighters[0].equipment;
    assert_eq!(e.items.iter().flatten().count(), 2);
    assert!(e.has(Gear::IronArmor));
    assert!(e.valid());
    fn e_migrate(c: &mut Campaign) {
        c.teams[0].fighters[0].equipment.migrate();
    }
}
#[test]
fn slots_gate_real_healing_and_passives_without_erasing_learned_skills() {
    let mut c = funded();
    c.teams[0].fighters[1].mastery.train(C::Healer, 300);
    c.teams[0].fighters[1].mastery.train(C::Guardian, 100);
    let before = c.teams[0].fighters[1].mastery;
    let defaults = SkillLoadout::defaults(C::RockThrower, before);
    assert!(defaults.has(S::Heal));
    let heal = defaults
        .active
        .iter()
        .position(|s| *s == Some(S::Heal))
        .unwrap();
    c.set_skill(0, 1, false, heal, None).unwrap();
    let (_, b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert!(!b.units[1].has_skill(C::Healer));
    c.set_skill(0, 1, false, 9, Some(S::Heal)).unwrap();
    c.set_skill(0, 1, true, 2, Some(S::Bulwark)).unwrap();
    assert!(c.set_skill(0, 1, true, 0, Some(S::Heal)).is_err());
    assert!(c.set_skill(0, 1, false, 0, Some(S::Hex)).is_err());
    assert!(c.set_skill(0, 1, false, 10, None).is_err());
    c.set_class(0, 1, C::Warrior).unwrap();
    let (_, b) = c
        .begin_battle(43, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert!(b.units[1].has_skill(C::Healer));
    assert_eq!(c.teams[0].fighters[1].mastery, before);
    assert_eq!(b.units[1].skills.unwrap().active.len(), 10);
    assert_eq!(b.units[1].skills.unwrap().passive.len(), 3);
    let json = serde_json::to_vec(&c).unwrap();
    let loaded: Campaign = serde_json::from_slice(&json).unwrap();
    loaded.validate().unwrap();
    assert_eq!(
        loaded.teams[0].fighters[1].skills,
        c.teams[0].fighters[1].skills
    );
}
#[test]
fn every_member_has_equal_workshop_access_and_every_mode_has_larger_health() {
    let mut c = funded();
    for slot in 0..3 {
        c.craft_item(0, slot, Gear::BalancedWeapon).unwrap();
        c.transfer_item(0, slot, L::Personal(0), D::Equipped(0))
            .unwrap();
        c.forge_item(0, slot, L::Equipped(0)).unwrap();
    }
    let (_, b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    for u in &b.units {
        assert!(u.health >= 300.);
        assert_eq!(u.health, u.max_health());
    }
    let lab = phalanx::Battle::new(42, false);
    for u in lab.units {
        assert_eq!(u.health, 300.);
    }
}

#[test]
fn charm_upgrades_change_hp_and_the_final_alloy_cannot_be_forged_again() {
    let mut c = funded();
    c.workshop(0, 0, Gear::VitalityCharm).unwrap();
    let (_, before) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    c.forge_item(0, 0, L::Equipped(2)).unwrap();
    let (_, after) = c
        .begin_battle(43, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert!(after.units[0].max_health() > before.units[0].max_health());
    while c
        .inventory_item(0, 0, L::Equipped(2))
        .unwrap()
        .forged()
        .is_some()
    {
        c.forge_item(0, 0, L::Equipped(2)).unwrap();
    }
    let before = serde_json::to_value(&c).unwrap();
    assert!(c.forge_item(0, 0, L::Equipped(2)).is_err());
    assert_eq!(before, serde_json::to_value(&c).unwrap());
    c.transfer_item(0, 0, L::Equipped(2), D::Shared).unwrap();
    let final_item = c.inventory_item(0, 0, L::Shared(0)).unwrap();
    assert_eq!(
        (final_item.alloy, final_item.level),
        (Item::MAX_ALLOY, Item::MAX_LEVEL)
    );
}
