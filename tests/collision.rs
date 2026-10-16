use phalanx::{
    Battle, Mind,
    collision::{self, BODY_DIAMETER},
    distance,
    terrain::Arena,
};
fn assert_clear(b: &Battle) {
    for (i, a) in b.units.iter().enumerate().filter(|(_, u)| u.health > 0.0) {
        for c in b.units.iter().skip(i + 1).filter(|u| u.health > 0.0) {
            assert!(
                distance(a.position, c.position) >= BODY_DIAMETER - 0.0003,
                "overlap {} {}: {}",
                a.id,
                c.id,
                distance(a.position, c.position)
            );
        }
    }
}
#[test]
fn swept_contacts_prevent_head_on_swaps_and_rendered_interpolation_overlap() {
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.health = 0.0;
    }
    b.units[0].health = 100.0;
    b.units[0].position = [-1., 0.];
    b.units[3].health = 100.0;
    b.units[3].position = [1., 0.];
    let before = b.units.clone();
    b.units[0].position = [2., 0.];
    b.units[3].position = [-2., 0.];
    let trace = collision::resolve(b.arena, &mut b.units, &before);
    assert!(trace.contacts > 0);
    assert_clear(&b);
    for n in 0..=1000 {
        let t = n as f32 / 1000.0;
        assert!(
            distance(trace.position(0, t).unwrap(), trace.position(3, t).unwrap())
                >= BODY_DIAMETER - 0.0003
        );
    }
}
#[test]
fn dense_initial_overlap_is_repaired_and_crowds_remain_clear_in_battle() {
    for arena in [Arena::OpenField, Arena::Choke, Arena::Citadel] {
        let mut b = Battle::with_arena(42, false, arena);
        for u in &mut b.units {
            u.mind = Mind::aggressive();
        }
        for _ in 0..500 {
            b.step();
            assert_clear(&b);
            if b.finished {
                break;
            }
        }
    }
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.position = [if u.team == 0 { -7. } else { 7. }, 0.];
    }
    b.step();
    assert_clear(&b);
}

#[test]
fn planted_guardian_stops_a_charge_and_reports_surface_contact() {
    let mut b = Battle::with_founders(42, false, Arena::OpenField);
    for u in &mut b.units {
        u.health = 0.;
    }
    b.units[0].health = 100.;
    b.units[0].position = [-2., 0.];
    b.units[3].health = 100.;
    b.units[3].class = Some(phalanx::FighterClass::Guardian);
    b.units[3].position = [0., 0.];
    let before = b.units.clone();
    b.units[0].position = [2., 0.];
    let trace = collision::resolve(b.arena, &mut b.units, &before);
    assert_eq!(b.units[3].position, [0., 0.]);
    assert_clear(&b);
    let hit = trace.impacts.iter().find(|c| c.a == 0 && c.b == 3).unwrap();
    assert!(hit.speed > 3.2);
    assert!(hit.velocity[0] > 0.);
    assert!((distance(hit.point, b.units[3].position) - collision::BODY_RADIUS).abs() < 0.003);
}
