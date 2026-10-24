//! Authored preparation presets. They set the existing seven mind traits; no policy is trained.
use crate::Mind;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleStyle {
    Vanguard,
    Protector,
    Skirmisher,
}
impl BattleStyle {
    pub const ALL: [Self; 3] = [Self::Vanguard, Self::Protector, Self::Skirmisher];
    pub fn name(self) -> &'static str {
        match self {
            Self::Vanguard => "VANGUARD",
            Self::Protector => "PROTECTOR",
            Self::Skirmisher => "SKIRMISHER",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Vanguard => {
                "Press the attack. Brave and committed; takes more risks in the open."
            }
            Self::Protector => "Stay with allies. Patient and cooperative; gives up some pursuit.",
            Self::Skirmisher => "Value cover and survival. Careful approaches; slower to commit.",
        }
    }
    pub fn mind(self) -> Mind {
        match self {
            Self::Vanguard => Mind {
                aggression: 0.9,
                caution: 0.2,
                courage: 0.85,
                teamwork: 0.55,
                patience: 0.25,
                self_preservation: 0.25,
                discipline: 0.7,
            },
            Self::Protector => Mind {
                aggression: 0.4,
                caution: 0.7,
                courage: 0.75,
                teamwork: 0.95,
                patience: 0.75,
                self_preservation: 0.45,
                discipline: 0.85,
            },
            Self::Skirmisher => Mind {
                aggression: 0.65,
                caution: 0.8,
                courage: 0.6,
                teamwork: 0.45,
                patience: 0.8,
                self_preservation: 0.8,
                discipline: 0.65,
            },
        }
    }
    pub fn matches(self, mind: Mind) -> bool {
        let values = |m: Mind| {
            [
                m.aggression,
                m.caution,
                m.courage,
                m.teamwork,
                m.patience,
                m.self_preservation,
                m.discipline,
            ]
        };
        values(self.mind())
            .into_iter()
            .zip(values(mind))
            .all(|(a, b)| (a - b).abs() < 0.04)
    }
}
