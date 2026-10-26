use phalanx::{
    Battle, FighterClass as C, Mind,
    equipment::Gear,
    learned::Controller,
    ninjutsu::{self as arts, Art, Smoke},
    progression::Campaign,
    skills::{Skill, SkillLoadout},
    terrain::Arena,
    theus::Sheet,
};
fn duel(arena: Arena, gear: Option<Gear>) -> Battle {
    let mut c = Campaign::new();
    for team in &mut c.teams {
        for f in &mut team.fighters {
            f.class = C::Warrior;
            f.mind = Mind::disciplined();
            f.skills = Some(SkillLoadout::default());
            f.theus = Some(Sheet {
                attributes: [10, 14, 10, 10],
                ..Default::default()
            });
        }
    }
    if let Some(g) = gear {
        c.workshop(0, 0, g).unwrap();
    }
    let (_, mut b) = c.begin_battle(6, arena, Controller::Utility).unwrap();
    for u in &mut b.units {
        u.health = 0.;
        u.cooldown = 999.;
    }
    for (id, p) in [(0, [0., 0.]), (3, [4., 0.])] {
        b.units[id].health = 1000.;
        b.units[id].position = p;
        b.units[id].target = p;
    }
    b
}
fn equip(b: &mut Battle, id: usize, skill: Skill) {
    b.units[id].skills.as_mut().unwrap().active[0] = Some(skill);
}
#[test]
fn wall_dance_traverses_a_solid_wall_above_its_top_and_lands() {
    let mut b = duel(Arena::Choke, None);
    b.units[0].position = [-2., -5.];
    b.units[0].target = [4., -5.];
    b.units[3].position = [4., -5.];
    equip(&mut b, 0, Skill::WallDance);
    let effort = b.units[0].theus.as_ref().unwrap().stamina;
    let mut over_wall = false;
    for _ in 0..15 {
        b.step();
        let u = &b.units[0];
        let tick = b.tick.saturating_sub(1) as f32;
        let p = arts::body(u, b.arena, tick);
        let foot = [p[0], p[1] - 0.85, p[2]];
        assert!(
            !arts::solid(b.arena, foot, 0.4),
            "wall penetration {foot:?}"
        );
        if b.arena.blocked(u.position) {
            over_wall = true;
            assert!(foot[1] > 2.8);
        }
    }
    assert!(over_wall, "the fighter only went around the wall");
    assert!(b.units[0].position[0] > 1.5);
    assert!(!arts::in_transit(&b.units[0]));
    assert!(b.ninjutsu.events.iter().any(|e| e.art == Art::WallVault));
    assert!(b.units[0].theus.as_ref().unwrap().stamina < effort);
}
#[test]
fn a_missing_leg_or_insufficient_effort_prevents_wall_traversal() {
    for wounded in [true, false] {
        let mut b = duel(Arena::Choke, None);
        b.units[0].position = [-2., -5.];
        b.units[0].target = [4., -5.];
        equip(&mut b, 0, Skill::WallDance);
        if wounded {
            b.units[0].lost_limbs = 4;
        } else {
            b.units[0].theus.as_mut().unwrap().stamina = 0.;
        }
        b.step();
        assert!(!arts::in_transit(&b.units[0]));
        assert!(!b.ninjutsu.events.iter().any(|e| e.art == Art::WallVault));
    }
}
#[test]
fn smoke_obscures_both_teams_retains_memory_and_then_clears() {
    let mut b = duel(Arena::OpenField, None);
    b.units[0].position = [-3., 0.];
    b.units[3].position = [3., 0.];
    phalanx::perception::update(&mut b.units, b.arena, 0);
    assert_eq!(b.units[0].visible_enemies, 1);
    assert_eq!(b.units[3].visible_enemies, 1);
    b.ninjutsu.smoke.push(Smoke {
        owner: 0,
        position: [0., 0., 0.],
        start: 0,
        end: 30,
    });
    phalanx::perception::update_in_field(&mut b.units, b.arena, 5, &b.ninjutsu);
    for id in [0, 3] {
        assert_eq!(b.units[id].visible_enemies, 0);
        assert!(!b.units[id].contacts.is_empty());
        assert!(b.units[id].contacts.iter().all(|c| !c.visible));
    }
    assert!(
        b.ninjutsu.clear_sight([-3., 5., 0.], [3., 5., 0.], 5.),
        "vaulting above smoke must clear the obscurant"
    );
    phalanx::perception::update_in_field(&mut b.units, b.arena, 31, &b.ninjutsu);
    for id in [0, 3] {
        assert_eq!(b.units[id].visible_enemies, 1);
    }
}
#[test]
fn actual_smoke_deployment_spends_finite_charges_and_effort() {
    let mut b = duel(Arena::OpenField, None);
    equip(&mut b, 0, Skill::SmokeBomb);
    b.units[0].health = 100.;
    b.tick = 20;
    phalanx::perception::update(&mut b.units, b.arena, b.tick);
    let effort = b.units[0].theus.as_ref().unwrap().stamina;
    b.step();
    assert_eq!(b.units[0].ninja.as_ref().unwrap().smoke_charges, 1);
    assert_eq!(b.ninjutsu.smoke.len(), 1);
    assert!(b.units[0].theus.as_ref().unwrap().stamina < effort);
    for _ in 0..160 {
        b.step();
        assert!(b.ninjutsu.smoke.len() <= arts::SMOKE_CAPACITY);
    }
    assert!(
        b.ninjutsu
            .events
            .iter()
            .filter(|e| e.art == Art::SmokeBomb && e.caster == 0)
            .count()
            <= 2
    );
}
#[test]
fn shadow_binding_is_opposed_spends_effort_and_can_be_resisted() {
    let mut success = false;
    let mut resisted = false;
    for seed in 0..64 {
        let mut b = duel(Arena::OpenField, None);
        b.seed = seed;
        b.tick = 23;
        equip(&mut b, 0, Skill::ShadowSeal);
        b.units[0].theus.as_mut().unwrap().sheet.attributes = [10, 10, 14, 10];
        b.units[3].theus.as_mut().unwrap().sheet.attributes = [10, 10, 10, 14];
        phalanx::perception::update(&mut b.units, b.arena, b.tick);
        b.step();
        let e = b
            .ninjutsu
            .events
            .iter()
            .find(|e| e.art == Art::GraveBind)
            .expect("no attempted binding");
        assert!(e.attack.is_some() && e.defence.is_some());
        assert!(b.units[0].theus.as_ref().unwrap().stamina <= 26.01);
        if e.success {
            success = true;
            assert!(b.units[3].ninja.as_ref().unwrap().bound_until > b.tick);
            assert!(b.units[3].movement_multiplier() < 0.5);
        } else {
            resisted = true;
            assert_eq!(b.units[3].ninja.as_ref().unwrap().bound_until, 0);
        }
    }
    assert!(success && resisted);
    // A cached sighting from the previous tick must not authorize a new
    // spell or weapon throw at a fighter who has already fallen.
    let mut fallen = duel(Arena::OpenField, Some(Gear::RazorWheel));
    equip(&mut fallen, 0, Skill::ShadowSeal);
    fallen.tick = 23;
    fallen.units[0].cooldown = 0.;
    phalanx::perception::update(&mut fallen.units, fallen.arena, fallen.tick);
    fallen.units[3].health = 0.;
    fallen.step();
    assert!(fallen.ninjutsu.events.is_empty());
    let mut b = duel(Arena::Choke, None);
    b.units[0].position = [-2., -5.];
    b.units[0].target = [4., -5.];
    b.units[3].position = [2., -5.];
    b.tick = 23;
    equip(&mut b, 0, Skill::ShadowSeal);
    phalanx::perception::update(&mut b.units, b.arena, b.tick);
    b.step();
    assert!(
        !b.ninjutsu.events.iter().any(|e| e.art == Art::GraveBind),
        "a caster used a hidden transform behind a wall"
    );
}
#[test]
fn chain_hooks_follow_real_successful_hits_and_respect_walls() {
    let mut b = duel(Arena::OpenField, Some(Gear::Kusarigama));
    b.units[0].cooldown = 0.;
    b.units[3].position = [2.8, 0.];
    assert_eq!(b.units[0].attack_range(), 3.3);
    for _ in 0..80 {
        b.step();
    }
    assert!(
        b.theus_events
            .iter()
            .any(|e| e.attacker == 0 && e.resolution.damage > 0.)
    );
    assert!(b.ninjutsu.events.iter().any(|e| e.art == Art::ChainSnare));
    assert!(b.units[3].health < 1000.);
    let mut blocked = duel(Arena::Choke, Some(Gear::Kusarigama));
    blocked.units[0].position = [-1.7, -5.];
    blocked.units[3].position = [1.7, -5.];
    blocked.units[0].cooldown = 0.;
    for _ in 0..5 {
        blocked.step();
    }
    assert!(
        !blocked
            .ninjutsu
            .events
            .iter()
            .any(|e| e.art == Art::ChainSnare)
    );
}
#[test]
fn razor_wheels_are_physical_outbound_and_returning_projectiles() {
    let mut b = duel(Arena::OpenField, Some(Gear::RazorWheel));
    b.units[0].cooldown = 0.;
    b.units[3].position = [4., 0.];
    let mut launched = false;
    let mut returned = false;
    let mut hits = 0;
    for _ in 0..80 {
        b.step();
        launched |= !b.ninjutsu.wheels.is_empty();
        returned |= b.ninjutsu.wheels.iter().any(|w| w.returning);
        hits = hits.max(
            b.ninjutsu
                .wheels
                .iter()
                .map(|w| w.hit.len())
                .max()
                .unwrap_or(0),
        );
        assert!(b.ninjutsu.wheels.len() <= arts::WHEEL_CAPACITY);
    }
    assert!(launched && returned && hits > 0);
    assert!(b.units[3].health < 1000.);
    assert!(b.ninjutsu.events.iter().any(|e| e.art == Art::WheelThrow));
    let mut wall = duel(Arena::Choke, Some(Gear::RazorWheel));
    wall.units[0].position = [-2., -5.];
    wall.units[3].position = [2., -5.];
    wall.units[0].cooldown = 0.;
    for _ in 0..5 {
        wall.step();
    }
    assert!(
        wall.ninjutsu.wheels.is_empty(),
        "a blade was launched through opaque masonry"
    );
}
#[test]
fn ninja_loadouts_and_odd_weapons_survive_save_reload_without_spending_veteran_progress() {
    let mut c = Campaign::new();
    c.workshop(0, 0, Gear::Kusarigama).unwrap();
    c.workshop(0, 1, Gear::RazorWheel).unwrap();
    let loaded: Campaign = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    loaded.validate().unwrap();
    assert_eq!(loaded.teams[0].gold, 90);
    assert!(loaded.teams[0].fighters[0].equipment.has(Gear::Kusarigama));
    assert!(loaded.teams[0].fighters[1].equipment.has(Gear::RazorWheel));
    let mut loaded = loaded;
    let (_, b) = loaded
        .begin_battle(6, Arena::OpenField, Controller::Utility)
        .unwrap();
    assert_eq!(b.units[0].ninja.as_ref().unwrap().smoke_charges, 2);
    assert!(
        b.units[0].equipped_skill(Skill::WallDance)
            && b.units[0].equipped_skill(Skill::SmokeBomb)
            && b.units[0].equipped_skill(Skill::ShadowSeal)
    );
    assert!(
        Battle::new(6, false)
            .units
            .iter()
            .all(|u| u.ninja.is_none())
    );
}

#[test]
fn simultaneous_wall_dancers_reserve_separate_flight_paths() {
    let mut b = duel(Arena::Choke, None);
    b.units[0].position = [-2., -5.];
    b.units[0].target = [4., -5.];
    b.units[0].target = [4., -5.];
    b.units[3].position = [2., -5.];
    b.units[3].target = [-4., -5.];
    equip(&mut b, 0, Skill::WallDance);
    equip(&mut b, 3, Skill::WallDance);
    b.step();
    assert!(arts::in_transit(&b.units[0]));
    assert!(
        !arts::in_transit(&b.units[3]),
        "opposing ninjas reserved intersecting trajectories"
    );
    let l = b.units[0].ninja.as_ref().unwrap().leap.as_ref().unwrap();
    assert!((l.touch[0] + 1.5).abs() < 0.01 && l.touch[1] > 1.);
    for _ in 0..20 {
        b.step();
        let tick = b.tick.saturating_sub(1) as f32;
        let a = arts::body(&b.units[0], b.arena, tick);
        let c = arts::body(&b.units[3], b.arena, tick);
        assert!((a.iter().zip(c).map(|(x, y)| (x - y).powi(2)).sum::<f32>()).sqrt() > 0.89);
    }
}
