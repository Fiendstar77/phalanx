use phalanx::{
    Battle, CommandTitle, FighterClass, Mind,
    navigation::plan,
    pressure::{self, Lesson},
    progression::Campaign,
    psyche::{Reaction, Response},
    terrain::Arena,
};

fn paint(campaign: &mut Campaign, mind: Mind, class: FighterClass, isolation: f32) {
    for team in &mut campaign.teams {
        for fighter in &mut team.fighters {
            fighter.mind = mind;
            if fighter.title != CommandTitle::General {
                fighter.class = class;
            }
            if isolation > 0.0 {
                fighter.psyche.isolation = Response {
                    reaction: Reaction::Fear,
                    intensity: isolation,
                };
            }
        }
    }
}

fn lesson(teams: &[phalanx::progression::Phalanx; 2], seed: u64) -> Lesson {
    match pressure::generate(teams, seed) {
        Arena::Pressure(field) => field.lesson,
        other => panic!("expected a pressure field, got {other:?}"),
    }
}

#[test]
fn the_same_squads_and_seed_rebuild_the_same_field() {
    let campaign = Campaign::new();
    let first = pressure::generate(&campaign.teams, 11);
    let second = pressure::generate(&campaign.teams, 11);
    assert_eq!(first, second);
    let encoded = serde_json::to_string(&first).unwrap();
    let decoded: Arena = serde_json::from_str(&encoded).unwrap();
    assert_eq!(first, decoded);
    assert_eq!(
        Arena::Pressure(match first {
            Arena::Pressure(field) => field,
            _ => unreachable!(),
        })
        .next(),
        Arena::Ridge
    );
}

#[test]
fn lessons_follow_the_shared_weakness_and_ignore_ranged_labels() {
    let fearful = Mind {
        aggression: 0.2,
        caution: 0.9,
        courage: 0.4,
        teamwork: 0.2,
        patience: 0.9,
        self_preservation: 0.5,
        discipline: 0.5,
    };
    let mut isolated = Campaign::new();
    paint(&mut isolated, fearful, FighterClass::Scout, 0.9);
    assert_eq!(lesson(&isolated.teams, 4), Lesson::Split);

    let reckless = Mind {
        aggression: 0.95,
        caution: 0.05,
        courage: 0.8,
        teamwork: 0.85,
        patience: 0.45,
        self_preservation: 0.2,
        discipline: 0.3,
    };
    let mut exposed = Campaign::new();
    paint(&mut exposed, reckless, FighterClass::Scout, 0.0);
    assert_eq!(lesson(&exposed.teams, 4), Lesson::Exposure);

    let steady = Mind {
        aggression: 0.2,
        caution: 0.85,
        courage: 0.6,
        teamwork: 0.9,
        patience: 0.9,
        self_preservation: 0.5,
        discipline: 0.8,
    };
    let mut slow = Campaign::new();
    paint(&mut slow, steady, FighterClass::Guardian, 0.0);
    assert_eq!(lesson(&slow.teams, 4), Lesson::Approach);

    let mut warriors = exposed.clone();
    let mut rangers = exposed.clone();
    paint(&mut warriors, reckless, FighterClass::Warrior, 0.0);
    paint(&mut rangers, reckless, FighterClass::Ranger, 0.0);
    assert_eq!(
        pressure::generate(&warriors.teams, 4),
        pressure::generate(&rangers.teams, 4),
        "a ranged label must not pick a different field"
    );
}

#[test]
fn a_repeated_lesson_is_skipped_for_the_next_best() {
    let fearful = Mind {
        aggression: 0.2,
        caution: 0.9,
        courage: 0.4,
        teamwork: 0.2,
        patience: 0.9,
        self_preservation: 0.5,
        discipline: 0.5,
    };
    let mut campaign = Campaign::new();
    paint(&mut campaign, fearful, FighterClass::Scout, 0.9);
    assert_eq!(lesson(&campaign.teams, 2), Lesson::Split);
    campaign.teams[0].fighters[0]
        .history
        .push(pressure::fought(Lesson::Split));
    assert_ne!(lesson(&campaign.teams, 2), Lesson::Split);
}

