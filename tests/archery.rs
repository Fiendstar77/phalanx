use phalanx::{
    Battle, CommandTitle, FighterClass as C, Mind, Rock, Role, archery,
    equipment::Gear,
    learned::Controller,
    perception::{Contact, Source},
    progression::Campaign,
    terrain::Arena,
    weapons::ProjectileWeapon as W,
};
fn archer() -> Battle {
    let mut c = Campaign::new();
    c.teams[0].fighters[0].class = C::Ranger;
    let (_, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    for u in &mut b.units {
        u.health = 0.;
        u.action = 6;
        u.cooldown = 999.;
    }
    b.tick = 1;
    b.units[0].health = 1000.;
    b.units[0].position = [0., 0.];
    b.units[0].action = 1;
    b.units[0].cooldown = 0.;
    b.units[0].mind = Mind::aggressive();
    b.units[3].health = 1000.;
    b.units[3].position = [6., 0.];
    b.units[3].target = [6., 0.];
    b
}
#[test]
fn ranger_starter_bows_and_crafted_bows_remain_independent_of_office_and_class() {
    let b = archer();
    assert_eq!(b.units[0].projectile_weapon(), Some(W::Bow));
    assert_eq!(b.units[0].ammunition, 12);
    let mut c = Campaign::new();
    c.teams[0].gold = 1000;
    c.teams[0].salvage = 1000;
    assert!(c.workshop(0, 1, Gear::Bow).is_err());
    c.teams[0].fighters[1].mastery.train_weapon(W::Atlatl, 100);
    c.teams[0].fighters[1].mastery.train(C::Healer, 100);
    c.teams[0].fighters[1].mastery.train(C::BannerBearer, 100);
    c.teams[0].fighters[1].class = C::Warrior;
    c.workshop(0, 1, Gear::Bow).unwrap();
    c.workshop(0, 1, Gear::WarBanner).unwrap();
    let mut c: Campaign = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    c.validate().unwrap();
    let (_, b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    let u = &b.units[1];
    assert_eq!(u.class, Some(C::Warrior));
    assert_eq!(u.command_title, Some(CommandTitle::Queen));
    assert_eq!(u.projectile_weapon(), Some(W::Bow));
    assert!(u.has_skill(C::Healer));
    assert!(u.banner_active());
    let f = &mut c.teams[0].fighters[0];
    f.class = C::Warrior;
    f.mastery.train(C::Ranger, 100);
    let (_, b) = c
        .begin_battle(43, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(b.units[0].projectile_weapon(), Some(W::Bow));
}
#[test]
fn arrow_release_spends_one_arrow_after_the_draw_and_starts_at_the_bow() {
    let mut b = archer();
    b.step();
    assert!(b.units[0].windup.is_some());
    assert_eq!(b.units[0].ammunition, 12);
    for _ in 0..12 {
        b.step();
        if b.units[0].weapon_shots[W::Bow.index()] > 0 {
            break;
        }
    }
    assert_eq!(b.units[0].ammunition, 11);
    assert_eq!(b.units[0].weapon_shots[W::Bow.index()], 1);
    let arrow = b.rocks.iter().find(|p| p.weapon == W::Bow).unwrap();
    assert_eq!(
        arrow.from,
        archery::arrow_origin(b.units[0].position, arrow.to)
    );
    assert!(phalanx::distance(arrow.from, b.units[0].position) > 0.3);
}
#[test]
fn blocked_archers_find_a_real_walkable_lane_instead_of_firing_through_allies() {
    let mut b = archer();
    b.units[1].health = 1000.;
    b.units[1].position = [3., 0.];
    let enemy = Contact {
        enemy: 3,
        position: [6., 0.],
        health: 100.,
        role: Role::Fighter,
        range: 1.5,
        seen_tick: 1,
        source: Source::OwnSight,
        visible: true,
    };
    let allies = vec![&b.units[1]];
    assert!(!archery::clear_shot(
        b.arena,
        archery::muzzle([0., 0.], [6., 0.]),
        [6., 0.],
        &allies
    ));
    let p = archery::plan(b.arena, &b.units[0], &allies, &enemy, 1).unwrap();
    assert_eq!(p.action, 0);
    assert!(p.goal[1].abs() > 0.3);
    assert!(b.arena.walk_segment([0., 0.], p.goal));
    assert!(archery::clear_shot(
        b.arena,
        archery::muzzle(p.goal, enemy.position),
        enemy.position,
        &allies
    ));
    b.step();
    assert!(b.units[0].windup.is_none());
    assert_eq!(b.units[0].ammunition, 12);
    for _ in 0..20 {
        b.step();
    }
    assert!(b.units[0].position[1].abs() > 0.2 || b.units[0].rocks_thrown > 0);
}
#[test]
fn allies_crossing_a_released_arrow_stop_it_without_friendly_damage() {
    let mut b = archer();
    b.units[0].cooldown = 999.;
    b.units[0].action = 6;
    b.units[1].health = 1000.;
    b.units[1].position = [3., 0.];
    b.rocks.push(Rock {
        weapon: W::Bow,
        shooter: 0,
        team: 0,
        from: [0.4, 0.],
        to: [6., 0.],
        age: 0,
        flight_ticks: 1,
        damage: 13.,
    });
    b.step();
    assert_eq!(b.units[1].health, 1000.);
    assert_eq!(b.units[3].health, 1000.);
    assert_eq!(b.units[0].arrow_blocks, 1);
    assert!(b.rocks.is_empty());
    assert!(b.damage_events.is_empty());
}
#[test]
fn a_rush_cancels_drawing_and_forces_a_sidearm_then_bow_range_can_reopen() {
    let mut b = archer();
    b.step();
    assert!(b.units[0].windup.is_some());
    b.units[3].position = [1.2, 0.];
    b.step();
    assert!(b.units[0].windup.is_none());
    assert!(b.units[0].bow_melee);
    assert!(!b.units[0].is_ranged());
    assert_eq!(b.units[0].ammunition, 12);
    assert!(b.units[3].health < 1000.);
    b.units[3].position = [6., 0.];
    b.step();
    assert!(!b.units[0].bow_melee);
    assert!(b.units[0].is_ranged());
}
#[test]
fn three_weapon_save_arrays_upgrade_without_losing_old_practice() {
    let old = serde_json::json!({"practice":[0,0,0,0,0,0,0,0,0,0],"weapons":[11,22,33]});
    let restored: phalanx::mastery::Mastery = serde_json::from_value(old).unwrap();
    assert_eq!(restored.weapons, [11, 22, 33, 0]);
    assert!(restored.valid());
    let mut c = Campaign::new();
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    while !b.finished {
        b.step();
    }
    c.settle(id, &b).unwrap();
    let mut json = serde_json::to_value(&c).unwrap();
    for teams in ["teams", "rivals"] {
        for t in json[teams].as_array_mut().unwrap() {
            for f in t["fighters"].as_array_mut().unwrap() {
                f["mastery"]["weapons"].as_array_mut().unwrap().pop();
            }
        }
    }
    for t in json["last_report"]["fighters"].as_array_mut().unwrap() {
        for f in t.as_array_mut().unwrap() {
            f["weapon_training"].as_array_mut().unwrap().pop();
        }
    }
    let restored: Campaign = serde_json::from_value(json).unwrap();
    restored.validate().unwrap();
}
#[test]
fn archer_battles_make_progress_without_clipping_or_endless_retreat() {
    for arena in [Arena::OpenField, Arena::Choke] {
        for seed in [2, 42, 123] {
            let mut c = Campaign::new();
            for f in &mut c.teams[0].fighters {
                f.class = C::Ranger;
            }
            let (_, mut b) = c.begin_battle(seed, arena, Controller::Utility).unwrap();
            for _ in 0..600 {
                b.step();
                for (i, a) in b.units.iter().enumerate().filter(|(_, u)| u.health > 0.) {
                    assert!(!b.arena.blocked(a.position));
                    for v in b.units.iter().skip(i + 1).filter(|u| u.health > 0.) {
                        assert!(
                            phalanx::distance(a.position, v.position)
                                >= phalanx::collision::BODY_DIAMETER - 0.0003
                        );
                    }
                }
                if b.finished {
                    break;
                }
            }
            assert!(b.finished, "unfinished {arena:?}, seed {seed}");
            assert!(b.units.iter().map(|u| u.damage_dealt).sum::<f32>() > 100.);
        }
    }
}

#[test]
fn an_ally_entering_the_firing_lane_cancels_the_draw_without_spending_an_arrow() {
    let mut b = archer();
    b.step();
    assert!(b.units[0].windup.is_some());
    b.units[1].health = 1000.;
    b.units[1].position = [3., 0.];
    b.step();
    assert!(b.units[0].windup.is_none());
    assert_eq!(b.units[0].ammunition, 12);
    assert_eq!(b.units[0].rocks_thrown, 0);
    assert!(b.units[0].decision_reason.contains("lane"));
}
#[test]
fn the_quiver_is_finite_and_empty_archers_close_with_a_sidearm() {
    let mut b = archer();
    b.units[3].class = Some(C::RockThrower);
    b.units[3].role = Role::RockThrower;
    b.units[3].reset_weapon();
    b.units[3].action = 1;
    for _ in 0..260 {
        b.step();
    }
    assert_eq!(b.units[0].weapon_shots[W::Bow.index()], 12);
    assert_eq!(b.units[0].ammunition, 0);
    assert!(!b.units[0].is_ranged());
    assert!(phalanx::distance(b.units[0].position, b.units[3].position) < 1.6);
    assert!(b.units[0].damage_dealt > 12. * W::Bow.damage() * 1.1);
}

#[test]
fn a_missing_arm_prevents_drawing_until_a_prosthetic_restores_it() {
    let mut b = archer();
    b.step();
    assert!(b.units[0].windup.is_some());
    b.units[0].lost_limbs = 1;
    b.step();
    assert!(!b.units[0].is_ranged());
    assert!(b.units[0].windup.is_none());
    assert_eq!(b.units[0].ammunition, 12);
    b.units[0].prosthetics = 1;
    assert!(b.units[0].bow_available());
    assert!(b.units[0].is_ranged());
}

#[test]
fn null_rival_recognizes_bow_loadouts_even_on_warrior_classes() {
    let mut c = Campaign::new();
    c.teams[0].gold = 1000;
    c.teams[0].salvage = 1000;
    for i in 0..3 {
        c.teams[0].fighters[i].class = C::Warrior;
        c.teams[0].fighters[i].mastery.train_weapon(W::Atlatl, 100);
        c.workshop(0, i, Gear::Bow).unwrap();
    }
    c.select_rival(4).unwrap();
    let player = c.teams[0].clone();
    phalanx::rivals::evolve(&mut c.teams[1], &player, 0.);
    assert_eq!(c.teams[1].fighters[0].class, C::Scout);
    assert_eq!(c.teams[1].fighters[1].class, C::Guardian);
    assert_eq!(c.teams[1].fighters[2].class, C::Ranger);
}
