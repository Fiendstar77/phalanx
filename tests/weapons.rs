use phalanx::{
    Battle, FighterClass as C, Mind, Rock, Role, equipment::Gear, learned::Controller,
    progression::Campaign, terrain::Arena, weapons::ProjectileWeapon as W,
};

fn armed_campaign(weapon: W) -> Campaign {
    let mut c = Campaign::new();
    c.teams[0].gold = 1000;
    c.teams[0].salvage = 1000;
    let f = &mut c.teams[0].fighters[0];
    f.class = C::RockThrower;
    f.mastery.train_weapon(W::Rock, 100);
    if weapon == W::Atlatl {
        f.mastery.train_weapon(W::Spear, 100);
    }
    if weapon != W::Rock {
        c.workshop(
            0,
            0,
            if weapon == W::Spear {
                Gear::ThrowingSpear
            } else {
                Gear::Atlatl
            },
        )
        .unwrap();
    }
    c
}
fn duel(weapon: W) -> Battle {
    let mut c = armed_campaign(weapon);
    let (_, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    for u in &mut b.units {
        // These fixtures measure projectile geometry and release timing.
        u.theus = None;
        u.health = 0.;
    }
    b.units[0].health = 1000.;
    b.units[0].position = [0., 0.];
    b.units[0].mind = Mind::aggressive();
    b.units[0].action = 1;
    b.units[3].health = 1000.;
    b.units[3].position = [5.5, 0.];
    b.units[3].class = Some(C::RockThrower);
    b.units[3].role = Role::RockThrower;
    b.units[3].cooldown = 999.;
    b.units[3].action = 1;
    b
}
#[test]
fn recipes_require_earned_predecessor_practice_and_rejections_are_atomic() {
    let mut c = Campaign::new();
    c.teams[0].salvage = 100;
    let before = serde_json::to_value(&c).unwrap();
    assert!(
        c.workshop(0, 1, Gear::ThrowingSpear)
            .unwrap_err()
            .contains("100 Rock")
    );
    assert!(c.workshop(0, 1, Gear::Atlatl).is_err());
    assert_eq!(serde_json::to_value(&c).unwrap(), before);
    c.teams[0].fighters[1].mastery.train_weapon(W::Rock, 100);
    c.workshop(0, 1, Gear::ThrowingSpear).unwrap();
    c.workshop(0, 1, Gear::ThrowingSpear).unwrap();
    assert!(!c.teams[0].fighters[1].equipment.has(Gear::ThrowingSpear));
    c.workshop(0, 1, Gear::ThrowingSpear).unwrap();
    assert!(c.teams[0].fighters[1].equipment.has(Gear::ThrowingSpear));
    assert!(c.workshop(0, 1, Gear::Atlatl).is_err());
}
#[test]
fn distinct_windups_plant_feet_and_spend_ammunition_only_on_release() {
    let mut rock = duel(W::Rock);
    rock.step();
    assert_eq!(rock.units[0].rocks_thrown, 1);
    for weapon in [W::Spear, W::Atlatl] {
        let mut b = duel(weapon);
        b.step();
        assert!(b.units[0].windup.is_some());
        assert_eq!(b.units[0].ammunition, weapon.ammunition());
        assert!(b.rocks.is_empty());
        let position = b.units[0].position;
        let steps = (weapon.windup() / phalanx::DT).round() as usize;
        for _ in 0..steps - 1 {
            b.step();
            assert_eq!(b.units[0].position, position);
            assert!(b.rocks.is_empty());
        }
        b.step();
        assert_eq!(b.units[0].ammunition, weapon.ammunition() - 1);
        assert_eq!(b.units[0].weapon_shots[weapon.index()], 1);
        assert_eq!(b.rocks[0].weapon, weapon);
        assert_eq!(
            b.rocks[0].flight_ticks,
            (phalanx::distance(b.rocks[0].from, b.rocks[0].to) / weapon.speed() / phalanx::DT)
                .ceil() as u32
        );
    }
}
#[test]
fn losing_sight_or_taking_a_heavy_hit_cancels_a_windup_without_spending_ammo() {
    let mut b = duel(W::Atlatl);
    b.step();
    b.units[3].position = [10., 7.];
    b.step();
    assert!(b.units[0].windup.is_none());
    assert_eq!(b.units[0].ammunition, 4);
    assert_eq!(b.units[0].rocks_thrown, 0);
    let mut b = duel(W::Atlatl);
    b.step();
    b.rocks.push(Rock {
        weapon: W::Rock,
        shooter: 3,
        team: 1,
        from: [1., 0.],
        to: [0., 0.],
        age: 0,
        flight_ticks: 1,
        damage: 30.,
    });
    b.step();
    assert!(b.units[0].windup.is_none());
    assert_eq!(b.units[0].ammunition, 4);
}
#[test]
fn spears_intercept_the_first_body_without_endpoint_splash_or_tunneling() {
    let mut b = duel(W::Spear);
    b.units[0].cooldown = 999.;
    b.units[0].position = [-4., 0.];
    b.units[3].position = [-1., 0.];
    b.units[3].action = 6;
    b.units[4].health = 1000.;
    b.units[4].position = [1., 0.];
    b.units[4].action = 6;
    b.units[4].cooldown = 999.;
    b.rocks.push(Rock {
        weapon: W::Atlatl,
        shooter: 0,
        team: 0,
        from: [-4., 0.],
        to: [4., 0.],
        age: 0,
        flight_ticks: 1,
        damage: 26.,
    });
    b.step();
    assert!(b.units[3].health < 1000.);
    assert_eq!(b.units[4].health, 1000.);
    assert!(b.rocks.is_empty());
    let e = b.damage_events.iter().find(|e| e.defender == 3).unwrap();
    assert!(!e.blunt);
    let t = (e.position[0] + 4.) / 8.;
    assert!(
        (phalanx::distance(e.position, b.motion.position(3, t).unwrap())
            - phalanx::collision::BODY_RADIUS)
            .abs()
            < 0.001
    );
    assert!((e.velocity[0] - (23. - (b.units[3].position[0] + 1.) / phalanx::DT)).abs() < 0.001);
    assert!(e.velocity[0] > 0.);
}
#[test]
fn empty_bundles_switch_to_melee_and_refill_at_the_next_battle() {
    let mut b = duel(W::Spear);
    for _ in 0..260 {
        b.step();
    }
    assert_eq!(b.units[0].ammunition, 0);
    assert_eq!(b.units[0].rocks_thrown, 6);
    assert!(!b.units[0].is_ranged());
    assert_eq!(b.units[0].attack_range(), 1.5);
    assert!(phalanx::distance(b.units[0].position, b.units[3].position) < 1.6);
    assert!(b.units[0].damage_dealt > 6. * W::Spear.damage() * 1.1);
    let mut c = armed_campaign(W::Spear);
    let (_, next) = c
        .begin_battle(43, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(next.units[0].ammunition, 6);
}
#[test]
fn technology_persists_and_a_queen_can_launch_heal_and_carry_a_banner_as_a_warrior() {
    let mut c = armed_campaign(W::Atlatl);
    c.teams[0].fighters[0].class = C::Warrior;
    c.teams[0].fighters[0].mastery.train(C::Healer, 100);
    c.teams[0].fighters[0].mastery.train(C::BannerBearer, 100);
    c.assign_title(0, 0, phalanx::CommandTitle::Queen).unwrap();
    c.workshop(0, 0, Gear::WarBanner).unwrap();
    let bytes = serde_json::to_vec(&c).unwrap();
    let mut c: Campaign = serde_json::from_slice(&bytes).unwrap();
    c.validate().unwrap();
    let (_, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    b.units[0].position = [0., 0.];
    b.units[0].action = 1;
    b.units[1].position = [-1., 0.];
    b.units[1].health = 20.;
    for u in b.units.iter_mut().skip(3) {
        u.position = [5., (u.id - 3) as f32 * 1.3];
    }
    for _ in 0..120 {
        b.step();
        if b.units[0].healing_done > 0. && b.units[0].weapon_shots[W::Atlatl.index()] > 0 {
            break;
        }
    }
    assert_eq!(b.units[0].command_title, Some(phalanx::CommandTitle::Queen));
    assert_eq!(b.units[0].class, Some(C::Warrior));
    assert!(b.units[0].banner_active());
    assert!(b.units[0].healing_done > 0.);
    assert!(b.units[0].weapon_shots[W::Atlatl.index()] > 0);
}
#[test]
fn completed_battles_train_the_used_weapon_once_and_legacy_mastery_defaults() {
    let mut c = armed_campaign(W::Spear);
    c.teams[0].fighters[0].mastery.train_weapon(W::Spear, 90);
    let (id, mut b) = c
        .begin_battle(42, Arena::OpenField, Controller::Utility)
        .unwrap();
    while !b.finished {
        b.step();
    }
    assert!(b.units[0].weapon_shots[W::Spear.index()] > 0);
    c.settle(id, &b).unwrap();
    let report = &c.last_report.as_ref().unwrap().fighters[0][0];
    assert_eq!(report.discoveries, vec![Gear::Atlatl]);
    assert!(report.weapon_training[W::Spear.index()] > 0);
    assert_eq!(c.teams[0].fighters[2].mastery.weapons, [0; 4]);
    assert!(c.teams[0].fighters[0].mastery.weapon_points(W::Spear) > 0);
    assert_eq!(c.teams[0].fighters[0].mastery.weapon_points(W::Rock), 100);
    let earned = c.teams[0].fighters[0].mastery;
    assert!(c.settle(id, &b).is_err());
    assert_eq!(c.teams[0].fighters[0].mastery, earned);
    let legacy = serde_json::json!({"practice":[0,0,0,0,0,0,0,0,0,0]});
    let restored: phalanx::mastery::Mastery = serde_json::from_value(legacy).unwrap();
    assert_eq!(restored.weapons, [0; 4]);
    assert!(restored.valid());
}

#[test]
fn shafts_miss_a_target_that_left_the_fixed_trajectory() {
    let mut b = duel(W::Spear);
    b.units[0].cooldown = 999.;
    b.units[0].position = [-4., 0.];
    b.units[3].position = [4., 2.];
    b.rocks.push(Rock {
        weapon: W::Spear,
        shooter: 0,
        team: 0,
        from: [-4., 0.],
        to: [4., 0.],
        age: 0,
        flight_ticks: 1,
        damage: 18.,
    });
    b.step();
    assert_eq!(b.units[3].health, 1000.);
    assert!(b.damage_events.is_empty());
    assert!(b.rocks.is_empty());
}
#[test]
fn an_obstructed_windup_cancels_and_rivals_buy_only_discovered_technologies() {
    let mut b = duel(W::Spear);
    b.units[0].position = [-2., 3.5];
    b.units[3].position = [2., 3.5];
    b.step();
    assert!(b.units[0].windup.is_some());
    b.arena = Arena::Choke;
    b.step();
    assert!(b.units[0].windup.is_none());
    assert_eq!(b.units[0].ammunition, 6);
    assert_eq!(b.units[0].rocks_thrown, 0);
    let mut c = Campaign::new();
    c.select_rival(0).unwrap();
    let player = c.teams[0].clone();
    let rival = &mut c.teams[1];
    rival.gold = 1000;
    rival.salvage = 1000;
    rival.fighters[0].mastery.train_weapon(W::Rock, 100);
    phalanx::rivals::evolve(rival, &player, 0.);
    assert!(rival.fighters[0].equipment.has(Gear::ThrowingSpear));
    assert!(!rival.fighters[0].equipment.owns(Gear::Atlatl));
    assert!(rival.gold < 1000 && rival.salvage < 1000);
    assert_eq!(rival.fighters[0].mastery.weapon_points(W::Spear), 0);
    rival.fighters[0].mastery.train_weapon(W::Spear, 100);
    phalanx::rivals::evolve(rival, &player, 0.);
    assert!(rival.fighters[0].equipment.has(Gear::Atlatl));
    c.validate().unwrap();
}
