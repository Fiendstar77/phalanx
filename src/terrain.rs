//! Authored arena rules shared by movement, route planning, combat and rendering.
use crate::{collision::BODY_RADIUS, distance};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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
    Gauntlet,
    Quarry,
    Mire,
    Pressure(crate::pressure::PressureField),
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
            Self::Gauntlet => "THE GAUNTLET",
            Self::Quarry => "THE QUARRY",
            Self::Mire => "THE MIRE",
            Self::Pressure(field) => field.name(),
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
            "gauntlet" => Some(Self::Gauntlet),
            "quarry" => Some(Self::Quarry),
            "mire" => Some(Self::Mire),
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
            Self::GrandArena => Self::Gauntlet,
            Self::Gauntlet => Self::Quarry,
            Self::Quarry => Self::Mire,
            Self::Mire => Self::Ridge,
            Self::Pressure(_) => Self::Ridge,
        }
    }
    pub fn large(self) -> bool {
        matches!(
            self,
            Self::Citadel
                | Self::Highlands
                | Self::Ruins
                | Self::GrandArena
                | Self::Gauntlet
                | Self::Quarry
                | Self::Mire
        )
    }
    pub fn half_size(self) -> [f32; 2] {
        if let Self::Pressure(field) = self {
            return field.half();
        }
        if self == Self::GrandArena {
            return [30.0, 24.0];
        }
        if matches!(
            self,
            Self::Highlands | Self::Ruins | Self::Gauntlet | Self::Quarry | Self::Mire
        ) {
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
            Self::Ridge | Self::Citadel | Self::Highlands | Self::GrandArena | Self::Quarry
        ) || matches!(self, Self::Pressure(field) if field.lesson == crate::pressure::Lesson::Ridge || field.layout > 0)
    }
    pub fn spawn(self, team: usize, slot: usize) -> [f32; 2] {
        if let Self::Pressure(field) = self {
            return [
                if team == 0 {
                    -field.spawn_x()
                } else {
                    field.spawn_x()
                },
                slot as f32 * 1.6 - 3.2,
            ];
        }
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
    pub fn walls(self) -> crate::procedural::Walls {
        crate::procedural::Walls::from_slice(match self {
            Self::Gauntlet => &[
                ([-6., -5.], [2.0, 18.]),
                ([6., 5.], [2.0, 18.]),
                ([-15., 6.], [2.0, 6.]),
                ([15., -6.], [2.0, 6.]),
            ],
            Self::Quarry => &[
                ([0., 0.], [4., 8.]),
                ([-11., -9.], [2., 5.]),
                ([11., 9.], [2., 5.]),
            ],
            Self::Mire => &[([0., -11.], [2., 5.]), ([0., 11.], [2., 5.])],
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
            Self::Pressure(field) => return field.walls(),
            Self::Citadel => &[
                ([0.0, -12.0], [2.0, 6.0]),
                ([0.0, 0.0], [2.0, 6.0]),
                ([0.0, 12.0], [2.0, 6.0]),
                ([-9.0, 0.0], [2.0, 8.0]),
                ([9.0, 0.0], [2.0, 8.0]),
            ],
            _ => &[],
        })
    }
    pub fn challenge(self) -> &'static str {
        match self {
            Self::Gauntlet => {
                "Switchback ruins: keep the squad moving through two offset choke points."
            }
            Self::Quarry => {
                "Two ridges and a central pit: contest elevation or flank the stonework."
            }
            Self::Mire => {
                "Deep mud slows the direct route. Dry causeways reward a longer approach."
            }
            _ => "",
        }
    }
    pub fn high_ground(self, p: [f32; 2]) -> [f32; 2] {
        if let Self::Pressure(field) = self
            && field.generator > 0
        {
            return crate::procedural::high_points(field)
                .into_iter()
                .filter(|point| !self.blocked(*point))
                .max_by(|a, b| {
                    self.elevation(*a)
                        .total_cmp(&self.elevation(*b))
                        .then_with(|| distance(p, *b).total_cmp(&distance(p, *a)))
                })
                .unwrap_or([0., 0.]);
        }
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
        if leg.is_multiple_of(5) {
            return self.spawn(1 - team, slot);
        }
        let sign = if team == 0 { 1.0 } else { -1.0 };
        let spread = (slot % 3) as f32 * 0.7 - 0.7;
        if let Self::Pressure(field) = self
            && field.generator > 0
        {
            // Revisit shared crossings rather than sweeping empty deployment-side corners.
            // These are public map objectives, never hidden enemy coordinates.
            let z = match leg % 5 {
                2 => crate::procedural::flank_z(field),
                4 => -crate::procedural::flank_z(field),
                _ => 0.,
            };
            return [spread * 0.5, z];
        }
        if self == Self::Gauntlet {
            let point = match leg % 5 {
                1 => [0.0, 16.0],
                2 => [12.0, -16.0],
                3 => [0.0, -16.0],
                _ => [-12.0, 16.0],
            };
            return [point[0] * sign, point[1] * sign + spread];
        }
        if !self.large() {
            let [hx, hz] = self.half_size();
            let (x, z) = match leg % 5 {
                1 => (0.60, 0.65),
                2 => (0.60, -0.65),
                3 => (-0.60, -0.65),
                _ => (-0.60, 0.65),
            };
            return [x * hx * sign, z * hz * sign + spread];
        }
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
        if let Self::Pressure(field) = self {
            return field.elevation(p);
        }
        if matches!(self, Self::Ruins | Self::Gauntlet) {
            return 0.0;
        }
        if self == Self::Mire {
            return if (p[1].abs() - 6.0).abs() < 1.5 {
                0.35
            } else {
                0.0
            };
        }
        if self == Self::Quarry {
            return [[-6., -9.], [6., 9.]]
                .into_iter()
                .map(|c| 4.5 * (1.0 - distance(p, c) / 8.0).max(0.0))
                .fold(0.0, f32::max);
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
        if let Self::Pressure(field) = self {
            return field.cover(p);
        }
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
        if self == Self::Mire {
            return if (p[1].abs() - 6.0).abs() < 1.5 || p[0].abs() > 17.0 {
                0.0
            } else {
                1.6
            };
        }
        if self == Self::Quarry {
            return f32::from(p[0].abs() < 4.0);
        }
        if let Self::Pressure(field) = self {
            return field.roughness(p);
        }
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
        if let Self::Pressure(field) = self {
            return field.blocked(p);
        }
        if self.large() {
            let size = self.half_size();
            return p[0].abs() > size[0] - BODY_RADIUS
                || p[1].abs() > size[1] - BODY_RADIUS
                || self.walls().iter().any(|(c, s)| {
                    (p[0] - c[0]).abs() < s[0] * 0.5 + BODY_RADIUS
                        && (p[1] - c[1]).abs() < s[1] * 0.5 + BODY_RADIUS
                });
        }
        p[0].abs() > 11.0 - BODY_RADIUS
            || p[1].abs() > 8.0 - BODY_RADIUS
            || (self == Self::Choke
                && p[0].abs() < 1.0 + BODY_RADIUS
                && p[1].abs() > 2.0 - BODY_RADIUS)
    }
    /// Clearance test includes a body-radius allowance around the choke walls.
    pub fn clear_segment(self, a: [f32; 2], b: [f32; 2]) -> bool {
        if self.blocked(a) || self.blocked(b) {
            return false;
        }
        for (center, size) in self.walls().iter() {
            let mut enter = 0.0_f32;
            let mut leave = 1.0_f32;
            for axis in 0..2 {
                let half = size[axis] * 0.5 + BODY_RADIUS;
                let delta = b[axis] - a[axis];
                if delta.abs() < 1e-8 {
                    if (a[axis] - center[axis]).abs() >= half {
                        enter = 2.0;
                        break;
                    }
                } else {
                    let t0 = (center[axis] - half - a[axis]) / delta;
                    let t1 = (center[axis] + half - a[axis]) / delta;
                    enter = enter.max(t0.min(t1));
                    leave = leave.min(t0.max(t1));
                }
            }
            if enter < leave - 1e-6 && leave > 0.0 && enter < 1.0 {
                return false;
            }
        }
        true
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
        if let Self::Pressure(field) = self {
            if field.generator > 0 {
                return crate::procedural::cover_anchors(field)
                    .into_iter()
                    .filter(|pocket| !self.blocked(*pocket) && self.cover(*pocket) > 0.)
                    .min_by(|a, b| distance(p, *a).total_cmp(&distance(p, *b)));
            }
            return field
                .cover_anchors()
                .iter()
                .copied()
                .filter(|pocket| self.cover(*pocket) > 0.0)
                .min_by(|a, b| distance(p, *a).total_cmp(&distance(p, *b)));
        }
        self.cover_points()
            .iter()
            .copied()
            .min_by(|a, b| distance(p, *a).total_cmp(&distance(p, *b)))
    }
}
