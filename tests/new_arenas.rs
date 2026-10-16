use phalanx::{Battle, Mind, navigation::plan, terrain::Arena};
#[test]
fn new_battlefields_have_connected_spawns_and_real_enemy_contact() {
    for arena in [Arena::Gauntlet, Arena::Quarry, Arena::Mire] {
        assert!(arena.large());
        assert!(!arena.challenge().is_empty());
        assert_eq!(
            Arena::parse(match arena {
                Arena::Gauntlet => "gauntlet",
                Arena::Quarry => "quarry",
                _ => "mire",
            }),
            Some(arena)
        );
        for team in 0..2 {
            for slot in 0..10 {
                assert!(!arena.blocked(arena.spawn(team, slot)));
            }
        }
        for leg in 1..5 {
            assert!(
                !arena.blocked(arena.search_waypoint(0, 0, leg)),
                "patrol {arena:?} {leg}"
            );
        }
        let route = plan(
            arena,
            arena.spawn(0, 0),
            arena.spawn(1, 0),
            Mind::aggressive(),
            0,
            &[],
        );
        assert!(!route.points.is_empty(), "{arena:?}");
        assert!(
            route
                .points
                .windows(2)
                .all(|p| arena.walk_segment(p[0], p[1]))
        );
        let mut battle = Battle::with_founders(42, false, arena);
        while !battle.finished {
            battle.step();
            assert!(battle.units.iter().all(|u| !arena.blocked(u.position)));
        }
        assert!(
            battle.units.iter().any(|u| u.damage_dealt > 0.0),
            "No contact in {arena:?}"
        );
    }
}
#[test]
fn the_mire_rewards_dry_causeways_and_quarry_has_contested_elevation() {
    assert!(Arena::Mire.roughness([0.0, 0.0]) > Arena::Mire.roughness([0.0, 6.0]));
    assert!(Arena::Quarry.elevation([-6.0, -9.0]) > 3.0);
    assert!(Arena::Quarry.blocked([0.0, 0.0]));
}
