use phalanx::{
    Mind, learned::Controller, navigation, progression::Campaign, region, terrain::Arena,
};
fn company() -> Campaign {
    let mut c = Campaign::new();
    c.teams[0].gold = 1000;
    c.unlock_fourth().unwrap();
    c
}
#[test]
fn larger_location_is_four_fighter_only_and_routes_follow_the_same_land() {
    let mut small = Campaign::new();
    assert!(region::address(small.battlefield(42)).is_none());
    let mut c = company();
    let arena = c.battlefield(42);
    assert_eq!(arena.half_size(), [50., 50.]);
    for side in 0..2 {
        for slot in 0..4 {
            assert!(!arena.blocked(arena.spawn(side, slot)));
        }
    }
    for goal in [
        [35., 30.],
        [-35., -30.],
        [0., 20.],
        [19., -15.],
        [-19., 15.],
    ] {
        let route =
            navigation::plan_known(arena, arena.spawn(0, 0), goal, Mind::disciplined(), &[]);
        assert!(!route.points.is_empty(), "unreachable {goal:?}");
        assert!(
            route
                .points
                .windows(2)
                .all(|p| arena.walk_segment(p[0], p[1]))
        );
    }
    for p in region::anchors() {
        assert!(arena.cover(p) > 0.);
        assert!(!arena.blocked(p), "cover inside wall {p:?}");
    }
    assert!(arena.elevation([19., -15.]) > 3.);
    assert!(
        arena.roughness([
            10.,
            region::road(
                match arena {
                    Arena::Pressure(f) => f,
                    _ => unreachable!(),
                },
                10.
            ) + 5.
        ]) > 0.5
    );
    assert!(arena.blocked([49., 0.]));
}
#[test]
fn return_visits_keep_destroyed_timber_identity_and_elapsed_time() {
    let mut c = company();
    let arena = c.battlefield(42);
    let (id, mut b) = c.begin_battle(7, arena, Controller::Utility).unwrap();
    let ids: Vec<_> = b.units.iter().map(|u| u.fighter_id.clone()).collect();
    // Terrain fixture represents a resolved breach; real attack destruction is
    // covered separately in procedural.rs. Capture must not repair that terrain.
    if let Arena::Pressure(ref mut f) = b.arena {
        f.destroyed = 1;
    }
    c.capture_minds(&b).unwrap();
    let path = std::env::temp_dir().join(format!("phalanx-region-{}.json", std::process::id()));
    c.save(&path).unwrap();
    let mut restored = Campaign::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    let next = restored.battlefield(999);
    assert_eq!(region::address(next).unwrap().seed, 42);
    assert_eq!(region::address(next).unwrap().terrain_revision, 1);
    assert!(!next.blocked([-5., -5.]));
    assert!(next.blocked([-18., 10.]));
    while !b.finished {
        b.step();
    }
    c.settle(id, &b).unwrap();
    assert_eq!(c.yard.as_ref().unwrap().elapsed_ticks, b.tick as u64);
    let saved = serde_json::to_string(&c).unwrap();
    assert!(c.settle(id, &b).is_err());
    assert_eq!(saved, serde_json::to_string(&c).unwrap());
    let next = c.battlefield(1000);
    let (_, next_battle) = c.begin_battle(1000, next, Controller::Utility).unwrap();
    assert_eq!(
        ids,
        next_battle
            .units
            .iter()
            .map(|u| u.fighter_id.clone())
            .collect::<Vec<_>>()
    );
    let report = c.last_report.as_ref().unwrap();
    assert_eq!(report.world_address.as_ref().unwrap().seed, 42);
    assert_eq!(report.gold[0], report.gold[1]);
    assert_eq!(report.salvage[0], report.salvage[1]);
}
#[test]
fn old_campaigns_do_not_acquire_invented_world_history() {
    let c = Campaign::new();
    let mut value = serde_json::to_value(c).unwrap();
    value.as_object_mut().unwrap().remove("yard");
    let c: Campaign = serde_json::from_value(value).unwrap();
    c.validate().unwrap();
    assert!(c.yard.is_none());
    assert!(region::address(Arena::Ridge).is_none());
}

#[test]
fn recorded_region_combat_replays_and_keeps_location_context_on_each_frame() {
    for seed in [6, 42, 72] {
        let mut c = company();
        let arena = c.battlefield(seed);
        let (_, mut b) = c.begin_battle(seed, arena, Controller::Utility).unwrap();
        let mut reference = b.clone();
        let root = std::env::temp_dir().join(format!(
            "phalanx-region-trace-{}-{seed}",
            std::process::id()
        ));
        let mut recorder = phalanx::study::Recorder::start(&root, &b).unwrap();
        while !b.finished {
            recorder.step(&mut b).unwrap();
            reference.step();
        }
        assert_eq!(
            serde_json::to_value(&b).unwrap(),
            serde_json::to_value(reference).unwrap()
        );
        let rows = std::fs::read_to_string(recorder.directory.join("decisions.jsonl")).unwrap();
        for line in rows.lines() {
            let r: serde_json::Value = serde_json::from_str(line).unwrap();
            if ["start", "combat_frame", "end"].contains(&r["event"].as_str().unwrap()) {
                assert_eq!(r["world_context"]["location"], "hollow_procession");
                assert_eq!(r["world_context"]["seed"], seed);
            }
        }
        let report: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(recorder.directory.join("report.json")).unwrap(),
        )
        .unwrap();
        println!(
            "region seed={seed} ticks={} damage={} mean_ms={} max_ms={}",
            b.tick,
            b.damage_events.len(),
            report["mean_recorded_step_wall_ms"],
            report["max_recorded_step_wall_ms"]
        );
        assert!(
            !b.damage_events.is_empty(),
            "larger map never brought opponents into contact"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
