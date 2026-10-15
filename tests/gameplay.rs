use phalanx::{
    Battle, CommandTitle, DT, FighterClass, Mind, learned::Controller, navigation::plan,
    progression::Campaign, terrain::Arena,
};
#[test]
fn retraining_is_transactional_and_preserves_the_veteran() {
    let mut c = Campaign::new();
    let identity = c.teams[0].fighters[0].fighter_id.clone();
    c.set_class(0, 0, FighterClass::Healer).unwrap();
    assert_eq!(c.teams[0].gold, 90);
    assert_eq!(c.teams[0].fighters[0].fighter_id, identity);
    let before = serde_json::to_string(&c).unwrap();
    assert!(c.set_class(0, 2, FighterClass::Healer).is_err());
    assert!(c.set_class(0, 0, FighterClass::Healer).is_err());
    assert_eq!(before, serde_json::to_string(&c).unwrap());
    c.teams[0].gold = 0;
    assert!(c.set_class(0, 0, FighterClass::Guardian).is_err());
    assert_eq!(c.teams[0].fighters[0].class, FighterClass::Healer);
}
#[test]
fn gold_rewards_only_settle_once_and_old_saves_receive_starting_purse() {
    let mut c = Campaign::new();
    let mut old = serde_json::to_value(&c).unwrap();
    for team in old["teams"].as_array_mut().unwrap() {
        team.as_object_mut().unwrap().remove("gold");
    }
    let migrated: Campaign = serde_json::from_value(old).unwrap();
    assert_eq!(migrated.teams[0].gold, 100);
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    while !b.finished {
        b.step();
    }
    c.settle(id, &b).unwrap();
    for team in 0..2 {
        assert_eq!(
            c.teams[team].gold,
            if b.winner() == Some(team) { 130 } else { 115 }
        );
    }
    let before = serde_json::to_string(&c).unwrap();
    assert!(c.settle(id, &b).is_err());
    assert_eq!(before, serde_json::to_string(&c).unwrap());
}
#[test]
fn healer_restores_living_ally_but_cannot_resurrect() {
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.position = if u.team == 0 { [-7., 0.] } else { [8., 6.] };
    }
    b.units[0].class = Some(FighterClass::Healer);
    b.units[0].position = [-8., 0.];
    b.units[1].health = 40.;
    b.units[2].health = 0.;
    b.step();
    assert_eq!(b.units[1].health, 52.);
    assert_eq!(b.units[2].health, 0.);
    assert_eq!(b.units[0].support_target, Some(1));
    assert!(b.units[0].cooldown > DT);
}
#[test]
fn classes_have_real_tradeoffs_and_general_stays_paladin() {
    let mut c = Campaign::new();
    for class in FighterClass::ALL {
        c.teams[0].fighters[0].class = class;
        let (_, b) = c
            .begin_battle(42, Arena::Ridge, Controller::Utility)
            .unwrap();
        assert_eq!(b.units[0].class, Some(class));
        assert_eq!(b.units[0].role, class.role());
        assert_eq!(b.units[2].command_title, Some(CommandTitle::General));
        assert_eq!(b.units[2].class, Some(FighterClass::Paladin));
        if class == FighterClass::Guardian {
            assert!(b.units[0].incoming_multiplier() < 0.7);
            assert!(b.units[0].max_health() > 130.);
        }
        if class == FighterClass::Scout {
            assert!(b.units[0].movement_multiplier() > 1.4);
            assert!(b.units[0].max_health() < 110.);
        }
    }
}
#[test]
fn new_maps_offer_connected_routes_and_complete_searching_battles() {
    for arena in [Arena::Highlands, Arena::Ruins, Arena::GrandArena] {
        let route = plan(
            arena,
            arena.spawn(0, 0),
            arena.spawn(1, 0),
            Mind::aggressive(),
            0,
            &[],
        );
        assert!(route.points.len() > 2, "{arena:?}");
        assert!(
            route
                .points
                .windows(2)
                .all(|p| arena.walk_segment(p[0], p[1]))
        );
        assert!(phalanx::distance(*route.points.last().unwrap(), arena.spawn(1, 0)) < 1.);
        let mut b = Battle::with_founders(42, false, arena);
        while !b.finished {
            b.step();
            assert!(
                b.units.iter().all(|u| !arena.blocked(u.position)),
                "{arena:?}"
            );
        }
        assert!(
            b.units.iter().any(|u| u.damage_dealt > 0.),
            "no enemy contact in {arena:?}"
        );
    }
}

