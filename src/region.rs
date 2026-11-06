//! First persistent Yard location. The same terrain serves both cameras and combat.
use crate::{
    pressure::{Lesson, PressureField},
    procedural::{Wall, Walls},
    terrain::Arena,
};
use serde::{Deserialize, Serialize};
pub const GENERATOR: u8 = 2;
pub const NAME: &str = "THE HOLLOW PROCESSION";
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Yard {
    pub seed: Option<u64>,
    pub destroyed: u8,
    pub elapsed_ticks: u64,
}
impl Yard {
    pub fn valid(&self) -> bool {
        self.destroyed <= 15
            && self.elapsed_ticks <= 4_000_000_000
            && (self.seed.is_some() || self.destroyed == 0 && self.elapsed_ticks == 0)
    }
    pub fn arena(&mut self, seed: u64) -> Arena {
        let seed = *self.seed.get_or_insert(seed);
        Arena::Pressure(PressureField {
            generator: GENERATOR,
            seed,
            destroyed: self.destroyed,
            lesson: Lesson::Exposure,
            reach: 2,
            layout: 1,
            ..Default::default()
        })
    }
    pub fn remember(&mut self, battle: &crate::Battle, completed: bool) {
        if let Arena::Pressure(f) = battle.arena
            && f.generator == GENERATOR
            && self.seed == Some(f.seed)
        {
            self.destroyed |= f.destroyed;
            if completed {
                self.elapsed_ticks = self.elapsed_ticks.saturating_add(battle.tick as u64);
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Address {
    pub location: String,
    pub seed: u64,
    pub origin: [i32; 2],
    pub terrain_revision: u32,
}
pub fn address(arena: Arena) -> Option<Address> {
    match arena {
        Arena::Pressure(f) if f.generator == GENERATOR => Some(Address {
            location: "hollow_procession".into(),
            seed: f.seed,
            origin: [0, 0],
            terrain_revision: f.destroyed.count_ones(),
        }),
        _ => None,
    }
}
pub fn road(f: PressureField, x: f32) -> f32 {
    (x * 0.065).sin() * 3.5 + ((f.seed % 7) as f32 - 3.) * 0.25
}
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn hill(p: [f32; 2], c: [f32; 2], r: [f32; 2], h: f32) -> f32 {
    let d = (((p[0] - c[0]) / r[0]).powi(2) + ((p[1] - c[1]) / r[1]).powi(2)).sqrt();
    h * smooth((1. - d) / 0.65)
}
pub fn elevation(f: PressureField, p: [f32; 2]) -> f32 {
    let edge = smooth((p[0].abs().max(p[1].abs()) - 40.) / 12.);
    let terrace = hill(p, [-19., 15.], [18., 14.], 4.8).max(hill(p, [19., -15.], [18., 14.], 4.8));
    let shrine = hill(p, [0., 0.], [12., 15.], 2.2);
    let fault = 0.20 * (p[0] * 0.21 + (p[1] * 0.12).sin() * 2.).sin() * (p[1] * 0.13).cos();
    let ridge = 10. * edge + terrace.max(shrine) + fault;
    let on_road = 1. - smooth(((p[1] - road(f, p[0])).abs() - 1.) / 3.);
    (ridge * (1. - on_road * 0.60) + 0.18).max(0.)
}
pub fn cover(_: PressureField, p: [f32; 2]) -> f32 {
    f32::from(
        anchors()
            .iter()
            .any(|a| ((p[0] - a[0]) / 3.8).powi(2) + ((p[1] - a[1]) / 2.4).powi(2) < 1.),
    )
}
pub fn roughness(f: PressureField, p: [f32; 2]) -> f32 {
    let off = (p[1] - road(f, p[0])).abs();
    if off < 2.2 {
        0.
    } else if off < 7.5 && p[0].abs() < 31. {
        0.8
    } else if (p[1].abs() - 26.).abs() < 4. && p[0].abs() < 35. {
        0.45
    } else {
        0.
    }
}
pub fn anchors() -> [[f32; 2]; 12] {
    [
        [-10., 10.],
        [10., -10.],
        [-22., -11.],
        [22., 11.],
        [-8., -18.],
        [8., 18.],
        [-30., 18.],
        [30., -18.],
        [-16., 28.],
        [16., -28.],
        [-34., -25.],
        [34., 25.],
    ]
}
pub fn walls() -> Walls {
    let walls: [Wall; 12] = [
        ([-5., -5.], [1.4, 4.]),
        ([5., 5.], [1.4, 4.]),
        ([-8., 4.], [3.5, 1.2]),
        ([8., -4.], [3.5, 1.2]),
        ([-18., 10.], [8., 2.]),
        ([18., -10.], [8., 2.]),
        ([-12., -18.], [2., 8.]),
        ([12., 18.], [2., 8.]),
        ([0., -49.], [100., 2.]),
        ([0., 49.], [100., 2.]),
        ([-49., 0.], [2., 96.]),
        ([49., 0.], [2., 96.]),
    ];
    Walls::from_slice(&walls)
}
fn mineral(p: [f32; 2], frequency: f32) -> f32 {
    let x = p[0] * frequency;
    let z = p[1] * frequency;
    let ix = x.floor() as i32;
    let iz = z.floor() as i32;
    let sample = |x: i32, z: i32| {
        let n = (x as u32).wrapping_mul(374_761_393) ^ (z as u32).wrapping_mul(668_265_263);
        let n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
        (n ^ (n >> 16)) as f32 / u32::MAX as f32
    };
    let u = smooth(x - x.floor());
    let v = smooth(z - z.floor());
    let a = sample(ix, iz);
    let b = sample(ix + 1, iz);
    let c = sample(ix, iz + 1);
    let d = sample(ix + 1, iz + 1);
    (a + (b - a) * u) * (1. - v) + (c + (d - c) * u) * v
}
/// Continuous surface paint; independent of the navigation grid and ground tessellation.
pub fn pigment(f: PressureField, p: [f32; 2]) -> [f32; 3] {
    let y = elevation(f, p);
    let d = (p[1] - road(f, p[0])).abs();
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t);
    let mut color = mix(
        [0.25, 0.30, 0.28],
        [0.61, 0.61, 0.49],
        smooth((y - 0.9) / 2.8),
    );
    let basin = smooth((d - 1.9) / 0.65)
        * (1. - smooth((d - 7.) / 0.8))
        * (1. - smooth((p[0].abs() - 30.) / 2.));
    color = mix(color, [0.045, 0.085, 0.105], basin);
    color = mix(color, [0.49, 0.25, 0.23], 1. - smooth((d - 1.3) / 0.65));
    let reed = anchors()
        .iter()
        .map(|a| {
            let r = (((p[0] - a[0]) / 3.8).powi(2) + ((p[1] - a[1]) / 2.4).powi(2)).sqrt();
            1. - smooth((r - 0.87) / 0.18)
        })
        .fold(0., f32::max);
    color = mix(color, [0.12, 0.23, 0.22], reed * 0.78);
    let bands = (y * 12. + (p[0] * 0.07).sin() * 0.7).sin();
    let sediment = smooth((bands - 0.75) / 0.25) * smooth((y - 0.6) / 1.2) * 0.045;
    // Fine mineral grain, larger weathering and delicate engraved road grooves.
    let grain = (mineral(p, 6.) - 0.5) * 0.028;
    let weather = (mineral(p, 0.85) - 0.5) * 0.040;
    let groove = smooth(((p[0] * 8. + (p[1] * 0.6).sin() * 0.3).sin() - 0.92) / 0.08)
        * 0.025
        * (1. - smooth((d - 1.1) / 0.4));
    color.map(|v| (v + grain + weather - sediment - groove).clamp(0., 1.))
}
