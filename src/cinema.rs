//! Shot grammar for a three-against-three fight. It reads bodies. It does not touch the simulation.
use crate::psyche::{encircled, isolated};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shot {
    Establish,
    Approach,
    Contact,
    Mistake,
    Wound,
    Aftermath,
}

impl Shot {
    pub fn name(self) -> &'static str {
        match self {
            Self::Establish => "ESTABLISH",
            Self::Approach => "APPROACH",
            Self::Contact => "CONTACT",
            Self::Mistake => "THE MISTAKE",
            Self::Wound => "THE WOUND",
            Self::Aftermath => "AFTERMATH",
        }
    }

    fn min_hold(self) -> f32 {
        match self {
            Self::Establish => 2.4,
            Self::Approach => 2.0,
            Self::Contact => 1.6,
            Self::Mistake => 2.2,
            Self::Wound => 1.5,
            Self::Aftermath => f32::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FighterView {
    pub id: usize,
    pub team: u8,
    pub position: [f32; 2],
    pub health: f32,
    pub max_health: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Aim {
    pub look: [f32; 2],
    pub zoom: f32,
    pub pitch: f32,
    pub orbit_rate: f32,
    pub shot: Shot,
}

#[derive(Clone, Debug)]
pub struct Director {
    shot: Shot,
    held: f32,
    focus: Option<usize>,
    tick: u32,
    health: Vec<f32>,
}

impl Default for Director {
    fn default() -> Self {
        Self {
            shot: Shot::Establish,
            held: 0.0,
            focus: None,
            tick: 0,
            health: Vec::new(),
        }
    }
}

impl Director {
    pub fn shot(&self) -> Shot {
        self.shot
    }

    pub fn update(
        &mut self,
        dt: f32,
        tick: u32,
        fighters: &[FighterView],
        finished: bool,
        wide_zoom: f32,
    ) -> Aim {
        if tick < self.tick || fighters.len() != self.health.len() {
            self.shot = Shot::Establish;
            self.held = 0.0;
            self.focus = None;
            self.health = fighters.iter().map(|f| f.health).collect();
        }
        self.tick = tick;
        let drops: Vec<(usize, f32)> = fighters
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                let before = self.health.get(i).copied().unwrap_or(f.health);
                let drop = before - f.health;
                (drop > 1.0).then_some((f.id, drop))
            })
            .collect();
        self.health = fighters.iter().map(|f| f.health).collect();
        let living: Vec<&FighterView> = fighters.iter().filter(|f| f.health > 0.0).collect();
        let gap = closest_gap(&living);
        let big = drops
            .iter()
            .filter(|(id, drop)| {
                fighters
                    .iter()
                    .find(|f| f.id == *id)
                    .is_some_and(|f| *drop >= 40.0 || *drop >= f.max_health * 0.12)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1));
        let mistake = mistake_focus(&living);
        let next = if finished {
            Shot::Aftermath
        } else if big.is_some() && (self.shot != Shot::Wound || self.held >= 0.7) {
            Shot::Wound
        } else if self.held < self.shot.min_hold() {
            self.shot
        } else if mistake.is_some() && !matches!(self.shot, Shot::Mistake) {
            Shot::Mistake
        } else if gap.is_some_and(|(_, _, d)| d < 7.0) || !drops.is_empty() {
            Shot::Contact
        } else if matches!(self.shot, Shot::Establish) {
            Shot::Approach
        } else {
            self.shot
        };
        if next != self.shot {
            self.shot = next;
            self.held = 0.0;
            self.focus = match next {
                Shot::Wound => big.copied().map(|(id, _)| id),
                Shot::Mistake => mistake,
                Shot::Contact => gap.map(|(a, _, _)| a),
                _ => None,
            };
        }
        self.held += dt.max(0.0);
        self.aim(&living, wide_zoom)
    }

    fn aim(&self, living: &[&FighterView], wide_zoom: f32) -> Aim {
        let center = centroid(living).unwrap_or([0.0, 0.0]);
        let duel = self.focus.and_then(|id| {
            let actor = living.iter().find(|f| f.id == id)?;
            let opponent = living
                .iter()
                .filter(|f| f.team != actor.team)
                .min_by(|a, b| {
                    crate::distance(actor.position, a.position)
                        .total_cmp(&crate::distance(actor.position, b.position))
                })?;
            Some((
                [
                    (actor.position[0] + opponent.position[0]) * 0.5,
                    (actor.position[1] + opponent.position[1]) * 0.5,
                ],
                crate::distance(actor.position, opponent.position),
            ))
        });
        let (look, zoom, pitch, orbit_rate) = match self.shot {
            Shot::Establish => (center, wide_zoom, 0.78, 0.04),
            Shot::Approach => (center, (wide_zoom * 0.62).clamp(10.0, 28.0), 0.22, 0.1),
            Shot::Contact => (
                duel.map_or(center, |(midpoint, _)| midpoint),
                duel.map_or(9.0, |(_, gap)| (gap * 1.25 + 5.5).clamp(6.5, 12.0)),
                0.30,
                0.11,
            ),
            Shot::Mistake | Shot::Wound => (
                self.focus
                    .and_then(|id| living.iter().find(|f| f.id == id).map(|f| f.position))
                    .unwrap_or(center),
                if self.shot == Shot::Wound { 6.0 } else { 8.0 },
                0.3,
                0.03,
            ),
            Shot::Aftermath => (center, (wide_zoom * 0.85).clamp(12.0, 40.0), 0.55, 0.035),
        };
        Aim {
            look,
            zoom,
            pitch,
            orbit_rate,
            shot: self.shot,
        }
    }
}

fn centroid(living: &[&FighterView]) -> Option<[f32; 2]> {
    if living.is_empty() {
        return None;
    }
    let n = living.len() as f32;
    Some([
        living.iter().map(|f| f.position[0]).sum::<f32>() / n,
        living.iter().map(|f| f.position[1]).sum::<f32>() / n,
    ])
}

fn closest_gap(living: &[&FighterView]) -> Option<(usize, usize, f32)> {
    let mut best = None;
    for a in living {
        for b in living {
            if a.team == b.team {
                continue;
            }
            let d = crate::distance(a.position, b.position);
            if best.is_none_or(|(_, _, old)| d < old) {
                best = Some((a.id, b.id, d));
            }
        }
    }
    best
}

fn mistake_focus(living: &[&FighterView]) -> Option<usize> {
    living.iter().find_map(|f| {
        let allies: Vec<[f32; 2]> = living
            .iter()
            .filter(|o| o.team == f.team && o.id != f.id)
            .map(|o| o.position)
            .collect();
        let enemies: Vec<[f32; 2]> = living
            .iter()
            .filter(|o| o.team != f.team)
            .map(|o| o.position)
            .collect();
        let exposed = isolated(f.position, &allies) || encircled(f.position, &enemies);
        let threat = enemies
            .iter()
            .any(|e| crate::distance(f.position, *e) < 8.0);
        (exposed && threat).then_some(f.id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trio(team: u8, x: f32) -> Vec<FighterView> {
        (0..3)
            .map(|i| FighterView {
                id: team as usize * 3 + i,
                team,
                position: [x, i as f32 * 1.6 - 1.6],
                health: 300.0,
                max_health: 300.0,
            })
            .collect()
    }

    fn both(left: f32, right: f32) -> Vec<FighterView> {
        let mut f = trio(0, left);
        f.extend(trio(1, right));
        f
    }

    #[test]
    fn opening_is_a_wide_establish_and_it_does_not_cut_early() {
        let mut director = Director::default();
        let fighters = both(-12.0, 12.0);
        let aim = director.update(0.5, 0, &fighters, false, 30.0);
        assert_eq!(aim.shot, Shot::Establish);
        assert!(aim.zoom > 20.0);
        let aim = director.update(0.5, 1, &fighters, false, 30.0);
        assert_eq!(aim.shot, Shot::Establish);
    }

    #[test]
    fn a_closing_company_becomes_an_approach() {
        let mut director = Director::default();
        let fighters = both(-12.0, 12.0);
        director.update(2.5, 0, &fighters, false, 30.0);
        let aim = director.update(0.1, 1, &fighters, false, 30.0);
        assert_eq!(aim.shot, Shot::Approach);
        assert!(aim.pitch < 0.4);
    }

    #[test]
    fn a_real_hit_takes_the_wound_and_names_the_body() {
        let mut director = Director::default();
        let mut fighters = both(-4.0, 4.0);
        director.update(0.2, 0, &fighters, false, 30.0);
        fighters[1].health = 240.0;
        let aim = director.update(0.1, 1, &fighters, false, 30.0);
        assert_eq!(aim.shot, Shot::Wound);
        assert_eq!(aim.look, fighters[1].position);
        assert!(aim.zoom < 8.0);
    }

    #[test]
    fn the_ending_holds_the_aftermath() {
        let mut director = Director::default();
        let fighters = both(-2.0, 2.0);
        let aim = director.update(0.1, 40, &fighters, true, 30.0);
        assert_eq!(aim.shot, Shot::Aftermath);
        let aim = director.update(5.0, 41, &fighters, true, 30.0);
        assert_eq!(aim.shot, Shot::Aftermath);
    }

    #[test]
    fn a_restart_returns_to_the_establish() {
        let mut director = Director::default();
        let fighters = both(-2.0, 2.0);
        director.update(3.0, 80, &fighters, false, 30.0);
        let aim = director.update(0.1, 0, &fighters, false, 30.0);
        assert_eq!(aim.shot, Shot::Establish);
    }
}
