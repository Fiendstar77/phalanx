use phalanx::{CommandTitle, learned::Controller, progression::Campaign, terrain::Arena};

#[test]
fn king_creation_validates_atomically_and_cannot_replace_an_established_life() {
    let mut c = Campaign::new_prologue();
    c.founders_prepared = false;
    c.validate().unwrap();
    let before = serde_json::to_value(&c).unwrap();
    let mut king = phalanx::genesis::reveal("ASH", 0, 0, phalanx::FighterClass::Scout).unwrap();
    let origin = phalanx::genesis::Origin {
        version: 1,
        word: "ASH".into(),
        rolls: [0; 3],
    };
    king.name = String::new();
    assert!(
        c.found_king("The Uncrowned", king.clone(), origin.clone())
            .is_err()
    );
    assert_eq!(serde_json::to_value(&c).unwrap(), before);
    king.name = "Aurel".into();
    c.found_king("The Uncrowned", king.clone(), origin.clone())
        .unwrap();
    let accepted = serde_json::to_value(&c).unwrap();
    assert_eq!(c.teams[0].fighters[0].appearance.age_years, 16);
    assert_eq!(
        c.teams[0].fighters[0].fighter_id,
        before["teams"][0]["fighters"][0]["fighter_id"]
    );
    assert_eq!(
        accepted["prologue"]["waiting"],
        before["prologue"]["waiting"]
    );
    assert!(c.found_king("Another", king, origin).is_err());
    assert_eq!(serde_json::to_value(&c).unwrap(), accepted);
}

fn encounter(c: &mut Campaign, seed: u64) -> phalanx::Battle {
    let (id, mut battle) = c
        .begin_battle(seed, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert!(battle.units.iter().all(|u| !u.carries_standard));
    while !battle.finished {
        battle.step();
    }
    c.settle(id, &battle).unwrap();
    assert!(c.settle(id, &battle).is_err());
    battle
}

#[test]
fn opening_requires_actual_victories_and_preserves_identity_and_learning_through_the_campaign() {
    let mut c = Campaign::new_prologue();
    c.validate().unwrap();
    let king = c.teams[0].fighters[0].fighter_id.clone();
    assert_eq!(c.teams[0].fighters.len(), 1);
    assert_eq!(c.teams[0].fighters[0].appearance.age_years, 16);
    assert!(c.unlock_fourth().is_err());
    assert!(c.assign_title(0, 0, CommandTitle::King).is_err());
    assert!(c.select_rival(0).is_err());
    c.advance_prologue().unwrap();
    assert_eq!(c.prologue.as_ref().unwrap().chapter, 0);
    let mut seed = 42;
    for chapter in 0..3 {
        assert_eq!(c.prologue.as_ref().unwrap().chapter, chapter);
        assert_eq!(c.teams[0].fighters.len(), chapter as usize + 1);
        let mut won = false;
        for _ in 0..24 {
            let battle = encounter(&mut c, seed);
            seed += 1;
            if battle.winner() == Some(0) {
                won = true;
                break;
            }
            assert!(!c.prologue.as_ref().unwrap().cleared());
            let before = c.teams[0].fighters[0].battles;
            c.advance_prologue().unwrap();
            assert_eq!(c.teams[0].fighters[0].battles, before);
        }
        assert!(
            won,
            "opening chapter {chapter} did not produce a win in 24 seeds"
        );
        let learned = serde_json::to_value(&c.teams[0].fighters[0].history).unwrap();
        let battles = c.teams[0].fighters[0].battles;
        // A save/load boundary immediately after victory must preserve the pending transition.
        let saved = serde_json::to_vec(&c).unwrap();
        c = serde_json::from_slice(&saved).unwrap();
        c.validate().unwrap();
        c.advance_prologue().unwrap();
        assert_eq!(c.teams[0].fighters[0].fighter_id, king);
        assert_eq!(c.teams[0].fighters[0].battles, battles);
        assert_eq!(
            serde_json::to_value(&c.teams[0].fighters[0].history).unwrap(),
            learned
        );
        c.validate().unwrap();
    }
    assert!(!c.prologue_active());
    assert_eq!(c.teams[0].fighters[0].appearance.age_years, 18);
    assert_eq!(c.teams[0].fighters[0].title, CommandTitle::King);
    assert_eq!(c.prologue.as_ref().unwrap().joins.len(), 2);
    assert_eq!(c.tutorial.completed(), 0);
    assert!(c.advance_prologue().is_err());
    phalanx::tutorial::prepare_next(&mut c).unwrap();
    c.teams[0].gold = 250;
    c.unlock_fourth().unwrap();
    assert_eq!(c.teams[0].fighters.len(), 4);
    assert_eq!(c.teams[0].fighters[0].fighter_id, king);
    c.validate().unwrap();
}

#[test]
fn malformed_openings_are_rejected_and_legacy_saves_stay_three_founder_campaigns() {
    let c = Campaign::new_prologue();
    let mut bad = c.clone();
    bad.prologue.as_mut().unwrap().chapter = 2;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.prologue.as_mut().unwrap().waiting[0][0].fighter_id =
        c.teams[0].fighters[0].fighter_id.clone();
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.teams[0].fighters[0].appearance.age_years = 0;
    assert!(bad.validate().is_err());
    assert!(
        c.teams[0].validate(c.next_battle_id).is_err(),
        "solo teams cannot enter league snapshots"
    );
    let mut legacy = serde_json::to_value(Campaign::new()).unwrap();
    legacy.as_object_mut().unwrap().remove("prologue");
    for team in legacy["teams"].as_array_mut().unwrap() {
        for f in team["fighters"].as_array_mut().unwrap() {
            f["appearance"].as_object_mut().unwrap().remove("age_years");
        }
    }
    let legacy: Campaign = serde_json::from_value(legacy).unwrap();
    legacy.validate().unwrap();
    assert!(!legacy.prologue_active());
    assert_eq!(legacy.teams[0].fighters.len(), 3);
    assert_eq!(legacy.teams[0].fighters[0].appearance.age_years, 24);
}
