//! Fixed-step combat model, independent of rendering and wall-clock time.
use serde::{Deserialize, Serialize};
pub mod appearance;
pub mod archery;
pub mod async_exchange;
pub mod challenges;
pub mod cinema;
pub mod cognition;
pub mod collision;
pub mod equipment;
pub mod genesis;
pub mod heraldry;
mod injuries;
pub mod inventory;
pub mod learned;
pub mod mastery;
pub mod navigation;
pub mod ninjutsu;
pub mod perception;
pub mod pressure;
pub mod procedural;
pub mod progression;
pub mod psyche;
pub mod rivals;
pub mod skills;
pub mod styles;
pub mod telemetry;
pub mod terrain;
pub mod theus;
pub mod theus_combat;
pub mod tutorial;
pub mod weapons;
use learned::{Comparison, ComparisonSummary, Controller, LearnedPolicy};
use navigation::{Route, plan_known};
use perception::{COMMUNICATION_RANGE, Contact, can_see, cooperative, threats};
use std::sync::Arc;
use terrain::Arena;
use weapons::{ProjectileWeapon, Windup};

pub const ACTION_COUNT: usize = 9;

pub const DT: f32 = 0.1;
pub const ACTIONS: [&str; ACTION_COUNT] = [
    "ADVANCE",
    "ATTACK",
    "HIGH GROUND",
    "FLANK",
    "REGROUP",
    "RETREAT",
    "HOLD",
    "SEEK COVER",
    "SUPPORT ALLY",
];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mind {
    pub aggression: f32,
    pub caution: f32,
    pub courage: f32,
    pub teamwork: f32,
    pub patience: f32,
    pub self_preservation: f32,
    pub discipline: f32,
}
impl Mind {
    pub fn aggressive() -> Self {
        Self {
            aggression: 0.9,
            caution: 0.2,
            courage: 0.85,
            teamwork: 0.25,
            patience: 0.2,
            self_preservation: 0.25,
            discipline: 0.3,
        }
    }
    pub fn disciplined() -> Self {
        Self {
            aggression: 0.5,
            caution: 0.8,
            courage: 0.65,
            teamwork: 0.9,
            patience: 0.8,
            self_preservation: 0.65,
            discipline: 0.9,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Role {
    Fighter,
    RockThrower,
}
impl Role {
    pub fn range(self) -> f32 {
        if self == Self::Fighter { 1.5 } else { 6.5 }
    }
    pub fn name(self) -> &'static str {
        if self == Self::Fighter {
            "MELEE"
        } else {
            "ROCK THROWER"
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CommandTitle {
    King,
    Queen,
    General,
    Captain,
    Champion,
    Member,
}
impl CommandTitle {
    pub const ALL: [Self; 6] = [
        Self::King,
        Self::Queen,
        Self::General,
        Self::Captain,
        Self::Champion,
        Self::Member,
    ];
    pub fn description(self) -> &'static str {
        match self {
            Self::King | Self::Queen => "+10 HP, +10% damage",
            Self::General => "10% less incoming damage",
            Self::Captain => "+10% movement",
            Self::Champion => "+10% damage",
            Self::Member => "No title bonus",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::King => "KING",
            Self::Queen => "QUEEN",
            Self::General => "GENERAL",
            Self::Captain => "CAPTAIN",
            Self::Champion => "CHAMPION",
            Self::Member => "MEMBER",
        }
    }
    pub fn royal(self) -> bool {
        matches!(self, Self::King | Self::Queen)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FighterClass {
    Warrior,
    RockThrower,
    Paladin,
    Guardian,
    Ranger,
    Scout,
    Healer,
    Berserker,
    Hexer,
    BannerBearer,
    Duelist,
    Assassin,
    Monk,
    Shinobi,
    Reaper,
    Alchemist,
    Warlock,
    Acrobat,
}
impl FighterClass {
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|c| *c == self).unwrap()
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Warrior => "WARRIOR",
            Self::RockThrower => "ROCK THROWER",
            Self::Paladin => "PALADIN",
            Self::Guardian => "GUARDIAN",
            Self::Ranger => "RANGER",
            Self::Scout => "SCOUT",
            Self::Healer => "HEALER",
            Self::Berserker => "BERSERKER",
            Self::Hexer => "HEXER",
            Self::BannerBearer => "BANNER BEARER",
            Self::Duelist => "DUELIST",
            Self::Assassin => "ASSASSIN",
            Self::Monk => "MONK",
            Self::Shinobi => "SHINOBI",
            Self::Reaper => "REAPER",
            Self::Alchemist => "ALCHEMIST",
            Self::Warlock => "WARLOCK",
            Self::Acrobat => "ACROBAT",
        }
    }
    pub const ALL: [Self; 18] = [
        Self::Warrior,
        Self::Guardian,
        Self::Ranger,
        Self::Scout,
        Self::Healer,
        Self::Berserker,
        Self::Hexer,
        Self::BannerBearer,
        Self::RockThrower,
        Self::Paladin,
        Self::Duelist,
        Self::Assassin,
        Self::Monk,
        Self::Shinobi,
        Self::Reaper,
        Self::Alchemist,
        Self::Warlock,
        Self::Acrobat,
    ];
    /// Existing authored silhouettes remain usable by the new mechanical classes.
    pub fn silhouette(self) -> Self {
        match self {
            Self::Duelist => Self::Warrior,
            Self::Assassin | Self::Shinobi | Self::Acrobat => Self::Scout,
            Self::Monk => Self::Healer,
            Self::Reaper => Self::Berserker,
            Self::Alchemist => Self::Healer,
            Self::Warlock => Self::Hexer,
            _ => self,
        }
    }
    pub fn next(self) -> Self {
        Self::ALL[(Self::ALL.iter().position(|c| *c == self).unwrap() + 1) % Self::ALL.len()]
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Warrior => "Balanced frontline fighter; steady damage.",
            Self::Guardian => "35% damage resistance; slow and durable.",
            Self::Ranger => {
                "Bow, 12 arrows; finds firing lanes, keeps distance and draws a sidearm when rushed."
            }
            Self::Scout => "45% faster movement; fragile flanker.",
            Self::Healer => "Heals nearby wounded allies; can combine healing with ranged gear.",
            Self::Berserker => "Damage rises as health falls; vulnerable to hits.",
            Self::Hexer => "Ranged hex weakens nearby enemies' damage.",
            Self::BannerBearer => "Nearby allies move faster and strike harder.",
            Self::RockThrower => "Slow ranged projectile; needs protection.",
            Self::Paladin => "Blesses nearby allies with courage and protection.",
            Self::Duelist => "Active parries gain +2 defence and stronger ripostes.",
            Self::Assassin => "Exploit exposed enemies for +35% damage; fast but fragile.",
            Self::Monk => "Recover effort faster; fluid unarmed counters and greater endurance.",
            Self::Shinobi => "Three smoke bombs; faster wall recovery and smoke ambushes.",
            Self::Reaper => "Longer melee reach; increasingly dangerous to wounded enemies.",
            Self::Alchemist => "Toxic smoke drains enemy effort while covering allies.",
            Self::Warlock => "Stronger resisted bindings drain enemy effort and last longer.",
            Self::Acrobat => "Fast traversal; improved acrobatics and longer wall holds.",
        }
    }
    pub fn role(self) -> Role {
        if matches!(
            self,
            Self::RockThrower | Self::Ranger | Self::Hexer | Self::Warlock | Self::Alchemist
        ) {
            Role::RockThrower
        } else {
            Role::Fighter
        }
    }
}
pub const PALADIN_AURA_RADIUS: f32 = 4.0;
pub const BATTLE_TICK_LIMIT: u32 = 3600;
fn is_zero(v: &u32) -> bool {
    *v == 0
}
fn is_zero_byte(v: &u8) -> bool {
    *v == 0
}
fn is_false(v: &bool) -> bool {
    !*v
}
fn is_zero_f32(v: &f32) -> bool {
    *v == 0.0
}
#[derive(Clone, Debug, Serialize)]
pub struct Rock {
    pub weapon: ProjectileWeapon,
    pub shooter: usize,
    pub team: usize,
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub age: u32,
    pub flight_ticks: u32,
    pub damage: f32,
}
#[derive(Clone, Debug, Serialize)]
pub struct Unit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ninja: Option<ninjutsu::Fighter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theus: Option<theus::Actor>,
    pub skills: Option<skills::SkillLoadout>,
    pub id: usize,
    pub team: usize,
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_title: Option<CommandTitle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<FighterClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fighter_id: Option<String>,
    #[serde(skip_serializing_if = "is_zero")]
    pub experience: u32,
    #[serde(skip_serializing_if = "is_zero")]
    pub battle_count: u32,
    #[serde(skip_serializing_if = "is_false")]
    pub blessed: bool,
    #[serde(skip_serializing_if = "is_zero_byte")]
    pub lost_limbs: u8,
    #[serde(skip_serializing_if = "is_zero_byte")]
    pub prosthetics: u8,
    pub equipment: equipment::Equipment,
    pub mastery: mastery::Mastery,
    pub carries_standard: bool,
    pub heal_cooldown: f32,
    pub impact_cooldown: f32,
    pub ammunition: u8,
    pub windup: Option<Windup>,
    pub weapon_shots: [u32; 4],
    pub bow_melee: bool,
    pub last_shot_target: Option<[f32; 2]>,
    pub bow_release: f32,
    pub arrow_blocks: u32,
    pub archery_retreat_until: u32,
    pub archery_retreat_after: u32,
    pub position: [f32; 2],
    pub health: f32,
    pub mind: Mind,
    pub action: usize,
    pub scores: [f32; ACTION_COUNT],
    pub target: [f32; 2],
    pub enemy: Option<usize>,
    pub cooldown: f32,
    pub damage_dealt: f32,
    pub healing_done: f32,
    pub distance_moved: f32,
    pub decision_counts: [u32; ACTION_COUNT],
    pub route: Route,
    pub path_cursor: usize,
    pub route_goal: [f32; 2],
    pub route_mind: Mind,
    pub cover_ticks: u32,
    pub rough_ticks: u32,
    pub alive_ticks: u32,
    pub high_ground_ticks: u32,
    pub rocks_thrown: u32,
    pub rocks_hit: u32,
    pub contacts: Vec<Contact>,
    pub visible_enemies: usize,
    pub shared_reports: u32,
    pub support_target: Option<usize>,
    pub flank_anchor: Option<[f32; 2]>,
    pub flank_until: u32,
    pub decision_reason: String,
    pub comparison: Option<Comparison>,
    #[serde(skip_serializing_if = "search_not_started")]
    pub search_waypoint: u32,
    /// Persistent scars copied in for a Founders battle. Blank for the Lab.
    #[serde(skip_serializing_if = "psyche::is_blank_field")]
    pub psyche: psyche::Psyche,
    #[serde(skip_serializing_if = "is_zero")]
    pub exposure_isolation: u32,
    #[serde(skip_serializing_if = "is_zero")]
    pub exposure_encirclement: u32,
    #[serde(skip_serializing_if = "is_zero")]
    pub exposure_ally_loss: u32,
    #[serde(skip_serializing_if = "is_zero")]
    pub exposure_failed_charge: u32,
    #[serde(skip_serializing_if = "is_zero_byte")]
    pub witnessed_dead: u8,
    #[serde(skip_serializing_if = "is_false")]
    pub charging: bool,
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub charge_health: f32,
}
impl Unit {
    pub fn has_skill(&self, class: FighterClass) -> bool {
        use skills::Skill as S;
        let skill = match class {
            FighterClass::Healer => S::Heal,
            FighterClass::Paladin => S::Bless,
            FighterClass::BannerBearer => S::Rally,
            FighterClass::Hexer => S::Hex,
            FighterClass::RockThrower => S::Stonecast,
            FighterClass::Ranger => S::Archery,
            FighterClass::Guardian => S::Bulwark,
            FighterClass::Scout => S::Fleetfoot,
            FighterClass::Berserker => S::Bloodrage,
            FighterClass::Warrior => S::WeaponDrill,
            FighterClass::Duelist => S::Fencing,
            FighterClass::Assassin => S::Ambush,
            FighterClass::Monk => S::Flow,
            FighterClass::Shinobi => S::Veilcraft,
            FighterClass::Reaper => S::Reaping,
            FighterClass::Alchemist => S::ToxicSmoke,
            FighterClass::Warlock => S::Occult,
            FighterClass::Acrobat => S::Springheel,
        };
        self.equipped_skill(skill)
    }
    pub fn equipped_skill(&self, skill: skills::Skill) -> bool {
        self.skills
            .unwrap_or_else(|| {
                skills::SkillLoadout::defaults(
                    self.class.unwrap_or(FighterClass::Warrior),
                    self.mastery,
                )
            })
            .has(skill)
    }
    pub fn trained_role(&self) -> Role {
        if self.equipment.projectile_weapon().is_some()
            || self.class.is_some_and(|_| {
                [
                    FighterClass::RockThrower,
                    FighterClass::Ranger,
                    FighterClass::Hexer,
                    FighterClass::Warlock,
                    FighterClass::Alchemist,
                ]
                .into_iter()
                .any(|c| self.has_skill(c))
            })
            || [
                FighterClass::RockThrower,
                FighterClass::Ranger,
                FighterClass::Hexer,
                FighterClass::Warlock,
                FighterClass::Alchemist,
            ]
            .into_iter()
            .any(|c| self.has_skill(c))
        {
            Role::RockThrower
        } else {
            Role::Fighter
        }
    }
    pub fn projectile_weapon(&self) -> Option<ProjectileWeapon> {
        if self.equipment.has(equipment::Gear::Kusarigama)
            || self.equipment.has(equipment::Gear::RazorWheel)
        {
            return None;
        }
        self.equipment.projectile_weapon().or_else(|| {
            (self.role == Role::RockThrower || self.trained_role() == Role::RockThrower).then_some(
                if self.has_skill(FighterClass::Ranger) {
                    ProjectileWeapon::Bow
                } else {
                    ProjectileWeapon::Rock
                },
            )
        })
    }
    pub fn bow_available(&self) -> bool {
        self.projectile_weapon() == Some(ProjectileWeapon::Bow)
            && self.ammunition > 0
            && self.lost_limbs & !self.prosthetics & 3 == 0
    }
    pub fn bow_follow_through(&self) -> bool {
        self.projectile_weapon() == Some(ProjectileWeapon::Bow)
            && self.bow_release > 0.0
            && self.lost_limbs & !self.prosthetics & 3 == 0
    }
    pub fn bow_aim(&self) -> [f32; 2] {
        if self.windup.is_some() {
            self.target
        } else if self.bow_follow_through() {
            self.last_shot_target.unwrap_or(self.target)
        } else {
            self.target
        }
    }
    pub fn is_ranged(&self) -> bool {
        self.projectile_weapon().is_some()
            && self.ammunition > 0
            && !self.bow_melee
            && (self.projectile_weapon() != Some(ProjectileWeapon::Bow) || self.bow_available())
    }
    pub fn attack_range(&self) -> f32 {
        if self.equipment.has(equipment::Gear::Kusarigama) {
            return 3.3;
        }
        if self.equipment.has(equipment::Gear::RazorWheel) {
            return 5.5;
        }
        self.projectile_weapon()
            .filter(|_| self.is_ranged())
            .map(ProjectileWeapon::range)
            .unwrap_or(if self.has_skill(FighterClass::Reaper) {
                2.2
            } else {
                1.5
            })
    }
    pub fn reset_weapon(&mut self) {
        self.role = self.trained_role();
        self.ammunition = self
            .projectile_weapon()
            .map(|w| w.ammunition())
            .unwrap_or(0);
        self.windup = None;
        self.bow_melee = false;
        self.bow_release = 0.0;
        self.last_shot_target = None;
    }
    pub fn weapon_status(&self) -> String {
        if let Some(gear) = self.equipment.slots[0]
            .filter(|g| matches!(g, equipment::Gear::Kusarigama | equipment::Gear::RazorWheel))
        {
            return format!("{} | {}", gear.name(), gear.effect());
        }
        match self.projectile_weapon() {
            None => "Melee weapon".into(),
            Some(w) => format!(
                "{} | {} | {}",
                w.name(),
                if w == ProjectileWeapon::Rock {
                    "unlimited".into()
                } else {
                    format!("{}/{} ammunition", self.ammunition, w.ammunition())
                },
                if w == ProjectileWeapon::Bow && self.lost_limbs & !self.prosthetics & 3 != 0 {
                    "Arm wound: sidearm until a prosthetic is fitted".into()
                } else if self.ammunition == 0 || self.bow_melee {
                    if self.bow_melee {
                        "sidearm; rushed at close range".into()
                    } else {
                        "sidearm; closing to melee".into()
                    }
                } else if let Some(draw) = &self.windup {
                    format!(
                        "{} {:.1}s",
                        if draw.weapon == ProjectileWeapon::Bow {
                            "drawing"
                        } else {
                            "winding up"
                        },
                        draw.remaining
                    )
                } else {
                    format!("{:.1} range", self.attack_range())
                }
            ),
        }
    }
    pub fn displays_standard(&self) -> bool {
        self.carries_standard
            || self.equipment.has(equipment::Gear::WarBanner)
            || self.class == Some(FighterClass::BannerBearer)
    }
    pub fn banner_active(&self) -> bool {
        self.has_skill(FighterClass::BannerBearer) && self.displays_standard()
    }
    pub fn healing_power(&self) -> f32 {
        12.0 * (1.0 + self.mastery.rank(FighterClass::Healer) as f32 * 0.25)
    }

