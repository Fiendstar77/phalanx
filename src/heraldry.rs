//! Every squad owns a flag design and a designated standard-bearer.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Emblem {
    #[default]
    Sun,
    Crescent,
    Diamond,
    Cross,
    Chevrons,
}
impl Emblem {
    pub const ALL: [Self; 5] = [
        Self::Sun,
        Self::Crescent,
        Self::Diamond,
        Self::Cross,
        Self::Chevrons,
    ];
    pub fn next(self) -> Self {
        Self::ALL[(Self::ALL.iter().position(|e| *e == self).unwrap() + 1) % Self::ALL.len()]
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Sun => "Sun",
            Self::Crescent => "Crescent",
            Self::Diamond => "Diamond",
            Self::Cross => "Cross",
            Self::Chevrons => "Chevrons",
        }
    }
    /// Authored geometric insignia in centered flag coordinates.
    pub fn contains(self, x: f32, y: f32) -> bool {
        match self {
            Self::Sun => {
                x * x + y * y < 0.32 * 0.32
                    || ((x.atan2(y) * 8.0).cos() > 0.7 && x * x + y * y < 0.46 * 0.46)
            }
            Self::Crescent => {
                x * x + y * y < 0.43 * 0.43 && (x - 0.20).powi(2) + (y - 0.09).powi(2) > 0.35 * 0.35
            }
            Self::Diamond => x.abs() + y.abs() < 0.52,
            Self::Cross => (x.abs() < 0.12 && y.abs() < 0.48) || (y.abs() < 0.12 && x.abs() < 0.43),
            Self::Chevrons => {
                (y + x.abs() * 0.75 - 0.20).abs() < 0.07 || (y + x.abs() * 0.75 + 0.16).abs() < 0.07
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Heraldry {
    pub accent: [f32; 3],
    pub emblem: Emblem,
}
impl Default for Heraldry {
    fn default() -> Self {
        Self {
            accent: [0.96, 0.94, 0.85],
            emblem: Emblem::Sun,
        }
    }
}
impl Heraldry {
    pub fn valid(self) -> bool {
        self.accent
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    }
    pub fn flag_pixel(self, primary: [f32; 3], u: f32, v: f32) -> [u8; 4] {
        let border = !(0.055..=0.945).contains(&u) || !(0.035..=0.965).contains(&v);
        let mark = self.emblem.contains((u - 0.5) * 1.35, (v - 0.48) * 1.8);
        // A dark outline keeps the insignia readable even with similar player colors.
        let outline = !mark
            && [(0.025, 0.0), (-0.025, 0.0), (0.0, 0.025), (0.0, -0.025)]
                .into_iter()
                .any(|(dx, dy)| {
                    self.emblem
                        .contains((u - 0.5) * 1.35 + dx, (v - 0.48) * 1.8 + dy)
                });
        let color = if border || mark {
            self.accent
        } else if outline {
            [0.035, 0.04, 0.045]
        } else {
            primary
        };
        [
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
            255,
        ]
    }
}
pub fn default_bearer() -> usize {
    2
}
