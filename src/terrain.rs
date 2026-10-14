//! Authored arena rules shared by movement, route planning, combat and rendering.
use crate::distance;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Arena {
    #[default]
    Ridge,
    Choke,
    OpenField,
}
impl Arena {
    pub fn name(self) -> &'static str {
        match self {
            Self::Ridge => "THE RIDGE",
            Self::Choke => "THE CHOKE",
            Self::OpenField => "OPEN FIELD",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "ridge" => Some(Self::Ridge),
            "choke" => Some(Self::Choke),
            "open" => Some(Self::OpenField),
            _ => None,
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Ridge => Self::Choke,
            Self::Choke => Self::OpenField,
            Self::OpenField => Self::Ridge,
        }
    }
    pub fn elevation(self, p: [f32; 2]) -> f32 {
        if self == Self::Ridge {
            crate::elevation(p)
        } else {
            0.0
        }
    }
    pub fn cover(self, p: [f32; 2]) -> f32 {
        if self == Self::OpenField {
            return 0.0;
        }
        if p[0].abs() >= 2.0 && p[0].abs() <= 7.0 && p[1].abs() >= 4.0 && p[1].abs() <= 6.5 {
            1.0
        } else {
            0.0
        }
    }
    pub fn roughness(self, p: [f32; 2]) -> f32 {
        if self != Self::OpenField && p[0].abs() < 4.0 && p[1].abs() < 2.5 {
            1.0
        } else {
            0.0
        }
    }
    pub fn blocked(self, p: [f32; 2]) -> bool {
        p[0].abs() > 10.7
            || p[1].abs() > 7.7
            || (self == Self::Choke && p[0].abs() < 1.3 && p[1].abs() > 1.7)
    }
    /// Clearance test includes a 0.3-unit allowance around the choke walls.
    pub fn clear_segment(self, a: [f32; 2], b: [f32; 2]) -> bool {
        let steps = (distance(a, b) / 0.15).ceil().max(1.0) as usize;
        (0..=steps).all(|i| {
            let t = i as f32 / steps as f32;
            !self.blocked([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t])
        })
    }
    pub fn nearest_cover(self, p: [f32; 2]) -> Option<[f32; 2]> {
        if self == Self::OpenField {
            return None;
        }
        [[-4.5, -5.0], [-4.5, 5.0], [4.5, -5.0], [4.5, 5.0]]
            .into_iter()
            .min_by(|a, b| distance(p, *a).total_cmp(&distance(p, *b)))
    }
}