#[test]
fn every_lesson_is_symmetric_and_has_a_route_for_both_sides() {
    let fields = [
        Lesson::Split,
        Lesson::Ridge,
        Lesson::Exposure,
        Lesson::Approach,
    ]
    .into_iter()
    .flat_map(|lesson| {
        [0u8, 1, 2].into_iter().flat_map(move |reach| {
            [false, true].into_iter().flat_map(move |wide| {
                [0, 1].map(move |cover_extra| phalanx::pressure::PressureField {
                    lesson,
                    cover_extra,
                    wide,
                    reach,
                    ..Default::default()
                })
            })
        })
    });
    for field in fields {
        let arena = Arena::Pressure(field);
        let hx = field.half()[0] as i32 - 1;
        let hz = field.half()[1] as i32 - 1;
        for x in -hx..=hx {
            for z in -hz..=hz {
                let p = [x as f32, z as f32];
                let mirror = [-p[0], p[1]];
                let flip = [p[0], -p[1]];
                assert_eq!(arena.blocked(p), arena.blocked(mirror));
                assert_eq!(arena.blocked(p), arena.blocked(flip));
                assert!((arena.elevation(p) - arena.elevation(mirror)).abs() < 1e-5);
                assert!((arena.elevation(p) - arena.elevation(flip)).abs() < 1e-5);
                assert_eq!(arena.cover(p), arena.cover(mirror));
                assert_eq!(arena.roughness(p), arena.roughness(flip));
            }
        }
        let start = [-field.spawn_x(), 0.0];
        let goal = [field.spawn_x(), 0.0];
        assert!(!arena.blocked(start) && !arena.blocked(goal));
        let route = plan(arena, start, goal, Mind::disciplined(), 0, &[]);
        assert!(
            route.points.len() > 2,
            "{:?} produced no route around {:?}",
            field.lesson,
            arena.walls()
        );
        assert!(
            route
                .points
                .iter()
                .all(|p| !arena.blocked(*p) || *p == start || *p == goal)
        );
    }
    let split = Arena::Pressure(phalanx::pressure::PressureField {
        lesson: Lesson::Split,
        cover_extra: 0,
        wide: false,
        reach: 0,
        ..Default::default()
    });
    assert!(split.blocked([0.0, 0.0]));
    assert!(!split.blocked([0.0, 4.5]));
    assert!(!split.blocked([0.0, -4.5]));
    assert!(split.clear_segment([-8.0, 4.5], [8.0, 4.5]));
    assert!(split.clear_segment([-8.0, -4.5], [8.0, -4.5]));
    assert!(!split.clear_segment([-8.0, 0.0], [8.0, 0.0]));
    let hill = Arena::Pressure(phalanx::pressure::PressureField {
        lesson: Lesson::Ridge,
        cover_extra: 0,
        wide: false,
        reach: 0,
        ..Default::default()
    });
    assert!(hill.elevation([0.0, 0.0]) > hill.elevation([8.0, 0.0]));
    assert!(hill.has_high_ground());
    let crossing = Arena::Pressure(phalanx::pressure::PressureField {
        lesson: Lesson::Approach,
        cover_extra: 0,
        wide: true,
        reach: 0,
        ..Default::default()
    });
    assert!(crossing.roughness([0.0, 2.0]) > 0.0);
    assert_eq!(crossing.roughness([9.0, 0.0]), 0.0);
    assert!((crossing.spawn(0, 0)[0] + crossing.spawn(1, 0)[0]).abs() < 1e-5);

    let skirmish = Arena::Pressure(phalanx::pressure::PressureField {
        lesson: Lesson::Split,
        cover_extra: 0,
        wide: false,
        reach: 1,
        ..Default::default()
    });
    assert_eq!(skirmish.half_size(), [18.0, 13.0]);
    assert!(skirmish.blocked([0.0, 0.0]));
    assert!(!skirmish.blocked([0.0, 7.0]));
    assert!(skirmish.clear_segment([-14.0, 7.0], [14.0, 7.0]));
    assert!(skirmish.clear_segment([-14.0, -7.0], [14.0, -7.0]));
    let company = Arena::Pressure(phalanx::pressure::PressureField {
        lesson: Lesson::Split,
        cover_extra: 0,
        wide: true,
        reach: 2,
        ..Default::default()
    });
    assert_eq!(company.half_size(), [24.0, 17.0]);
    assert_eq!(company.walls().len(), 3);
    assert!(company.blocked([0.0, 0.0]));
    assert!(company.blocked([12.0, 0.0]));
    assert!(company.clear_segment([-20.0, 10.0], [20.0, 10.0]));
    assert!(company.clear_segment([-20.0, -10.0], [20.0, -10.0]));
    let hill = Arena::Pressure(phalanx::pressure::PressureField {
        lesson: Lesson::Ridge,
        cover_extra: 0,
        wide: false,
        reach: 1,
        ..Default::default()
    });
    assert!(hill.elevation([0.0, 0.0]) > hill.elevation([16.0, 0.0]));
    let crossing = Arena::Pressure(phalanx::pressure::PressureField {
        lesson: Lesson::Approach,
        cover_extra: 0,
        wide: false,
        reach: 1,
        ..Default::default()
    });
    assert!(crossing.roughness([0.0, 0.0]) > 0.0);
    assert_eq!(crossing.roughness([16.0, 0.0]), 0.0);

    let three = pressure::generate(&Campaign::new().teams, 3);
    match three {
        Arena::Pressure(field) => {
            assert_eq!(field.reach, 1);
            assert_eq!(field.half(), [18.0, 13.0]);
        }
        other => panic!("expected a pressure field, got {other:?}"),
    }
    let mut four = Campaign::new();
    let extra = four.teams[0].fighters[0].clone();
    four.teams[0].fighters.push(extra);
    match pressure::generate(&four.teams, 3) {
        Arena::Pressure(field) => assert_eq!(field.reach, 2),
        other => panic!("expected a pressure field, got {other:?}"),
    }
    let old = r#"{"pressure":{"lesson":"ridge","cover_extra":1,"wide":true}}"#;
    match serde_json::from_str::<Arena>(old).unwrap() {
        Arena::Pressure(field) => {
            assert_eq!(field.reach, 0);
            assert_eq!(field.half(), [11.0, 8.0]);
        }
        other => panic!("expected a pressure field, got {other:?}"),
    }
}

