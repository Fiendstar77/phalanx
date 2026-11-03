use phalanx::{
    learned::Controller,
    progression::{BATTLE_PRACTICE, BATTLE_SALVAGE, Campaign, gold_reward},
    psyche::{Exposure, Moment, Psyche},
    terrain::Arena,
};

#[test]
fn result_labels_do_not_change_practice_resources_temperament_or_memories() {
    let mut original = Campaign::new_prologue();
    let (id, mut battle) = original
        .begin_battle(44, Arena::OpenField, Controller::Utility)
        .unwrap();
    while !battle.finished {
        battle.step();
    }
    let mut reference = None;
    for result in ["Red wins", "Blue wins", "Draw"] {
        // Staged settlement counterfactual: keep the complete physical bout and
        // all exposures identical; vary only its declared result label.
        let mut fixture = battle.clone();
        fixture.result = result.into();
        let mut campaign = original.clone();
        campaign.settle(id, &fixture).unwrap();
        let measured: Vec<_> = campaign
            .teams
            .iter()
            .enumerate()
            .map(|(team, squad)| {
                let f = &squad.fighters[0];
                let before = &original.teams[team].fighters[0];
                assert_eq!(f.experience - before.experience, BATTLE_PRACTICE);
                assert_eq!(
                    f.mastery.points(f.class) - before.mastery.points(f.class),
                    BATTLE_PRACTICE
                );
                assert_eq!(f.mind, fixture.units[team].mind);
                assert_eq!(squad.gold - original.teams[team].gold, gold_reward(1));
                assert_eq!(squad.salvage - original.teams[team].salvage, BATTLE_SALVAGE);
                (
                    f.experience,
                    f.mastery,
                    f.mind,
                    f.psyche,
                    f.theus,
                    squad.gold,
                    squad.salvage,
                )
            })
            .collect();
        if let Some(expected) = &reference {
            assert_eq!(&measured, expected);
        } else {
            reference = Some(measured);
        }
    }
}

#[test]
fn repeated_real_bouts_give_equal_credit_and_keep_actual_skill_practice_and_memories() {
    let mut c = Campaign::new_prologue();
    let starting_minds = [c.teams[0].fighters[0].mind, c.teams[1].fighters[0].mind];
    for (i, seed) in (42..48).enumerate() {
        let (id, mut b) = c
            .begin_battle(seed, Arena::OpenField, Controller::Utility)
            .unwrap();
        while !b.finished {
            b.step();
        }
        c.settle(id, &b).unwrap();
        for (team, initial) in starting_minds.iter().enumerate() {
            let f = &c.teams[team].fighters[0];
            assert_eq!(f.experience, (i as u32 + 1) * BATTLE_PRACTICE);
            assert_eq!(f.mind, *initial);
            assert_eq!(f.psyche.confidence, 0.);
            assert!(f.theus.unwrap().practice.iter().any(|p| *p > 0));
            assert!(f.psyche.isolation.intensity > 0.);
            assert_eq!(c.teams[team].gold, 100 + (i as u32 + 1) * gold_reward(1));
        }
    }
}

#[test]
fn historical_confidence_is_retained_without_a_decision_bonus() {
    let mut old = Psyche {
        confidence: 1.,
        ..Psyche::default()
    };
    let mut plain_scores = [0.5; 9];
    let mut old_scores = plain_scores;
    phalanx::psyche::bias_scores(Psyche::default(), Moment::default(), &mut plain_scores);
    phalanx::psyche::bias_scores(old, Moment::default(), &mut old_scores);
    assert_eq!(plain_scores, old_scores);
    old.absorb(
        phalanx::Mind::disciplined(),
        Exposure {
            isolation_ticks: 200,
            ..Exposure::default()
        },
    );
    assert_eq!(old.confidence, 1.);
    assert!(old.isolation.intensity > 0.);
}