    pub fn level(&self) -> u32 {
        1 + (self.experience / 100).min(9)
    }
    pub fn class_name(&self) -> &'static str {
        self.class
            .map(FighterClass::name)
            .unwrap_or(self.role.name())
    }
    pub fn incoming_multiplier(&self) -> f32 {
        self.equipment.incoming()
            * (if self.blessed { 0.90 } else { 1.0 })
            * if self.command_title == Some(CommandTitle::General) {
                0.90
            } else {
                1.0
            }
            * if self.has_skill(FighterClass::Guardian) {
                0.65
            } else {
                1.0
            }
            * if self.has_skill(FighterClass::Berserker) {
                1.15
            } else {
                1.0
            }
    }
    pub fn max_health(&self) -> f32 {
        3.0 * ((if self.command_title.is_some_and(CommandTitle::royal) {
            110.0
        } else {
            100.0
        }) + self.equipment.health()
            + (self.level() - 1) as f32 * 2.0
            + if self.has_skill(FighterClass::Guardian) {
                30.0
            } else {
                0.0
            }
            - if self.has_skill(FighterClass::Scout) {
                15.0
            } else {
                0.0
            }
            + if self.has_skill(FighterClass::Monk) {
                10.0
            } else if self.has_skill(FighterClass::Assassin) {
                -10.0
            } else {
                0.0
            })
    }
    pub fn health_fraction(&self) -> f32 {
        self.health / self.max_health()
    }
    pub fn movement_multiplier(&self) -> f32 {
        self.equipment.speed()
            * if self.has_skill(FighterClass::Assassin) {
                1.15
            } else if self.has_skill(FighterClass::Acrobat) {
                1.20
            } else {
                1.0
            }
            * if self.command_title == Some(CommandTitle::Captain) {
                1.10
            } else {
                1.0
            }
            * if self.has_skill(FighterClass::Scout) {
                1.45
            } else {
                1.0
            }
            * if self.has_skill(FighterClass::Guardian) {
                0.8
            } else {
                1.0
            }
            * 0.65_f32.powi(((self.lost_limbs & !self.prosthetics) & 12).count_ones() as i32)
            * if self.prosthetics & 3 != 0 { 0.85 } else { 1.0 }
            * self.theus.as_ref().map_or(1., theus::Actor::movement)
            * if self
                .ninja
                .as_ref()
                .is_some_and(|n| n.bound_until > self.theus.as_ref().map_or(0, |a| a.clock))
            {
                0.2
            } else {
                1.0
            }
    }
    pub fn damage_multiplier(&self) -> f32 {
        self.equipment.damage()
            * if self.has_skill(FighterClass::Reaper) {
                1.12
            } else {
                1.0
            }
            * if self.has_skill(FighterClass::Berserker) {
                1.15 + (1.0 - self.health_fraction()).clamp(0.0, 1.0) * 0.85
            } else {
                1.0
            }
            * if self.class == Some(FighterClass::Healer) {
                0.65
            } else {
                1.0
            }
            * if self.has_skill(FighterClass::Guardian) {
                0.8
            } else {
                1.0
            }
            * if self.mastery.retained(FighterClass::Warrior)
                && self.equipped_skill(skills::Skill::WeaponDrill)
            {
                1.05
            } else {
                1.0
            }
            * if self.command_title == Some(CommandTitle::Champion) {
                1.10
            } else {
                1.0
            }
            * (if self.command_title.is_some_and(CommandTitle::royal) {
                1.10
            } else {
                1.0
            })
            * (1.0 + (self.level() - 1) as f32 * 0.01)
            * (if self.blessed { 1.05 } else { 1.0 })
            * 0.80_f32.powi(((self.lost_limbs & !self.prosthetics) & 3).count_ones() as i32)
            * if self.prosthetics & 3 != 0 { 3.0 } else { 1.0 }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct DamageEvent {
    pub sequence: u64,
    pub tick: u32,
    pub attacker: usize,
    pub defender: usize,
    pub amount: f32,
    pub velocity: [f32; 2],
    pub position: [f32; 2],
    pub severed: u8,
    pub blunt: bool,
}
/// A successful restoration after simultaneous damage, including its actual capped amount.
#[derive(Clone, Debug, Serialize)]
pub struct HealEvent {
    pub tick: u32,
    pub healer: usize,
    pub target: usize,
    pub amount: f32,
}
#[derive(Clone, Debug, Serialize)]
pub struct Battle {
    #[serde(skip_serializing_if = "ninjutsu::Field::empty")]
    pub ninjutsu: ninjutsu::Field,
    #[serde(skip)]
    pub motion: collision::Motion,
    pub seed: u64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub founders: bool,
    pub tick: u32,
    pub units: Vec<Unit>,
    pub finished: bool,
    pub result: String,
    pub swapped: bool,
    pub arena: Arena,
    pub rocks: Vec<Rock>,
    pub heals: Vec<HealEvent>,
    pub damage_events: Vec<DamageEvent>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub theus_events: Vec<theus::Event>,
    pub next_damage_event: u64,
    pub initial_arena: Arena,
    pub barricade_health: [f32; 4],
    pub schema_version: u32,
    pub controller: Controller,
    pub learned_team: Option<usize>,
    #[serde(skip)]
    pub policy: Option<Arc<LearnedPolicy>>,
    pub disagreements: Vec<Comparison>,
    pub comparison_summary: ComparisonSummary,
    pub learned_fallbacks: u32,
    pub simulation_version: String,
    /// Why a Founders pressure field was chosen. Empty for authored arenas.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub pressure_note: String,
    pub ally_distance_sum: [f64; 2],
    pub ally_distance_samples: [u32; 2],
}
pub fn elevation(p: [f32; 2]) -> f32 {
    let x = (1.0 - p[0].abs() / 3.5).max(0.0);
    let z = (1.0 - p[1].abs() / 7.0).max(0.0);
    x * z * 1.7
}
pub fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}
// Stateless noise isolates decision variation from the number of previous random draws.
fn noise(seed: u64, tick: u32, id: usize, action: usize) -> f32 {
    let mut n = seed
        ^ (tick as u64).wrapping_mul(0x9e3779b97f4a7c15)
        ^ ((id * ACTION_COUNT + action) as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((n ^ (n >> 31)) as u32 as f32 / u32::MAX as f32 - 0.5) * 0.035
}
fn search_not_started(leg: &u32) -> bool {
    *leg == 0
}
impl Battle {
    pub fn new(seed: u64, swapped: bool) -> Self {
        Self::with_arena(seed, swapped, Arena::Ridge)
    }
    pub fn with_arena(seed: u64, swapped: bool, arena: Arena) -> Self {
        Self::with_setup(seed, swapped, arena, false)
    }
    pub fn with_founders(seed: u64, swapped: bool, arena: Arena) -> Self {
        Self::with_setup(seed, swapped, arena, true)
    }
    pub fn winner(&self) -> Option<usize> {
        match self.result.as_str() {
            "Red wins" => Some(0),
            "Blue wins" => Some(1),
            _ => None,
        }
    }
    pub fn team_size(&self) -> usize {
        if self.founders {
            self.units.len() / 2
        } else {
            self.arena.team_size()
        }
    }
    pub fn unit_label(&self, id: usize) -> String {
        let u = &self.units[id];
        let team = if u.team == 0 { "RED" } else { "BLUE" };
        if let Some(title) = u.command_title {
            format!("{team} {}", title.name())
        } else {
            format!("{team}-{:02}", id % self.team_size() + 1)
        }
    }
    pub fn with_setup(seed: u64, swapped: bool, arena: Arena, founders: bool) -> Self {
        Self::with_roster(seed, swapped, arena, founders, 3)
    }
    /// Validated local league sizes; the Lab keeps its authored arena roster.
    pub fn with_roster(
        seed: u64,
        swapped: bool,
        arena: Arena,
        founders: bool,
        size: usize,
    ) -> Self {
        assert!((3..=4).contains(&size));
        let team_size = if founders { size } else { arena.team_size() };
        let units = (0..2 * team_size)
            .map(|id| {
                let team = id / team_size;
                let aggressive = (team == 0) != swapped;
                let position = if founders {
                    [arena.spawn(team, 0)[0], (id % team_size) as f32 * 1.6 - 1.6]
                } else {
                    arena.spawn(team, id % team_size)
                };
                Unit {
                    ninja: None,
                    theus: None,
                    id,
                    team,
                    command_title: if founders {
                        Some(
                            [
                                CommandTitle::King,
                                CommandTitle::Queen,
                                CommandTitle::General,
                            ]
                            .get(id % team_size)
                            .copied()
                            .unwrap_or(CommandTitle::Member),
                        )
                    } else {
                        None
                    },
                    class: if founders {
                        Some(
                            [
                                FighterClass::Warrior,
                                FighterClass::RockThrower,
                                FighterClass::Paladin,
                            ]
                            .get(id % team_size)
                            .copied()
                            .unwrap_or(FighterClass::Warrior),
                        )
                    } else {
                        None
                    },
                    fighter_id: None,
                    skills: None,
                    experience: 0,
                    battle_count: 0,
                    blessed: false,
                    lost_limbs: 0,
                    prosthetics: 0,
                    equipment: equipment::Equipment::default(),
                    mastery: mastery::Mastery::default(),
                    carries_standard: false,
                    heal_cooldown: 0.0,
                    impact_cooldown: 0.0,
                    ammunition: u8::MAX,
                    windup: None,
                    weapon_shots: [0; 4],
                    bow_melee: false,
                    last_shot_target: None,
                    bow_release: 0.0,
                    arrow_blocks: 0,
                    archery_retreat_until: 0,
                    archery_retreat_after: 0,
                    role: if (founders && id % team_size == 1)
                        || (!founders && id % team_size >= arena.fighters())
                    {
                        Role::RockThrower
                    } else {
                        Role::Fighter
                    },
                    position,
                    health: if founders && id % team_size < 2 {
                        330.0
                    } else {
                        300.0
                    },
                    mind: if aggressive {
                        Mind::aggressive()
                    } else {
                        Mind::disciplined()
                    },
                    action: 6,
                    scores: [0.0; ACTION_COUNT],
                    target: position,
                    enemy: None,
                    cooldown: 0.0,
                    damage_dealt: 0.0,
                    healing_done: 0.0,
                    distance_moved: 0.0,
                    decision_counts: [0; ACTION_COUNT],
                    route: Route::default(),
                    path_cursor: 0,
                    route_goal: position,
                    route_mind: Mind::aggressive(),
                    cover_ticks: 0,
                    rough_ticks: 0,
                    alive_ticks: 0,
                    high_ground_ticks: 0,
                    rocks_thrown: 0,
                    rocks_hit: 0,
                    contacts: Vec::new(),
                    visible_enemies: 0,
                    shared_reports: 0,
                    support_target: None,
                    flank_anchor: None,
                    flank_until: 0,
                    decision_reason: "Awaiting first decision".into(),
                    comparison: None,
                    search_waypoint: 0,
                    psyche: psyche::Psyche::default(),
                    exposure_isolation: 0,
                    exposure_encirclement: 0,
                    exposure_ally_loss: 0,
                    exposure_failed_charge: 0,
                    witnessed_dead: 0,
                    charging: false,
                    charge_health: 0.0,
                }
            })
            .collect();
        Self {
            seed,
            founders,
            tick: 0,
            units,
            finished: false,
            result: "Ready".into(),
            swapped,
            arena,
            rocks: Vec::new(),
            heals: Vec::new(),
            damage_events: Vec::new(),
            theus_events: Vec::new(),
            ninjutsu: Default::default(),
            next_damage_event: 0,
            initial_arena: arena,
            barricade_health: if matches!(arena,Arena::Pressure(f) if f.layout>0) {
                [45.0; 4]
            } else {
                [0.0; 4]
            },
            motion: collision::Motion::default(),
            schema_version: 2,
            controller: Controller::Utility,
            learned_team: None,
            policy: Some(LearnedPolicy::bundled()),
            disagreements: Vec::new(),
            comparison_summary: ComparisonSummary::default(),
            learned_fallbacks: 0,
            simulation_version: env!("CARGO_PKG_VERSION").into(),
            pressure_note: String::new(),
            ally_distance_sum: [0.0; 2],
            ally_distance_samples: [0; 2],
        }
    }
    /// Living agents whose latest decision has this exact, valid disagreement pair.
    pub fn disagreement_units(&self, utility: usize, learned: usize) -> Vec<usize> {
        self.units
            .iter()
            .filter(|u| {
                u.health > 0.0
                    && utility != learned
                    && u.comparison
                        .as_ref()
                        .is_some_and(|c| c.utility == utility && c.learned == Some(learned))
            })
            .map(|u| u.id)
            .collect()
    }
    pub fn survivors(&self, team: usize) -> usize {
        self.units
            .iter()
            .filter(|u| u.team == team && u.health > 0.0)
            .count()
    }
    pub fn step(&mut self) {
        self.advance(None);
    }
    pub fn step_recorded(&mut self) -> Vec<telemetry::DecisionRecord> {
        let mut records = Vec::new();
        self.advance(Some(&mut records));
        records
    }
    fn advance(&mut self, mut records: Option<&mut Vec<telemetry::DecisionRecord>>) {
        if self.finished {
            return;
        }
        theus_combat::recover(self);
        ninjutsu::prepare(self);
        let aura: Vec<_> = self
            .units
            .iter()
            .map(|unit| {
                unit.health > 0.0
                    && self.units.iter().any(|p| {
                        p.id != unit.id
                            && p.team == unit.team
                            && p.health > 0.0
                            && p.has_skill(FighterClass::Paladin)
                            && distance(p.position, unit.position) <= PALADIN_AURA_RADIUS
                            && self.arena.clear_segment(p.position, unit.position)
                    })
            })
            .collect();
        for (unit, blessed) in self.units.iter_mut().zip(aura) {
            unit.blessed = blessed;
        }
        perception::update_in_field(&mut self.units, self.arena, self.tick, &self.ninjutsu);
        let snapshot = self.units.clone();
        let mut damage = vec![0.0; self.units.len()];
        let mut impacts = Vec::new();
        let mut barrier_damage = [0.0_f32; 4];
        let mut healing = vec![0.0_f32; self.units.len()];
        let mut casts = Vec::new();
        let mut healing_credit = vec![0.0; self.units.len()];
        let arena = self.arena;
        let team_size = self.team_size();
        let mut landed = Vec::new();
        for rock in &mut self.rocks {
            rock.age += 1;
            if rock.weapon == ProjectileWeapon::Rock && rock.age >= rock.flight_ticks {
                for target in &snapshot {
                    if target.team != rock.team
                        && target.health > 0.0
                        && ninjutsu::physical_height(target, arena, self.tick as f32) < 1.4
                        && distance(target.position, rock.to) < 0.75
                    {
                        let dealt = rock.damage
                            * target.incoming_multiplier()
                            * (1.0 - 0.3 * arena.cover(target.position));
                        damage[target.id] += dealt;
                        impacts.push((rock.shooter, target.id, dealt, true, 10.0, None));
                        landed.push((rock.shooter, dealt));
                    }
                }
            }
        }
        self.rocks
            .retain(|r| r.weapon != ProjectileWeapon::Rock || r.age < r.flight_ticks);
        for (id, dealt) in landed {
            self.units[id].damage_dealt += dealt;
            self.units[id].rocks_hit += 1;
        }
        for unit in &mut self.units {
            if unit.health <= 0.0 {
                continue;
            }
            unit.alive_ticks += 1;
            unit.high_ground_ticks += u32::from(arena.elevation(unit.position) > 0.8);
            unit.cover_ticks += u32::from(arena.cover(unit.position) > 0.0);
            unit.rough_ticks += u32::from(arena.roughness(unit.position) > 0.0);
            unit.cooldown = (unit.cooldown - DT).max(0.0);
            unit.bow_release = (unit.bow_release - DT).max(0.0);
            unit.impact_cooldown = (unit.impact_cooldown - DT).max(0.0);
            unit.heal_cooldown = (unit.heal_cooldown - DT).max(0.0);
            if ninjutsu::in_transit(unit) {
                continue;
            }
            let allies: Vec<_> = snapshot
                .iter()
                .filter(|a| {
                    a.team == unit.team
                        && a.id != unit.id
                        && a.health > 0.0
                        && distance(unit.position, a.position) <= COMMUNICATION_RANGE
                        && (can_see(arena, unit.position, a.position)
                            || (cooperative(unit.mind) && cooperative(a.mind)))
                })
                .collect();
            let mut enemy = cognition::focus_target(unit, &allies, self.tick);
            if let Some(priority) = theus_combat::priority_contact(unit, &snapshot, self.tick) {
                enemy = Some(priority);
                unit.decision_reason = "Theus: seize an ally's opening".into();
            }
            theus_combat::mobility(unit, enemy.as_ref(), arena, self.tick);
            let bow = unit.bow_available();
            if bow {
                let nearest = unit
                    .contacts
                    .iter()
                    .filter(|c| c.visible)
                    .min_by(|a, b| {
                        distance(unit.position, a.position)
                            .total_cmp(&distance(unit.position, b.position))
                    })
                    .cloned();
                let close = nearest
                    .as_ref()
                    .map(|c| distance(unit.position, c.position))
                    .unwrap_or(f32::INFINITY);
                unit.bow_melee = if unit.bow_melee {
                    close < archery::REOPEN_RANGE
                } else {
                    close < archery::MIN_RANGE
                };
                if close < 4.0 && self.tick >= unit.archery_retreat_after {
                    unit.archery_retreat_until = self.tick + 12;
                    unit.archery_retreat_after = self.tick + 36;
                }
                if unit.bow_melee {
                    enemy = nearest;
                    unit.windup = None;
                }
            } else {
                unit.bow_melee = false;
            }
            let bow_plan = if bow {
                enemy
                    .as_ref()
                    .and_then(|e| archery::plan(arena, unit, &allies, e, self.tick))
            } else {
                None
            };

            // Enemy deployment zones are authored objectives, not live enemy positions.
            let search_goal = |leg: u32| {
                if self.founders && leg.is_multiple_of(5) {
                    [
                        arena.spawn(1 - unit.team, 0)[0],
                        (unit.id % team_size) as f32 * 1.6 - 1.6,
                    ]
                } else {
                    arena.search_waypoint(unit.team, unit.id % team_size, leg)
                }
            };
            let mut objective = search_goal(unit.search_waypoint);
            // Reaching an empty deployment zone starts a sweep on every map,
            // including generated fields. Never consult unseen enemy positions.
            if (self.founders || arena.large())
                && enemy.is_none()
                && distance(unit.position, objective) < 1.8
            {
                unit.search_waypoint += 1;
                objective = search_goal(unit.search_waypoint);
            }
            let enemy_position = enemy.as_ref().map(|e| e.position).unwrap_or(objective);
            let d = enemy
                .as_ref()
                .map(|e| distance(unit.position, e.position))
                .unwrap_or(f32::INFINITY);
            let shot_clear = if bow && !unit.bow_melee {
                d >= archery::MIN_RANGE
                    && archery::clear_shot(
                        arena,
                        archery::muzzle(unit.position, enemy_position),
                        enemy_position,
                        &allies,
                    )
            } else {
                arena.clear_segment(unit.position, enemy_position)
            };
            let known_threats = threats(unit, self.tick);
            let wounded = if unit.has_skill(FighterClass::Healer) {
                allies
                    .iter()
                    .copied()
                    .filter(|a| a.health < a.max_health() - 0.01)
                    .min_by(|a, b| a.health_fraction().total_cmp(&b.health_fraction()))
            } else {
                None
            };
            let support = wounded
                .map(|ally| (ally, 2.0 * (1.0 - ally.health_fraction())))
                .or_else(|| cognition::support_ally(unit, &allies, self.tick));
            let center = if allies.is_empty() {
                unit.position
            } else {
                [
                    allies.iter().map(|a| a.position[0]).sum::<f32>() / allies.len() as f32,
                    allies.iter().map(|a| a.position[1]).sum::<f32>() / allies.len() as f32,
                ]
            };
            let isolation = (distance(unit.position, center) / 5.0).min(1.0);
            let injury = 1.0 - unit.health_fraction();
            let local_threat = (unit
                .contacts
                .iter()
                .filter(|c| distance(unit.position, c.position) < 3.5)
                .map(|c| c.confidence(self.tick))
                .sum::<f32>()
                / 3.0)
                .min(1.0);
            // Founders remember who was cut off, surrounded, or saw an ally fall.
            // The Lab does not record these, so its serialized battles stay unchanged.
            if self.founders {
                let mates: Vec<[f32; 2]> = snapshot
                    .iter()
                    .filter(|ally| {
                        ally.team == unit.team && ally.id != unit.id && ally.health > 0.0
                    })
                    .map(|ally| ally.position)
                    .collect();
                let foes: Vec<[f32; 2]> = snapshot
                    .iter()
                    .filter(|enemy| enemy.team != unit.team && enemy.health > 0.0)
                    .map(|enemy| enemy.position)
                    .collect();
                if psyche::isolated(unit.position, &mates) {
                    unit.exposure_isolation = unit.exposure_isolation.saturating_add(1);
                }
                if psyche::encircled(unit.position, &foes) {
                    unit.exposure_encirclement = unit.exposure_encirclement.saturating_add(1);
                }
                for ally in snapshot.iter().filter(|ally| {
                    ally.team == unit.team
                        && ally.id != unit.id
                        && ally.health <= 0.0
                        && distance(unit.position, ally.position) <= 8.0
                }) {
                    let bit = 1_u8 << (ally.id % team_size);
                    if unit.witnessed_dead & bit == 0 {
                        unit.witnessed_dead |= bit;
                        unit.exposure_ally_loss = unit.exposure_ally_loss.saturating_add(1);
                    }
                }
            }
            let ridge = if arena.large() {
                arena.high_ground(unit.position)
            } else {
                [
                    if unit.team == 0 { -0.6 } else { 0.6 },
                    unit.position[1].clamp(-2.0, 2.0),
                ]
            };
            let mut decision_features = Vec::new();
            let old_flank_until = unit.flank_until;
            let old_flank_anchor = unit.flank_anchor;
            if self.tick.is_multiple_of(5) {
                let mut m = unit.mind;
                if unit.blessed {
                    m.courage = (m.courage + 0.10).min(1.0);
                    m.discipline = (m.discipline + 0.10).min(1.0);
                }
                let ranged = unit.is_ranged();
                let in_range = enemy.as_ref().is_some_and(|e| e.visible)
                    && d < unit.attack_range()
                    && shot_clear;
                let arrived = distance(unit.position, ridge) < 0.9;
                let previous_action = unit.action;
                unit.scores = [
                    0.25 + 0.65 * m.aggression + 0.2 * m.courage - 0.25 * m.caution * local_threat,
                    if in_range {
                        0.85 + 0.4 * m.aggression + 0.2 * m.courage
                            - 0.15 * injury * m.self_preservation
                    } else {
                        -1.0
                    },
                    if !arena.has_high_ground()
                        || arrived
                        || in_range
                        || (arena.large() && enemy.is_none() && self.tick >= 180)
                    {
                        -1.0
                    } else {
                        0.25 + 0.35 * m.caution + 0.25 * m.discipline + 0.2 * m.patience
                            - 0.015 * distance(unit.position, ridge)
                    },
                    if let Some(e) = &enemy {
                        let engaged = allies
                            .iter()
                            .filter(|a| {
                                a.enemy == Some(e.enemy)
                                    && a.action == 1
                                    && (can_see(arena, unit.position, a.position)
                                        || (cooperative(unit.mind) && cooperative(a.mind)))
                            })
                            .count() as f32;
                        let ongoing = unit.action == 3
                            && unit
                                .flank_anchor
                                .is_some_and(|p| distance(unit.position, p) > 0.8);
                        if e.visible
                            && !in_range
                            && d > 2.0
                            && d < 9.0
                            && engaged > 0.0
                            && (ongoing || self.tick >= unit.flank_until)
                        {
                            0.25 + 0.2 * m.aggression
                                + 0.35 * m.discipline
                                + 0.3 * m.teamwork
                                + 0.2 * engaged.min(2.0)
                                + if ongoing { 0.1 } else { 0.0 }
                        } else {
                            -1.0
                        }
                    } else {
                        -1.0
                    },
                    if isolation > 0.4 {
                        0.1 + 0.75 * m.teamwork * isolation + 0.2 * m.discipline
                            - 0.15 * m.aggression
                    } else {
                        -1.0
                    },
                    if injury > 0.55 || (ranged && d < 2.7) {
                        (if ranged && d < 2.7 { 1.1 } else { 0.15 })
                            + injury * (0.65 * m.self_preservation + 0.55 * m.caution)
                            + 0.25 * local_threat
                            - 0.2 * m.courage
                    } else {
                        -1.0
                    },
                    if arena.has_high_ground() && arrived && d > 3.0 && self.tick < 180 {
                        0.35 + 0.35 * m.patience + 0.2 * m.discipline
                    } else {
                        -1.0
                    },
                    if arena.cover(unit.position) < 0.5
                        && !in_range
                        && arena.nearest_cover(unit.position).is_some()
                    {
                        0.1 + 0.4 * m.caution
                            + 0.3 * m.self_preservation
                            + 0.2 * injury
                            + 0.15 * local_threat
                    } else {
                        -1.0
                    },
                    if let Some((ally, urgency)) = support {
                        0.15 + 0.65 * m.teamwork + 0.25 * m.discipline + 0.45 * urgency
                            - 0.025 * distance(unit.position, ally.position)
                    } else {
                        -1.0
                    },
                ];
                if self.founders {
                    // Discipline goals modify utility proposals while preserving validation and controller logging.
                    if let Some(ally) = wounded {
                        unit.scores[8] = if unit.class != Some(FighterClass::Healer)
                            && distance(unit.position, ally.position) < 4.0
                            && arena.clear_segment(unit.position, ally.position)
                        {
                            0.3
                        } else {
                            1.8 + (1.0 - ally.health_fraction())
                        };
                        if distance(unit.position, ally.position) < 4.0
                            && arena.clear_segment(unit.position, ally.position)
                        {
                            unit.scores[6] = if unit.has_skill(FighterClass::Healer)
                                && unit.class == Some(FighterClass::Healer)
                            {
                                2.1
                            } else {
                                0.5
                            };
                        }
                        if enemy.as_ref().is_some_and(|e| e.visible) && d < 2.7 {
                            unit.scores[5] = 3.5;
                        }
                    }
                    match unit.class {
                        Some(FighterClass::Scout) if unit.scores[3] >= 0.0 => {
                            unit.scores[3] += 0.55
                        }
                        Some(FighterClass::Guardian | FighterClass::Paladin)
                            if unit.scores[8] >= 0.0 =>
                        {
                            unit.scores[8] += 0.35
                        }
                        Some(FighterClass::Berserker) if unit.scores[1] >= 0.0 => {
                            unit.scores[1] += 0.35
                        }
                        Some(FighterClass::BannerBearer) if unit.scores[4] >= 0.0 => {
                            unit.scores[4] += 0.45
                        }
                        _ => {}
                    }
                }
                let scar = if self.founders {
                    if unit.charging && unit.charge_health - unit.health >= 8.0 {
                        unit.exposure_failed_charge = unit.exposure_failed_charge.saturating_add(1);
                        unit.charging = false;
                    }
                    let mates: Vec<[f32; 2]> = snapshot
                        .iter()
                        .filter(|ally| {
                            ally.team == unit.team && ally.id != unit.id && ally.health > 0.0
                        })
                        .map(|ally| ally.position)
                        .collect();
                    let foes: Vec<[f32; 2]> = snapshot
                        .iter()
                        .filter(|enemy| enemy.team != unit.team && enemy.health > 0.0)
                        .map(|enemy| enemy.position)
                        .collect();
                    let ally_hurt = snapshot.iter().any(|ally| {
                        ally.team == unit.team
                            && ally.id != unit.id
                            && distance(unit.position, ally.position) <= 8.0
                            && (ally.health <= 0.0 || ally.health_fraction() < 0.6)
                    });
                    psyche::bias_scores(
                        unit.psyche,
                        psyche::Moment {
                            isolated: psyche::isolated(unit.position, &mates),
                            encircled: psyche::encircled(unit.position, &foes),
                            ally_hurt,
                            closing: enemy.is_some() && !in_range,
                        },
                        &mut unit.scores,
                    )
                } else {
                    None
                };
                if self.founders {
                    // Scars color a decision, but cannot lock a healthy veteran
                    // into HOLD or endless flight. A flinch lasts under a second
                    // out of each eight-second recovery interval, staggered by ID.
                    let flinch = (self.tick + unit.id as u32 * 11) % 80 < 8;
                    let ridge_hold =
                        arena.has_high_ground() && arrived && d > 3.0 && self.tick < 180;
                    if !ridge_hold && (!flinch || in_range) {
                        unit.scores[6] = -1.0;
                    }
                    if (injury < 0.55
                        || enemy.is_none()
                        || (self.tick + unit.id as u32 * 11) % 80 >= 24)
                        && !(ranged && d < 2.7)
                        && !flinch
                    {
                        unit.scores[5] = -1.0;
                    }
                    // The attack objective survives fear penalties. Cautious
                    // personalities still prefer safer routes to that objective.
                    let resolve = if self.tick >= 80 || enemy.is_some() {
                        0.30
                    } else {
                        0.10
                    };
                    unit.scores[0] = unit.scores[0].max(0.25) + resolve + 0.15 * m.courage;
                    if in_range {
                        unit.scores[1] = unit.scores[1].max(0.85) + 0.45 + 0.20 * m.courage;
                    }
                    if enemy.is_some() {
                        for action in [4, 8] {
                            if unit.scores[action] >= 0.0
                                && unit.class != Some(FighterClass::Healer)
                            {
                                unit.scores[action] *= 0.65;
                            }
                        }
                    }
                    if self.tick >= 120 && unit.scores[7] >= 0.0 {
                        unit.scores[7] *= 0.5;
                    }
                    if matches!(previous_action, 0 | 3) && unit.scores[previous_action] >= 0.0 {
                        unit.scores[previous_action] += 0.12;
                    }
                }
                for (i, score) in unit.scores.iter_mut().enumerate() {
                    if *score >= 0.0 {
                        *score += noise(self.seed, self.tick, unit.id, i);
                    }
                }
                let m = unit.mind;
                let engaged = enemy
                    .as_ref()
                    .map(|e| {
                        allies
                            .iter()
                            .filter(|a| a.enemy == Some(e.enemy) && a.action == 1)
                            .count()
                    })
                    .unwrap_or(0);
                decision_features = vec![
                    m.aggression,
                    m.caution,
                    m.courage,
                    m.teamwork,
                    m.patience,
                    m.self_preservation,
                    m.discipline,
                    unit.health_fraction(),
                    f32::from(unit.is_ranged()),
                    unit.position[0] / arena.half_size()[0],
                    unit.position[1] / arena.half_size()[1],
                    arena.elevation(unit.position) / if arena.large() { 3.8 } else { 1.7 },
                    arena.cover(unit.position),
                    arena.roughness(unit.position),
                    (d / 20.0).min(1.0),
                    f32::from(enemy.as_ref().is_some_and(|e| e.visible)),
                    unit.contacts.len() as f32 / team_size as f32,
                    enemy.as_ref().map(|e| e.health / 100.0).unwrap_or(0.0),
                    enemy
                        .as_ref()
                        .map(|e| e.confidence(self.tick))
                        .unwrap_or(0.0),
                    allies.len() as f32 / (team_size - 1) as f32,
                    isolation,
                    local_threat,
                    support
                        .map(|(_, urgency)| (urgency / 2.0).min(1.0))
                        .unwrap_or(0.0),
                    engaged as f32 / (team_size - 1) as f32,
                    f32::from(distance(unit.position, ridge) < 0.9),
                    f32::from(self.tick < 180),
                    f32::from(self.tick >= old_flank_until),
                    f32::from(enemy.is_some()),
                    old_flank_anchor
                        .map(|p| (distance(unit.position, p) / 10.0).min(1.0))
                        .unwrap_or(1.0),
                ];
                if let Some(plan) = &bow_plan {
                    unit.scores[plan.action] = unit.scores[plan.action].max(4.0);
                }
                let utility_action = unit
                    .scores
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .unwrap()
                    .0;
                let mask = std::array::from_fn(|i| unit.scores[i] >= 0.0);
                let prediction = self
                    .policy
                    .as_ref()
                    .and_then(|model| model.predict(&decision_features, &mask));
                let learned_action = prediction
                    .map(|p| p.0)
                    .filter(|i| *i < ACTION_COUNT && mask[*i]);
                let active_controller = self
                    .learned_team
                    .map(|team| {
                        if unit.team == team {
                            Controller::Learned
                        } else {
                            Controller::Utility
                        }
                    })
                    .unwrap_or(self.controller);
                unit.action = if active_controller == Controller::Learned {
                    if let Some(action) = learned_action {
                        action
                    } else {
                        self.learned_fallbacks += 1;
                        utility_action
                    }
                } else {
                    utility_action
                };
                let comparison = Comparison {
                    tick: self.tick,
                    unit: unit.id,
                    utility: utility_action,
                    learned: learned_action,
                    executed: unit.action,
                    confidence: prediction.map(|p| p.1),
                    controller: active_controller,
                    disagreed: learned_action.is_some_and(|a| a != utility_action),
                };
                self.comparison_summary.record(&comparison);
                if comparison.disagreed {
                    self.disagreements.push(comparison.clone());
                }
                unit.comparison = Some(comparison);
                unit.decision_counts[unit.action] += 1;
                if unit.action == 3 && previous_action != 3 {
                    unit.flank_anchor = enemy
                        .as_ref()
                        .map(|e| cognition::flank_position(arena, unit, e));
                    unit.flank_until = self.tick + 60;
                } else if unit.action != 3 {
                    unit.flank_anchor = None;
                }
                unit.decision_reason = match unit.action {
                    0 => {
                        if arena.large() && enemy.is_none() && unit.search_waypoint > 0 {
                            "Scout patrol after deployment search"
                        } else if enemy.is_some() {
                            "Advance on observed/reported contact"
                        } else {
                            "Search opposing deployment zone"
                        }
                    }
                    1 => "Current sighting inside weapon range",
                    2 => "Caution, discipline and patience favor ridge",
                    3 => "Ally engages target; approach another angle",
                    4 => "Teamwork favors reducing ally separation",
                    5 => "Injury or close threat favors withdrawal",
                    6 => "Patience favors holding ridge position",
                    7 => "Exposure and self-preservation favor cover",
                    _ => "Known ally injury or pressure favors support",
                }
                .into();
                if active_controller == Controller::Learned && learned_action.is_some() {
                    unit.decision_reason = "Learned proposal; scores show utility baseline".into();
                } else if let Some(line) = scar.as_ref().filter(|line| line.action == unit.action) {
                    unit.decision_reason = line.text.into();
                }
                if self.founders {
                    if unit.action == 0 {
                        if !unit.charging {
                            unit.charging = true;
                            unit.charge_health = unit.health;
                        }
                    } else {
                        unit.charging = false;
                    }
                }
            }
            unit.enemy = enemy.as_ref().map(|e| e.enemy);
            unit.support_target = if unit.action == 8 {
                support.map(|(ally, _)| ally.id)
            } else {
                None
            };
            unit.target = match unit.action {
                2 => ridge,
                3 => unit.flank_anchor.unwrap_or(enemy_position),
                4 => center,
                5 => [
                    (unit.position[0] + (unit.position[0] - enemy_position[0]))
                        .clamp(-arena.half_size()[0] + 0.5, arena.half_size()[0] - 0.5),
                    (unit.position[1] + (unit.position[1] - enemy_position[1]))
                        .clamp(-arena.half_size()[1] + 1.0, arena.half_size()[1] - 1.0),
                ],
                6 => unit.position,
                7 => arena.nearest_cover(unit.position).unwrap_or(unit.position),
                8 => support
                    .map(|(ally, _)| {
                        if unit.has_skill(FighterClass::Healer) {
                            let away = [
                                ally.position[0] - enemy_position[0],
                                ally.position[1] - enemy_position[1],
                            ];
                            let length = (away[0] * away[0] + away[1] * away[1]).sqrt().max(0.001);
                            let behind = [
                                ally.position[0] + away[0] / length * 2.5,
                                ally.position[1] + away[1] / length * 2.5,
                            ];
                            if arena.walk_segment(ally.position, behind) {
                                behind
                            } else {
                                ally.position
                            }
                        } else {
                            cognition::support_position(arena, unit, ally)
                        }
                    })
                    .unwrap_or(unit.position),
                _ => enemy_position,
            };
            if let Some(plan) = &bow_plan
                && plan.action == unit.action
            {
                unit.target = plan.goal;
                unit.decision_reason = plan.reason.into();
            }
            if self.tick.is_multiple_of(5)
                && let Some(sink) = records.as_deref_mut()
            {
                sink.push(telemetry::DecisionRecord::capture(
                    self.tick,
                    unit,
                    decision_features.clone(),
                ));
            }
            let banner = snapshot
                .iter()
                .filter(|a| {
                    a.team == unit.team
                        && a.health > 0.0
                        && a.banner_active()
                        && distance(unit.position, a.position) < 5.0
                        && arena.clear_segment(unit.position, a.position)
                })
                .map(|a| {
                    1.0 + 0.15
                        * if a.equipment.has(equipment::Gear::WarBanner) {
                            a.equipment.power(2)
                        } else {
                            1.0
                        }
                })
                .fold(1.0_f32, f32::max);
            let hexed = snapshot.iter().any(|a| {
                a.team != unit.team
                    && a.health > 0.0
                    && a.has_skill(FighterClass::Hexer)
                    && distance(unit.position, a.position) < 5.0
                    && arena.clear_segment(unit.position, a.position)
            });
            let class_power = banner * (if hexed { 0.75 } else { 1.0 });
            if unit.has_skill(FighterClass::Healer)
                && unit.heal_cooldown <= 0.0
                && let Some(ally) = allies
                    .iter()
                    .filter(|a| {
                        a.health < a.max_health() - 0.01
                            && distance(unit.position, a.position) < 5.0
                            && arena.clear_segment(unit.position, a.position)
                    })
                    .min_by(|a, b| a.health_fraction().total_cmp(&b.health_fraction()))
            {
                let power = unit.healing_power();
                healing[ally.id] += power;
                casts.push((unit.id, ally.id, power));
                unit.heal_cooldown = 1.2;
                if unit.class == Some(FighterClass::Healer) && !unit.is_ranged() {
                    unit.cooldown = unit.cooldown.max(1.2);
                }
                unit.support_target = Some(ally.id);
                unit.decision_reason = "Healer restores a wounded ally".into();
            }
            // Windups use only a current, visible observation, never an unseen transform.
            let mut release = None;
            if let Some(mut draw) = unit.windup.take() {
                if unit.ammunition > 0
                    && (draw.weapon != ProjectileWeapon::Bow || unit.bow_available())
                    && enemy
                        .as_ref()
                        .is_some_and(|e| e.visible && e.enemy == draw.target)
                    && d < draw.weapon.range()
                    && shot_clear
                {
                    draw.remaining -= DT;
                    if draw.remaining <= 0.0001 {
                        release = Some(draw.weapon);
                    } else {
                        unit.windup = Some(draw);
                    }
                } else {
                    unit.cooldown = unit.cooldown.max(0.3);
                    unit.decision_reason = if draw.weapon == ProjectileWeapon::Bow {
                        "Bow draw cancelled: firing lane or range changed"
                    } else {
                        "Throw cancelled: target left sight or weapon range"
                    }
                    .into();
                }
            }
            theus_combat::cancel_windup(unit, enemy.as_ref(), d, shot_clear);
            if unit.action == 1
                && enemy.as_ref().is_some_and(|e| e.visible)
                && d < unit.attack_range()
                && (unit.is_ranged()
                    || d.hypot(ninjutsu::physical_height(
                        &snapshot[enemy.as_ref().unwrap().enemy],
                        arena,
                        self.tick as f32,
                    )) < unit.attack_range())
                && shot_clear
                && unit.cooldown <= 0.0
                && unit.windup.is_none()
                && release.is_none()
            {
                let advantage = 1.0
                    + 0.1
                        * (arena.elevation(unit.position) - arena.elevation(enemy_position))
                            .max(0.0);
                if unit.is_ranged() {
                    let weapon = unit.projectile_weapon().unwrap();
                    if weapon.windup() > 0.0 {
                        unit.windup = Some(Windup {
                            weapon,
                            target: enemy.as_ref().unwrap().enemy,
                            remaining: weapon.windup(),
                        });
                        unit.decision_reason = if weapon == ProjectileWeapon::Bow {
                            "Drawing and aiming the bow".into()
                        } else {
                            format!("Planting feet to launch {}", weapon.name())
                        };
                    } else {
                        release = Some(weapon);
                    }
                } else if !unit.equipment.has(equipment::Gear::RazorWheel)
                    && theus_combat::melee_ready(
                        unit,
                        enemy.as_ref().unwrap().enemy,
                        &snapshot,
                        self.tick,
                    )
                {
                    let dealt = (if unit.projectile_weapon().is_some() {
                        8.0
                    } else {
                        10.0
                    }) * unit.damage_multiplier()
                        * class_power
                        * snapshot[enemy.as_ref().unwrap().enemy].incoming_multiplier()
                        * advantage
                        * (1.0 - 0.3 * arena.cover(enemy_position));
                    damage[enemy.as_ref().unwrap().enemy] += dealt;
                    impacts.push((
                        unit.id,
                        enemy.as_ref().unwrap().enemy,
                        dealt,
                        matches!(
                            unit.class,
                            Some(FighterClass::Guardian | FighterClass::Paladin)
                        ),
                        3.5,
                        None,
                    ));
                    unit.damage_dealt += dealt;
                    unit.cooldown = 0.8;
                    if unit.projectile_weapon().is_some() {
                        unit.decision_reason = if unit.projectile_weapon()
                            == Some(ProjectileWeapon::Bow)
                            && unit.ammunition > 0
                            && !unit.bow_available()
                        {
                            "Arm wound prevents drawing; using a sidearm"
                        } else if unit.bow_melee {
                            "Rushed archer fights with a sidearm"
                        } else {
                            "Ammunition spent: fighting with sidearm"
                        }
                        .into();
                    }
                }
            }
            let released = release.is_some();
            if let Some(weapon) = release {
                let advantage = 1.0
                    + 0.1
                        * (arena.elevation(unit.position) - arena.elevation(enemy_position))
                            .max(0.0);
                let from = if weapon == ProjectileWeapon::Bow {
                    archery::arrow_origin(unit.position, enemy_position)
                } else {
                    unit.position
                };
                self.rocks.push(Rock {
                    weapon,
                    shooter: unit.id,
                    team: unit.team,
                    from,
                    to: enemy_position,
                    age: 0,
                    flight_ticks: (distance(from, enemy_position) / weapon.speed() / DT)
                        .ceil()
                        .max(1.0) as u32,
                    damage: weapon.damage() * unit.damage_multiplier() * class_power * advantage,
                });
                unit.last_shot_target = Some(enemy_position);
                if weapon == ProjectileWeapon::Bow {
                    unit.bow_release = 0.2;
                }
                unit.rocks_thrown += 1;
                unit.weapon_shots[weapon.index()] += 1;
                if weapon != ProjectileWeapon::Rock {
                    unit.ammunition = unit.ammunition.saturating_sub(1);
                }
                unit.cooldown = weapon.recovery()
                    * if unit.equipped_skill(skills::Skill::Quickdraw) {
                        0.625
                    } else {
                        1.0
                    };
                unit.decision_reason = format!("Released {}", weapon.name());
            }
            if let Some(exit) = ninjutsu::smoke_exit(unit, arena, &self.ninjutsu, self.tick) {
                unit.target = exit;
                unit.decision_reason = "Leave smoke by an obscured flank".into();
            }
            if bow && (unit.windup.is_some() || released) {
                unit.target = enemy_position;
            }
            if let Arena::Pressure(field) = arena
                && field.layout > 0
                && enemy.is_some()
                && unit.cooldown <= 0.0
                && !arena.clear_segment(unit.position, enemy_position)
                && let Some((index, _)) = crate::procedural::original_walls(field)
                    .iter()
                    .take(4)
                    .enumerate()
                    .filter(|(index, _)| field.destroyed & (1 << index) == 0)
                    .filter(|(_, (c, s))| {
                        let dx = ((unit.position[0] - c[0]).abs() - s[0] * 0.5).max(0.0);
                        let dz = ((unit.position[1] - c[1]).abs() - s[1] * 0.5).max(0.0);
                        (dx * dx + dz * dz).sqrt() < 1.8 && distance(*c, enemy_position) < d
                    })
                    .min_by(|(_, (a, _)), (_, (b, _))| {
                        distance(unit.position, *a).total_cmp(&distance(unit.position, *b))
                    })
            {
                barrier_damage[index] += 10.0 * unit.damage_multiplier();
                unit.cooldown = 0.8;
                unit.decision_reason = "Breaking a nearby barricade to open the engagement".into();
            }
            let stationary = (bow && unit.action == 1 && !unit.bow_melee && !shot_clear)
                || unit.windup.is_some()
                || released
                || unit.action == 6
                || (unit.action == 1
                    && enemy.as_ref().is_some_and(|e| e.visible)
                    && d < unit.attack_range()
                    && shot_clear);
            if stationary {
                unit.route = Route::default();
                unit.path_cursor = 0;
            } else {
                let expired = self.tick.is_multiple_of(15)
                    || unit.route.points.is_empty()
                    || distance(unit.route_goal, unit.target) > 1.0
                    || unit.route_mind != unit.mind;
                if expired {
                    unit.route =
                        plan_known(arena, unit.position, unit.target, unit.mind, &known_threats);
                    unit.path_cursor = 1;
                    unit.route_goal = unit.target;
                    unit.route_mind = unit.mind;
                }
                while unit.path_cursor < unit.route.points.len()
                    && distance(unit.position, unit.route.points[unit.path_cursor]) < 0.18
                    && (unit.path_cursor + 1 == unit.route.points.len()
                        || arena
                            .walk_segment(unit.position, unit.route.points[unit.path_cursor + 1]))
                {
                    unit.path_cursor += 1;
                }
                if let Some(waypoint) = unit.route.points.get(unit.path_cursor).copied() {
                    let remaining = distance(unit.position, waypoint);
                    if remaining > 0.001 {
                        let uphill =
                            (arena.elevation(waypoint) - arena.elevation(unit.position)).max(0.0);
                        let speed = 2.2
                            * unit.movement_multiplier()
                            * if bow { 0.82 } else { 1.0 }
                            * banner
                            / (1.0 + uphill * 0.35 + arena.roughness(unit.position) * 1.0);
                        let amount = (speed * DT).min(remaining);
                        let mut next = [
                            unit.position[0]
                                + (waypoint[0] - unit.position[0]) / remaining * amount,
                            unit.position[1]
                                + (waypoint[1] - unit.position[1]) / remaining * amount,
                        ];
                        next[0] =
                            next[0].clamp(-arena.half_size()[0] + 0.5, arena.half_size()[0] - 0.5);
                        next[1] =
                            next[1].clamp(-arena.half_size()[1] + 0.5, arena.half_size()[1] - 0.5);
                        if arena.walk_segment(unit.position, next) {
                            unit.distance_moved += distance(unit.position, next);
                            unit.position = next;
                        }
                    }
                }
            }
        }
        ninjutsu::pulls(self);
        self.motion = collision::resolve(arena, &mut self.units, &snapshot);
        // Spear/dart interception uses accepted body motion, including collision stops.
        for projectile in &mut self.rocks {
            if projectile.weapon == ProjectileWeapon::Rock || projectile.age == 0 {
                continue;
            }
            let at = |age: u32| {
                let t = (age as f32 / projectile.flight_ticks as f32).min(1.0);
                [
                    projectile.from[0] + (projectile.to[0] - projectile.from[0]) * t,
                    projectile.from[1] + (projectile.to[1] - projectile.from[1]) * t,
                ]
            };
            let (from, to) = (at(projectile.age - 1), at(projectile.age));
            let hit = self
                .units
                .iter()
                .filter(|u| {
                    u.id != projectile.shooter
                        && ninjutsu::physical_height(u, arena, self.tick as f32) < 1.4
                        && (u.team != projectile.team || projectile.weapon == ProjectileWeapon::Bow)
                        && snapshot[u.id].health > 0.0
                })
                .filter_map(|u| {
                    self.motion
                        .frames
                        .windows(2)
                        .filter_map(|frames| {
                            let (a, b) = (&frames[0], &frames[1]);
                            let point = |t: f32| {
                                [
                                    from[0] + (to[0] - from[0]) * t,
                                    from[1] + (to[1] - from[1]) * t,
                                ]
                            };
                            weapons::intercept(
                                point(a.time),
                                point(b.time),
                                a.positions[u.id],
                                b.positions[u.id],
                                collision::BODY_RADIUS,
                            )
                            .map(|t| (u.id, a.time + (b.time - a.time) * t))
                        })
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            if let Some((id, time)) = hit {
                if self.units[id].team == projectile.team {
                    self.units[projectile.shooter].arrow_blocks += 1;
                    projectile.age = projectile.flight_ticks;
                    continue;
                }

                let point = [
                    from[0] + (to[0] - from[0]) * time,
                    from[1] + (to[1] - from[1]) * time,
                ];
                let len = distance(projectile.from, projectile.to).max(0.001);
                let velocity = [
                    (projectile.to[0] - projectile.from[0]) / len * projectile.weapon.speed(),
                    (projectile.to[1] - projectile.from[1]) / len * projectile.weapon.speed(),
                ];
                let velocity = std::array::from_fn(|axis| {
                    velocity[axis]
                        - (self.units[id].position[axis] - snapshot[id].position[axis]) / DT
                });
                let dealt = projectile.damage
                    * self.units[id].incoming_multiplier()
                    * (1.0 - 0.3 * arena.cover(point));
                damage[id] += dealt;
                self.units[projectile.shooter].damage_dealt += dealt;
                self.units[projectile.shooter].rocks_hit += 1;
                impacts.push((
                    projectile.shooter,
                    id,
                    dealt,
                    false,
                    projectile.weapon.speed(),
                    Some((point, velocity)),
                ));
                projectile.age = projectile.flight_ticks;
            }
        }
        self.rocks.retain(|r| r.age < r.flight_ticks);
        for contact in &self.motion.impacts {
            let (a, b) = (contact.a, contact.b);
            if self.units[a].team == self.units[b].team || contact.speed < 3.2 {
                continue;
            }
            let (attacker, defender, sign) = if self.units[a].charging {
                (a, b, 1.0)
            } else if self.units[b].charging {
                (b, a, -1.0)
            } else {
                continue;
            };
            if self.units[attacker].impact_cooldown > 0.0 {
                continue;
            }
            let dealt = ((contact.speed - 2.5) * 2.0).clamp(1.0, 9.0)
                * self.units[defender].incoming_multiplier();
            damage[defender] += dealt;
            self.units[attacker].damage_dealt += dealt;
            self.units[attacker].impact_cooldown = 0.8;
            impacts.push((
                attacker,
                defender,
                dealt,
                true,
                contact.speed,
                Some((
                    contact.point,
                    [contact.velocity[0] * sign, contact.velocity[1] * sign],
                )),
            ));
            if self.units[defender].equipped_skill(skills::Skill::ShieldBash) {
                let recoil = dealt * 0.6 * self.units[attacker].incoming_multiplier();
                damage[attacker] += recoil;
                impacts.push((
                    defender,
                    attacker,
                    recoil,
                    true,
                    contact.speed,
                    Some((
                        contact.point,
                        [-contact.velocity[0] * sign, -contact.velocity[1] * sign],
                    )),
                ));
            }
        }
        ninjutsu::wheels(self, &mut impacts);
        theus_combat::exchanges(self, &snapshot, &mut damage, &mut impacts);
        ninjutsu::hooks(self);
        // Simultaneous damage avoids giving the first team update-order immunity.
        for unit in &mut self.units {
            if damage[unit.id] >= 12.0 && unit.windup.take().is_some() {
                unit.cooldown = unit.cooldown.max(0.35);
                unit.decision_reason = "Heavy impact interrupted the throw".into();
            }
            unit.health = (unit.health - damage[unit.id]).max(0.0);
            if unit.health > 0.0 {
                let restored = healing[unit.id].min((unit.max_health() - unit.health).max(0.0));
                unit.health += restored;
                if restored > 0.0 {
                    for &(healer, target, power) in &casts {
                        if target == unit.id {
                            healing_credit[healer] += restored * power / healing[unit.id];
                            self.heals.push(HealEvent {
                                tick: self.tick,
                                healer,
                                target,
                                amount: restored * power / healing[unit.id],
                            });
                        }
                    }
                }
            }
            if unit.health <= 0.0 {
                unit.windup = None;
                unit.visible_enemies = 0;
                for c in &mut unit.contacts {
                    c.visible = false;
                }
            }
        }
        for (attacker, defender, amount, blunt, speed, contact) in impacts {
            let a = &snapshot[attacker];
            let b = &snapshot[defender];
            // A rare, severe individual cutting hit can sever a living fighter's
            // limb. Crossing an HP threshold alone never amputates anything.
            let severed = if self.founders {
                injuries::sever(
                    self.seed,
                    self.next_damage_event,
                    &self.units[defender],
                    amount,
                    !blunt && contact.is_none(),
                )
            } else {
                0
            };
            self.units[defender].lost_limbs |= severed;
            self.units[defender].prosthetics &= !severed;
            let delta = [b.position[0] - a.position[0], b.position[1] - a.position[1]];
            let length = distance(a.position, b.position).max(0.001);
            let velocity = std::array::from_fn(|axis| {
                delta[axis] / length * speed
                    + ((self.units[attacker].position[axis] - a.position[axis])
                        - (self.units[defender].position[axis] - b.position[axis]))
                        / DT
            });
            let position = contact.map(|c| c.0).unwrap_or([
                b.position[0] - delta[0] / length * collision::BODY_RADIUS,
                b.position[1] - delta[1] / length * collision::BODY_RADIUS,
            ]);
            let velocity = contact.map(|c| c.1).unwrap_or(velocity);
            self.damage_events.push(DamageEvent {
                sequence: self.next_damage_event,
                tick: self.tick,
                attacker,
                defender,
                amount,
                velocity,
                position,
                severed,
                blunt,
            });
            self.next_damage_event += 1;
        }
        if self.damage_events.len() > 256 {
            self.damage_events.drain(..self.damage_events.len() - 256);
        }
        if let Arena::Pressure(mut field) = self.arena
            && field.layout > 0
        {
            let old = field.destroyed;
            for (i, damage) in barrier_damage.iter().enumerate() {
                self.barricade_health[i] = (self.barricade_health[i] - damage).max(0.0);
                if self.barricade_health[i] <= 0.0 {
                    field.destroyed |= 1 << i;
                }
            }
            if field.destroyed != old {
                self.arena = Arena::Pressure(field);
                for unit in &mut self.units {
                    unit.route = Route::default();
                }
            }
        }
        for (unit, credit) in self.units.iter_mut().zip(healing_credit) {
            unit.healing_done += credit;
            if credit > 0.
                && let Some(actor) = &mut unit.theus
            {
                actor.sheet.train(5, 2);
            }
        }
        if self.heals.len() > 64 {
            self.heals.drain(..self.heals.len() - 64);
        }
        for team in 0..2 {
            let alive: Vec<_> = self
                .units
                .iter()
                .filter(|u| u.team == team && u.health > 0.0)
                .collect();
            let mut sum = 0.0;
            let mut pairs = 0;
            for (i, a) in alive.iter().enumerate() {
                for b in alive.iter().skip(i + 1) {
                    sum += distance(a.position, b.position) as f64;
                    pairs += 1;
                }
            }
            if pairs > 0 {
                self.ally_distance_sum[team] += sum / pairs as f64;
                self.ally_distance_samples[team] += 1;
            }
        }
        self.tick += 1;
        let a = self.survivors(0);
        let b = self.survivors(1);
        if ((a == 0 || b == 0) && self.rocks.is_empty() && self.ninjutsu.wheels.is_empty())
            || self.tick >= BATTLE_TICK_LIMIT
        {
            self.finished = true;
            self.result = if a == 0 && b == 0 {
                "Draw"
            } else if a == 0 {
                "Blue wins"
            } else if b == 0 {
                "Red wins"
            } else {
                "Timeout / draw"
            }
            .into();
        } else {
            self.result = "Battle in progress".into();
        }
    }
}
