use phalanx::{FighterClass as C, progression::Campaign};
#[test]
fn founding_is_a_free_once_only_choice_and_legacy_teams_are_preserved() {
    let mut c = Campaign::new();
    let original = c.teams[0].clone();
    assert!(!c.founders_prepared);
    assert!(c.choose_founders("  ", [C::Warrior; 3]).is_err());
    assert_eq!(
        serde_json::to_string(&c.teams[0]).unwrap(),
        serde_json::to_string(&original).unwrap()
    );
    c.choose_founders(
        "  Penelope's Bad Decisions  ",
        [C::Guardian, C::Healer, C::Scout],
    )
    .unwrap();
    assert_eq!(c.teams[0].name, "Penelope's Bad Decisions");
    assert_eq!(c.teams[0].gold, 100);
    assert_eq!(c.teams[0].fighters.len(), 3);
    assert_eq!(c.teams[0].fighters[1].class, C::Healer);
    for (a, b) in c.teams[0].fighters.iter().zip(&original.fighters) {
        assert_eq!(a.fighter_id, b.fighter_id);
        assert_eq!(a.title, b.title);
        assert_eq!(a.experience, 0);
    }
    assert!(c.choose_founders("Again", [C::Warrior; 3]).is_err());
    let mut saved = serde_json::to_value(&c).unwrap();
    saved.as_object_mut().unwrap().remove("founders_prepared");
    let legacy: Campaign = serde_json::from_value(saved).unwrap();
    assert!(legacy.founders_prepared);
    assert_eq!(legacy.teams[0].name, c.teams[0].name);
    let mut copy = legacy.clone();
    copy.teams[0].fighters[0].appearance.hair = 10;
    copy.validate().unwrap();
    copy.teams[0].fighters[0].appearance.hair = 11;
    assert!(copy.validate().is_err());
}
