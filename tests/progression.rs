use phalanx::{
    Battle, FighterClass,
    learned::Controller,
    progression::{Campaign, PALETTE},
    terrain::Arena,
};
#[test]
fn paladin_aura_changes_real_damage_and_stops_on_death() {
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[0].health = 110.0;
    b.units[0].position = [0.0, 0.0];
    b.units[2].health = 100.0;
    b.units[2].position = [0.0, 2.0];
    b.units[3].health = 110.0;
    b.units[3].position = [1.0, 0.0];
    b.step();
    assert!(b.units[0].blessed);
    assert!(!b.units[2].blessed);
    assert!(!b.units[3].blessed);
    assert!((b.units[0].health - 100.1).abs() < 0.0001);
    assert!((b.units[3].health - 98.45).abs() < 0.0001);
    b.units[2].health = 0.0;
    b.step();
    assert!(!b.units[0].blessed);
}
#[test]
fn aura_obeys_radius_and_walls() {
    let mut b = Battle::with_founders(42, false, Arena::Choke);
    b.units[0].position = [-2.0, 5.0];
    b.units[2].position = [2.0, 5.0];
    b.step();
    assert!(!b.units[0].blessed);
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    b.units[0].position = [-5.0, 0.0];
    b.units[2].position = [0.0, 0.0];
    b.step();
    assert!(!b.units[0].blessed);
}
#[test]
fn persistent_squads_gain_experience_once_and_recover_as_the_same_veterans() {
    let mut c = Campaign::new();
    let ids: Vec<_> = c
        .teams
        .iter()
        .flat_map(|t| t.fighters.iter().map(|f| f.fighter_id.clone()))
        .collect();
    for seed in 42..46 {
        let (id, mut b) = c
            .begin_battle(seed, Arena::Ridge, Controller::Utility)
            .unwrap();
        assert!(c.settle(id, &b).is_err());
        while !b.finished {
            b.step();
        }
        c.settle(id, &b).unwrap();
        let settled = serde_json::to_string(&c).unwrap();
        assert!(c.settle(id, &b).is_err());
        assert_eq!(settled, serde_json::to_string(&c).unwrap());
    }
    assert!(c.teams[0].fighters[0].experience >= 100);
    let (_, b) = c
        .begin_battle(46, Arena::Ridge, Controller::Utility)
        .unwrap();
    assert_eq!(
        b.units
            .iter()
            .map(|u| u.fighter_id.clone().unwrap())
            .collect::<Vec<_>>(),
        ids
    );
    assert!(
        b.units
            .iter()
            .all(|u| u.health == u.max_health() && u.level() >= 2 && u.battle_count == 4)
    );
    assert_eq!(b.units[2].class, Some(FighterClass::Paladin));
    assert!(b.units[0].max_health() > 110.0);
    let mut restored: Campaign = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    let mut original = c.clone();
    let (_, a) = original
        .begin_battle(77, Arena::Choke, Controller::Learned)
        .unwrap();
    let (_, b) = restored
        .begin_battle(77, Arena::Choke, Controller::Learned)
        .unwrap();
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
}
#[test]
fn saves_preserve_colors_and_ids_and_reject_invalid_data_without_mutating_rewards() {
    let mut c = Campaign::new();
    c.teams[0].color = PALETTE[2];
    let root = std::env::temp_dir().join(format!(
        "phalanx-save-test-{}-{}",
        std::process::id(),
        c.teams[0].fighters[0].fighter_id
    ));
    let path = root.join("save.json");
    c.save(&path).unwrap();
    let loaded = Campaign::load(&path).unwrap();
    assert_eq!(
        serde_json::to_string(&c).unwrap(),
        serde_json::to_string(&loaded).unwrap()
    );
    c.teams[0].color = PALETTE[5];
    c.save(&path).unwrap();
    assert!(path.with_extension("json.bak").exists());
    let mut bad = c.clone();
    bad.teams[0].color[0] = 2.0;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.teams[0].fighters[1].fighter_id = bad.teams[0].fighters[0].fighter_id.clone();
    assert!(bad.validate().is_err());
    let (id, mut b) = c
        .begin_battle(42, Arena::Ridge, Controller::Utility)
        .unwrap();
    while !b.finished {
        b.step();
    }
    b.units[0].mind.courage = f32::NAN;
    let before = serde_json::to_string(&c).unwrap();
    assert!(c.settle(id, &b).is_err());
    assert_eq!(before, serde_json::to_string(&c).unwrap());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn veteran_power_and_battle_conditioned_habits_are_bounded() {
    let mut c = Campaign::new();
    c.teams[0].fighters[0].experience = 1_000_000;
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(b.units[0].level(), 10);
    assert_eq!(b.units[0].max_health(), 384.0);
    let old = c.teams[0].fighters[0].mind.self_preservation;
    while !b.finished {
        b.step();
    }
    b.units[0].health = 0.0;
    c.settle(id, &b).unwrap();
    assert_eq!(
        c.teams[0].fighters[0].mind.self_preservation,
        (old + 0.01).min(1.0)
    );
    c.validate().unwrap();
}

#[test]
fn appearance_is_backward_compatible_persistent_and_cannot_change_combat() {
    use phalanx::progression::Appearance;
    let campaign = Campaign::new();
    let mut old_save = serde_json::to_value(&campaign).unwrap();
    for team in old_save["teams"].as_array_mut().unwrap() {
        team.as_object_mut().unwrap().remove("salvage");
        for fighter in team["fighters"].as_array_mut().unwrap() {
            for field in [
                "appearance",
                "lost_limbs",
                "prosthetics",
                "wounds",
                "psyche",
            ] {
                fighter.as_object_mut().unwrap().remove(field);
            }
        }
    }
    let mut restored: Campaign = serde_json::from_value(old_save).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.teams[0].fighters[0].appearance.hair, 0);
    assert!(restored.teams[0].fighters[0].psyche.is_blank());
    restored.teams[0].fighters[0].appearance = Appearance {
        female: Some(true),
        hair: 6,
        face: 16,
        eyes: 15,
        nose: 16,
        skin: 7,
        hair_color: 4,
        cheeks: 4,
        mouth: 3,
        brows: 2,
    };
    let mut copy: Campaign =
        serde_json::from_str(&serde_json::to_string(&restored).unwrap()).unwrap();
    copy.validate().unwrap();
    assert_eq!(copy.teams[0].fighters[0].appearance.female, Some(true));
    assert_eq!(copy.teams[0].fighters[0].appearance.skin, 7);
    assert_eq!(copy.teams[0].fighters[0].appearance.hair_color, 4);
    assert_eq!(copy.teams[0].fighters[0].appearance.cheeks, 4);
    assert_eq!(copy.teams[0].fighters[0].appearance.mouth, 3);
    assert_eq!(copy.teams[0].fighters[0].appearance.brows, 2);
    let (_, mut original_battle) = campaign
        .clone()
        .begin_battle(42, Arena::Citadel, Controller::Utility)
        .unwrap();
    let (_, mut customized_battle) = copy
        .begin_battle(42, Arena::Citadel, Controller::Utility)
        .unwrap();
    for _ in 0..40 {
        original_battle.step();
        customized_battle.step();
    }
    for (original, customized) in original_battle.units.iter().zip(&customized_battle.units) {
        assert_eq!(original.position, customized.position);
        assert_eq!(original.health, customized.health);
        assert_eq!(original.action, customized.action);
    }
    copy.teams[0].fighters[0].appearance.eyes = 16;
    assert!(copy.validate().is_err());
    let untouched = Appearance::default().resolved(1, 2);
    assert_eq!(untouched.female, None);
    assert!((1..=6).contains(&untouched.hair));
    assert!((1..=16).contains(&untouched.face));
    assert!((1..=15).contains(&untouched.eyes));
    assert!((1..=16).contains(&untouched.nose));
    let bald = Appearance {
        female: Some(false),
        hair: 0,
        face: 0,
        eyes: 0,
        nose: 0,
        skin: 0,
        hair_color: 0,
        ..Appearance::default()
    };
    assert_eq!(bald.resolved(0, 0), bald);
    assert_eq!(
        phalanx::progression::limb_stand_in(0b0101, 0b0001, 1),
        phalanx::progression::LimbStandIn::Iron
    );
    assert_eq!(
        phalanx::progression::limb_stand_in(0b0101, 0b0001, 4),
        phalanx::progression::LimbStandIn::None
    );
    assert_eq!(
        phalanx::progression::limb_stand_in(0b0101, 0b0001, 2),
        phalanx::progression::LimbStandIn::None
    );
}