#[test]
fn banner_and_hex_change_actual_damage_without_changing_lab_rules() {
    fn hit(banner: bool, hex: bool) -> f32 {
        let mut b = Battle::with_founders(42, false, Arena::OpenField);
        for u in &mut b.units {
            u.position = [if u.team == 0 { -8. } else { 8. }, 6.];
            u.class = Some(FighterClass::Warrior);
            u.mind = Mind::aggressive();
        }
        b.units[0].position = [0., 0.];
        b.units[3].position = [1., 0.];
        b.units[1].position = [-2., 0.];
        b.units[4].position = [3., 0.];
        if banner {
            b.units[1].class = Some(FighterClass::BannerBearer);
        }
        if hex {
            b.units[4].class = Some(FighterClass::Hexer);
        }
        b.step();
        b.units[0].damage_dealt
    }
    let ordinary = hit(false, false);
    assert!(ordinary > 0.);
    assert!((hit(true, false) / ordinary - 1.15).abs() < 0.001);
    assert!((hit(false, true) / ordinary - 0.75).abs() < 0.001);
}

#[test]
fn fourth_slot_requires_savings_and_produces_a_persistent_matched_league() {
    let mut c = Campaign::new();
    let original = c.teams[0].fighters[0].fighter_id.clone();
    let before = serde_json::to_string(&c).unwrap();
    assert!(c.unlock_fourth().is_err());
    assert_eq!(before, serde_json::to_string(&c).unwrap());
    c.teams[0].gold = 1000;
    c.unlock_fourth().unwrap();
    assert_eq!(c.teams[0].gold, 0);
    assert_eq!(c.teams[0].fighters.len(), 4);
    assert_eq!(c.teams[1].fighters.len(), 4);
    assert_eq!(c.teams[0].fighters[0].fighter_id, original);
    assert_eq!(c.teams[0].fighters[3].title, CommandTitle::Member);
    assert_eq!(c.teams[0].fighters[3].experience, 0);
    assert!(c.unlock_fourth().is_err());
    let (id, mut b) = c
        .begin_battle(42, Arena::Choke, Controller::Utility)
        .unwrap();
    assert_eq!(b.units.len(), 8);
    assert_eq!(b.team_size(), 4);
    while !b.finished {
        b.step();
    }
    c.settle(id, &b).unwrap();
    c.validate().unwrap();
    assert!(
        c.teams
            .iter()
            .flat_map(|t| &t.fighters)
            .all(|f| f.battles == 1)
    );
    let restored: Campaign = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    restored.validate().unwrap();
}

#[test]
fn complete_cost_ladder_escalates_to_an_extreme_final_milestone() {
    use phalanx::progression::{SLOT_COSTS, gold_reward, slot_cost};
    assert_eq!(slot_cost(3), None);
    assert_eq!(slot_cost(21), None);
    assert_eq!(slot_cost(4), Some(1000));
    assert_eq!(slot_cost(20), Some(650_000));
    assert!(SLOT_COSTS.windows(2).all(|w| w[1] > w[0]));
    for league in 3..20 {
        assert!(gold_reward(league + 1, true) > gold_reward(league, true));
        assert!(gold_reward(league, false) < gold_reward(league, true));
    }
    assert_eq!(gold_reward(3, true), 30);
    assert_eq!(gold_reward(4, true), 44);
    assert!(slot_cost(20).unwrap() / gold_reward(19, true) > 800);
}
