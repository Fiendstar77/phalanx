//! Shared staged fixture for the interactive demo, example and integration tests.
//! Grants are explicit; normal planning, movement and Theus strikes still execute.
use phalanx::{
    Battle, FighterClass,
    learned::Controller,
    perception::{Contact, Source},
    progression::Campaign,
    skills::{Skill, SkillLoadout},
    terrain::Arena,
    theus::Sheet,
};

pub fn fixture(seed: u64) -> (Campaign, u64, Battle) {
    let mut campaign = Campaign::new();
    for team in &mut campaign.teams {
        for fighter in &mut team.fighters {
            fighter.skills = Some(SkillLoadout::default());
            fighter.theus = Some(Sheet {
                attributes: [6; 4],
                ..Default::default()
            });
        }
    }
    let scout = &mut campaign.teams[0].fighters[0];
    scout.name = "Mira".into();
    scout.class = FighterClass::Scout;
    scout.skills.as_mut().unwrap().active[0] = Some(Skill::WallDance);
    scout.theus = Some(Sheet {
        attributes: [10, 14, 10, 10],
        practice: [200, 0, 100, 0, 0, 0],
        ..Default::default()
    });
    scout.lost_limbs = 2;
    campaign.teams[1].fighters[0].name = "Gatekeeper".into();
    campaign.teams[1].fighters[0].class = FighterClass::Guardian;
    let (id, mut battle) = campaign
        .begin_battle(seed, Arena::Choke, Controller::Utility)
        .unwrap();
    for unit in &mut battle.units {
        unit.health = 0.;
        unit.cooldown = 999.;
    }
    battle.units[0].health = battle.units[0].max_health() * 0.28;
    battle.units[0].position = [-3.2, -5.];
    battle.units[0].target = [4., -5.];
    battle.units[0].cooldown = 0.;
    battle.units[3].health = battle.units[3].max_health();
    battle.units[3].position = [4., -5.];
    // A previously seen contact is remembered across the opaque wall. This
    // fixture grants knowledge explicitly, not direct sight or a successful hit.
    let target_role = battle.units[3].role;
    battle.units[0].contacts.push(Contact {
        enemy: 3,
        position: [4., -5.],
        health: 100.,
        role: target_role,
        range: 1.5,
        seen_tick: 0,
        source: Source::OwnSight,
        visible: false,
    });
    (campaign, id, battle)
}