#[test]
fn severe_wounds_are_nonfatal_and_prosthetics_are_a_real_tradeoff() {
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[0].health = b.units[0].max_health() * 0.5 + 1.0;
    b.units[0].position = [0.0, 0.0];
    b.units[3].health = 110.0;
    b.units[3].position = [1.0, 0.0];
    b.step();
    assert!(b.units[0].health > 0.0);
    assert_eq!(
        b.units[0].lost_limbs, 0,
        "Ordinary damage crossing half HP must not sever a limb"
    );
    assert!(
        b.units[0].damage_dealt > 0.0,
        "The wounded fighter still fights"
    );
    b.units[0].lost_limbs = 1;
    let natural = 1.1;
    assert!((b.units[0].damage_multiplier() - natural * 0.8).abs() < 0.0001);
    b.units[0].health = b.units[0].max_health() * 0.25 + 0.5;
    b.units[3].cooldown = 0.0;
    b.step();
    assert!(b.units[0].health > 0.0);
    assert_eq!(
        b.units[0].lost_limbs, 1,
        "Crossing quarter HP must not sever a second limb"
    );
    b.units[0].lost_limbs = 5;
    assert_eq!(b.units[0].movement_multiplier(), 0.65);
    b.units[0].prosthetics = 5;
    assert!((b.units[0].damage_multiplier() - natural * 3.0).abs() < 0.0001);
    assert_eq!(b.units[0].movement_multiplier(), 0.85);
    let mut lab = Battle::with_arena(42, false, Arena::OpenField);
    for u in &mut lab.units {
        u.health = 0.0;
    }
    lab.units[0].health = 51.0;
    lab.units[0].position = [0.0, 0.0];
    lab.units[lab.arena.team_size()].health = 100.0;
    lab.units[lab.arena.team_size()].position = [1.0, 0.0];
    lab.step();
    assert_eq!(lab.units[0].lost_limbs, 0);
}

