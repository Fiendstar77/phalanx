use phalanx::{
    learned::Controller,
    opportunity::{self, Phase, Primitive},
    progression::Campaign,
    terrain::Arena,
};
#[path = "support/opportunity.rs"]
mod support;
use support::fixture;

#[test]
fn search_composes_different_legal_sequences_from_geometry_and_perceived_contacts() {
    let (_, _, battle) = fixture(6);
    let scout = &battle.units[0];
    let contact = &scout.contacts[0];
    let wall = opportunity::compose(scout, contact, Arena::Choke, 0).unwrap();
    assert_eq!(
        wall.steps.iter().map(|s| s.primitive).collect::<Vec<_>>(),
        [
            Primitive::Run,
            Primitive::WallVault,
            Primitive::Run,
            Primitive::Strike
        ]
    );
    assert!(wall.expanded <= opportunity::MAX_EXPANSIONS);
    let open = opportunity::compose(scout, contact, Arena::OpenField, 0).unwrap();
    assert_eq!(
        open.steps.iter().map(|s| s.primitive).collect::<Vec<_>>(),
        [Primitive::Run, Primitive::Strike]
    );
    let mut in_range = scout.clone();
    in_range.position = [contact.position[0] - 0.7, contact.position[1]];
    assert!(opportunity::compose(&in_range, contact, Arena::OpenField, 0).is_none());
    let mut moved_hidden_enemy = battle.clone();
    moved_hidden_enemy.units[3].position = [8., 7.];
    let same =
        opportunity::compose(&moved_hidden_enemy.units[0], contact, Arena::Choke, 0).unwrap();
    assert_eq!(
        serde_json::to_value(wall).unwrap(),
        serde_json::to_value(same).unwrap()
    );
}

#[test]
fn training_effort_legs_and_hands_constrain_plans() {
    for condition in 0..4 {
        let (_, _, mut battle) = fixture(6);
        let scout = &mut battle.units[0];
        match condition {
            0 => scout.theus.as_mut().unwrap().sheet.practice[2] = 0,
            1 => scout.theus.as_mut().unwrap().stamina = 0.,
            2 => scout.lost_limbs |= 4,
            _ => scout.lost_limbs |= 1,
        }
        assert!(opportunity::compose(scout, &scout.contacts[0], Arena::Choke, 0).is_none());
    }
}

#[test]
fn a_lost_leg_between_run_and_vault_interrupts_instead_of_phasing() {
    let (_, _, mut battle) = fixture(6);
    for _ in 0..40 {
        battle.step();
        if battle.opportunities.active.iter().any(|a| {
            a.actor == 0
                && a.proposal.steps[a.cursor].primitive == Primitive::WallVault
                && a.vault.is_none()
        }) {
            battle.units[0].lost_limbs |= 4;
            battle.step();
            assert!(!phalanx::ninjutsu::in_transit(&battle.units[0]));
            assert!(
                battle
                    .opportunities
                    .events
                    .iter()
                    .any(|e| e.actor == 0 && e.phase == Phase::Cancelled)
            );
            return;
        }
    }
    panic!("fixture never approached a vault");
}

#[test]
fn a_proposal_alone_never_enters_the_legend_ledger() {
    let (mut campaign, id, mut battle) = fixture(6);
    battle.step();
    assert!(
        battle
            .opportunities
            .events
            .iter()
            .any(|e| e.phase == Phase::Proposed)
    );
    assert!(battle.opportunities.completed.is_empty());
    battle.finished = true;
    battle.result = "Draw".into();
    campaign.settle(id, &battle).unwrap();
    assert_eq!(campaign.teams[0].fighters[0].legend_count, 0);
}

#[test]
fn unrelated_projectiles_and_uncommitted_strikes_cannot_complete_a_plan() {
    let (_, _, mut battle) = fixture(6);
    let mut armed = None;
    for _ in 0..200 {
        if let Some(active) = battle
            .opportunities
            .active
            .iter()
            .find(|a| a.actor == 0 && a.proposal.steps[a.cursor].primitive == Primitive::Strike)
        {
            armed = Some(active.clone());
        }
        battle.step();
        if !battle.opportunities.completed.is_empty() {
            break;
        }
    }
    let witness = battle.opportunities.completed[0].clone();
    battle.opportunities.completed.clear();
    battle.opportunities.active = vec![armed.unwrap()];
    battle.opportunities.active[0].strike_selected_tick = None;
    battle.theus_events = vec![witness.strike];
    opportunity::observe(&mut battle, false);
    assert!(battle.opportunities.completed.is_empty());
    battle.opportunities.active[0].strike_selected_tick = Some(witness.strike_selected_tick);
    battle.theus_events[0].ranged = true;
    opportunity::observe(&mut battle, false);
    assert!(battle.opportunities.completed.is_empty());
    battle.theus_events[0].ranged = false;
    opportunity::observe(&mut battle, false);
    assert_eq!(battle.opportunities.completed.len(), 1);
}

