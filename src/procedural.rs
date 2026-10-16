//! Seeded tactical-zone layouts. Deployment, central objective and two flanks remain connected.
use crate::{Mind, navigation, pressure::PressureField, terrain::Arena};
pub type Wall = ([f32; 2], [f32; 2]);
#[derive(Clone, Copy)]
struct Layout {
    walls: [Wall; 4],
    len: usize,
}
const fn layouts() -> [Layout; 128] {
    let empty = Layout {
        walls: [([0., 0.], [0., 0.]); 4],
        len: 0,
    };
    let mut out = [empty; 128];
    let mut l = 0;
    while l < 8 {
        let x = 4.0 + (l % 3) as f32 * 1.5;
        let z = 2.5 + (l / 3) as f32;
        let base = [
            ([-x, -z], [1.2, 3.0]),
            ([x, z], [1.2, 3.0]),
            ([-x, z + 3.5], [2.4, 1.1]),
            ([x, -z - 3.5], [2.4, 1.1]),
        ];
        let mut mask = 0;
        while mask < 16 {
            let mut i = 0;
            while i < 4 {
                if mask & (1 << i) == 0 {
                    let n = out[l * 16 + mask].len;
                    out[l * 16 + mask].walls[n] = base[i];
                    out[l * 16 + mask].len += 1;
                }
                i += 1;
            }
            mask += 1;
        }
        l += 1;
    }
    out
}
const LAYOUTS: [Layout; 128] = layouts();
pub fn walls(field: PressureField) -> &'static [Wall] {
    let layout = &LAYOUTS
        [((field.layout.saturating_sub(1) as usize) % 8) * 16 + (field.destroyed as usize & 15)];
    &layout.walls[..layout.len]
}
pub fn original_walls(field: PressureField) -> &'static [Wall] {
    walls(PressureField {
        destroyed: 0,
        ..field
    })
}
pub fn zones(field: PressureField) -> [(&'static str, [f32; 2]); 7] {
    let half = field.half();
    [
        ("Deployment A", [-field.spawn_x(), 0.]),
        ("Covered approach", [-8., 0.]),
        ("High ground / objective", [0., 0.]),
        ("Upper flank", [0., half[1] - 3.]),
        ("Lower flank", [0., -half[1] + 3.]),
        ("Covered approach", [8., 0.]),
        ("Deployment B", [field.spawn_x(), 0.]),
    ]
}
pub fn connected(field: PressureField) -> bool {
    let arena = Arena::Pressure(field);
    let z = zones(field);
    [(0, 3), (0, 4), (3, 6), (4, 6), (0, 2), (2, 6)]
        .into_iter()
        .all(|(a, b)| {
            !navigation::plan(arena, z[a].1, z[b].1, Mind::disciplined(), 0, &[])
                .points
                .is_empty()
        })
}
pub fn decorate(mut field: PressureField, seed: u64) -> PressureField {
    field.seed = seed;
    let mut n = seed.wrapping_add(0x9e3779b97f4a7c15);
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    n ^= n >> 31;
    field.layout = 1 + ((n >> 29) % 8) as u8;
    if !connected(field) {
        field.layout = 1;
    }
    field
}
