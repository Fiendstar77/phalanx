use phalanx::{
    Battle, FighterClass, Mind,
    learned::Controller,
    psyche::{Reaction, Response},
    telemetry::FEATURE_NAMES,
    terrain::Arena,
};

fn isolated_fighters(size: usize, controller: Controller) -> Battle {
    let mut battle = Battle::with_roster(44, false, Arena::OpenField, true, size);
    battle.controller = controller;
    for unit in &mut battle.units {
        unit.health = if unit.id % size == 0 {
            unit.max_health()
        } else {
            0.
        };
        unit.mind = Mind::disciplined();
        unit.psyche.isolation = Response {
            reaction: if unit.team == 0 {
                Reaction::Fear
            } else {
                Reaction::Attachment
            },
            intensity: 1.,
        };
        unit.class = Some(FighterClass::Warrior);
    }
    battle
}

#[test]
fn solo_and_last_survivor_decisions_cannot_regroup_without_an_ally() {
    for size in [1, 3] {
        for controller in [Controller::Utility, Controller::Learned] {
            let mut battle = isolated_fighters(size, controller);
            let records = battle.step_recorded();
            assert_eq!(records.len(), 2);
            for record in records {
                assert!(
                    !record.action_mask[4],
                    "cannot regroup with no available ally"
                );
                assert!(!record.action_mask[8], "cannot support a nonexistent ally");
                assert_ne!(record.action, "REGROUP");
                assert!(
                    record.features.iter().all(|v| v.is_finite()),
                    "solo observation must be finite"
                );
                for feature in ["nearby_allies", "engaged_allies"] {
                    let index = FEATURE_NAMES.iter().position(|f| *f == feature).unwrap();
                    assert_eq!(record.features[index], 0.);
                }
                assert!(record.comparison.unwrap().learned.is_some());
            }
            assert_eq!(battle.learned_fallbacks, 0);
        }
    }
}

#[test]
fn fearful_duelists_and_last_survivors_approach_and_resolve_a_real_hit() {
    for size in [1, 3] {
        let mut battle = isolated_fighters(size, Controller::Utility);
        let memories = [battle.units[0].psyche, battle.units[size].psyche];
        for _ in 0..240 {
            battle.step();
            if !battle.damage_events.is_empty() {
                break;
            }
        }
        assert!(
            battle.units[0].distance_moved > 2.,
            "the fearful fighter must leave its spawn"
        );
        assert!(
            battle.units[size].distance_moved > 2.,
            "the attached fighter must leave its spawn"
        );
        assert!(
            battle
                .damage_events
                .iter()
                .any(|e| e.amount > 0. && e.attacker != e.defender),
            "a legal duel must make contact within 24 simulated seconds"
        );
        assert_eq!(battle.units[0].psyche, memories[0]);
        assert_eq!(battle.units[size].psyche, memories[1]);
    }
}