#[test]
fn battle_salvage_crafting_and_wounds_survive_the_next_battle() {
    let mut c = Campaign::new();
    c.teams[0].fighters[0].lost_limbs = 1;
    let before = serde_json::to_string(&c).unwrap();
    assert!(c.craft_prosthetic(0, 0).is_err());
    assert_eq!(before, serde_json::to_string(&c).unwrap());
    let fighter_id = c.teams[0].fighters[0].fighter_id.clone();
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(b.units[0].lost_limbs, 1);
    for u in &mut b.units {
        if u.team == 1 {
            u.health = 0.0;
        }
    }
    b.step();
    assert!(b.finished);
    c.settle(id, &b).unwrap();
    assert_eq!(c.teams[0].salvage, 10);
    assert_eq!(c.teams[0].fighters[0].wounds[0].battle_id, id);
    assert_eq!(c.teams[0].fighters[0].wounds[0].limb, 1);
    assert_eq!(c.craft_prosthetic(0, 0).unwrap(), "Left arm");
    assert_eq!(c.teams[0].salvage, 5);
    assert!(c.craft_prosthetic(0, 0).is_err());
    assert_eq!(c.teams[0].salvage, 5);
    let mut restored: Campaign = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    let (_, next) = restored
        .begin_battle(43, Arena::Citadel, Controller::Utility)
        .unwrap();
    assert_eq!(
        next.units[0].fighter_id.as_deref(),
        Some(fighter_id.as_str())
    );
    assert_eq!(next.units[0].lost_limbs, 1);
    assert_eq!(next.units[0].prosthetics, 1);
    assert!((next.units[0].damage_multiplier() - 3.3).abs() < 0.0001);
}

#[test]
fn the_same_events_scar_different_personalities() {
    use phalanx::psyche::{Exposure, Psyche, Reaction};
    let isolation = Exposure {
        isolation_ticks: 200,
        ..Exposure::default()
    };
    let mut reckless = Psyche::default();
    let mut attached = Psyche::default();
    let mut afraid = Psyche::default();
    let cautious = phalanx::Mind {
        aggression: 0.2,
        caution: 0.9,
        courage: 0.25,
        teamwork: 0.3,
        patience: 0.5,
        self_preservation: 0.85,
        discipline: 0.4,
    };
    reckless.absorb(phalanx::Mind::aggressive(), isolation);
    attached.absorb(phalanx::Mind::disciplined(), isolation);
    afraid.absorb(cautious, isolation);
    assert_eq!(reckless.isolation.reaction, Reaction::OverAggression);
    assert_eq!(attached.isolation.reaction, Reaction::Attachment);
    assert_eq!(afraid.isolation.reaction, Reaction::Fear);
    assert!((reckless.isolation.intensity - attached.isolation.intensity).abs() < 0.0001);
    assert!(reckless.isolation.intensity > 0.4);

    let surrounded = Exposure {
        encirclement_ticks: 80,
        ally_losses: 1,
        failed_charges: 1,
        survived_pressure: true,
        ..Exposure::default()
    };
    reckless.absorb(phalanx::Mind::aggressive(), surrounded);
    attached.absorb(phalanx::Mind::disciplined(), surrounded);
    assert_eq!(reckless.encirclement.reaction, Reaction::OverAggression);
    assert_eq!(attached.encirclement.reaction, Reaction::Fear);
    assert_eq!(reckless.ally_loss.reaction, Reaction::OverAggression);
    assert_eq!(attached.ally_loss.reaction, Reaction::Attachment);
    assert_eq!(reckless.failed_charge.reaction, Reaction::OverAggression);
    assert_eq!(attached.failed_charge.reaction, Reaction::Hesitation);
    assert!(reckless.confidence > attached.confidence);
    assert!(attached.confidence > 0.0);

    let mut replaced = Psyche::default();
    replaced.absorb(phalanx::Mind::aggressive(), isolation);
    replaced.absorb(cautious, isolation);
    assert!(replaced.isolation.reaction == Reaction::None);
    replaced.absorb(cautious, isolation);
    assert_eq!(replaced.isolation.reaction, Reaction::Fear);
}

