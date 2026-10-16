use phalanx::{
    FighterClass,
    equipment::Gear,
    heraldry::{Emblem, Heraldry},
    learned::Controller,
    progression::Campaign,
    terrain::Arena,
};
#[test]
fn every_squad_has_one_designated_standard_bearer_without_free_skills() {
    let mut c = Campaign::new();
    let (_, b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    for team in 0..2 {
        let bearers = b
            .units
            .iter()
            .filter(|u| u.team == team && u.carries_standard)
            .collect::<Vec<_>>();
        assert_eq!(bearers.len(), 1);
        assert!(!bearers[0].banner_active());
    }
    c.teams[0].standard_bearer = 1;
    c.teams[0].fighters[1]
        .mastery
        .train(FighterClass::BannerBearer, 100);
    let (_, b) = c
        .begin_battle(43, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert!(b.units[1].carries_standard && b.units[1].banner_active());
    assert!(!b.units[2].carries_standard);
    assert!(!b.units[1].equipment.has(Gear::WarBanner));
}
#[test]
fn heraldry_and_bearer_survive_disk_roundtrip_and_invalid_designs_are_rejected() {
    let mut c = Campaign::new();
    c.teams[0].heraldry = Heraldry {
        accent: [0.1, 0.2, 0.9],
        emblem: Emblem::Chevrons,
    };
    c.teams[0].standard_bearer = 0;
    let path = std::env::temp_dir().join(format!("phalanx-standard-{}.json", std::process::id()));
    c.save(&path).unwrap();
    let copy = Campaign::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(copy.teams[0].heraldry, c.teams[0].heraldry);
    assert_eq!(copy.teams[0].standard_bearer, 0);
    c.teams[0].standard_bearer = 3;
    assert!(c.validate().is_err());
    c.teams[0].standard_bearer = 0;
    c.teams[0].heraldry.accent[0] = f32::NAN;
    assert!(c.validate().is_err());
}
#[test]
fn older_saves_receive_a_mandatory_flag_design_and_bearer() {
    let mut json = serde_json::to_value(Campaign::new()).unwrap();
    for team in json["teams"].as_array_mut().unwrap() {
        team.as_object_mut().unwrap().remove("heraldry");
        team.as_object_mut().unwrap().remove("standard_bearer");
    }
    let c: Campaign = serde_json::from_value(json).unwrap();
    c.validate().unwrap();
    assert!(
        c.teams
            .iter()
            .all(|team| team.standard_bearer == 2 && team.heraldry.valid())
    );
}
