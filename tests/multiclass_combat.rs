use phalanx::{
    FighterClass as C,
    learned::Controller,
    progression::Campaign,
    skills::{Skill, SkillLoadout},
    terrain::Arena,
};

#[test]
fn retained_healing_stonecasting_and_blessing_execute_together_after_retraining_and_reload() {
    // Authored veteran profile; the mastery-retention suite separately earns the
    // threshold through four completed bouts. This checks execution of the mix.
    let mut campaign = Campaign::new();
    for team in &mut campaign.teams {
        for f in &mut team.fighters {
            f.class = C::Warrior;
            f.skills = Some(SkillLoadout::default());
            f.mind = phalanx::styles::BattleStyle::Vanguard.mind();
        }
    }
    for class in [C::Healer, C::RockThrower, C::Paladin] {
        campaign.teams[0].fighters[0].mastery.train(class, 100);
    }
    for (slot, skill) in [Skill::Heal, Skill::Stonecast, Skill::Bless]
        .into_iter()
        .enumerate()
    {
        campaign.set_skill(0, 0, false, slot, Some(skill)).unwrap();
    }
    campaign.set_class(0, 0, C::Scout).unwrap();
    campaign.set_class(0, 0, C::Warrior).unwrap();
    let mut restored: Campaign =
        serde_json::from_str(&serde_json::to_string(&campaign).unwrap()).unwrap();
    restored.validate().unwrap();
    let (_, mut battle) = restored
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    for (u, p) in battle.units.iter_mut().zip([
        [-3., 0.],
        [-3., 1.5],
        [-4., -2.],
        [3., 0.],
        [4., 2.],
        [4., -2.],
    ]) {
        u.position = p;
    }
    battle.units[1].health -= 80.;
    let mut healed = false;
    let mut blessed = false;
    for _ in 0..400 {
        battle.step();
        healed |= battle
            .heals
            .iter()
            .any(|h| h.healer == 0 && h.target == 1 && h.amount > 0.);
        blessed |= battle.units[1].blessed;
        if healed && blessed && battle.units[0].rocks_thrown > 0 {
            break;
        }
        if battle.finished {
            break;
        }
    }
    assert_eq!(battle.units[0].class, Some(C::Warrior));
    assert!(healed, "retained Mend wounds never restored real health");
    assert!(blessed, "retained blessing never reached a living ally");
    assert!(
        battle.units[0].rocks_thrown > 0,
        "retained Stonecast never launched a projectile"
    );
    assert!(battle.units[0].skills.unwrap().has(Skill::Heal));
    restored.set_skill(0, 0, false, 0, None).unwrap();
    let (_, mut unequipped) = restored
        .begin_battle(43, Arena::OpenField, Controller::Utility)
        .unwrap();
    unequipped.units[1].position = unequipped.units[0].position;
    unequipped.units[1].health -= 80.;
    for _ in 0..30 {
        unequipped.step();
    }
    assert!(!unequipped.heals.iter().any(|h| h.healer == 0));
    assert_eq!(restored.teams[0].fighters[0].mastery.points(C::Healer), 100);
}
