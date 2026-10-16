use phalanx::{
    Battle, Mind, distance,
    learned::Controller,
    pressure::{Lesson, PressureField},
    progression::Campaign,
    psyche::{Reaction, Response},
    terrain::Arena,
};

#[test]
fn saturated_scars_do_not_freeze_healthy_veterans_or_produce_endless_flight() {
    for arena in [
        Arena::OpenField,
        Arena::Pressure(PressureField {
            lesson: Lesson::Approach,
            cover_extra: 0,
            wide: false,
            reach: 1,
            ..Default::default()
        }),
    ] {
        let mut campaign = Campaign::new();
        for (team, squad) in campaign.teams.iter_mut().enumerate() {
            for fighter in &mut squad.fighters {
                fighter.mind = Mind::disciplined();
                fighter.psyche.encirclement = Response {
                    reaction: if team == 0 {
                        Reaction::Hesitation
                    } else {
                        Reaction::Fear
                    },
                    intensity: 1.0,
                };
                fighter.psyche.failed_charge = Response {
                    reaction: Reaction::Hesitation,
                    intensity: 1.0,
                };
            }
        }
        let (_, mut battle) = campaign
            .begin_battle(64, arena, Controller::Utility)
            .unwrap();
        while !battle.finished {
            battle.step();
        }
        assert!(battle.tick < 1200, "Veterans stalled in {arena:?}");
        for team in 0..2 {
            assert!(
                battle
                    .units
                    .iter()
                    .filter(|u| u.team == team)
                    .any(|u| u.damage_dealt > 0.0),
                "Team {team} never fought in {arena:?}"
            );
        }
    }
}

#[test]
fn arriving_in_an_empty_enemy_deployment_zone_starts_a_sweep() {
    for arena in [
        Arena::OpenField,
        Arena::Pressure(PressureField {
            lesson: Lesson::Approach,
            cover_extra: 0,
            wide: false,
            reach: 1,
            ..Default::default()
        }),
    ] {
        let mut battle = Battle::with_founders(42, false, arena);
        for unit in &mut battle.units {
            unit.health = if unit.id == 0 || unit.id == 3 {
                100.0
            } else {
                0.0
            };
            unit.mind = Mind::aggressive();
        }
        let arrived = [arena.spawn(1, 0)[0], -1.6];
        battle.units[0].position = arrived;
        battle.units[3].position = [-arrived[0], arena.half_size()[1] - 2.0];
        battle.tick = 600;
        battle.step();
        assert!(battle.units[0].contacts.is_empty());
        assert_eq!(battle.units[0].search_waypoint, 1);
        assert!(distance(battle.units[0].target, arrived) > 2.0);
        for _ in 0..30 {
            battle.step();
        }
        assert!(
            distance(battle.units[0].position, arrived) > 1.0,
            "Search failed to leave empty territory in {arena:?}"
        );
    }
}

#[test]
fn a_crowded_wounded_squad_still_makes_forward_progress() {
    let arena = Arena::GrandArena;
    let mut battle = Battle::with_founders(42, false, arena);
    for unit in &mut battle.units {
        let slot = unit.id % 10;
        unit.position = [
            if unit.team == 0 { -18.0 } else { 18.0 },
            -14.0 + slot as f32 * 0.07,
        ];
        unit.mind = Mind::aggressive();
        unit.lost_limbs = 12;
    }
    let before = battle.units.iter().map(|u| u.position).collect::<Vec<_>>();
    for _ in 0..30 {
        battle.step();
    }
    for (unit, start) in battle.units.iter().zip(before) {
        assert!(
            distance(unit.position, start) > 1.0,
            "Crowd pinned unit {}",
            unit.id
        );
        assert!(!arena.blocked(unit.position));
    }
}
