use phalanx::{
    Mind, learned::Controller, navigation, pressure, procedural, progression::Campaign,
    terrain::Arena,
};
use std::collections::{BTreeSet, VecDeque};

/// Independent flood fill on the actual navigation grid: no disconnected pockets or spawns.
fn reachable_floor(arena: Arena) {
    let [hx, hz] = arena.half_size();
    let w = (hx * 2.) as usize;
    let h = (hz * 2.) as usize;
    let point = |id: usize| [(id % w) as f32 - hx + 0.5, (id / w) as f32 - hz + 0.5];
    let open: Vec<_> = (0..w * h).map(|i| !arena.blocked(point(i))).collect();
    let start = open.iter().position(|v| *v).unwrap();
    let mut reached = vec![false; w * h];
    reached[start] = true;
    let mut queue = VecDeque::from([start]);
    while let Some(i) = queue.pop_front() {
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let x = (i % w) as i32 + dx;
            let z = (i / w) as i32 + dz;
            if x < 0 || z < 0 || x >= w as i32 || z >= h as i32 {
                continue;
            }
            let j = z as usize * w + x as usize;
            if open[j] && !reached[j] && arena.walk_segment(point(i), point(j)) {
                reached[j] = true;
                queue.push_back(j);
            }
        }
    }
    for (i, is_open) in open.iter().enumerate() {
        assert!(
            !is_open || reached[i],
            "isolated floor {:?} in {arena:?}",
            point(i)
        );
    }
    assert!(open.iter().filter(|v| **v).count() as f32 / open.len() as f32 > 0.72);
}

#[test]
fn seeded_grammar_preserves_clearance_fairness_and_three_connected_fronts() {
    let campaign = Campaign::new();
    let mut families = BTreeSet::new();
    let mut geometry = BTreeSet::new();
    for reach in [1, 2] {
        for seed in 0..48 {
            let Arena::Pressure(mut f) = pressure::generate(&campaign.teams, seed) else {
                panic!()
            };
            f.reach = reach;
            f.wide = reach == 2;
            families.insert(f.layout);
            let arena = Arena::Pressure(f);
            geometry.insert(format!("{:?}", arena.walls()));
            assert!(procedural::connected(f), "{f:?}");
            reachable_floor(arena);
            for side in 0..2 {
                for slot in 0..if reach == 1 { 3 } else { 4 } {
                    assert!(!arena.blocked(arena.spawn(side, slot)), "{f:?}");
                    for leg in 0..10 {
                        assert!(
                            !arena.blocked(arena.search_waypoint(side, slot, leg)),
                            "blocked patrol objective in {f:?}"
                        );
                    }
                }
            }
            for x in -15..=15 {
                for z in -11..=11 {
                    let p = [x as f32 * 0.9, z as f32 * 0.9];
                    let mirror = [-p[0], p[1]];
                    assert_eq!(arena.blocked(p), arena.blocked(mirror));
                    assert!((arena.elevation(p) - arena.elevation(mirror)).abs() < 1e-5);
                    assert_eq!(arena.cover(p), arena.cover(mirror));
                    assert_eq!(arena.roughness(p), arena.roughness(mirror));
                }
            }
            let peak = arena.high_ground(arena.spawn(0, 0));
            assert!(!arena.blocked(peak), "high ground inside a wall: {f:?}");
            assert!(arena.elevation(peak) > 0.7, "{f:?}");
            for p in procedural::cover_anchors(f) {
                assert!(!arena.blocked(p), "cover anchor in wall {p:?}: {f:?}");
                assert!(arena.cover(p) > 0.);
            }
        }
    }
    assert_eq!(families.len(), 4);
    assert!(
        geometry.len() > 80,
        "seed should change actual geometry, not only its label"
    );
}

#[test]
fn breaches_remove_only_timber_and_leave_all_routes_legal() {
    for seed in 0..8 {
        let Arena::Pressure(f) = pressure::generate(&Campaign::new().teams, seed) else {
            panic!()
        };
        let original = procedural::original_walls(f);
        let cleared = Arena::Pressure(pressure::PressureField { destroyed: 15, ..f });
        for (i, (center, _)) in original.iter().enumerate() {
            assert_eq!(cleared.blocked(*center), i >= 4);
        }
        reachable_floor(cleared);
    }
}