#[test]
fn a_removed_surface_invalidates_the_vault_before_it_starts() {
    let (_, _, mut battle) = fixture(6);
    for _ in 0..40 {
        battle.step();
        if battle.opportunities.active.iter().any(|a| {
            a.actor == 0
                && a.proposal.steps[a.cursor].primitive == Primitive::WallVault
                && a.vault.is_none()
        }) {
            battle.arena = Arena::OpenField;
            battle.step();
            assert!(!phalanx::ninjutsu::in_transit(&battle.units[0]));
            assert!(
                battle
                    .opportunities
                    .events
                    .iter()
                    .any(|e| e.phase == Phase::Cancelled && e.detail.contains("surface"))
            );
            return;
        }
    }
    panic!("fixture did not reach a vault opportunity");
}

#[test]
fn the_ledger_keeps_the_first_qualifying_witness_without_ranking_damage() {
    let (campaign, id, mut battle) = fixture(6);
    while !battle.finished {
        battle.step();
    }
    let first = phalanx::legend::capture(id, &battle, 0, &campaign).unwrap();
    let mut later = battle.opportunities.completed[0].clone();
    later.plan += 1;
    later.completed_tick += 1;
    later.strike.tick += 1;
    later.strike.resolution.damage *= 10.;
    battle.opportunities.completed.push(later);
    let retained = phalanx::legend::capture(id, &battle, 0, &campaign).unwrap();
    assert_eq!(retained.plan, first.plan);
    assert_eq!(retained.strike.damage, first.strike.damage);
    assert_eq!(battle.opportunities.completed.len(), 2);
}

#[test]
fn real_composition_replays_and_its_legend_survives_save_and_snapshot_without_stat_buffs() {
    let (mut campaign, id, mut battle) = fixture(6);
    let mut replay = battle.clone();
    while !battle.finished {
        battle.step();
        replay.step();
    }
    assert_eq!(
        serde_json::to_value(&battle).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
    let completed = battle
        .opportunities
        .completed
        .iter()
        .find(|c| c.actor == 0 && c.vault.is_some());
    assert!(
        completed.is_some(),
        "{}",
        serde_json::to_string(&battle.opportunities).unwrap()
    );
    let completed = completed.unwrap();
    assert!(completed.strike.resolution.damage > 0.);
    assert!(completed.vault.as_ref().unwrap().landed_tick <= completed.strike.tick);
    assert!(battle.comparison_summary.compared > 0);
    campaign.settle(id, &battle).unwrap();
    let fighter = &campaign.teams[0].fighters[0];
    assert_eq!(fighter.legend_count, 1);
    assert_eq!(fighter.legends.len(), 1);
    let entry = &fighter.legends[0];
    assert!(entry.valid(campaign.next_battle_id, &fighter.fighter_id));
    assert_eq!(entry.strike.damage, completed.strike.resolution.damage);
    assert!(fighter.display_name().contains("Wallrunner"));
    let encoded = serde_json::to_vec(&campaign).unwrap();
    let restored: Campaign = serde_json::from_slice(&encoded).unwrap();
    restored.validate().unwrap();
    let snapshot = phalanx::challenges::TeamSnapshot::from_campaign(&restored, 0).unwrap();
    let decoded: phalanx::challenges::TeamSnapshot =
        serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded.team.fighters[0].legend_count, 1);
    assert!(decoded.summary().contains("source-reported legends"));
    let challenge = phalanx::challenges::Challenge::new(decoded.clone(), decoded, 11).unwrap();
    let (namespaced, _) = challenge.start().unwrap();
    namespaced.validate().unwrap();
    assert_ne!(
        namespaced.teams[0].fighters[0].fighter_id,
        namespaced.teams[1].fighters[0].fighter_id
    );
    assert_eq!(
        namespaced.teams[0].fighters[0].legends[0].actor_fighter_id,
        fighter.fighter_id
    );
    let mut recruits = restored.clone();
    recruits.teams[0].gold = 1000;
    recruits.unlock_fourth().unwrap();
    assert!(recruits.teams[0].fighters[3].legends.is_empty());
    assert_eq!(recruits.teams[0].fighters[3].legend_count, 0);
    phalanx::rivals::initialize(&mut recruits);
    assert!(
        recruits
            .rivals
            .iter()
            .flat_map(|team| &team.fighters)
            .all(|f| f.legends.is_empty() && f.legend_count == 0)
    );
    let mut without_titles = restored.clone();
    without_titles.teams[0].fighters[0].legends.clear();
    without_titles.teams[0].fighters[0].legend_count = 0;
    let (_, a) = campaign
        .begin_battle(7, Arena::OpenField, Controller::Utility)
        .unwrap();
    let (_, b) = without_titles
        .begin_battle(7, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(a.units[0].max_health(), b.units[0].max_health());
    assert_eq!(
        a.units[0].damage_multiplier(),
        b.units[0].damage_multiplier()
    );
    assert_eq!(
        a.units[0].movement_multiplier(),
        b.units[0].movement_multiplier()
    );
    let mut invalid = restored;
    invalid.teams[0].fighters[0].legends[0].strike.damage = -1.;
    assert!(invalid.validate().is_err());
}
