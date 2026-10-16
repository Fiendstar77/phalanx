use phalanx::{
    FighterClass, equipment::Gear, learned::Controller, progression::Campaign, terrain::Arena,
};

#[test]
fn report_is_saved_once_and_does_not_change_with_post_battle_equipment() {
    let mut c = Campaign::new();
    c.set_class(0, 1, FighterClass::Healer).unwrap();
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    while !b.finished {
        b.step();
    }
    let purse = (c.teams[0].gold, c.teams[0].salvage);
    c.settle(id, &b).unwrap();
    let r = c.last_report.as_ref().unwrap();
    assert_eq!(r.gold[0], c.teams[0].gold - purse.0);
    assert_eq!(r.salvage[0], c.teams[0].salvage - purse.1);
    for u in &b.units {
        let row = &r.fighters[u.team][u.id % b.team_size()];
        assert_eq!(row.damage, u.damage_dealt);
        assert_eq!(row.healing, u.healing_done);
        assert_eq!(row.new_wounds, u.lost_limbs);
    }
    let report = serde_json::to_value(r).unwrap();
    c.workshop(0, 1, Gear::BalancedWeapon).unwrap();
    assert_eq!(report, serde_json::to_value(&c.last_report).unwrap());
    let settled = serde_json::to_value(&c).unwrap();
    assert!(c.settle(id, &b).is_err());
    assert_eq!(settled, serde_json::to_value(&c).unwrap());
    let path = std::env::temp_dir().join(format!("phalanx-report-{}.json", std::process::id()));
    c.save(&path).unwrap();
    let mut restored = Campaign::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(report, serde_json::to_value(&restored.last_report).unwrap());
    let (_, next) = restored
        .begin_battle(43, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert!(next.units[1].equipment.has(Gear::BalancedWeapon));
    assert!(next.units.iter().all(|u| u.health == u.max_health()));
    assert_eq!(report, serde_json::to_value(&restored.last_report).unwrap());
}
#[test]
fn healing_totals_outlive_the_bounded_effect_queue() {
    let mut c = Campaign::new();
    c.set_class(0, 1, FighterClass::Healer).unwrap();
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    for _ in 0..80 {
        for u in &mut b.units {
            u.position = if u.team == 0 { [-8.0, 0.0] } else { [8.0, 6.0] };
            u.target = u.position;
        }
        b.units[0].health = 40.0;
        b.units[1].cooldown = 0.0;
        b.units[1].heal_cooldown = 0.0;
        b.step();
    }
    assert_eq!(b.heals.len(), 64);
    assert_eq!(b.units[1].healing_done, 960.0);
    b.finished = true;
    b.result = "Red wins".into();
    c.settle(id, &b).unwrap();
    assert_eq!(
        c.last_report.as_ref().unwrap().fighters[0][1].healing,
        960.0
    );
}
#[test]
fn legacy_saves_allow_absent_report_and_malformed_reports_are_rejected() {
    let mut json = serde_json::to_value(Campaign::new()).unwrap();
    json.as_object_mut().unwrap().remove("last_report");
    let mut c: Campaign = serde_json::from_value(json).unwrap();
    c.validate().unwrap();
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    while !b.finished {
        b.step();
    }
    c.settle(id, &b).unwrap();
    c.last_report.as_mut().unwrap().fighters[0][0].healing = f32::NAN;
    assert!(c.validate().is_err());
}