#[test]
fn real_combat_breaches_generated_timber_without_damaging_masonry() {
    let Arena::Pressure(field) = pressure::generate(&Campaign::new().teams, 6) else {
        unreachable!()
    };
    let walls = procedural::original_walls(field);
    let (center, extent) = walls[0];
    let mut b = phalanx::Battle::with_founders(6, false, Arena::Pressure(field));
    for u in &mut b.units {
        u.health = 0.;
    }
    b.units[0].health = 100.;
    b.units[0].position = [center[0] - extent[0] * 0.5 - 0.8, center[1]];
    b.units[3].health = 100.;
    b.units[3].position = [center[0] + extent[0] * 0.5 + 0.8, center[1]];
    let enemy = b.units[3].clone();
    b.units[0].contacts.push(phalanx::perception::Contact {
        enemy: 3,
        position: enemy.position,
        health: 100.,
        role: enemy.role,
        range: enemy.attack_range(),
        seen_tick: 0,
        source: phalanx::perception::Source::OwnSight,
        visible: false,
    });
    b.barricade_health[0] = 1.;
    assert!(
        !b.arena
            .clear_segment(b.units[0].position, b.units[3].position)
    );
    b.step();
    let Arena::Pressure(after) = b.arena else {
        unreachable!()
    };
    assert_eq!(after.destroyed & 1, 1);
    assert!(!b.arena.blocked(center));
    assert!(walls.iter().skip(4).all(|(p, _)| b.arena.blocked(*p)));
    assert_eq!(b.initial_arena, Arena::Pressure(field));
}

#[test]
fn protected_flanks_offer_an_actual_tradeoff_against_an_exposed_shortcut() {
    let c = Campaign::new();
    let mut detours = 0;
    for seed in 0..12 {
        let arena = pressure::generate(&c.teams, seed);
        let Arena::Pressure(f) = arena else {
            unreachable!()
        };
        let start = [-f.spawn_x(), 0.];
        let goal = [f.spawn_x(), 0.];
        let direct = navigation::plan_known(
            arena,
            start,
            goal,
            Mind {
                caution: 0.,
                self_preservation: 0.,
                ..Mind::aggressive()
            },
            &[],
        );
        let threats = [navigation::Threat {
            position: [0., 0.],
            range: 6.,
            confidence: 1.,
        }];
        let safe = navigation::plan_known(arena, start, goal, Mind::disciplined(), &threats);
        let danger = |points: &[[f32; 2]]| {
            points
                .iter()
                .map(|p| navigation::threat_at_known(arena, *p, &threats))
                .sum::<f32>()
        };
        if safe.cost.distance > direct.cost.distance + 1.
            && danger(&safe.points) < danger(&direct.points)
        {
            detours += 1;
        }
    }
    assert!(
        detours >= 8,
        "only {detours} maps offered a longer, safer route"
    );
}

#[test]
fn distinct_seeded_graphs_and_serialized_maps_replay_exactly() {
    let campaign = Campaign::new();
    for seed in [0, 1, 2, 3, 42, 100, u64::MAX] {
        let arena = pressure::generate(&campaign.teams, seed);
        let loaded: Arena = serde_json::from_str(&serde_json::to_string(&arena).unwrap()).unwrap();
        assert_eq!(arena, loaded);
        let route = navigation::plan(
            arena,
            arena.spawn(0, 0),
            arena.spawn(1, 0),
            Mind::disciplined(),
            0,
            &[],
        );
        assert!(!route.points.is_empty());
        assert!(
            route
                .points
                .windows(2)
                .all(|p| arena.walk_segment(p[0], p[1])),
            "cached graph crossed a different seed's walls"
        );
        let (_, mut a) = campaign
            .clone()
            .begin_battle(seed, arena, Controller::Utility)
            .unwrap();
        let (_, mut b) = campaign
            .clone()
            .begin_battle(seed, loaded, Controller::Utility)
            .unwrap();
        for _ in 0..60 {
            a.step();
            b.step();
        }
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }
}

#[test]
fn each_family_produces_real_combat_in_both_playable_leagues() {
    let mut seen = BTreeSet::new();
    for seed in 0..30 {
        let mut c = Campaign::new();
        let Arena::Pressure(f) = pressure::generate(&c.teams, seed) else {
            panic!()
        };
        if !seen.insert(f.layout) {
            continue;
        }
        for count in [3, 4] {
            if count == 4 {
                c.teams[0].gold = 1000;
                c.unlock_fourth().unwrap();
            }
            let arena = pressure::generate(&c.teams, seed);
            let (_, mut battle) = c.begin_battle(seed, arena, Controller::Utility).unwrap();
            while !battle.finished {
                battle.step();
            }
            assert!(
                battle.units.iter().map(|u| u.damage_dealt).sum::<f32>() > 50.,
                "no engagement: {arena:?}"
            );
            assert!(
                battle
                    .units
                    .iter()
                    .all(|u| !battle.arena.blocked(u.position))
            );
            assert!(
                battle.winner().is_some(),
                "timeout {arena:?} tick {}",
                battle.tick
            );
        }
    }
    assert_eq!(seen.len(), 4);
}
