use phalanx::{Battle, Mind, Rock, Role, navigation::plan, terrain::Arena};

#[test]
fn every_team_has_three_fighters_and_two_rock_throwers() {
    let b = Battle::new(42, false);
    for team in 0..2 {
        assert_eq!(
            b.units
                .iter()
                .filter(|u| u.team == team && u.role == Role::RockThrower)
                .count(),
            2
        );
        assert_eq!(
            b.units
                .iter()
                .filter(|u| u.team == team && u.role == Role::Fighter)
                .count(),
            3
        );
    }
}
#[test]
fn choke_route_crosses_the_passage_and_never_a_wall() {
    let arena = Arena::Choke;
    let route = plan(arena, [-8.0, 5.0], [8.0, 5.0], Mind::aggressive(), 0, &[]);
    assert!(!route.points.is_empty());
    assert!(
        route
            .points
            .windows(2)
            .all(|w| arena.clear_segment(w[0], w[1]))
    );
    assert!(
        route
            .points
            .iter()
            .any(|p| p[0].abs() < 1.3 && p[1].abs() < 1.7)
    );
    assert!(!arena.clear_segment([-2.0, 5.0], [2.0, 5.0]));
}
#[test]
fn caution_changes_routes_through_the_same_terrain() {
    let mut low = Mind::aggressive();
    low.caution = 0.0;
    let mut high = low;
    high.caution = 1.0;
    let direct = plan(Arena::Ridge, [-8.0, 0.0], [8.0, 0.0], low, 0, &[]);
    let protected = plan(Arena::Ridge, [-8.0, 0.0], [8.0, 0.0], high, 0, &[]);
    assert_ne!(direct.points, protected.points);
    let cover = |r: &phalanx::navigation::Route| {
        r.points
            .iter()
            .filter(|p| Arena::Ridge.cover(**p) > 0.0)
            .count()
    };
    assert!(
        cover(&protected) > cover(&direct),
        "higher caution should use more covered cells"
    );
}
#[test]
fn rock_damage_is_delayed_until_impact() {
    let mut b = Battle::new(42, false);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[0].health = 100.0;
    b.units[0].role = Role::RockThrower;
    b.units[0].position = [-4.0, 0.0];
    b.units[5].health = 100.0;
    b.units[5].role = Role::Fighter;
    b.units[5].position = [0.6, 0.0];
    b.step();
    assert_eq!(b.units[5].health, 100.0);
    assert_eq!(b.rocks.len(), 1);
    let flight = b.rocks[0].flight_ticks;
    for _ in 0..flight {
        b.step();
    }
    assert!(b.units[5].health < 100.0);
    assert_eq!(b.units[0].rocks_hit, 1);
}
#[test]
fn walls_block_throws_even_when_the_enemy_is_in_range() {
    let mut b = Battle::with_arena(42, false, Arena::Choke);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[0].health = 100.0;
    b.units[0].role = Role::RockThrower;
    b.units[0].position = [-2.0, 4.0];
    b.units[5].health = 100.0;
    b.units[5].role = Role::RockThrower;
    b.units[5].position = [2.0, 4.0];
    b.step();
    assert!(b.rocks.is_empty());
    assert_eq!(b.units[0].health, 100.0);
    assert_eq!(b.units[5].health, 100.0);
}
#[test]
fn impact_can_miss_and_cover_reduces_damage() {
    for (aim, health) in [([-4.5, 5.0], 93.0), ([0.0, 0.0], 100.0)] {
        let mut b = Battle::new(42, false);
        for u in &mut b.units {
            u.health = 0.0;
        }
        b.units[5].health = 100.0;
        b.units[5].position = [-4.5, 5.0];
        b.rocks.push(Rock {
            weapon: phalanx::weapons::ProjectileWeapon::Rock,
            shooter: 0,
            team: 0,
            from: [-8.0, 5.0],
            to: aim,
            age: 0,
            flight_ticks: 1,
            damage: 10.0,
        });
        b.step();
        assert_eq!(b.units[5].health, health);
    }
}
#[test]
fn live_units_stay_outside_walls_in_all_arenas() {
    for arena in [Arena::Ridge, Arena::Choke, Arena::OpenField] {
        let mut b = Battle::with_arena(7, false, arena);
        for _ in 0..phalanx::BATTLE_TICK_LIMIT {
            b.step();
            for unit in &b.units {
                assert!(
                    !arena.blocked(unit.position),
                    "{} at {:?}",
                    unit.id,
                    unit.position
                );
            }
        }
        assert!(b.finished);
    }
}

#[test]
fn rough_ground_reduces_actual_movement_speed() {
    let travel = |z| {
        let mut b = Battle::with_arena(42, false, Arena::Choke);
        for u in &mut b.units {
            u.health = 0.0;
        }
        b.units[0].health = 100.0;
        b.units[0].position = [-3.5, z];
        b.units[5].health = 100.0;
        b.units[5].position = [8.0, z];
        b.step();
        b.units[0].distance_moved
    };
    assert!(travel(0.0) < travel(3.0));
}

#[test]
fn a_thrown_rock_can_survive_its_thrower_and_force_a_draw() {
    let mut b = Battle::with_arena(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[5].health = 5.0;
    b.units[5].position = [3.0, 0.0];
    b.rocks.push(Rock {
        weapon: phalanx::weapons::ProjectileWeapon::Rock,
        shooter: 0,
        team: 0,
        from: [0.0, 0.0],
        to: [3.0, 0.0],
        age: 0,
        flight_ticks: 2,
        damage: 10.0,
    });
    b.step();
    assert!(
        !b.finished,
        "wait for airborne rocks before declaring a winner"
    );
    b.step();
    assert_eq!(b.result, "Draw");
}
