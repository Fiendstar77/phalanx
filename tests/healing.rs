use phalanx::{
    Battle, CommandTitle, FighterClass, learned::Controller, progression::Campaign, terrain::Arena,
};

fn queen_healer() -> Battle {
    let mut c = Campaign::new();
    c.set_class(0, 1, FighterClass::Healer).unwrap();
    let (_, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    for u in &mut b.units {
        u.position = if u.team == 0 { [-7.0, 0.0] } else { [8.0, 6.0] };
    }
    b.units[1].position = [-8.0, 0.0];
    b
}
#[test]
fn queen_healing_reports_actual_restoration_and_cooldown() {
    let mut b = queen_healer();
    assert_eq!(b.units[1].command_title, Some(CommandTitle::Queen));
    b.units[0].health = b.units[0].max_health() - 5.0;
    b.step();
    assert_eq!(b.heals.len(), 1);
    let e = &b.heals[0];
    assert_eq!((e.healer, e.target), (1, 0));
    assert_eq!(e.amount, 5.0);
    assert_eq!(b.units[0].health, b.units[0].max_health());
    b.units[0].health -= 30.0;
    b.step();
    assert_eq!(b.heals.len(), 1, "cooldown prevents a second cast");
}
#[test]
fn no_false_healing_for_dead_full_health_or_blocked_allies() {
    let mut b = queen_healer();
    b.units[0].health = 0.0;
    b.step();
    assert!(b.heals.is_empty());
    let mut b = queen_healer();
    b.units[0].health = 40.0;
    b.units[0].position = [0.0, 8.0];
    b.step();
    assert!(b.heals.is_empty(), "out of range cannot trigger an effect");
}
#[test]
fn multiple_healers_share_capped_restoration_without_double_counting() {
    let mut b = queen_healer();
    b.units[2].class = Some(FighterClass::Healer);
    b.units[0].health = b.units[0].max_health() - 5.0;
    b.step();
    assert_eq!(b.heals.len(), 2);
    assert_eq!(b.heals.iter().map(|e| e.amount).sum::<f32>(), 5.0);
}
