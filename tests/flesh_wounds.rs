use phalanx::{
    Battle, FighterClass as C,
    injuries::Flesh,
    learned::Controller,
    progression::Campaign,
    tactics::{Phase, State},
    terrain::Arena,
    theus::Sheet,
};
fn duel(seed: u64) -> (Campaign, u64, Battle) {
    let mut campaign = Campaign::new();
    for team in &mut campaign.teams {
        for f in &mut team.fighters {
            f.class = C::Warrior;
            f.mind = phalanx::styles::BattleStyle::Vanguard.mind();
            f.theus = Some(Sheet {
                attributes: [14, 12, 10, 10],
                practice: [200, 0, 0, 0, 0, 0],
                ..Default::default()
            });
        }
    }
    let (id, mut battle) = campaign
        .begin_battle(seed, Arena::OpenField, Controller::Utility)
        .unwrap();
    for u in &mut battle.units {
        u.health = 0.;
    }
    for (id, p) in [(0, [-0.8, 0.]), (3, [0.8, 0.])] {
        battle.units[id].health = battle.units[id].max_health();
        battle.units[id].position = p;
        battle.units[id].tactics = State {
            scout: Some(id),
            first_report: Some(0),
            phase: Phase::Engage,
        };
    }
    (campaign, id, battle)
}
fn actual_wound() -> (Campaign, u64, Battle, Battle) {
    for seed in 0..40 {
        let (campaign, id, initial) = duel(seed);
        let mut battle = initial.clone();
        while !battle.finished {
            battle.step();
            if battle.units.iter().any(|u| !u.flesh.blank()) {
                return (campaign, id, initial, battle);
            }
        }
    }
    panic!("no severe cutting wound in the bounded real-combat fixture");
}
#[test]
fn wounds_have_real_qualifying_hits_and_replay_exactly() {
    let (_, _, mut replay, battle) = actual_wound();
    while replay.tick < battle.tick {
        replay.step();
    }
    assert_eq!(
        serde_json::to_value(&replay).unwrap(),
        serde_json::to_value(&battle).unwrap()
    );
    for unit in &battle.units {
        for mark in &unit.flesh.marks {
            let hit = battle
                .damage_events
                .iter()
                .find(|e| e.sequence == mark.hit_sequence)
                .unwrap();
            assert_eq!(hit.defender, unit.id);
            assert_eq!(hit.flesh_region, Some(mark.region));
            assert!(!hit.blunt);
            assert_eq!(hit.severed, 0);
            assert!(hit.amount >= unit.max_health() * 0.04);
            assert!(mark.blood_remaining > 0.);
            assert!(unit.flesh.valid());
        }
    }
}
#[test]
fn bleeding_spends_health_and_effort_with_bounded_receipts_and_can_be_stanched() {
    let (_, _, _, mut battle) = actual_wound();
    let id = battle
        .units
        .iter()
        .position(|u| u.flesh.bleeding())
        .unwrap();
    for u in &mut battle.units {
        u.cooldown = 999.;
        u.theus.as_mut().unwrap().strike_at = u32::MAX;
        u.theus.as_mut().unwrap().windup_target = None;
    }
    let mut control = battle.clone();
    control.units[id].flesh = Flesh::default();
    let hp = battle.units[id].health;
    let loss = battle.units[id].bleeding_loss;
    let budget = battle.units[id]
        .flesh
        .marks
        .iter()
        .map(|w| w.blood_remaining)
        .sum::<f32>();
    battle.step();
    control.step();
    assert!(battle.units[id].health < hp);
    assert!((hp - battle.units[id].health - (battle.units[id].bleeding_loss - loss)).abs() < 0.001);
    assert!(
        battle.units[id].theus.as_ref().unwrap().stamina
            < control.units[id].theus.as_ref().unwrap().stamina
    );
    assert!(battle.units[id].movement_multiplier() < control.units[id].movement_multiplier());
    assert!(
        battle
            .bleed_events
            .iter()
            .any(|e| e.unit == id && e.amount > 0.)
    );
    battle.units[id].flesh.stanch(1000.);
    assert!(!battle.units[id].flesh.bleeding());
    let stopped = battle.units[id].health;
    battle.step();
    assert_eq!(battle.units[id].health, stopped);
    assert!(battle.units[id].bleeding_loss - loss <= budget);
}
#[test]
fn an_actual_heal_stanches_the_patients_recorded_wound() {
    let (_, _, _, mut battle) = actual_wound();
    let patient = battle
        .units
        .iter()
        .position(|u| u.flesh.bleeding())
        .unwrap();
    let team = battle.units[patient].team;
    let medic = team * battle.team_size() + (patient % battle.team_size() + 1) % battle.team_size();
    for u in &mut battle.units {
        u.cooldown = 999.;
        u.theus.as_mut().unwrap().windup_target = None;
        u.theus.as_mut().unwrap().strike_at = u32::MAX;
    }
    let p = battle.units[patient].position;
    battle.units[medic].class = Some(C::Healer);
    battle.units[medic].skills = None;
    battle.units[medic].health = battle.units[medic].max_health();
    battle.units[medic].position = [p[0], p[1] + 0.8];
    battle.units[medic].heal_cooldown = 0.;
    let before = battle.units[patient]
        .flesh
        .marks
        .iter()
        .map(|w| w.blood_remaining)
        .sum::<f32>();
    battle.step();
    let healed = battle
        .heals
        .iter()
        .filter(|e| e.healer == medic && e.target == patient)
        .map(|e| e.amount)
        .sum::<f32>();
    assert!(healed > 0.);
    let after = battle.units[patient]
        .flesh
        .marks
        .iter()
        .map(|w| w.blood_remaining)
        .sum::<f32>();
    assert!(after <= (before - healed).max(0.) + 0.001);
}
#[test]
fn scars_keep_identity_and_hit_provenance_through_settlement_save_and_recovery() {
    let (mut campaign, id, _, mut battle) = actual_wound();
    let injured = battle.units.iter().position(|u| !u.flesh.blank()).unwrap();
    let identity = battle.units[injured].fighter_id.clone();
    let first = battle.units[injured].flesh.marks[0].clone();
    while !battle.finished {
        battle.step();
    }
    campaign.settle(id, &battle).unwrap();
    let mut restored: Campaign =
        serde_json::from_str(&serde_json::to_string(&campaign).unwrap()).unwrap();
    restored.validate().unwrap();
    let fighter =
        &mut restored.teams[battle.units[injured].team].fighters[injured % battle.team_size()];
    assert_eq!(Some(fighter.fighter_id.clone()), identity);
    assert!(
        fighter
            .flesh
            .marks
            .iter()
            .any(|w| w.source_battle == Some(id)
                && w.hit_sequence == first.hit_sequence
                && w.region == first.region)
    );
    assert!(!fighter.flesh.bleeding());
    fighter.flesh.settle(id + 1);
    fighter.flesh.settle(id + 2);
    assert!(fighter.flesh.marks.iter().all(|w| w.recovery_bouts == 0));
    assert!(!fighter.flesh.blank());
    fighter.flesh.marks[0].blood_remaining = f32::NAN;
    assert!(restored.validate().is_err());
    let mut old = serde_json::to_value(Campaign::new()).unwrap();
    for team in old["teams"].as_array_mut().unwrap() {
        for f in team["fighters"].as_array_mut().unwrap() {
            f.as_object_mut().unwrap().remove("flesh");
        }
    }
    let legacy: Campaign = serde_json::from_value(old).unwrap();
    legacy.validate().unwrap();
    assert!(
        legacy
            .teams
            .iter()
            .flat_map(|t| &t.fighters)
            .all(|f| f.flesh.blank())
    );
}
