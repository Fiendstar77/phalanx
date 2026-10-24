//! Seeded battlefield grammar shared by collision, navigation, combat and presentation.
//! Version zero preserves saved maps. Version one adds three fronts and permanent ruins.
use crate::{Mind, navigation, pressure::PressureField, terrain::Arena};
use std::ops::Deref;
pub type Wall = ([f32; 2], [f32; 2]);
pub const MAX_WALLS: usize = 12;

/// An allocation-free wall list, including permanent masonry after four timber barricades.
#[derive(Clone, Copy, Debug)]
pub struct Walls {
    items: [Wall; MAX_WALLS],
    len: usize,
}
impl Default for Walls {
    fn default() -> Self {
        Self {
            items: [([0.; 2], [0.; 2]); MAX_WALLS],
            len: 0,
        }
    }
}
impl Walls {
    pub fn from_slice(walls: &[Wall]) -> Self {
        let mut out = Self::default();
        for &wall in walls {
            out.push(wall);
        }
        out
    }
    fn push(&mut self, wall: Wall) {
        self.items[self.len] = wall;
        self.len += 1;
    }
}
impl Deref for Walls {
    type Target = [Wall];
    fn deref(&self) -> &[Wall] {
        &self.items[..self.len]
    }
}

pub fn hash(mut seed: u64) -> u64 {
    seed = seed.wrapping_add(0x9e3779b97f4a7c15);
    seed = (seed ^ (seed >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    seed = (seed ^ (seed >> 27)).wrapping_mul(0x94d049bb133111eb);
    seed ^ (seed >> 31)
}
fn variation(field: PressureField, salt: u64) -> f32 {
    (hash(field.seed ^ salt) % 1024) as f32 / 1023.0
}
fn scale(field: PressureField) -> [f32; 2] {
    let half = field.half();
    [half[0] / 18., half[1] / 13.]
}
fn normalized(field: PressureField, p: [f32; 2]) -> [f32; 2] {
    let s = scale(field);
    [p[0] / s[0], p[1] / s[1]]
}
pub fn family(field: PressureField) -> &'static str {
    match field.layout.saturating_sub(1) % 4 {
        0 => "THE BROKEN GATE",
        1 => "CROWN OF STONE",
        2 => "THE LONG CAUSEWAY",
        _ => "THE SUNKEN COURT",
    }
}
pub fn briefing(field: PressureField) -> &'static str {
    match field.layout.saturating_sub(1) % 4 {
        0 => {
            "A narrow gate offers the shortest engagement. Ruined wings screen two covered flanks. Breach timber to open crossfire into the courtyard."
        }
        1 => {
            "Twin heights overlook an exposed saddle. Broken walls interrupt firing lanes. Covered outer routes trade distance for protection."
        }
        2 => {
            "Dry causeways cross a slow basin. The direct crossing is exposed; flank cover protects the longer approach. Breach barricades to change lanes."
        }
        _ => {
            "A low central court invites a quick clash. Raised shoulders command its approaches, while ruined corners hide flanking movements."
        }
    }
}
/// Reflection across X gives both sides identical approaches and deployment clearance.
pub fn original_walls(field: PressureField) -> Walls {
    if field.generator == 0 {
        let l = field.layout.saturating_sub(1) % 8;
        let x = 4. + (l % 3) as f32 * 1.5;
        let z = 2.5 + (l / 3) as f32;
        return Walls::from_slice(&[
            ([-x, -z], [1.2, 3.]),
            ([x, z], [1.2, 3.]),
            ([-x, z + 3.5], [2.4, 1.1]),
            ([x, -z - 3.5], [2.4, 1.1]),
        ]);
    }
    let s = scale(field);
    let spread = variation(field, 17) * 0.8;
    let length = variation(field, 39) * 0.8;
    let mut out = Walls::default();
    let mut quadrants = |x: f32, z: f32, sx: f32, sz: f32| {
        for a in [-1., 1.] {
            for b in [-1., 1.] {
                out.push(([x * a * s[0], z * b * s[1]], [sx * s[0], sz * s[1]]));
            }
        }
    };
    match field.layout.saturating_sub(1) % 4 {
        0 => {
            quadrants(6. + spread, 2.4, 1., 1.8 + length);
            quadrants(4.6, 6. + spread, 4. + length, 1.3);
        }
        1 => {
            quadrants(7. + spread, 3.3, 1.1, 2. + length);
            quadrants(3.8, 5.3 + spread, 3. + length, 1.3);
        }
        2 => {
            quadrants(7. + spread, 2.6, 1.1, 1.8 + length);
            quadrants(7., 6.8 + spread, 3.4 + length, 1.2);
        }
        _ => {
            quadrants(8. + spread, 2.3, 1.1, 1.7 + length);
            quadrants(4.7 + spread, 5., 1.4, 3. + length);
        }
    }
    let (z, width, depth) = match field.layout.saturating_sub(1) % 4 {
        0 => (3.9 + spread, 1.8, 3.3 + length),
        1 => (7.8, 3.6 + length, 1.3),
        2 => (5., 1.6, 3.3 + length),
        _ => (8.5, 4. + length, 1.3),
    };
    for sign in [-1., 1.] {
        out.push(([0., sign * z * s[1]], [width * s[0], depth * s[1]]));
    }
    out
}
pub fn walls(field: PressureField) -> Walls {
    let mut out = Walls::default();
    for (i, &wall) in original_walls(field).iter().enumerate() {
        if i >= 4 || field.destroyed & (1 << i) == 0 {
            out.push(wall);
        }
    }
    out
}
pub fn flank_z(field: PressureField) -> f32 {
    field.half()[1] * 0.80
}
pub fn zones(field: PressureField) -> [(&'static str, [f32; 2]); 7] {
    let x = field.half()[0] * 0.55;
    [
        ("Deployment A", [-field.spawn_x(), 0.]),
        ("Covered approach", [-x, 0.]),
        ("Central crossing", [0., 0.]),
        ("Upper flank", [0., flank_z(field)]),
        ("Lower flank", [0., -flank_z(field)]),
        ("Covered approach", [x, 0.]),
        ("Deployment B", [field.spawn_x(), 0.]),
    ]
}
pub fn cover_anchors(field: PressureField) -> [[f32; 2]; 12] {
    let s = scale(field);
    let x = 9.5 + variation(field, 9);
    let z = 8.5 + variation(field, 12) * 0.6;
    [
        [-x, z],
        [x, z],
        [-x, -z],
        [x, -z],
        [-3., 9.7],
        [3., 9.7],
        [-3., -9.7],
        [3., -9.7],
        [-11.5, 4.3],
        [11.5, 4.3],
        [-11.5, -4.3],
        [11.5, -4.3],
    ]
    .map(|p| [p[0] * s[0], p[1] * s[1]])
}
fn mound(p: [f32; 2], center: [f32; 2], radius: [f32; 2], height: f32) -> f32 {
    let r = (((p[0] - center[0]) / radius[0]).powi(2) + ((p[1] - center[1]) / radius[1]).powi(2))
        .sqrt();
    let t = (1. - r).clamp(0., 1.);
    height * t * t * (3. - 2. * t)
}
pub fn high_points(field: PressureField) -> [[f32; 2]; 3] {
    let s = scale(field);
    let z = match field.layout.saturating_sub(1) % 4 {
        1 => 3.4,
        3 => 6.,
        _ => 0.,
    };
    [[0., z * s[1]], [0., -z * s[1]], [0., 0.]]
}
pub fn elevation(field: PressureField, p: [f32; 2]) -> f32 {
    let q = normalized(field, p);
    let height = 1.6
        + variation(field, 31)
        + if field.lesson == crate::pressure::Lesson::Ridge {
            0.7
        } else {
            0.
        };
    let base = match field.layout.saturating_sub(1) % 4 {
        0 => mound(q, [0., 0.], [7.5, 8.], height),
        1 => mound(q, [0., 3.4], [7., 3.6], height).max(mound(q, [0., -3.4], [7., 3.6], height)),
        2 => {
            mound(q, [0., 0.], [6., 3.5], height * 0.65)
                + mound([q[0], q[1].abs()], [0., 9.5], [7., 2.5], height * 0.5)
        }
        _ => mound([q[0], q[1].abs()], [0., 6.], [7.5, 4.5], height),
    };
    let strata = 0.10 * (q[0] * 0.55).cos() * (q[1] * 0.5).cos();
    let edge = (1. - (q[0].abs() / 16.).powi(4)).clamp(0., 1.);
    (base + (strata + 0.1) * edge).max(0.)
}
pub fn cover(field: PressureField, p: [f32; 2]) -> f32 {
    let q = normalized(field, p);
    let flank = q[1].abs() > 7.6 && q[1].abs() < 11.2 && q[0].abs() < 12.;
    let s = scale(field);
    let pockets = cover_anchors(field).iter().any(|a| {
        ((p[0] - a[0]) / s[0]).powi(2) / 2.3_f32.powi(2)
            + ((p[1] - a[1]) / s[1]).powi(2) / 1.35_f32.powi(2)
            < 1.
    });
    f32::from(flank || pockets)
}
pub fn roughness(field: PressureField, p: [f32; 2]) -> f32 {
    let q = normalized(field, p);
    let crossing = field.layout.saturating_sub(1) % 4 == 2
        || field.lesson == crate::pressure::Lesson::Approach;
    if crossing && q[0].abs() < 5.5 && q[1].abs() < 7.3 {
        return if (q[1].abs() - 2.).abs() < 1.0 {
            0.
        } else {
            0.85
        };
    }
    if q[0].abs() < 12. && q[0].abs() > 4. && (q[1].abs() - 8.8).abs() < 1.8 {
        0.35
    } else {
        0.
    }
}
/// Presentation only; cover and mud retain distinct colors in every biome.
pub fn palette(field: PressureField) -> [[f32; 3]; 4] {
    match hash(field.seed ^ 83) % 4 {
        0 => [
            [0.36, 0.39, 0.26],
            [0.20, 0.32, 0.16],
            [0.27, 0.22, 0.16],
            [0.60, 0.55, 0.42],
        ],
        1 => [
            [0.48, 0.35, 0.24],
            [0.30, 0.36, 0.18],
            [0.30, 0.23, 0.19],
            [0.70, 0.55, 0.37],
        ],
        2 => [
            [0.27, 0.32, 0.38],
            [0.17, 0.30, 0.29],
            [0.22, 0.22, 0.26],
            [0.49, 0.56, 0.61],
        ],
        _ => [
            [0.38, 0.36, 0.31],
            [0.29, 0.35, 0.22],
            [0.24, 0.22, 0.20],
            [0.61, 0.58, 0.50],
        ],
    }
}
/// Actual body-clearance routes through the center and BOTH flanks.
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
    field.generator = 1;
    field.seed = seed;
    field.layout = 1 + (hash(seed) % 4) as u8;
    field.destroyed = 0;
    field
}