#[test]
fn founders_can_fight_on_the_generated_field() {
    let mut campaign = Campaign::new();
    let arena = pressure::generate(&campaign.teams, 5);
    let (id, mut battle) = campaign
        .begin_battle(5, arena, phalanx::learned::Controller::Utility)
        .unwrap();
    assert!(!battle.pressure_note.is_empty());
    assert!(
        battle
            .units
            .iter()
            .all(|unit| !arena.blocked(unit.position))
    );
    assert_eq!(battle.units[0].position[0], -battle.units[3].position[0]);
    for _ in 0..300 {
        battle.step();
        for unit in &battle.units {
            assert!(unit.position.iter().all(|value| value.is_finite()));
            let p =
                phalanx::ninjutsu::body(unit, battle.arena, battle.tick.saturating_sub(1) as f32);
            assert!(
                !phalanx::ninjutsu::solid(battle.arena, [p[0], p[1] - 0.85, p[2]], 0.),
                "point {:?} unit {} tick {} ninja {:?} arena {:?}",
                p,
                unit.id,
                battle.tick,
                unit.ninja,
                battle.arena
            );
        }
    }
    if battle.finished {
        campaign.settle(id, &battle).unwrap();
        assert!(campaign.settle(id, &battle).is_err());
    } else {
        assert!(campaign.settle(id, &battle).is_err());
    }
    let mut lab = Battle::new(5, false);
    lab.step();
    assert!(
        !serde_json::to_string(&lab)
            .unwrap()
            .contains("pressure_note")
    );
}
