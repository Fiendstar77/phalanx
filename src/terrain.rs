//! Authored arena rules shared by movement, route planning, combat and rendering.
use crate::distance;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Arena {
    #[default]
    Ridge,
    Choke,
    OpenField,
    Citadel,
    Highlands,
    Ruins,
    GrandArena,
}
impl Arena {
    pub fn name(self) -> &'static str {
        match self {
            Self::Ridge => "THE RIDGE",
            Self::Choke => "THE CHOKE",
            Self::OpenField => "OPEN FIELD",
            Self::Citadel => "THE CITADEL",
            Self::Highlands => "THE HIGHLANDS",
            Self::Ruins => "THE RUINS",
            Self::GrandArena => "THE GRAND ARENA",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "ridge" => Some(Self::Ridge),
            "choke" => Some(Self::Choke),
            "open" => Some(Self::OpenField),
            "citadel" => Some(Self::Citadel),
            "highlands" => Some(Self::Highlands),
            "ruins" => Some(Self::Ruins),
            "grand" => Some(Self::GrandArena),
            _ => None,
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Ridge => Self::Choke,
            Self::Choke => Self::OpenField,
            Self::OpenField => Self::Citadel,
            Self::Citadel => Self::Highlands,
            Self::Highlands => Self::Ruins,
            Self::Ruins => Self::GrandArena,
            Self::GrandArena => Self::Ridge,
        }
    }
    pub fn large(self) -> bool {
        matches!(
            self,
            Self::Citadel | Self::Highlands | Self::Ruins | Self::GrandArena
        )
    }
    pub fn half_size(self) -> [f32; 2] {
        if self == Self::GrandArena {
            return [30.0, 24.0];
        }
        if matches!(self, Self::Highlands | Self::Ruins) {
            return [25.0, 20.0];
        }
        if self == Self::Citadel {
            [22.0, 20.0]
        } else {
            [11.0, 8.0]
        }
    }
    pub fn team_size(self) -> usize {
        if self.large() { 10 } else { 5 }
    }
    pub fn fighters(self) -> usize {
        if self.large() { 6 } else { 3 }
    }
    pub fn has_high_ground(self) -> bool {
        matches!(
            self,
            Self::Ridge | Self::Citadel | Self::Highlands | Self::GrandArena
        )
    }
    pub fn spawn(self, team: usize, slot: usize) -> [f32; 2] {
        if self.large() && self != Self::Citadel {
            return [
                if team == 0 {
                    -self.half_size()[0] + 4.0
                } else {
                    self.half_size()[0] - 4.0
                },
                slot as f32 * 1.8 - 8.1,
            ];
        }
        if self == Self::Citadel {
            [
                if team == 0 { -18.0 } else { 18.0 },
                slot as f32 * 1.8 - 8.1,
            ]
        } else {
            [if team == 0 { -8.0 } else { 8.0 }, slot as f32 * 1.6 - 3.2]
        }
    }
    pub fn walls(self) -> &'static [([f32; 2], [f32; 2])] {
        match self {
            Self::Ruins => &[
                ([-8., -9.], [2., 10.]),
                ([-8., 9.], [2., 10.]),
                ([8., -9.], [2., 10.]),
                ([8., 9.], [2., 10.]),
                ([0., 0.], [3., 8.]),
            ],
            Self::GrandArena => &[
                ([0., -16.], [3., 9.]),
                ([0., 0.], [3., 8.]),
                ([0., 16.], [3., 9.]),
                ([-12., 0.], [2., 8.]),
                ([12., 0.], [2., 8.]),
            ],
            Self::Choke => &[([0.0, -5.0], [2.0, 6.0]), ([0.0, 5.0], [2.0, 6.0])],
            Self::Citadel => &[
                ([0.0, -12.0], [2.0, 6.0]),
                ([0.0, 0.0], [2.0, 6.0]),
                ([0.0, 12.0], [2.0, 6.0]),
                ([-9.0, 0.0], [2.0, 8.0]),
                ([9.0, 0.0], [2.0, 8.0]),
            ],
            _ => &[],
        }
    }
    pub fn high_ground(self, p: [f32; 2]) -> [f32; 2] {
        if self.large() {
            [[-6.0, -9.0], [6.0, 9.0]]
                .into_iter()
                .min_by(|a, b| distance(p, *a).total_cmp(&distance(p, *b)))
                .unwrap()
        } else {
            [0.0, (p[1] * 0.35).clamp(-2.5, 2.5)]
        }
    }
    pub fn search_waypoint(self, team: usize, slot: usize, leg: u32) -> [f32; 2] {
        if !self.large() || leg.is_multiple_of(5) {
            return self.spawn(1 - team, slot);
        }
        let sign = if team == 0 { 1.0 } else { -1.0 };
        let spread = (slot % 3) as f32 * 0.7 - 0.7;
        match leg % 5 {
            1 => [6.0 * sign + spread, 9.0 * sign + spread],
            2 => [14.0 * sign, -11.0 * sign + spread],
            3 => [-6.0 * sign + spread, -9.0 * sign + spread],
            _ => [-14.0 * sign, 11.0 * sign + spread],
        }
    }
    pub fn bridge_height(self, p: [f32; 2]) -> f32 {
        if self == Self::Citadel && (p[1].abs() - 6.0).abs() <= 1.25 && p[0].abs() < 12.0 {
            3.2 * ((12.0 - p[0].abs()) / 4.0).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
    pub fn elevation(self, p: [f32; 2]) -> f32 {
        self.ground_elevation(p).max(self.bridge_height(p))
    }
    pub fn ground_elevation(self, p: [f32; 2]) -> f32 {
        if self == Self::Ruins {
            return 0.0;
        }
        if self == Self::Highlands {
            return [[-9., -7.], [9., 7.], [0., 0.]]
                .into_iter()
                .map(|c| 4.0 * (1.0 - distance(p, c) / 9.0).max(0.0))
                .fold(0.0, f32::max);
        }
        if self.large() {
            [[-6.0, -9.0], [6.0, 9.0]]
                .into_iter()
                .map(|c| 3.8 * (1.0 - distance(p, c) / 7.0).max(0.0))
                .fold(0.0, f32::max)
        } else if self == Self::Ridge {
            crate::elevation(p)
        } else {
            0.0
        }
    }
    pub fn cover(self, p: [f32; 2]) -> f32 {
        if self.large() {
            return f32::from(
                self.cover_points()
                    .iter()
                    .any(|c| (p[0] - c[0]).abs() < 2.5 && (p[1] - c[1]).abs() < 2.0),
            );
        }
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
        if self.large() {
            return f32::from(
                (p[0].abs() < 4.0 && p[1].abs() < 8.0) || (p[0].abs() > 10.0 && p[1].abs() < 2.0),
            );
        }
        if self != Self::OpenField && p[0].abs() < 4.0 && p[1].abs() < 2.5 {
            1.0
        } else {
            0.0
        }
    }
    pub fn blocked(self, p: [f32; 2]) -> bool {
        if self.large() {
            let size = self.half_size();
            return p[0].abs() > size[0] - 0.3
                || p[1].abs() > size[1] - 0.3
                || self.walls().iter().any(|(c, s)| {
                    (p[0] - c[0]).abs() < s[0] * 0.5 + 0.3 && (p[1] - c[1]).abs() < s[1] * 0.5 + 0.3
                });
        }
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
    /// Walking cannot jump a raised deck's sides; it must use its ramp entrances.
    pub fn walk_segment(self, a: [f32; 2], b: [f32; 2]) -> bool {
        if !self.clear_segment(a, b) {
            return false;
        }
        if self != Self::Citadel {
            return true;
        }
        let steps = (distance(a, b) / 0.15).ceil().max(1.0) as usize;
        let mut height = self.elevation(a);
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let h = self.elevation([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            if (h - height).abs() > 0.35 {
                return false;
            }
            height = h;
        }
        true
    }
    fn cover_points(self) -> &'static [[f32; 2]] {
        if self.large() {
            &[
                [-14.0, -11.0],
                [-14.0, 11.0],
                [14.0, -11.0],
                [14.0, 11.0],
                [-5.0, 12.0],
                [5.0, -12.0],
            ]
        } else {
            &[[-4.5, -5.0], [-4.5, 5.0], [4.5, -5.0], [4.5, 5.0]]
        }
    }
    pub fn nearest_cover(self, p: [f32; 2]) -> Option<[f32; 2]> {
        if self == Self::OpenField {
            return None;
        }
        self.cover_points()
            .iter()
            .copied()
            .min_by(|a, b| distance(p, *a).total_cmp(&distance(p, *b)))
    }
}