#[test]
fn isolation_fear_and_recklessness_change_the_same_decision() {
    use phalanx::psyche::{Reaction, Response};
    let mut fear = Battle::with_founders(7, false, Arena::OpenField);
    let mut rage = fear.clone();
    let mut plain = fear.clone();
    for battle in [&mut fear, &mut rage, &mut plain] {
        for unit in &mut battle.units {
            if unit.team == 1 {
                unit.health = 0.0;
            }
            unit.mind = phalanx::Mind::aggressive();
        }
        battle.units[0].position = [-6.0, 0.0];
        battle.units[1].position = [6.0, 3.0];
        battle.units[2].position = [6.0, -3.0];
    }
    fear.units[0].psyche.isolation = Response {
        reaction: Reaction::Fear,
        intensity: 1.0,
    };
    rage.units[0].psyche.isolation = Response {
        reaction: Reaction::OverAggression,
        intensity: 1.0,
    };
    fear.step();
    rage.step();
    plain.step();
    assert_eq!(
        plain.units[0].action, 0,
        "an unscarred fighter still advances: {:?}",
        plain.units[0].scores
    );
    assert_eq!(
        fear.units[0].action, 4,
        "isolation fear regroups: {:?} {}",
        fear.units[0].scores, fear.units[0].decision_reason
    );
    assert!(fear.units[0].decision_reason.contains("Isolation fear"));
    assert_eq!(
        rage.units[0].action, 0,
        "the same isolation drives a charge: {:?}",
        rage.units[0].scores
    );
    assert!(rage.units[0].scores[0] > fear.units[0].scores[0]);
    assert!(fear.units[0].scores[4] > rage.units[0].scores[4]);
    assert!(
        rage.units[0]
            .decision_reason
            .contains("Isolation drives a charge")
    );
}

#[test]
fn scars_persist_across_save_and_the_next_battle() {
    use phalanx::psyche::Reaction;
    let mut campaign = Campaign::new();
    let (id, mut battle) = campaign
        .begin_battle(9, Arena::OpenField, Controller::Utility)
        .unwrap();
    battle.units[0].exposure_isolation = 200;
    battle.units[3].exposure_isolation = 200;
    battle.finished = true;
    battle.result = "Draw".into();
    campaign.settle(id, &battle).unwrap();
    assert_eq!(
        campaign.teams[0].fighters[0].psyche.isolation.reaction,
        Reaction::OverAggression
    );
    assert_eq!(
        campaign.teams[1].fighters[0].psyche.isolation.reaction,
        Reaction::Attachment
    );
    let mut loaded: Campaign =
        serde_json::from_str(&serde_json::to_string(&campaign).unwrap()).unwrap();
    loaded.validate().unwrap();
    let (_, next) = loaded
        .begin_battle(10, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(next.units[0].psyche, campaign.teams[0].fighters[0].psyche);
    assert_eq!(next.units[3].psyche, campaign.teams[1].fighters[0].psyche);
    let mut invalid = campaign.clone();
    invalid.teams[0].fighters[0].psyche.isolation.intensity = 2.0;
    assert!(invalid.validate().is_err());
    invalid.teams[0].fighters[0].psyche.isolation.intensity = 0.0;
    assert!(invalid.validate().is_err());
}

#[test]
fn lab_battles_do_not_record_scars() {
    let mut battle = Battle::new(42, false);
    for _ in 0..40 {
        battle.step();
    }
    let json = serde_json::to_string(&battle).unwrap();
    assert!(!json.contains("exposure_isolation"));
    assert!(!json.contains("\"psyche\""));
    assert!(battle.units.iter().all(|unit| unit.psyche.is_blank()));
}
