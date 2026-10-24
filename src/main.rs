mod audio;
mod battlefield;
mod camp;
mod exchange;
mod founding;
mod front;
mod player;
mod presentation;
mod rpg;
mod visuals;
use bevy::{
    asset::RenderAssetUsages,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    mesh::PrimitiveTopology,
    prelude::*,
    render::{
        RenderPlugin,
        settings::{PowerPreference, WgpuSettings},
    },
};
use phalanx::{
    ACTION_COUNT, ACTIONS, Battle, CommandTitle, DT, FighterClass, Mind, PALADIN_AURA_RADIUS, Role,
    learned::Controller,
    navigation::threat_at_known,
    perception::{self, Source},
    progression::{Campaign, PALETTE},
    terrain::Arena,
};

fn frame_zoom(arena: Arena) -> f32 {
    if arena.large() {
        52.0
    } else {
        (arena.half_size()[0] * 2.15).clamp(24.0, 72.0)
    }
}

#[derive(Resource)]
struct Session {
    founding: founding::Draft,
    player: player::View,
    map_briefing: bool,
    exchange: exchange::State,
    preview: Option<phalanx::Unit>,
    battle: Battle,
    menu: bool,
    ladder: bool,
    camp_page: Option<camp::Page>,
    running: bool,
    selected: usize,
    speed: f32,
    accumulator: f32,
    paths: usize,
    overlay: usize,
    disagreement_pair: Option<(usize, usize)>,
    orbit: f32,
    zoom: f32,
    pitch: f32,
    camera_target: Vec3,
    camera_follow: bool,
    previous_positions: Vec<[f32; 2]>,
    logged_disagreements: usize,
    run_id: u32,
    disagreement_log: Option<std::io::BufWriter<std::fs::File>>,
    campaign: Campaign,
    active_battle_id: Option<u64>,
    rewarded: bool,
    save_notice: String,
    autosave: bool,
}
impl Session {
    fn new(mut battle: Battle) -> Self {
        let path = std::path::Path::new("outputs/phalanx-save.json");
        let mut notice = String::new();
        let mut autosave = true;
        let mut campaign = if path.exists() {
            match Campaign::load(path) {
                Ok(c) => {
                    notice = "Loaded persistent squads".into();
                    c
                }
                Err(e) => {
                    notice = format!("Save unreadable: {e}; original untouched");
                    autosave = false;
                    Campaign::new()
                }
            }
        } else {
            Campaign::new()
        };
        let active_battle_id = if battle.founders {
            let seed = if std::env::args().any(|a| a == "--seed") {
                battle.seed
            } else {
                campaign.teams[0].fighters[0]
                    .history
                    .last()
                    .map(|h| h.seed.wrapping_add(1))
                    .unwrap_or(battle.seed)
            };
            let arena = if std::env::args().any(|a| a == "--arena") {
                battle.arena
            } else {
                phalanx::pressure::generate(&campaign.teams, seed)
            };
            match campaign.begin_battle(seed, arena, battle.controller) {
                Ok((id, b)) => {
                    if !b.pressure_note.is_empty() {
                        notice = if notice.is_empty() {
                            b.pressure_note.clone()
                        } else {
                            format!("{notice} | {}", b.pressure_note)
                        };
                    }
                    battle = b;
                    Some(id)
                }
                Err(e) => {
                    notice = e;
                    None
                }
            }
        } else {
            None
        };
        let previous_positions = battle.units.iter().map(|u| u.position).collect();
        let paths = if battle.arena.large() { 1 } else { 0 };
        let zoom = frame_zoom(battle.arena);
        let founding =
            founding::Draft::new(&campaign, battle.founders && !campaign.founders_prepared);
        let mut session = Self {
            founding,
            player: player::View::default(),
            map_briefing: true,
            exchange: exchange::State::default(),
            battle,
            preview: None,
            menu: true,
            ladder: false,
            running: false,
            selected: 0,
            speed: 1.0,
            accumulator: 0.0,
            paths,
            overlay: 0,
            disagreement_pair: None,
            orbit: 0.0,
            zoom,
            pitch: 0.72,
            camera_target: Vec3::new(3.5, 0.0, 0.0),
            camera_follow: false,
            previous_positions,
            logged_disagreements: 0,
            run_id: 0,
            disagreement_log: None,
            campaign,
            active_battle_id,
            camp_page: None,
            rewarded: false,
            save_notice: notice,
            autosave,
        };
        if session.battle.founders && session.autosave {
            session.save_campaign();
        }
        session.refresh_preview();
        session
    }
    fn visual_unit(&self, id: usize) -> Option<&phalanx::Unit> {
        if id == rpg::PREVIEW_ID {
            self.preview.as_ref()
        } else {
            self.battle.units.get(id)
        }
    }
    fn refresh_preview(&mut self) {
        self.preview = if self.battle.founders {
            Some(camp::prepared(self, self.selected))
        } else {
            None
        };
    }
    fn team_color(&self, team: usize) -> Color {
        if self.battle.founders {
            let c = self.campaign.teams[team].color;
            Color::srgb(c[0], c[1], c[2])
        } else if team == 0 {
            Color::srgb(0.92, 0.28, 0.25)
        } else {
            Color::srgb(0.20, 0.58, 0.94)
        }
    }
    fn team_name(&self, team: usize) -> &str {
        if self.battle.founders {
            &self.campaign.teams[team].name
        } else if team == 0 {
            "RED"
        } else {
            "BLUE"
        }
    }
    fn unit_label(&self, id: usize) -> String {
        let u = &self.battle.units[id];
        if self.battle.founders {
            format!(
                "{} {}",
                self.team_name(u.team),
                u.command_title.unwrap().name()
            )
        } else {
            self.battle.unit_label(id)
        }
    }
    fn save_campaign(&mut self) -> bool {
        if self.exchange.live.is_some() {
            self.save_notice = "Frozen challenge: return to your campaign to save squads".into();
            return false;
        }
        if !self.battle.founders {
            self.save_notice = "Switch to Founders to save progression".into();
            return false;
        }
        let capture = if self.rewarded {
            Ok(())
        } else {
            self.campaign.capture_minds(&self.battle)
        };
        match capture.and_then(|()| {
            self.campaign
                .save(std::path::Path::new("outputs/phalanx-save.json"))
        }) {
            Ok(()) => {
                self.save_notice = "Saved squads, experience and colors".into();
                true
            }
            Err(e) => {
                self.save_notice = format!("Save failed: {e}");
                false
            }
        }
    }
    fn settle_finished(&mut self) {
        if self.exchange.live.is_some() {
            exchange::finish(self);
            return;
        }
        if self.battle.founders
            && self.battle.finished
            && !self.rewarded
            && let Some(id) = self.active_battle_id
        {
            match self.campaign.settle(id, &self.battle) {
                Ok(()) => {
                    self.rewarded = true;
                    self.menu = true;
                    self.running = false;
                    self.camp_page = Some(camp::Page::Report);
                    let size = self.battle.team_size();
                    for u in &mut self.battle.units {
                        u.mind = self.campaign.teams[u.team].fighters[u.id % size].mind;
                    }
                    if self.autosave {
                        if self.save_campaign() {
                            self.save_notice =
                                "XP + habits saved. Next battle continues your squads.".into();
                        }
                    } else {
                        self.save_notice =
                            "XP earned in memory; campaign autosaving is disabled.".into();
                    }
                }
                Err(e) => {
                    self.rewarded = true;
                    self.save_notice = format!("Rewards rejected: {e}");
                }
            }
        }
    }
    fn restart_founders(&mut self, seed: u64, arena: Arena, controller: Controller) {
        match self.campaign.begin_battle(seed, arena, controller) {
            Ok((id, b)) => {
                self.battle = b;
                self.active_battle_id = Some(id);
                self.rewarded = false;
            }
            Err(e) => self.save_notice = e,
        }
    }
    fn remember_positions(&mut self) {
        for unit in &self.battle.units {
            self.previous_positions[unit.id] = unit.position;
        }
    }
    fn presented(&self, id: usize) -> [f32; 2] {
        let current = self.battle.units[id].position;
        if !self.running {
            return current;
        }
        if let Some(p) = self
            .battle
            .motion
            .position(id, (self.accumulator / DT).clamp(0.0, 1.0))
        {
            return p;
        }
        let old = self.previous_positions[id];
        let alpha = (self.accumulator / DT).clamp(0.0, 1.0);
        [
            old[0] + (current[0] - old[0]) * alpha,
            old[1] + (current[1] - old[1]) * alpha,
        ]
    }
}
#[derive(Component)]
struct Pawn(usize);
#[derive(Component)]
struct RockVisual {
    slot: usize,
    weapon: phalanx::weapons::ProjectileWeapon,
}
#[derive(Component)]
struct GroundArena(Arena);
#[derive(Component)]
struct ChokeWall(usize);
#[derive(Component)]
struct Slab;
#[derive(Component)]
struct Bridge;
#[derive(Component)]
struct PawnPart(usize, bool);
#[derive(Component)]
struct Insignia(usize, bool);
#[derive(Resource)]
struct PawnAssets {
    red: Handle<StandardMaterial>,
    blue: Handle<StandardMaterial>,
    gold: Handle<StandardMaterial>,
    silver: Handle<StandardMaterial>,
}
#[derive(Component, Default)]
struct PressLatch(bool);
#[derive(Component)]
struct Inspector;
#[derive(Component)]
struct InspectorPanel;
type MainCameraFilter = (With<Camera3d>, Without<visuals::PortraitCamera>);
#[derive(Component)]
struct TraitValue(usize);
#[derive(Component)]
struct Status;
#[derive(Component)]
struct DivergenceSummary;
#[derive(Component)]
struct PairRow(usize);
#[derive(Component, Clone, Copy, Debug)]
enum Control {
    Menu,
    Camp(camp::Page),
    SelectFighter(usize),
    Ladder,
    Recruit,
    ChooseRival(usize),
    ExportSnapshot,
    RefreshSnapshots,
    ImportSnapshot(usize),
    StartChallenge,
    ChallengeSeed,
    ExportChallengeResult,
    LeaveChallenge,
    Continue,
    Quit,
    Class(FighterClass),
    Title(CommandTitle),
    FlagEmblem,
    FlagAccent,
    FlagPrimary,
    StandardBearer,
    Run,
    Pause,
    Step,
    Reset,
    Swap,
    Speed,
    Paths,
    Arena,
    GenerateArena,
    BattlefieldBriefing,
    ResearchView,
    TogglePlay,
    NextTutorial,
    StarterClass(usize, FighterClass),
    ClearPhalanxName,
    ConfirmFounders,
    RosterSide(usize),
    RosterPage(i32),
    BattleStyle(phalanx::styles::BattleStyle),
    FollowFighter,
    FrameBattle,
    Overlay,
    Controller,
    Pair(usize, usize),
    ClearPair,
    Setup,
    NextBattle,
    Save,
    Load,
    TeamColor(usize),
    Appearance(usize),
    Prosthetic,
    Workshop(phalanx::equipment::Gear),
    NextMatch,
    Aggressive,
    Disciplined,
    Trait(usize, f32),
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arena = args
        .windows(2)
        .find(|w| w[0] == "--arena")
        .map(|w| Arena::parse(&w[1]).expect("arena must be ridge, choke, open, or citadel"))
        .unwrap_or_default();
    let seed = args
        .windows(2)
        .find(|w| w[0] == "--seed")
        .map(|w| {
            w[1].parse::<u64>()
                .expect("seed must be an unsigned integer")
        })
        .unwrap_or(42);
    if args.iter().any(|a| a == "--audio-witness") {
        audio::export_witness(seed).expect("export synthesized audio");
        return;
    }
    if args.iter().any(|a| a == "--headless") {
        let mut battle = Battle::with_setup(
            seed,
            args.iter().any(|a| a == "--swap"),
            arena,
            !args.iter().any(|a| a == "--lab"),
        );
        while !battle.finished {
            battle.step();
        }
        let output = serde_json::to_string_pretty(&battle).unwrap();
        if let Some(path) = args.windows(2).find(|w| w[0] == "--output").map(|w| &w[1]) {
            std::fs::write(path, output).expect("cannot write telemetry");
        } else {
            println!("{output}");
        }
        return;
    }
    let mut battle = Battle::with_setup(
        seed,
        args.iter().any(|a| a == "--swap"),
        arena,
        !args.iter().any(|a| a == "--lab"),
    );
    battle.controller = if args.iter().any(|a| a == "--learned") {
        Controller::Learned
    } else {
        Controller::Utility
    };
    let mut session = Session::new(battle);
    let visual_witness = args.iter().any(|a| a == "--visual-witness");
    if visual_witness {
        session.founding.open = false;
        session.menu = false;
        session.running = true;
        session.speed = 4.0;
        if args.iter().any(|a| a == "--weapon-witness") {
            use phalanx::equipment::Gear;
            use phalanx::weapons::ProjectileWeapon as W;
            session.autosave = false;
            session.campaign = Campaign::new();
            session.campaign.teams[0].gold = 1000;
            session.campaign.teams[0].salvage = 1000;
            let spear = args.iter().any(|a| a == "--spear-witness");
            let bow = args.iter().any(|a| a == "--bow-witness");
            if bow {
                session.campaign.teams[0].fighters[0].class = FighterClass::Ranger;
            }

            let f = &mut session.campaign.teams[0].fighters[1];
            f.class = if bow && !args.iter().any(|a| a == "--hybrid-bow-witness") {
                FighterClass::Ranger
            } else {
                FighterClass::Warrior
            };
            if bow {
                f.mastery.train_weapon(W::Atlatl, 100);
            }
            f.mastery.train(FighterClass::Healer, 100);
            f.mastery.train(FighterClass::BannerBearer, 100);
            f.mastery.train_weapon(W::Rock, 100);
            if !spear {
                f.mastery.train_weapon(W::Spear, 100);
            }
            if args.iter().any(|a| a == "--weapon-report-witness") {
                f.mastery.train_weapon(W::Spear, 90);
            }
            session
                .campaign
                .workshop(
                    0,
                    1,
                    if bow {
                        Gear::Bow
                    } else if spear {
                        Gear::ThrowingSpear
                    } else {
                        Gear::Atlatl
                    },
                )
                .unwrap();
            session.campaign.workshop(0, 1, Gear::WarBanner).unwrap();
            session.restart_founders(seed, Arena::OpenField, Controller::Utility);
            session.selected = if args.iter().any(|a| a == "--male-bow-witness") {
                0
            } else {
                1
            };
            session.speed = 1.0;
            session.zoom = 17.0;
            for u in &mut session.battle.units {
                u.position = [
                    if u.team == 0 { -2.5 } else { 2.5 },
                    (u.id % 3) as f32 * 1.3 - 1.3,
                ];
                u.target = [if u.team == 0 { 2.5 } else { -2.5 }, u.position[1]];
            }
            session.battle.units[0].health = 25.;
            session.remember_positions();
            if args.iter().any(|a| a == "--bow-draw-witness") {
                for _ in 0..100 {
                    session.battle.step();
                    if session.battle.units[session.selected]
                        .windup
                        .as_ref()
                        .is_some_and(|w| w.weapon == W::Bow && w.remaining < 0.3)
                    {
                        break;
                    }
                }
                session.remember_positions();
            }
            if args
                .iter()
                .any(|a| a == "--weapon-loadout-witness" || a == "--bow-draw-witness")
            {
                session.running = false;
                session.camera_follow = true;
                session.zoom = if bow { 3.8 } else { 4.8 };
                session.orbit = 1.1;
                session.pitch = 0.18;
            }
            if args.iter().any(|a| a == "--weapon-workshop-witness") {
                apply(Control::Camp(camp::Page::Workshop), &mut session);
            }
            if args.iter().any(|a| a == "--weapon-report-witness") {
                while !session.battle.finished {
                    session.battle.step();
                }
                session.settle_finished();
                session.save_notice =
                    "Staged QA match: weapon practice and recipe discovery verified.".into();
                apply(Control::Camp(camp::Page::Report), &mut session);
            }
        }

        if args.iter().any(|a| a == "--refinement-witness") {
            let field = phalanx::pressure::generate(&session.campaign.teams, seed);
            session.restart_founders(seed, field, Controller::Utility);
            session.menu = false;
            session.running = true;
            session.speed = 2.0;
            session.zoom = 25.0;
        }

        if args.iter().any(|a| a == "--face-witness") {
            session.running = false;
            if std::env::var_os("PHALANX_WITNESS_CLOSE").is_some() {
                session.camera_follow = true;
                session.zoom = 3.3;
                session.pitch = 0.12;
                session.orbit = 1.35;
            }
            session.selected = std::env::var("PHALANX_FACE_UNIT")
                .ok()
                .and_then(|id| id.parse::<usize>().ok())
                .filter(|id| *id < session.battle.units.len())
                .unwrap_or(0);
        }
        if std::env::var_os("PHALANX_WITNESS_CLOSE").is_some() {
            let unit = &mut session.battle.units[session.selected];
            unit.target = [unit.position[0] + 1.0, unit.position[1]];
        }
        if let Ok(name) = std::env::var("PHALANX_EQUIPMENT_CLASS")
            && let Some(class) = FighterClass::ALL
                .into_iter()
                .find(|c| c.name().eq_ignore_ascii_case(&name))
        {
            session.battle.units[session.selected].class = Some(class);
            session.battle.units[session.selected].role = class.role();
        }
        if args.iter().any(|a| a == "--healing-witness") {
            session.autosave = false;
            session.speed = 1.0;
            session.selected = 1;
            session.campaign.teams[0].gold = 200;
            session.campaign.teams[0].salvage = 100;
            apply(Control::Class(FighterClass::Healer), &mut session);
            if args.iter().any(|a| a == "--hybrid-witness") {
                session.campaign.teams[0].fighters[1]
                    .mastery
                    .train(FighterClass::Healer, 100);
                session.campaign.teams[0].fighters[1]
                    .mastery
                    .train(FighterClass::BannerBearer, 100);
                session.battle.units[1].mastery = session.campaign.teams[0].fighters[1].mastery;
                apply(Control::Class(FighterClass::RockThrower), &mut session);
                apply(Control::Title(CommandTitle::Captain), &mut session);
            }
            for gear in phalanx::equipment::Gear::ALL {
                apply(Control::Workshop(gear), &mut session);
            }
            for unit in &mut session.battle.units {
                unit.position = if unit.team == 0 {
                    [-4.0 + unit.id as f32 * 1.1, 0.0]
                } else {
                    [12.0, 8.0 + (unit.id % 3) as f32]
                };
                unit.target = unit.position;
            }
            if args.iter().any(|a| a == "--hybrid-witness") {
                for unit in session.battle.units.iter_mut().filter(|u| u.team == 1) {
                    unit.position = [2.0 + (unit.id % 3) as f32 * 0.8, 0.0];
                    unit.target = unit.position;
                    unit.class = Some(FighterClass::Guardian);
                    unit.role = Role::Fighter;
                    unit.health = unit.max_health();
                    unit.cooldown = 100.0;
                }
            }
            session.battle.units[0].health = 20.0;
            session.remember_positions();
        }
        if args.iter().any(|a| a == "--camp-witness") {
            session.autosave = false;
            session.campaign = Campaign::new();
            session
                .campaign
                .set_class(0, 1, FighterClass::Healer)
                .unwrap();
            session.restart_founders(42, Arena::OpenField, Controller::Utility);
            while !session.battle.finished {
                session.battle.step();
            }
            session.settle_finished();
            session.selected = 1;
            apply(
                Control::Workshop(phalanx::equipment::Gear::BalancedWeapon),
                &mut session,
            );
            if args.iter().any(|a| a == "--rival-witness") {
                apply(Control::Camp(camp::Page::Rivals), &mut session);
            }
            if args.iter().any(|a| a == "--workshop-witness") {
                apply(Control::Camp(camp::Page::Workshop), &mut session);
            }
        }
        if args.iter().any(|a| a == "--standard-witness") {
            session.autosave = false;
            session.running = false;
            session.selected = 2;
            session.camera_follow = true;
            session.zoom = 5.3;
            session.orbit = if args.iter().any(|a| a == "--standard-back-witness") {
                4.2
            } else {
                1.1
            };
            session.pitch = 0.22;
            session.battle.units[2].target = [
                session.battle.units[2].position[0] + 1.0,
                session.battle.units[2].position[1],
            ];
        }
        if args.iter().any(|a| a == "--menu-witness") {
            session.menu = true;
            session.ladder = args.iter().any(|a| a == "--ladder-witness");
            session.running = false;
        }
    }
    if args.iter().any(|a| a == "--rpg-witness") {
        use phalanx::equipment::{Gear, Item};
        session.autosave = false;
        session.campaign = Campaign::new();
        session.campaign.teams[0].gold = 8000;
        session.campaign.teams[0].salvage = 4000;
        let f = &mut session.campaign.teams[0].fighters[1];
        f.class = FighterClass::Warrior;
        f.mastery.train(FighterClass::Healer, 300);
        f.mastery.train(FighterClass::BannerBearer, 100);
        f.mastery.train(FighterClass::Ranger, 100);
        f.lost_limbs = 1 | 8;
        f.prosthetics = 8;
        for gear in [Gear::Bow, Gear::TrailArmor, Gear::WarBanner] {
            session.campaign.workshop(0, 1, gear).unwrap();
        }
        session
            .campaign
            .craft_item(0, 1, Gear::TemperedWeapon)
            .unwrap();
        session.campaign.teams[0].inventory =
            vec![Item::new(Gear::IronArmor), Item::new(Gear::VitalityCharm)];
        session.campaign.teams[0].fighters[1].equipment.items[3]
            .as_mut()
            .unwrap()
            .level = 5;
        session.restart_founders(seed, Arena::OpenField, Controller::Utility);
        session.selected = args
            .windows(2)
            .find(|w| w[0] == "--rpg-member-witness")
            .and_then(|w| w[1].parse::<usize>().ok())
            .unwrap_or(1)
            .min(session.battle.units.len() - 1);
        session.menu = true;
        session.running = false;
        session.camp_page = Some(if args.iter().any(|a| a == "--skills-witness") {
            camp::Page::Skills
        } else if args.iter().any(|a| a == "--forge-witness") {
            camp::Page::Forge
        } else {
            camp::Page::Workshop
        });
        session.refresh_preview();
    }
    if let Some(path) = args
        .windows(2)
        .find(|w| w[0] == "--snapshot")
        .map(|w| &w[1])
    {
        apply(Control::Camp(camp::Page::Challenges), &mut session);
        exchange::import_file(&mut session, std::path::Path::new(path));
    }
    if args.iter().any(|a| a == "--snapshot-witness") {
        session.autosave = false;
        session.campaign = Campaign::new();
        session.campaign.teams[1].name = "The Visiting Phalanx".into();
        session.restart_founders(seed, Arena::OpenField, Controller::Utility);
        let packet =
            phalanx::challenges::TeamSnapshot::from_campaign(&session.campaign, 1).unwrap();
        let path = std::path::Path::new("outputs/challenges/inbox/visiting-team.json");
        packet.save(path).unwrap();
        apply(Control::Camp(camp::Page::Challenges), &mut session);
        apply(Control::ImportSnapshot(0), &mut session);
        if args.iter().any(|a| a == "--snapshot-report-witness") {
            apply(Control::StartChallenge, &mut session);
            while !session.battle.finished {
                session.battle.step();
            }
            session.settle_finished();
            apply(Control::ExportChallengeResult, &mut session);
        }
    }
    if args.iter().any(|a| a == "--battlefield-witness") {
        session.autosave = false;
        session.campaign = Campaign::new();
        if args.iter().any(|a| a == "--league-four") {
            session.campaign.teams[0].gold = 1000;
            session.campaign.unlock_fourth().unwrap();
        }
        session.restart_founders(seed.wrapping_sub(1), Arena::OpenField, Controller::Utility);
        apply(Control::GenerateArena, &mut session);
    }
    if args.iter().any(|a| a == "--player-demo") {
        session.autosave = false;
        session.campaign = Campaign::new();
        session.restart_founders(seed.wrapping_sub(1), Arena::OpenField, Controller::Utility);
        session.founding = founding::Draft::new(&session.campaign, true);
        session.menu = true;
        session.running = false;
        session.speed = 1.;
    }
    std::fs::create_dir_all("outputs").expect("create log directory");
    let log_path = format!(
        "outputs/disagreements-{}.jsonl",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    session.disagreement_log = Some(std::io::BufWriter::new(
        std::fs::File::create(&log_path).expect("create disagreement log"),
    ));
    if let Some(log) = &mut session.disagreement_log {
        use std::io::Write;
        writeln!(log,"{}",serde_json::json!({"event":"session","schema_version":1,"simulation_version":env!("CARGO_PKG_VERSION"),"actions":ACTIONS,"policy_source_sha256":session.battle.policy.as_ref().map(|p|&p.source_sha256)})).expect("write log header");
    }
    info!("Live controller disagreement log: {log_path}");
    let mut app = App::new();
    if args.iter().any(|a| a == "--battle") {
        session.menu = false;
    }
    app.add_plugins((
        visuals::BattleVisuals,
        audio::GameAudio,
        front::FrontEnd,
        presentation::PresentationGuard,
    ))
    .add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    power_preference: PowerPreference::HighPerformance,
                    instance_flags: presentation::player_instance_flags(
                        WgpuSettings::default().instance_flags,
                    ),
                    adapter_name: std::env::var("WGPU_ADAPTER_NAME").ok(),
                    ..default()
                }
                .into(),
                ..default()
            })
            .set(AssetPlugin {
                file_path: if std::path::Path::new("assets/characters").exists() {
                    std::env::current_dir()
                        .unwrap()
                        .join("assets")
                        .to_string_lossy()
                        .into_owned()
                } else {
                    "assets".into()
                },
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: if visual_witness {
                        "PHALANX | Visual QA".into()
                    } else {
                        "PHALANX | Minds x Terrain".into()
                    },
                    resolution: (1440, 900).into(),
                    visible: !visual_witness && !args.iter().any(|a| a == "--memory-soak"),
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.055)))
    .insert_resource(session)
    .add_plugins((
        camp::Camp,
        rpg::Rpg,
        exchange::Exchange,
        battlefield::Battlefield,
        player::Player,
        founding::Founding,
    ))
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            controls,
            keyboard,
            scroll_inspector,
            simulate,
            settle_progression,
            sync_team_colors,
            log_disagreements,
            sync_terrain,
            sync_bridges,
            sync_rocks,
            sync_pawns,
            sync_insignia,
            pick,
            camera_motion,
            overlays,
            update_text,
            update_minds,
            update_workshop,
            update_status,
            update_divergence,
        )
            .chain(),
    );
    if visual_witness || args.iter().any(|a| a == "--memory-soak") {
        app.insert_resource(bevy::winit::WinitSettings::continuous());
    }
    if args.iter().any(|a| a == "--profile") {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin {
                filter: Some(
                    [
                        FrameTimeDiagnosticsPlugin::FPS,
                        FrameTimeDiagnosticsPlugin::FRAME_TIME,
                    ]
                    .into_iter()
                    .collect(),
                ),
                ..default()
            },
        ));
    }
    app.run();
}
fn font(size: f32) -> TextFont {
    TextFont {
        font_size: FontSize::Px(size),
        ..default()
    }
}
fn button(parent: &mut ChildSpawnerCommands, title: &str, control: Control) {
    parent
        .spawn((
            Button,
            PressLatch::default(),
            control,
            Node {
                padding: UiRect::axes(px(12), px(6)),
                margin: UiRect::all(px(3)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.18, 0.25)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(title),
                font(15.0),
                TextColor(if matches!(control, Control::TeamColor(_)) {
                    Color::srgb(0.03, 0.05, 0.07)
                } else {
                    Color::srgb(0.85, 0.91, 0.96)
                }),
            ));
        });
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    session: Res<Session>,
) {
    commands.spawn((
        GroundArena(session.battle.arena),
        Mesh3d(meshes.add(terrain_mesh(session.battle.arena))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::default(),
    ));
    let stone_wall = materials.add(Color::srgb(0.43, 0.46, 0.46));
    let timber_wall = materials.add(Color::srgb(0.39, 0.23, 0.11));
    commands.insert_resource(battlefield::WallMaterials {
        stone: stone_wall.clone(),
        timber: timber_wall,
    });
    for id in 0..phalanx::procedural::MAX_WALLS {
        commands.spawn((
            ChokeWall(id),
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(stone_wall.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    let deck_material = materials.add(Color::srgb(0.48, 0.38, 0.24));
    let rail_material = materials.add(Color::srgb(0.62, 0.68, 0.72));
    for z in [-6.0, 6.0] {
        commands.spawn((
            Bridge,
            Mesh3d(meshes.add(Cuboid::new(16.0, 0.22, 2.5))),
            MeshMaterial3d(deck_material.clone()),
            Transform::from_xyz(0.0, 3.09, z),
            Visibility::Hidden,
        ));
        for sign in [-1.0_f32, 1.0] {
            commands.spawn((
                Bridge,
                Mesh3d(meshes.add(Cuboid::new((4.0_f32 * 4.0 + 3.2 * 3.2).sqrt(), 0.16, 2.5))),
                MeshMaterial3d(deck_material.clone()),
                Transform::from_xyz(sign * 10.0, 1.52, z)
                    .with_rotation(Quat::from_rotation_z(-sign * 0.8_f32.atan())),
                Visibility::Hidden,
            ));
            commands.spawn((
                Bridge,
                Mesh3d(meshes.add(Cuboid::new(16.0, 0.25, 0.12))),
                MeshMaterial3d(rail_material.clone()),
                Transform::from_xyz(0.0, 3.45, z + sign * 1.12),
                Visibility::Hidden,
            ));
            let base = Arena::Citadel.ground_elevation([sign * 6.0, z]);
            let height = 3.0 - base;
            commands.spawn((
                Bridge,
                Mesh3d(meshes.add(Cuboid::new(0.5, height, 0.5))),
                MeshMaterial3d(rail_material.clone()),
                Transform::from_xyz(sign * 6.0, base + height * 0.5, z),
                Visibility::Hidden,
            ));
        }
    }
    let stone = meshes.add(Sphere::new(0.16));
    let shaft = meshes.add(Cuboid::new(0.035, 0.035, 1.15));
    let point = meshes.add(Cone::new(0.07, 0.22));
    let feather = meshes.add(Cuboid::new(0.16, 0.015, 0.18));
    let arrow_shaft = meshes.add(Cuboid::new(0.018, 0.018, 1.0));
    let arrow_tip = meshes.add(Cone::new(0.035, 0.10));
    let arrow_feather = meshes.add(Cuboid::new(0.09, 0.01, 0.15));
    let arrow_color = materials.add(Color::srgb(0.18, 0.65, 0.25));
    let stone_mat = materials.add(Color::srgb(0.70, 0.68, 0.60));
    let wood_mat = materials.add(Color::srgb(0.38, 0.18, 0.06));
    let steel_mat = materials.add(Color::srgb(0.72, 0.78, 0.82));
    let feather_mat = materials.add(Color::srgb(0.75, 0.16, 0.06));
    for weapon in phalanx::weapons::ProjectileWeapon::ALL {
        for slot in 0..32 {
            let mut entity = commands.spawn((
                RockVisual { slot, weapon },
                Transform::default(),
                Visibility::Hidden,
            ));
            if weapon == phalanx::weapons::ProjectileWeapon::Rock {
                entity.insert((Mesh3d(stone.clone()), MeshMaterial3d(stone_mat.clone())));
            } else if weapon == phalanx::weapons::ProjectileWeapon::Bow {
                entity.with_children(|p| {
                    p.spawn((
                        Mesh3d(arrow_shaft.clone()),
                        MeshMaterial3d(wood_mat.clone()),
                        Transform::default(),
                    ));
                    p.spawn((
                        Mesh3d(arrow_tip.clone()),
                        MeshMaterial3d(steel_mat.clone()),
                        Transform::from_xyz(0., 0., 0.53)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    ));
                    for angle in [0., std::f32::consts::FRAC_PI_2] {
                        p.spawn((
                            Mesh3d(arrow_feather.clone()),
                            MeshMaterial3d(arrow_color.clone()),
                            Transform::from_xyz(0., 0., -0.4)
                                .with_rotation(Quat::from_rotation_z(angle)),
                        ));
                    }
                });
            } else {
                entity.with_children(|p| {
                    p.spawn((
                        Mesh3d(shaft.clone()),
                        MeshMaterial3d(wood_mat.clone()),
                        Transform::default(),
                    ));
                    p.spawn((
                        Mesh3d(point.clone()),
                        MeshMaterial3d(steel_mat.clone()),
                        Transform::from_xyz(0., 0., 0.68)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    ));
                    if matches!(
                        weapon,
                        phalanx::weapons::ProjectileWeapon::Atlatl
                            | phalanx::weapons::ProjectileWeapon::Bow
                    ) {
                        for angle in [0., std::f32::consts::FRAC_PI_2] {
                            p.spawn((
                                Mesh3d(feather.clone()),
                                MeshMaterial3d(feather_mat.clone()),
                                Transform::from_xyz(0., 0., -0.48)
                                    .with_rotation(Quat::from_rotation_z(angle)),
                            ));
                        }
                    }
                });
            }
        }
    }
    commands.spawn((
        Slab,
        Mesh3d(meshes.add(Cuboid::new(22.4, 0.5, 16.4))),
        MeshMaterial3d(materials.add(Color::srgb(0.07, 0.11, 0.15))),
        Transform::from_xyz(0.0, -0.3, 0.0),
    ));
    let head = meshes.add(Sphere::new(0.2));
    let base = meshes.add(Cylinder::new(0.43, 0.09));
    let red = materials.add(Color::srgb(0.92, 0.28, 0.25));
    let blue = materials.add(Color::srgb(0.2, 0.58, 0.94));
    let gold = materials.add(Color::srgb(1.0, 0.72, 0.08));
    let silver = materials.add(Color::srgb(0.88, 0.93, 1.0));
    let badge_background = materials.add(Color::srgb(0.04, 0.07, 0.1));
    let crown_band = meshes.add(Cylinder::new(0.25, 0.10));
    let crown_tip = meshes.add(Cone::new(0.07, 0.17));
    let patch = meshes.add(Cuboid::new(0.33, 0.32, 0.025));
    let stripe = meshes.add(Cuboid::new(0.16, 0.045, 0.035));
    commands.insert_resource(PawnAssets {
        red: red.clone(),
        blue: blue.clone(),
        gold: gold.clone(),
        silver: silver.clone(),
    });
    for id in 0..20 {
        let material = if id < 5 { red.clone() } else { blue.clone() };
        commands
            .spawn((Pawn(id), Transform::default(), Visibility::Inherited))
            .with_children(|p| {
                p.spawn((
                    Insignia(id, true),
                    Mesh3d(crown_band.clone()),
                    MeshMaterial3d(gold.clone()),
                    Transform::from_xyz(0.0, 0.71, 0.0),
                    Visibility::Hidden,
                ));
                for point in 0..5 {
                    let angle = point as f32 * std::f32::consts::TAU / 5.0;
                    p.spawn((
                        Insignia(id, true),
                        Mesh3d(crown_tip.clone()),
                        MeshMaterial3d(gold.clone()),
                        Transform::from_xyz(angle.cos() * 0.19, 0.81, angle.sin() * 0.19),
                        Visibility::Hidden,
                    ));
                }
                p.spawn((
                    Insignia(id, false),
                    Mesh3d(patch.clone()),
                    MeshMaterial3d(badge_background.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.30),
                    Visibility::Hidden,
                ));
                for row in [-0.07_f32, 0.06] {
                    for sign in [-1.0_f32, 1.0] {
                        p.spawn((
                            Insignia(id, false),
                            Mesh3d(stripe.clone()),
                            MeshMaterial3d(gold.clone()),
                            Transform::from_xyz(sign * 0.06, row, 0.32)
                                .with_rotation(Quat::from_rotation_z(sign * 0.6)),
                            Visibility::Hidden,
                        ));
                    }
                }
                {
                    p.spawn((
                        PawnPart(id, true),
                        Mesh3d(head.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_xyz(0.0, 0.45, 0.0),
                    ));
                }
                p.spawn((
                    PawnPart(id, false),
                    Mesh3d(base.clone()),
                    MeshMaterial3d(material),
                    Transform::from_xyz(0.0, -0.57, 0.0),
                ));
            });
    }
    commands.spawn((
        bevy::camera::visibility::RenderLayers::layer(0).with(1),
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_xyz(-6.0, 12.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        IsDefaultUiCamera,
        Transform::from_xyz(0.0, 21.0, 24.0).looking_at(Vec3::new(3.5, 0.0, 0.0), Vec3::Y),
    ));
    commands
        .spawn((
            player::ResearchHud,
            Node {
                position_type: PositionType::Absolute,
                top: px(22),
                left: px(26),
                right: px(370),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("PHALANX"),
                font(32.0),
                TextColor(Color::srgb(0.91, 0.94, 0.96)),
            ));
            p.spawn((
                Text::new("AUTONOMOUS COMBAT / MINDS x TERRAIN"),
                font(13.0),
                TextColor(Color::srgb(0.45, 0.65, 0.71)),
            ));
            p.spawn((
                Status,
                Text::new(""),
                font(17.0),
                Node {
                    margin: UiRect::top(px(12)),
                    ..default()
                },
            ));
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16),
                top: px(16),
                bottom: px(90),
                width: px(330),
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(18)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.055, 0.085, 0.97)),
            InspectorPanel,
            player::ResearchHud,
            ScrollPosition::default(),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("AGENT INSPECTOR"),
                font(20.0),
                TextColor(Color::srgb(0.65, 0.85, 0.88)),
            ));
            p.spawn((
                Inspector,
                Text::new(""),
                font(12.0),
                Node {
                    margin: UiRect::top(px(14)),
                    ..default()
                },
            ));
            p.spawn((
                Text::new("LOOK | Body / Hair / Face / Eyes / Nose / Skin / Dye"),
                font(12.0),
                Node {
                    margin: UiRect::top(px(10)),
                    ..default()
                },
            ));
            p.spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|row| {
                for (i, label) in ["Body", "Hair", "Face", "Eyes", "Nose", "Skin", "Dye"]
                    .iter()
                    .enumerate()
                {
                    button(row, label, Control::Appearance(i));
                }
                button(
                    row,
                    "Craft iron I (15 gold + 5 salvage)",
                    Control::Prosthetic,
                );
                for gear in phalanx::equipment::Gear::ALL {
                    let (gold, salvage) = gear.cost();
                    button(
                        row,
                        &format!("{} ({gold}g/{salvage}s) | {}", gear.name(), gear.effect()),
                        Control::Workshop(gear),
                    );
                }
                for class in FighterClass::ALL {
                    button(row, class.name(), Control::Class(class));
                }
            });
            p.spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|row| {
                button(row, "Aggressive", Control::Aggressive);
                button(row, "Disciplined", Control::Disciplined);
            });
            for (i, name) in [
                "Aggression",
                "Caution",
                "Courage",
                "Teamwork",
                "Patience",
                "Self-preservation",
                "Discipline",
            ]
            .iter()
            .enumerate()
            {
                p.spawn(Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        TraitValue(i),
                        Text::new(*name),
                        font(13.0),
                        Node {
                            width: px(165),
                            ..default()
                        },
                    ));
                    button(row, "-", Control::Trait(i, -0.1));
                    button(row, "+", Control::Trait(i, 0.1));
                });
            }
        });
    commands
        .spawn((
            player::ResearchHud,
            Node {
                position_type: PositionType::Absolute,
                left: px(26),
                top: px(230),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Start,
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                DivergenceSummary,
                Text::new(""),
                font(13.0),
                TextColor(Color::srgb(0.6, 0.78, 0.82)),
            ));
            for rank in 0..3 {
                p.spawn((
                    Button,
                    PairRow(rank),
                    Control::Pair(ACTION_COUNT, ACTION_COUNT),
                    PressLatch::default(),
                    BackgroundColor(Color::srgb(0.12, 0.18, 0.25)),
                    Node {
                        padding: UiRect::axes(px(5), px(2)),
                        margin: UiRect::top(px(2)),
                        display: Display::None,
                        ..default()
                    },
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new(""),
                        font(13.0),
                        TextColor(Color::srgb(0.85, 0.91, 0.96)),
                    ));
                });
            }
        });
    commands.spawn((player::ResearchHud, Node { position_type: PositionType::Absolute, left: px(20), bottom: px(18), flex_direction: FlexDirection::Column, ..default() })).with_children(|p| {
        p.spawn((Text::new("WASD move | RMB orbit | MMB pan | Wheel zoom | Shift faster | F follow | Home arena | Space pause/run"),font(13.0),TextColor(Color::srgb(0.6,0.72,0.78))));
        p.spawn(Node { margin: UiRect::top(px(8)),flex_wrap:FlexWrap::Wrap,width:vw(95.0), ..default() }).with_children(|row| {
            button(row,"Player view [I]",Control::ResearchView);button(row,"Menu",Control::Menu);button(row,"Squad workshop",Control::Camp(camp::Page::Workshop));button(row,"Battle report",Control::Camp(camp::Page::Report));button(row,"Run",Control::Run); button(row,"Pause",Control::Pause); button(row,"Step",Control::Step); button(row,"Reset",Control::Reset); button(row,"Swap profiles",Control::Swap); button(row,"Speed",Control::Speed); button(row,"Paths",Control::Paths); button(row,"Arena",Control::Arena); button(row,"Map briefing",Control::BattlefieldBriefing); button(row,"Overlay",Control::Overlay); button(row,"Utility/Learned",Control::Controller); button(row,"Show all",Control::ClearPair); button(row,"Founders/Lab",Control::Setup); button(row,"Next battle",Control::NextBattle);button(row,"Save",Control::Save);button(row,"Load",Control::Load);
            for (i,name) in ["Amber","Teal","Violet","Coral","Azure","Lime","Pink","Ivory"].iter().enumerate() {button(row,name,Control::TeamColor(i));}
        });
    });
}
fn apply(control: Control, session: &mut Session) {
    if session.founding.open
        && !matches!(
            control,
            Control::StarterClass(..)
                | Control::ClearPhalanxName
                | Control::ConfirmFounders
                | Control::Quit
        )
    {
        return;
    }
    debug!(?control, "Battle control");
    session.settle_finished();
    if session.exchange.live.is_some() {
        if matches!(control, Control::Reset) {
            exchange::reset(session);
            return;
        }
        if !matches!(
            control,
            Control::Menu
                | Control::Camp(_)
                | Control::SelectFighter(_)
                | Control::Continue
                | Control::Quit
                | Control::Run
                | Control::Pause
                | Control::Step
                | Control::Speed
                | Control::Paths
                | Control::Overlay
                | Control::BattlefieldBriefing
                | Control::ResearchView
                | Control::TogglePlay
                | Control::RosterSide(_)
                | Control::RosterPage(_)
                | Control::FollowFighter
                | Control::FrameBattle
                | Control::Pair(..)
                | Control::ClearPair
                | Control::NextMatch
                | Control::ExportChallengeResult
                | Control::LeaveChallenge
        ) {
            session.save_notice = "Challenge teams are frozen. Return to your campaign to prepare fighters or choose another match.".into();
            return;
        }
    }
    let controller = session.battle.controller;
    let was_founders = session.battle.founders;
    if matches!(
        control,
        Control::Reset | Control::Swap | Control::Arena | Control::Setup
    ) && was_founders
    {
        let _ = session.campaign.capture_minds(&session.battle);
    }
    match control {
        Control::StarterClass(slot, class) => {
            if session.founding.open && slot < 3 {
                session.founding.classes[slot] = class;
            }
        }
        Control::ClearPhalanxName => {
            if session.founding.open {
                session.founding.name.clear();
            }
        }
        Control::ConfirmFounders => {
            if !session.founding.open {
                return;
            }
            match session
                .campaign
                .choose_founders(&session.founding.name, session.founding.classes)
            {
                Ok(()) => {
                    session.founding.open = false;
                    // Refresh the staged snapshot before the ordinary tutorial flow captures minds.
                    session.restart_founders(session.battle.seed, session.battle.arena, controller);
                    apply(Control::NextTutorial, session);
                }
                Err(e) => session.save_notice = e,
            }
        }
        Control::NextTutorial => {
            if !session.battle.founders || (session.battle.tick > 0 && !session.battle.finished) {
                session.save_notice = "Finish this battle before starting another tutorial".into();
                return;
            }
            if session.battle.tick == 0 {
                let _ = session.campaign.capture_minds(&session.battle);
            }
            match phalanx::tutorial::prepare_next(&mut session.campaign) {
                Ok(lesson) => {
                    let seed = session.battle.seed.wrapping_add(1);
                    let arena = phalanx::pressure::generate(&session.campaign.teams, seed);
                    session.restart_founders(seed, arena, controller);
                    session.running = false;
                    session.accumulator = 0.;
                    session.menu = false;
                    session.camp_page = None;
                    session.map_briefing = false;
                    session.paths = 2;
                    session.save_notice = format!("{}: {}", lesson.title, lesson.advice);
                    if session.autosave {
                        session.save_campaign();
                    }
                }
                Err(e) => session.save_notice = e,
            }
        }
        Control::TogglePlay => session.running = !session.running && !session.battle.finished,
        Control::ResearchView => session.player.research = !session.player.research,
        Control::RosterSide(side) => {
            session.player.side = side.min(1);
            session.player.page = 0;
        }
        Control::RosterPage(delta) => {
            let pages = player::page_count(session.battle.team_size());
            session.player.page =
                (session.player.page as i32 + delta).clamp(0, pages as i32 - 1) as usize;
        }
        Control::FollowFighter => {
            session.camera_follow = true;
            session.zoom = 12.;
            session.pitch = 0.7;
        }
        Control::FrameBattle => {
            session.camera_follow = false;
            session.camera_target = Vec3::new(3.5, 0., 0.);
            session.zoom = frame_zoom(session.battle.arena);
            session.pitch = 0.72;
        }
        Control::BattleStyle(style) => {
            if !session.battle.founders || (session.battle.tick > 0 && !session.battle.finished) {
                session.save_notice = "Choose a fighting style before or after a battle".into();
                return;
            }
            let u = &session.battle.units[session.selected];
            let (team, slot) = (u.team, u.id % session.battle.team_size());
            session.campaign.teams[team].fighters[slot].mind = style.mind();
            if session.battle.tick == 0 {
                session.battle.units[session.selected].mind = style.mind();
            }
            session.refresh_preview();
            if session.autosave {
                session.save_campaign();
            }
            session.save_notice = format!("{}: {}", style.name(), style.description());
        }
        Control::Camp(page) => {
            if session.battle.founders {
                session.menu = true;
                session.running = false;
                session.ladder = false;
                session.camp_page = Some(page);
                if page == camp::Page::Challenges {
                    exchange::refresh_files(session);
                }
            } else {
                session.save_notice =
                    "Squad workshop and reports belong to Founders campaigns".into();
            }
        }
        Control::SelectFighter(id) => {
            if id < session.battle.units.len() {
                session.selected = id;
                session.map_briefing = false;
                session.player.side = session.battle.units[id].team;
                session.player.page = (id % session.battle.team_size()) / player::CARDS_PER_PAGE;
            }
        }
        Control::ExportSnapshot => exchange::export_team(session),
        Control::RefreshSnapshots => exchange::refresh_files(session),
        Control::ImportSnapshot(index) => {
            if let Some(path) = session.exchange.files.get(index).cloned() {
                exchange::import_file(session, &path);
            }
        }
        Control::StartChallenge => exchange::start(session),
        Control::ChallengeSeed => session.exchange.seed = session.exchange.seed.wrapping_add(1),
        Control::ExportChallengeResult => exchange::export_result(session),
        Control::LeaveChallenge => exchange::leave(session),
        Control::ChooseRival(index) => {
            if !session.battle.founders || (session.battle.tick > 0 && !session.battle.finished) {
                session.save_notice = "Finish the battle before choosing a rival".into();
                return;
            }
            match session.campaign.select_rival(index) {
                Ok(()) => {
                    let seed = session.battle.seed.wrapping_add(1);
                    let arena = phalanx::pressure::generate(&session.campaign.teams, seed);
                    session.restart_founders(seed, arena, controller);
                    session.running = false;
                    session.accumulator = 0.0;
                    if session.autosave {
                        session.save_campaign();
                    }
                    session.save_notice = format!(
                        "Opponent: {}. Their fighters and history persist between matches.",
                        session.campaign.teams[1].name
                    );
                }
                Err(e) => session.save_notice = e,
            }
        }
        Control::Ladder => session.ladder = !session.ladder,
        Control::Recruit => {
            if !session.battle.founders || (session.battle.tick != 0 && !session.battle.finished) {
                session.save_notice = "Finish or reset your battle before recruiting".into();
                return;
            }
            match session.campaign.unlock_fourth() {
                Ok(()) => {
                    session.restart_founders(
                        session.battle.seed.wrapping_add(1),
                        session.battle.arena,
                        controller,
                    );
                    session.previous_positions =
                        session.battle.units.iter().map(|u| u.position).collect();
                    session.selected = 0;
                    session.accumulator = 0.;
                    session.run_id += 1;
                    session.running = false;
                    if session.autosave {
                        session.save_campaign();
                    }
                    session.save_notice =
                        "League 4 unlocked: a new Warrior joins each side. 1000 gold spent.".into();
                }
                Err(e) => session.save_notice = e,
            }
        }
        Control::Menu => {
            session.camp_page = None;
            session.menu = true;
            session.running = false;
        }
        Control::Continue => {
            session.camp_page = None;
            session.menu = false;
        }
        Control::Quit => {} // handled by the front-end AppExit writer
        Control::FlagEmblem
        | Control::FlagAccent
        | Control::FlagPrimary
        | Control::StandardBearer => {
            if !session.battle.founders || (session.battle.tick > 0 && !session.battle.finished) {
                session.save_notice =
                    "Prepare your standard before or after a Founders battle".into();
                return;
            }
            let u = &session.battle.units[session.selected];
            let (team, slot, size) = (
                u.team,
                u.id % session.battle.team_size(),
                session.battle.team_size(),
            );
            let squad = &mut session.campaign.teams[team];
            match control {
                Control::FlagEmblem => squad.heraldry.emblem = squad.heraldry.emblem.next(),
                Control::FlagPrimary => {
                    let index = PALETTE
                        .iter()
                        .position(|c| *c == squad.color)
                        .map(|i| (i + 1) % PALETTE.len())
                        .unwrap_or(0);
                    squad.color = PALETTE[index];
                }
                Control::FlagAccent => {
                    let index = PALETTE
                        .iter()
                        .position(|c| *c == squad.heraldry.accent)
                        .map(|i| (i + 1) % PALETTE.len())
                        .unwrap_or(0);
                    squad.heraldry.accent = PALETTE[index];
                }
                _ => squad.standard_bearer = slot,
            }
            if session.battle.tick == 0 {
                let bearer = squad.standard_bearer;
                for u in session.battle.units.iter_mut().filter(|u| u.team == team) {
                    u.carries_standard = u.id % size == bearer;
                }
            }
            if session.autosave {
                session.save_campaign();
            }
            session.save_notice =
                "Standard updated: every squad has a flag; learned rally skills remain separate."
                    .into();
        }
        Control::Title(title) => {
            if !session.battle.founders || (session.battle.tick != 0 && !session.battle.finished) {
                session.save_notice = "Assign titles before or after a Founders battle".into();
                return;
            }
            let u = &session.battle.units[session.selected];
            let (team, slot) = (u.team, u.id % session.battle.team_size());
            match session.campaign.assign_title(team, slot, title) {
                Ok(()) => {
                    if session.battle.tick == 0 {
                        let size = session.battle.team_size();
                        for u in session.battle.units.iter_mut().filter(|u| u.team == team) {
                            u.command_title =
                                Some(session.campaign.teams[team].fighters[u.id % size].title);
                            u.health = u.max_health();
                        }
                    }
                    if session.autosave {
                        session.save_campaign();
                    }
                    session.save_notice = format!(
                        "Assigned {}: {}. Occupied titles swap holders; discipline and mastery stay with each fighter.",
                        title.name(),
                        title.description()
                    );
                }
                Err(e) => session.save_notice = e,
            }
        }
        Control::Class(class) => {
            if !session.battle.founders || (session.battle.tick != 0 && !session.battle.finished) {
                session.save_notice = "Retrain before or after a Founders battle".into();
                return;
            }
            let u = &session.battle.units[session.selected];
            let (team, slot) = (u.team, u.id % session.battle.team_size());
            match session.campaign.set_class(team, slot, class) {
                Ok(()) => {
                    if session.battle.tick == 0 {
                        let u = &mut session.battle.units[session.selected];
                        u.class = Some(class);
                        u.skills = session.campaign.teams[team].fighters[slot].skills;
                        u.reset_weapon();
                        u.health = u.max_health();
                    }
                    if session.autosave {
                        session.save_campaign();
                    }
                    session.save_notice =
                        format!("{}: {} (10 gold)", class.name(), class.description());
                }
                Err(e) => session.save_notice = e,
            }
        }
        Control::Workshop(gear) => {
            if session.battle.founders && (session.battle.tick == 0 || session.battle.finished) {
                let u = &session.battle.units[session.selected];
                let (team, slot) = (u.team, u.id % session.battle.team_size());
                match session.campaign.workshop(team, slot, gear) {
                    Ok(message) => {
                        if session.battle.tick == 0 {
                            let equipment = session.campaign.teams[team].fighters[slot].equipment;
                            let u = &mut session.battle.units[session.selected];
                            u.equipment = equipment;
                            u.reset_weapon();
                            u.health = u.max_health();
                        }
                        if session.autosave {
                            session.save_campaign();
                        }
                        session.save_notice = format!("{message}. Ready next battle.");
                    }
                    Err(e) => session.save_notice = e,
                }
            } else {
                session.save_notice = "Use the workshop before or after a Founders battle".into();
            }
        }
        Control::Prosthetic => {
            if session.battle.founders && (session.battle.tick == 0 || session.battle.finished) {
                let u = &session.battle.units[session.selected];
                let (team, slot) = (u.team, u.id % session.battle.team_size());
                match session.campaign.craft_prosthetic(team, slot) {
                    Ok(limb) => {
                        if session.battle.tick == 0 {
                            session.battle.units[session.selected].prosthetics =
                                session.campaign.teams[team].fighters[slot].prosthetics;
                        }
                        if session.autosave {
                            session.save_campaign();
                        }
                        session.save_notice = format!(
                            "Crafted Iron I {limb}; 15 gold + 5 salvage spent. Ready next battle."
                        );
                    }
                    Err(e) => session.save_notice = e,
                }
            } else {
                session.save_notice = "Fit prosthetics before or after a Founders battle".into();
            }
        }
        Control::Appearance(choice) => {
            if session.battle.founders {
                let u = &session.battle.units[session.selected];
                let slot = u.id % session.battle.team_size();
                let look = &mut session.campaign.teams[u.team].fighters[slot].appearance;
                *look = look.resolved(u.team, slot);
                match choice {
                    0 => {
                        look.female = Some(
                            !look
                                .female
                                .unwrap_or(u.command_title == Some(CommandTitle::Queen)),
                        )
                    }
                    1 => look.hair = (look.hair + 1) % 11,
                    2 => look.face = (look.face + 1) % 17,
                    3 => look.eyes = (look.eyes + 1) % 16,
                    4 => look.nose = (look.nose + 1) % 17,
                    5 => look.skin = (look.skin + 1) % 8,
                    _ => look.hair_color = (look.hair_color + 1) % 7,
                }
                if session.autosave {
                    session.save_campaign();
                }
            } else {
                session.save_notice = "Customization belongs to your persistent Founders".into();
            }
        }

        Control::Run => session.running = !session.battle.finished,
        Control::Pause => session.running = false,
        Control::Step => {
            session.running = false;
            session.remember_positions();
            session.battle.step();
        }
        Control::Reset => {
            session.battle = Battle::with_setup(
                session.battle.seed,
                session.battle.swapped,
                session.battle.arena,
                session.battle.founders,
            );
            session.running = false;
            session.accumulator = 0.0;
        }
        Control::Swap => {
            session.battle = Battle::with_setup(
                session.battle.seed,
                !session.battle.swapped,
                session.battle.arena,
                session.battle.founders,
            );
            session.running = false;
            session.accumulator = 0.0;
        }
        Control::NextBattle => {
            if !session.battle.founders || !session.battle.finished {
                session.save_notice = "Finish a Founders battle before advancing".into();
                return;
            }
            let seed = session.battle.seed.wrapping_add(1);
            session.restart_founders(
                seed,
                phalanx::pressure::generate(&session.campaign.teams, seed),
                controller,
            );
            if !session.battle.pressure_note.is_empty() {
                session.save_notice = session.battle.pressure_note.clone();
            }
            session.menu = false;
            session.camp_page = None;
            session.running = false;
            session.accumulator = 0.0;
        }
        Control::Save => {
            session.autosave = true;
            session.save_campaign();
        }
        Control::Load => match Campaign::load(std::path::Path::new("outputs/phalanx-save.json")) {
            Ok(c) => {
                session.campaign = c;
                let arena =
                    phalanx::pressure::generate(&session.campaign.teams, session.battle.seed);
                session.restart_founders(session.battle.seed, arena, controller);
                if !session.battle.pressure_note.is_empty() {
                    session.save_notice = session.battle.pressure_note.clone();
                }
                session.running = false;
                session.accumulator = 0.0;
                session.autosave = true;
                session.save_notice = "Loaded squads; fresh battle".into();
            }
            Err(e) => {
                session.save_notice = format!("Load failed: {e}");
                return;
            }
        },
        Control::TeamColor(i) => {
            if session.battle.founders {
                let team = session.battle.units[session.selected].team;
                session.campaign.teams[team].color = PALETTE[i];
                if session.autosave {
                    session.save_campaign();
                }
            } else {
                session.save_notice = "Team heraldry is available in Founders".into();
            }
        }
        Control::Speed => {
            session.speed = if session.speed < 10.0 {
                if session.speed == 4.0 {
                    10.0
                } else {
                    session.speed * 2.0
                }
            } else {
                1.0
            }
        }
        Control::Controller => {
            session.battle.controller = if session.battle.controller == Controller::Utility {
                Controller::Learned
            } else {
                Controller::Utility
            };
            session.battle.learned_team = None;
        }
        Control::Pair(utility, learned) => {
            if utility < ACTION_COUNT && learned < ACTION_COUNT {
                let pair = (utility, learned);
                session.disagreement_pair = if session.disagreement_pair == Some(pair) {
                    None
                } else {
                    Some(pair)
                };
                if let Some((u, l)) = session.disagreement_pair
                    && let Some(id) = session.battle.disagreement_units(u, l).first()
                {
                    session.selected = *id;
                    session.running = false;
                }
            }
        }
        Control::ClearPair => session.disagreement_pair = None,
        Control::NextMatch => {
            if let Some((u, l)) = session.disagreement_pair {
                let ids = session.battle.disagreement_units(u, l);
                if let Some(id) = ids
                    .iter()
                    .copied()
                    .find(|id| *id > session.selected)
                    .or_else(|| ids.first().copied())
                {
                    session.selected = id;
                    session.running = false;
                }
            }
        }
        Control::Setup => {
            let entering = !session.battle.founders;
            let arena = if entering {
                phalanx::pressure::generate(&session.campaign.teams, session.battle.seed)
            } else if matches!(session.battle.arena, Arena::Pressure(_)) {
                Arena::Ridge
            } else {
                session.battle.arena
            };
            session.battle =
                Battle::with_setup(session.battle.seed, session.battle.swapped, arena, entering);
            session.running = false;
            session.accumulator = 0.0;
        }
        Control::Paths => session.paths = (session.paths + 1) % 3,
        Control::Overlay => session.overlay = (session.overlay + 1) % 5,
        Control::BattlefieldBriefing => session.map_briefing = !session.map_briefing,
        Control::GenerateArena => {
            if !session.battle.founders || (session.battle.tick > 0 && !session.battle.finished) {
                session.save_notice =
                    "Generate a battlefield before starting or after finishing a Founders battle"
                        .into();
                return;
            }
            if session.battle.tick == 0 {
                let _ = session.campaign.capture_minds(&session.battle);
            }
            let seed = session.battle.seed.wrapping_add(1);
            let arena = phalanx::pressure::generate(&session.campaign.teams, seed);
            session.restart_founders(seed, arena, controller);
            session.save_notice = session.battle.pressure_note.clone();
            session.menu = false;
            session.camp_page = None;
            session.running = false;
            session.accumulator = 0.;
            session.paths = 2;
            session.map_briefing = true;
        }
        Control::Arena => {
            session.battle = Battle::with_setup(
                session.battle.seed,
                session.battle.swapped,
                session.battle.arena.next(),
                session.battle.founders,
            );
            session.running = false;
            session.accumulator = 0.0;
        }
        Control::Aggressive => session.battle.units[session.selected].mind = Mind::aggressive(),
        Control::Disciplined => session.battle.units[session.selected].mind = Mind::disciplined(),
        Control::Trait(i, change) => {
            let m = &mut session.battle.units[session.selected].mind;
            let field = match i {
                0 => &mut m.aggression,
                1 => &mut m.caution,
                2 => &mut m.courage,
                3 => &mut m.teamwork,
                4 => &mut m.patience,
                5 => &mut m.self_preservation,
                _ => &mut m.discipline,
            };
            *field = (*field + change).clamp(0.0, 1.0);
        }
    }
    if matches!(
        control,
        Control::Reset | Control::Swap | Control::Arena | Control::Setup
    ) && session.battle.founders
    {
        if matches!(control, Control::Swap) && was_founders {
            for i in 0..session.battle.team_size() {
                let a = session.campaign.teams[0].fighters[i].mind;
                session.campaign.teams[0].fighters[i].mind =
                    session.campaign.teams[1].fighters[i].mind;
                session.campaign.teams[1].fighters[i].mind = a;
            }
        }
        session.restart_founders(session.battle.seed, session.battle.arena, controller);
    }
    if matches!(
        control,
        Control::Trait(..) | Control::Aggressive | Control::Disciplined
    ) && session.battle.founders
    {
        let _ = session.campaign.capture_minds(&session.battle);
        if session.autosave {
            session.save_campaign();
        }
    }
    if matches!(
        control,
        Control::Reset
            | Control::Swap
            | Control::Arena
            | Control::Setup
            | Control::NextBattle
            | Control::NextTutorial
            | Control::GenerateArena
            | Control::Load
            | Control::ChooseRival(_)
    ) {
        if matches!(control, Control::Arena) {
            session.paths = if session.battle.arena.large() { 1 } else { 0 };
        }
        session.disagreement_pair = None;
        session.camera_follow = false;
        session.camera_target = Vec3::new(3.5, 0.0, 0.0);
        session.pitch = 0.72;
        session.battle.controller = controller;
        session.logged_disagreements = 0;
        session.run_id += 1;
        session.previous_positions = session.battle.units.iter().map(|u| u.position).collect();
        session.selected = session.selected.min(session.battle.units.len() - 1);
        session.zoom = frame_zoom(session.battle.arena);
        session.remember_positions();
    }
}
fn controls(
    mut session: ResMut<Session>,
    mut buttons: Query<(
        &Interaction,
        &Control,
        &mut BackgroundColor,
        &mut PressLatch,
    )>,
) {
    for (interaction, control, mut color, mut latch) in &mut buttons {
        color.0 = match interaction {
            Interaction::Pressed => Color::srgb(0.2, 0.43, 0.47),
            Interaction::Hovered => Color::srgb(0.2, 0.29, 0.36),
            _ => {
                if let Control::TeamColor(i) = control {
                    let c = PALETTE[*i];
                    Color::srgb(c[0], c[1], c[2])
                } else {
                    Color::srgb(0.12, 0.18, 0.25)
                }
            }
        };
        if *interaction == Interaction::Pressed
            && !latch.0
            && (!session.menu
                || matches!(
                    control,
                    Control::Continue
                        | Control::StarterClass(..)
                        | Control::ClearPhalanxName
                        | Control::ConfirmFounders
                        | Control::Camp(_)
                        | Control::SelectFighter(_)
                        | Control::BattleStyle(_)
                        | Control::NextTutorial
                        | Control::Menu
                        | Control::Workshop(_)
                        | Control::Prosthetic
                        | Control::Class(_)
                        | Control::Title(_)
                        | Control::FlagEmblem
                        | Control::FlagAccent
                        | Control::FlagPrimary
                        | Control::StandardBearer
                        | Control::NextBattle
                        | Control::Ladder
                        | Control::Recruit
                        | Control::ChooseRival(_)
                        | Control::ExportSnapshot
                        | Control::RefreshSnapshots
                        | Control::ImportSnapshot(_)
                        | Control::StartChallenge
                        | Control::ChallengeSeed
                        | Control::ExportChallengeResult
                        | Control::LeaveChallenge
                        | Control::Quit
                        | Control::Arena
                        | Control::GenerateArena
                        | Control::Save
                        | Control::Load
                        | Control::Setup
                ))
        {
            apply(*control, &mut session);
        }
        latch.0 = *interaction == Interaction::Pressed;
    }
}
fn keyboard(keys: Res<ButtonInput<KeyCode>>, mut session: ResMut<Session>) {
    if session.founding.open {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        if session.camp_page.take().is_some() {
            session.menu = true;
        } else {
            session.menu = !session.menu;
        }
        session.running = false;
        return;
    }
    if session.menu {
        return;
    }
    if keys.just_pressed(KeyCode::KeyI) {
        apply(Control::ResearchView, &mut session);
    }
    if keys.just_pressed(KeyCode::Space) {
        let c = if session.running {
            Control::Pause
        } else {
            Control::Run
        };
        apply(c, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyR) {
        apply(Control::Reset, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyN) {
        apply(Control::Step, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyC) {
        apply(Control::Controller, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyM) {
        apply(Control::Arena, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyG) {
        apply(Control::GenerateArena, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyO) {
        apply(Control::Overlay, &mut session);
    }
    if keys.just_pressed(KeyCode::Escape) {
        apply(Control::ClearPair, &mut session);
    }
    if keys.just_pressed(KeyCode::KeyJ) {
        apply(Control::NextMatch, &mut session);
    }
    if keys.just_pressed(KeyCode::Tab) {
        let next = (session.selected + 1) % session.battle.units.len();
        apply(Control::SelectFighter(next), &mut session);
    }
}
fn simulate(time: Res<Time>, mut session: ResMut<Session>) {
    if session.menu || !session.running || session.battle.finished {
        return;
    }
    session.accumulator += time.delta_secs().min(0.25) * session.speed;
    while session.accumulator >= DT {
        session.accumulator -= DT;
        session.remember_positions();
        session.battle.step();
        if session.battle.finished {
            session.running = false;
            break;
        }
    }
}
fn settle_progression(mut session: ResMut<Session>) {
    session.settle_finished();
}
fn sync_team_colors(
    session: Res<Session>,
    assets: Res<PawnAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (team, handle) in [&assets.red, &assets.blue].into_iter().enumerate() {
        let color = session.team_color(team);
        if materials.get(handle).is_some_and(|m| m.base_color != color)
            && let Some(mut material) = materials.get_mut(handle)
        {
            material.base_color = color;
        }
    }
}
fn sync_pawns(
    session: Res<Session>,
    assets: Res<PawnAssets>,
    mut pawns: Query<(&Pawn, &mut Transform, &mut Visibility)>,
    mut parts: Query<
        (
            &PawnPart,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Without<Pawn>,
    >,
) {
    for (pawn, mut transform, mut visible) in &mut pawns {
        let Some(unit) = session.battle.units.get(pawn.0) else {
            if pawn.0 != rpg::PREVIEW_ID {
                *visible = Visibility::Hidden;
            }
            continue;
        };
        let p = session.presented(unit.id);
        let next = Vec3::new(p[0], session.battle.arena.elevation(p) + 0.64, p[1]);
        if transform.translation != next {
            transform.translation = next;
        }
        let facing_target =
            if unit.projectile_weapon() == Some(phalanx::weapons::ProjectileWeapon::Bow) {
                unit.bow_aim()
            } else {
                unit.target
            };
        let direction = Vec3::new(facing_target[0] - p[0], 0.0, facing_target[1] - p[1]);
        if direction.length_squared() > 0.01 && unit.health > 0.0 {
            let facing = Transform::IDENTITY
                .looking_to(-direction.normalize(), Vec3::Y)
                .rotation;
            transform.rotation = if unit.bow_follow_through()
                || unit
                    .windup
                    .as_ref()
                    .is_some_and(|w| w.weapon == phalanx::weapons::ProjectileWeapon::Bow)
            {
                facing
            } else {
                transform.rotation.slerp(facing, 0.18)
            };
        }
        // Corpses remain visible so the death clip can finish.
        *visible = Visibility::Inherited;
    }
    for (part, mut visible, mut material) in &mut parts {
        if let Some(unit) = session.battle.units.get(part.0) {
            *visible = if !part.1 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            let color = if unit.team == 0 {
                &assets.red
            } else {
                &assets.blue
            };
            if material.0 != *color {
                material.0 = color.clone();
            }
        } else {
            *visible = Visibility::Hidden;
        }
    }
}

fn sync_insignia(
    session: Res<Session>,
    assets: Res<PawnAssets>,
    mut pieces: Query<(
        &Insignia,
        &mut Visibility,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    for (piece, mut visible, mut material) in &mut pieces {
        let title = session.visual_unit(piece.0).and_then(|u| u.command_title);
        let alive = session.visual_unit(piece.0).is_some_and(|u| u.health > 0.0);
        let show = alive
            && if piece.1 {
                title.is_some_and(CommandTitle::royal)
            } else {
                title == Some(CommandTitle::General)
            };
        let next = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visible != next {
            *visible = next;
        }
        if piece.1 {
            let color = if title == Some(CommandTitle::Queen) {
                &assets.silver
            } else {
                &assets.gold
            };
            if material.0 != *color {
                material.0 = color.clone();
            }
        }
    }
}
fn pick(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), MainCameraFilter>,
    buttons: Query<&Interaction, With<Button>>,
    mut session: ResMut<Session>,
) {
    if session.menu
        || !mouse.just_pressed(MouseButton::Left)
        || buttons.iter().any(|i| *i != Interaction::None)
    {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if cursor.x > window.width() - 365.0 || cursor.y > window.height() - 100.0 {
        return;
    }
    let (camera, transform) = *camera;
    let mut best = (35.0, session.selected);
    for unit in &session.battle.units {
        if unit.health <= 0.0 {
            continue;
        }
        let p = session.presented(unit.id);
        if let Ok(screen) = camera.world_to_viewport(
            transform,
            Vec3::new(p[0], session.battle.arena.elevation(p) + 0.6, p[1]),
        ) {
            let d = screen.distance(cursor);
            if d < best.0 {
                best = (d, unit.id);
            }
        }
    }
    if session.selected != best.1 {
        apply(Control::SelectFighter(best.1), &mut session);
    }
    debug!(?cursor, selected = best.1, "Agent selection");
}
// Mouse/window/input are separate Bevy-injected parameters.
#[allow(clippy::too_many_arguments)]
fn camera_motion(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    window: Single<&Window>,
    mut session: ResMut<Session>,
    mut camera: Single<&mut Transform, MainCameraFilter>,
    mut initialized: Local<bool>,
) {
    let dt = time.delta_secs().min(0.1);
    let over_world = !session.menu
        && window
            .cursor_position()
            .is_some_and(|p| p.x < window.width() - 365.0 && p.y < window.height() - 100.0);
    let delta = motion.read().fold(Vec2::ZERO, |sum, e| sum + e.delta);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if over_world {
        if mouse.pressed(MouseButton::Middle) || (mouse.pressed(MouseButton::Right) && shift) {
            let right = Vec3::new(session.orbit.cos(), 0.0, -session.orbit.sin());
            let forward = Vec3::new(session.orbit.sin(), 0.0, session.orbit.cos());
            let scale = session.zoom * 0.0014;
            session.camera_target += (-right * delta.x + forward * delta.y) * scale;
            if delta.length_squared() > 0.0 {
                session.camera_follow = false;
            }
        } else if mouse.pressed(MouseButton::Right) {
            session.orbit -= delta.x * 0.005;
            session.pitch = (session.pitch + delta.y * 0.004).clamp(0.07, 1.47);
        }
    }
    for event in wheel.read() {
        if over_world {
            let amount = if event.unit == bevy::input::mouse::MouseScrollUnit::Line {
                event.y
            } else {
                event.y * 0.04
            };
            session.zoom *= (-amount * 0.12).exp();
        }
    }
    if keys.pressed(KeyCode::KeyQ) {
        session.orbit -= dt * 0.7;
    }
    if keys.pressed(KeyCode::KeyE) {
        session.orbit += dt * 0.7;
    }
    if !session.menu {
        let movement = Vec2::new(
            (keys.pressed(KeyCode::KeyD) as u8 as f32) - (keys.pressed(KeyCode::KeyA) as u8 as f32),
            (keys.pressed(KeyCode::KeyW) as u8 as f32) - (keys.pressed(KeyCode::KeyS) as u8 as f32),
        )
        .normalize_or_zero();
        if movement != Vec2::ZERO {
            let right = Vec3::new(session.orbit.cos(), 0.0, -session.orbit.sin());
            let forward = Vec3::new(-session.orbit.sin(), 0.0, -session.orbit.cos());
            let speed = (session.zoom * 0.65).clamp(2.0, 30.0) * if shift { 2.0 } else { 1.0 };
            session.camera_target += (right * movement.x + forward * movement.y) * speed * dt;
            session.camera_follow = false;
        }
    }
    if keys.just_pressed(KeyCode::KeyF) {
        session.camera_follow = true;
        session.zoom = 2.6;
        session.pitch = 0.18;
    }
    if keys.just_pressed(KeyCode::Home) {
        session.camera_follow = false;
        session.camera_target = Vec3::new(3.5, 0.0, 0.0);
        session.zoom = frame_zoom(session.battle.arena);
        session.pitch = 0.72;
    }
    if session.camera_follow {
        let p = session.presented(session.selected);
        session.camera_target = Vec3::new(p[0], session.battle.arena.elevation(p) + 0.85, p[1]);
    }
    session.zoom = session.zoom.clamp(1.1, 100.0);
    let size = session.battle.arena.half_size();
    session.camera_target.x = session.camera_target.x.clamp(-size[0] * 1.5, size[0] * 1.5);
    session.camera_target.z = session.camera_target.z.clamp(-size[1] * 1.5, size[1] * 1.5);
    let radius = session.zoom * session.pitch.cos();
    let mut eye = session.camera_target
        + Vec3::new(
            session.orbit.sin() * radius,
            session.zoom * session.pitch.sin(),
            session.orbit.cos() * radius,
        );
    eye.y = eye
        .y
        .max(session.battle.arena.elevation([eye.x, eye.z]) + 0.22);
    let wanted = Transform::from_translation(eye).looking_at(session.camera_target, Vec3::Y);
    let blend = if *initialized {
        1.0 - (-dt * 18.0).exp()
    } else {
        1.0
    };
    *initialized = true;
    camera.translation = camera.translation.lerp(wanted.translation, blend);
    camera.rotation = camera.rotation.slerp(wanted.rotation, blend);
}
fn overlays(session: Res<Session>, mut gizmos: Gizmos) {
    let arena = session.battle.arena;
    let u = &session.battle.units[session.selected];
    let rendered = session.presented(u.id);
    if u.health > 0.0 {
        gizmos.circle(
            Isometry3d::new(
                Vec3::new(rendered[0], arena.elevation(rendered) + 0.08, rendered[1]),
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ),
            0.65,
            Color::srgb(1.0, 0.85, 0.35),
        );
        if u.is_ranged() {
            gizmos.circle(
                Isometry3d::new(
                    Vec3::new(rendered[0], arena.elevation(rendered) + 0.12, rendered[1]),
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                ),
                u.attack_range(),
                Color::srgba(1.0, 0.8, 0.35, 0.4),
            );
        }
    }
    if let Some((utility, learned)) = session.disagreement_pair {
        for id in session.battle.disagreement_units(utility, learned) {
            let p = session.presented(id);
            let height = arena.elevation(p);
            gizmos.circle(
                Isometry3d::new(
                    Vec3::new(p[0], height + 0.15, p[1]),
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                ),
                0.9,
                Color::srgb(1.0, 0.3, 0.85),
            );
            gizmos.line(
                Vec3::new(p[0], height + 1.3, p[1]),
                Vec3::new(p[0], height + 2.1, p[1]),
                Color::srgb(1.0, 0.3, 0.85),
            );
        }
    }
    for unit in &session.battle.units {
        if unit.health > 0.0 && unit.has_skill(FighterClass::Paladin) {
            let p = session.presented(unit.id);
            gizmos.circle(
                Isometry3d::new(
                    Vec3::new(p[0], arena.elevation(p) + 0.1, p[1]),
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                ),
                PALADIN_AURA_RADIUS,
                Color::srgb(1.0, 0.9, 0.45),
            );
        }
        if unit.health > 0.0 && unit.blessed {
            let p = session.presented(unit.id);
            gizmos.circle(
                Isometry3d::new(
                    Vec3::new(p[0], arena.elevation(p) + 1.7, p[1]),
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                ),
                0.22,
                Color::srgb(1.0, 0.9, 0.45),
            );
        }
    }
    if session.paths < 2 {
        for unit in &session.battle.units {
            if unit.health <= 0.0 || (session.paths == 1 && unit.id != session.selected) {
                continue;
            }
            let color = session.team_color(unit.team);
            let mut previous = session.presented(unit.id);
            for waypoint in unit.route.points.iter().skip(unit.path_cursor) {
                // Sample each actual route edge so its overlay follows the ground.
                for i in 0..4 {
                    let t = i as f32 / 4.0;
                    let v = (i + 1) as f32 / 4.0;
                    let p = [
                        previous[0] + (waypoint[0] - previous[0]) * t,
                        previous[1] + (waypoint[1] - previous[1]) * t,
                    ];
                    let q = [
                        previous[0] + (waypoint[0] - previous[0]) * v,
                        previous[1] + (waypoint[1] - previous[1]) * v,
                    ];
                    gizmos.line(
                        Vec3::new(p[0], arena.elevation(p) + 0.12, p[1]),
                        Vec3::new(q[0], arena.elevation(q) + 0.12, q[1]),
                        color,
                    );
                }
                previous = *waypoint;
            }
        }
    }
    if session.overlay == 4 {
        for contact in &u.contacts {
            let color = if contact.visible {
                Color::srgb(0.2, 1.0, 0.6)
            } else if matches!(contact.source, Source::Ally(_)) {
                Color::srgb(0.8, 0.4, 1.0)
            } else {
                Color::srgb(1.0, 0.75, 0.25)
            };
            let p = contact.position;
            let center = Vec3::new(p[0], arena.elevation(p) + 0.2, p[1]);
            gizmos.circle(
                Isometry3d::new(center, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                0.35 + (1.0 - contact.confidence(session.battle.tick)) * 0.55,
                color,
            );
            gizmos.line(center, center + Vec3::Y * 1.8, color);
        }
        if u.action == 8 || u.action == 3 {
            let a = Vec3::new(
                u.position[0],
                arena.elevation(u.position) + 0.25,
                u.position[1],
            );
            let p = u.target;
            let b = Vec3::new(p[0], arena.elevation(p) + 0.25, p[1]);
            gizmos.line(a, b, Color::srgb(1.0, 0.85, 0.3));
        }
    }
    if session.overlay > 0 && session.overlay < 4 {
        let known = perception::threats(u, session.battle.tick);
        for x in -(arena.half_size()[0] as i32 - 1)..=(arena.half_size()[0] as i32 - 1) {
            for z in -(arena.half_size()[1] as i32 - 1)..=(arena.half_size()[1] as i32 - 1) {
                let p = [x as f32 + 0.5, z as f32 + 0.5];
                if arena.blocked(p) {
                    continue;
                }
                let color = match session.overlay {
                    1 => {
                        if arena.cover(p) > 0.0 {
                            Color::srgb(0.2, 0.95, 0.5)
                        } else if arena.roughness(p) > 0.0 {
                            Color::srgb(0.9, 0.55, 0.2)
                        } else {
                            Color::srgb(0.25, 0.4, 0.5)
                        }
                    }
                    2 => {
                        let t = (threat_at_known(arena, p, &known) / 2.0).min(1.0);
                        Color::srgb(0.15 + t * 0.85, 0.55 * (1.0 - t), 0.25 * (1.0 - t))
                    }
                    _ => {
                        let h = arena.elevation(p) / 1.7;
                        Color::srgb(0.2 + h * 0.65, 0.3 + h * 0.65, 0.8)
                    }
                };
                gizmos.rect(
                    Isometry3d::new(
                        Vec3::new(p[0], arena.elevation(p) + 0.16, p[1]),
                        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                    ),
                    Vec2::splat(0.85),
                    color,
                );
            }
        }
    }
}
fn update_text(session: Res<Session>, mut text: Query<&mut Text, With<Inspector>>) {
    let u = &session.battle.units[session.selected];
    let progress = if session.battle.founders {
        let f = &session.campaign.teams[u.team].fighters[u.id % session.battle.team_size()];
        format!(
            "{} L{} XP{} B{}",
            if session.battle.finished { "NEXT" } else { "" },
            1 + (f.experience / 100).min(9),
            f.experience % 100,
            f.battles
        )
    } else {
        String::new()
    };
    let scores: String = if u.decision_counts.iter().sum::<u32>() == 0 {
        "Awaiting first decision.\n".into()
    } else {
        ACTIONS
            .iter()
            .zip(u.scores)
            .map(|(name, score)| {
                if score < 0.0 {
                    format!("{name:<13} --\n")
                } else {
                    format!("{name:<13} {score:.2}\n")
                }
            })
            .collect()
    };
    let focus = u
        .enemy
        .and_then(|id| u.contacts.iter().find(|c| c.enemy == id))
        .map(|c| {
            let source = if c.visible {
                "own sight".into()
            } else {
                match c.source {
                    Source::OwnSight => "own memory".into(),
                    Source::Ally(id) => {
                        format!("ally {:02} report", id % session.battle.team_size() + 1)
                    }
                }
            };
            format!(
                "Contact {:02}: {} ({:.1}s)",
                c.enemy % session.battle.team_size() + 1,
                source,
                session.battle.tick.saturating_sub(c.seen_tick) as f32 * DT
            )
        })
        .unwrap_or_else(|| "No contact: search deployment".into());
    let support = u
        .support_target
        .map(|id| format!("Supporting ally {:02}", id % session.battle.team_size() + 1))
        .unwrap_or_else(|| "".into());
    let comparison = u
        .comparison
        .as_ref()
        .map(|c| {
            format!(
                "UTILITY: {}\nPYTORCH: {}\nAGREEMENT: {}\nEXECUTED [{}]: {}",
                ACTIONS[c.utility],
                c.learned.map(|i| ACTIONS[i]).unwrap_or("UNAVAILABLE"),
                if c.disagreed {
                    "NO"
                } else if c.learned.is_some() {
                    "YES"
                } else {
                    "UNKNOWN"
                },
                c.controller.name(),
                ACTIONS[c.executed]
            )
        })
        .unwrap_or_else(|| "Comparison awaits first decision".into());
    for mut t in &mut text {
        let content = format!(
            "{}  HP {:.0}/{:.0}\n{} | {}{}\nSurface: cover {:.0}% / rough {:.0}%\nVisible {} / known {} / reports {}\n{}\n{}\n{}\n\nLAST DECISION SCORES\n{}\nRoute cost {:.1} | {} waypoints\nExposure {:.1} / threat {:.1}\nProjectiles thrown {} / hits {} / ally blocks {}\nWhy (last decision):\n{}",
            format_args!(
                "{}
Missing {} / Iron I {} | Salvage {}
Damage x{:.2} / Movement x{:.2} / Incoming x{:.2}\nGear: {}\nWeapon: {}\nWorkshop: craft once; click owned recipe to equip/remove free.",
                session.unit_label(u.id),
                limb_names(u.lost_limbs),
                limb_names(u.prosthetics),
                if session.battle.founders {
                    session.campaign.teams[u.team].salvage
                } else {
                    0
                },
                u.damage_multiplier(),
                u.movement_multiplier(),
                u.incoming_multiplier(),
                if session.battle.founders { session.campaign.teams[u.team].fighters[u.id % session.battle.team_size()].equipment.summary() } else { u.equipment.summary() },
                u.weapon_status()
            ),
            u.health,
            u.max_health(),
            format_args!(
                "{} {} {}",
                u.class_name(),
                progress,
                if !session.battle.founders {
                    ""
                } else if u.has_skill(FighterClass::Paladin) {
                    if u.health > 0.0 {
                        "AURA SOURCE"
                    } else {
                        "AURA OFFLINE"
                    }
                } else if u.blessed {
                    "BLESSED/MOTIVATED"
                } else {
                    "OUTSIDE AURA"
                }
            ),
            if session.battle.founders {
                format!(
                    "\n{}",
                    session.campaign.teams[u.team].fighters[u.id % session.battle.team_size()]
                        .psyche
                        .summary()
                )
            } else {
                String::new()
            },
            if u.health <= 0.0 {
                "ELIMINATED"
            } else {
                ACTIONS[u.action]
            },
            session.battle.arena.cover(u.position) * 100.0,
            session.battle.arena.roughness(u.position) * 100.0,
            u.visible_enemies,
            u.contacts.len(),
            u.contacts
                .iter()
                .filter(|c| matches!(c.source, Source::Ally(_)))
                .count(),
            focus,
            support,
            comparison,
            scores,
            u.route.cost.total(),
            u.route.points.len().saturating_sub(u.path_cursor),
            u.route.cost.exposure,
            u.route.cost.threat,
            u.rocks_thrown,
            u.rocks_hit,
            u.arrow_blocks,
            u.decision_reason
        );
        if t.0 != content {
            t.0 = content;
        }
    }
}

fn update_status(session: Res<Session>, mut text: Query<&mut Text, With<Status>>) {
    for mut t in &mut text {
        let content = format!(
            "{} | {:.1}s | x{:.0} | Seed {}\n{} {} / {}     {} {} / {}\n{} | {}{}\nController: {} | disagreements {} | Paths {}\nOverlay: {} | {}",
            if session.running { "RUNNING" } else { "PAUSED" },
            session.battle.tick as f32 * DT,
            session.speed,
            session.battle.seed,
            session.team_name(0),
            session.battle.survivors(0),
            session.battle.team_size(),
            session.team_name(1),
            session.battle.survivors(1),
            session.battle.team_size(),
            format_args!(
                "{} | {}",
                session
                    .battle
                    .winner()
                    .map(|team| format!("{} wins", session.team_name(team)))
                    .unwrap_or_else(|| session.battle.result.clone()),
                if session.battle.founders {
                    "FOUNDERS"
                } else {
                    "LAB"
                }
            ),
            session.battle.arena.name(),
            if session.battle.pressure_note.is_empty()
                || matches!(session.battle.arena, Arena::Pressure(f) if f.generator > 0)
            {
                String::new()
            } else {
                format!("\n{}", session.battle.pressure_note)
            },
            session.battle.controller.name(),
            session.battle.disagreements.len(),
            ["ALL", "SELECTED", "OFF"][session.paths],
            [
                "OFF",
                "TERRAIN",
                "THREAT (known contacts)",
                "ELEVATION",
                "KNOWLEDGE"
            ][session.overlay],
            match session.overlay {
                2 => "green=low / red=high",
                3 => "blue=low / pale=high",
                4 => "green=seen / gold=memory / purple=report",
                _ => "green=cover / brown=rough",
            }
        );
        let content = if session.battle.founders {
            format!(
                "{content}\nGold {} | Salvage {} | {}",
                session.campaign.teams[0].gold,
                session.campaign.teams[0].salvage,
                if !session.battle.pressure_note.is_empty()
                    && session.save_notice.contains(&session.battle.pressure_note)
                {
                    ""
                } else {
                    &session.save_notice
                }
            )
        } else {
            content
        };
        if t.0 != content {
            t.0 = content;
        }
    }
}

fn update_divergence(
    session: Res<Session>,
    summary_entity: Single<Entity, With<DivergenceSummary>>,
    mut rows: Query<(&PairRow, &mut Control, &Children, &mut Node)>,
    mut texts: Query<&mut Text>,
) {
    let summary = &session.battle.comparison_summary;
    let agreement = summary
        .agreement_rate()
        .map(|r| format!("{:.1}%", r * 100.0))
        .unwrap_or_else(|| "N/A".into());
    let mut content = format!(
        "ALL DECISIONS | Agreement: {agreement} / {} compared / {} unavailable",
        summary.compared, summary.unavailable
    );
    if let Some((u, l)) = session.disagreement_pair {
        let count = session.battle.disagreement_units(u, l).len();
        content.push_str(&format!("\nFILTER: {} -> {} | {count} live matches\nPink = latest matching decisions | D next | Esc clear", ACTIONS[u], ACTIONS[l]));
    } else {
        content.push_str("\nClick a pair to inspect live matches (pauses if found)");
    }
    let pairs = summary.top_disagreements(3);
    if pairs.is_empty() {
        content.push_str("\nTop disagreements: none yet");
    }
    if let Ok(mut t) = texts.get_mut(*summary_entity)
        && t.0 != content
    {
        t.0 = content;
    }
    for (rank, mut control, children, mut node) in &mut rows {
        if let Some(pair) = pairs.get(rank.0) {
            node.display = Display::Flex;
            let u = ACTIONS.iter().position(|a| *a == pair.utility).unwrap();
            let l = ACTIONS.iter().position(|a| *a == pair.learned).unwrap();
            *control = Control::Pair(u, l);
            let marker = if session.disagreement_pair == Some((u, l)) {
                "* "
            } else {
                ""
            };
            let label = format!(
                "{marker}{} -> {}  {}",
                pair.utility, pair.learned, pair.count
            );
            for child in children.iter() {
                if let Ok(mut t) = texts.get_mut(child)
                    && t.0 != label
                {
                    t.0 = label.clone();
                }
            }
        } else {
            node.display = Display::None;
        }
    }
}

fn terrain_mesh(arena: Arena) -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let size = arena.half_size();
    for x in -(size[0] as i32 * 2)..(size[0] as i32 * 2) {
        for z in -(size[1] as i32 * 2)..(size[1] as i32 * 2) {
            let a = [x as f32 * 0.5, z as f32 * 0.5];
            let b = [a[0] + 0.5, a[1]];
            let c = [a[0], a[1] + 0.5];
            let d = [a[0] + 0.5, a[1] + 0.5];
            let middle = [a[0] + 0.25, a[1] + 0.25];
            let color = if let Arena::Pressure(field) = arena
                && field.generator > 0
            {
                let palette = phalanx::procedural::palette(field);
                let height = arena.elevation(middle);
                let color = if arena.cover(middle) > 0. {
                    palette[1]
                } else if arena.roughness(middle) > 0.5 {
                    palette[2]
                } else if height > 0.9 {
                    palette[3]
                } else {
                    palette[0]
                };
                let terrace = if (height * 5.).fract() < 0.14 {
                    0.86
                } else {
                    1.
                };
                Color::srgb(color[0] * terrace, color[1] * terrace, color[2] * terrace)
            } else if arena == Arena::Quarry && arena.cover(middle) == 0.0 {
                Color::srgb(0.42, 0.39, 0.32)
            } else if arena == Arena::Mire && arena.roughness(middle) > 0.0 {
                Color::srgb(0.16, 0.23, 0.20)
            } else if arena == Arena::Gauntlet && arena.cover(middle) == 0.0 {
                Color::srgb(0.27, 0.29, 0.32)
            } else if arena.cover(middle) > 0.0 {
                Color::srgb(0.24, 0.34, 0.13)
            } else if arena.roughness(middle) > 0.0 {
                Color::srgb(0.37, 0.29, 0.19)
            } else {
                Color::srgb(0.31, 0.34, 0.21)
            };
            let hash = ((x * 73 + z * 131).unsigned_abs() % 17) as f32 / 17.0;
            let shade = 0.90 + hash * 0.15 + arena.ground_elevation(middle) * 0.025;
            let linear = color.to_linear().to_f32_array();
            let shaded = [linear[0] * shade, linear[1] * shade, linear[2] * shade, 1.0];
            for p in [a, c, b, b, c, d] {
                positions.push([p[0], arena.ground_elevation(p), p[1]]);
                colors.push(shaded);
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.compute_normals();
    mesh
}
fn sync_terrain(
    session: Res<Session>,
    wall_materials: Res<battlefield::WallMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut ground: Single<(&Mesh3d, &mut GroundArena)>,
    mut walls: Query<
        (
            &ChokeWall,
            &mut Visibility,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Without<Slab>,
    >,
    mut slab: Single<&mut Transform, With<Slab>>,
) {
    let (mesh, rendered) = &mut *ground;
    if rendered.0 != session.battle.arena {
        if let Some(mut asset) = meshes.get_mut(&mesh.0) {
            *asset = terrain_mesh(session.battle.arena);
        }
        rendered.0 = session.battle.arena;
    }
    let arena = session.battle.arena;
    let size = arena.half_size();
    let slab_scale = Vec3::new(
        (size[0] * 2.0 + 0.4) / 22.4,
        1.0,
        (size[1] * 2.0 + 0.4) / 16.4,
    );
    if slab.scale != slab_scale {
        slab.scale = slab_scale;
    }
    for (wall, mut visible, mut transform, mut material) in &mut walls {
        if let Some((center, extent)) = arena.walls().get(wall.0) {
            let timber = if let Arena::Pressure(field) = arena {
                field.layout > 0 && wall.0 < 4 - (field.destroyed & 15).count_ones() as usize
            } else {
                false
            };
            let handle = if timber {
                &wall_materials.timber
            } else {
                &wall_materials.stone
            };
            if material.0 != *handle {
                material.0 = handle.clone();
            }
            *visible = Visibility::Inherited;
            let h = if timber { 1.5 } else { 2.4 };
            let wall_translation =
                Vec3::new(center[0], arena.elevation(*center) + h * 0.5, center[1]);
            let wall_scale = Vec3::new(extent[0], h, extent[1]);
            if transform.translation != wall_translation {
                transform.translation = wall_translation;
            }
            if transform.scale != wall_scale {
                transform.scale = wall_scale;
            }
        } else {
            *visible = Visibility::Hidden;
        }
    }
}
fn sync_bridges(session: Res<Session>, mut bridges: Query<&mut Visibility, With<Bridge>>) {
    let visible = if session.battle.arena == Arena::Citadel {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut bridge in &mut bridges {
        if *bridge != visible {
            *bridge = visible;
        }
    }
}
fn sync_rocks(
    session: Res<Session>,
    mut visuals: Query<(&RockVisual, &mut Transform, &mut Visibility)>,
) {
    let arena = session.battle.arena;
    for (visual, mut transform, mut visibility) in &mut visuals {
        if let Some(rock) = session
            .battle
            .rocks
            .get(visual.slot)
            .filter(|p| p.weapon == visual.weapon)
        {
            *visibility = Visibility::Inherited;
            let fraction = if session.running {
                session.accumulator / DT
            } else {
                0.0
            };
            let t = ((rock.age as f32 + fraction) / rock.flight_ticks as f32).min(1.0);
            let x = rock.from[0] + (rock.to[0] - rock.from[0]) * t;
            let z = rock.from[1] + (rock.to[1] - rock.from[1]) * t;
            let y = arena.elevation(rock.from)
                + (arena.elevation(rock.to) - arena.elevation(rock.from)) * t
                + if rock.weapon == phalanx::weapons::ProjectileWeapon::Bow {
                    phalanx::archery::GRIP_HEIGHT
                } else {
                    0.9
                }
                + (t * std::f32::consts::PI).sin() * rock.weapon.arc();
            transform.translation = Vec3::new(x, y, z);
            let dy = arena.elevation(rock.to) - arena.elevation(rock.from)
                + std::f32::consts::PI * rock.weapon.arc() * (t * std::f32::consts::PI).cos();
            let direction = Vec3::new(rock.to[0] - rock.from[0], dy, rock.to[1] - rock.from[1])
                .normalize_or_zero();
            transform.rotation = Quat::from_rotation_arc(Vec3::Z, direction);
            transform.scale = Vec3::splat(
                if rock.weapon == phalanx::weapons::ProjectileWeapon::Atlatl {
                    1.2
                } else if rock.weapon == phalanx::weapons::ProjectileWeapon::Bow {
                    0.70
                } else {
                    1.0
                },
            );
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn update_minds(session: Res<Session>, mut values: Query<(&TraitValue, &mut Text)>) {
    let m = session.battle.units[session.selected].mind;
    let traits = [
        m.aggression,
        m.caution,
        m.courage,
        m.teamwork,
        m.patience,
        m.self_preservation,
        m.discipline,
    ];
    let names = [
        "Aggression",
        "Caution",
        "Courage",
        "Teamwork",
        "Patience",
        "Self-preservation",
        "Discipline",
    ];
    for (value, mut text) in &mut values {
        let label = format!("{} {:.2}", names[value.0], traits[value.0]);
        if text.0 != label {
            text.0 = label;
        }
    }
}
fn log_disagreements(mut session: ResMut<Session>) {
    use std::io::Write;
    if session.logged_disagreements >= session.battle.disagreements.len() {
        return;
    }
    let fresh = session.battle.disagreements[session.logged_disagreements..].to_vec();
    session.logged_disagreements = session.battle.disagreements.len();
    let run_id = session.run_id;
    let seed = session.battle.seed;
    let arena = session.battle.arena;
    if let Some(log) = &mut session.disagreement_log {
        for comparison in fresh {
            let event = serde_json::json!({"event":"disagreement","run_id":run_id,"seed":seed,"arena":arena,"comparison":comparison});
            if let Err(e) = writeln!(log, "{event}") {
                error!("Disagreement log write failed: {e}");
            }
        }
        if let Err(e) = log.flush() {
            error!("Disagreement log flush failed: {e}");
        }
    }
}

fn scroll_inspector(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    window: Single<&Window>,
    mut panels: Query<&mut ScrollPosition, With<InspectorPanel>>,
) {
    let over = window
        .cursor_position()
        .is_some_and(|p| p.x > window.width() - 365.0 && p.y < window.height() - 100.0);
    for event in wheel.read() {
        if over {
            for mut scroll in &mut panels {
                scroll.0.y = (scroll.0.y - event.y * 28.0).max(0.0);
            }
        }
    }
}

fn limb_names(bits: u8) -> String {
    let names: Vec<_> = ["L arm", "R arm", "L leg", "R leg"]
        .into_iter()
        .enumerate()
        .filter_map(|(i, name)| (bits & (1 << i) != 0).then_some(name))
        .collect();
    if names.is_empty() {
        "none".into()
    } else {
        names.join(", ")
    }
}

fn update_workshop(
    session: Res<Session>,
    buttons: Query<(&Control, &Children)>,
    mut labels: Query<&mut Text>,
) {
    let u = &session.battle.units[session.selected];
    for (control, children) in &buttons {
        let Control::Workshop(gear) = *control else {
            continue;
        };
        let label = if session.battle.founders {
            let fighter =
                &session.campaign.teams[u.team].fighters[u.id % session.battle.team_size()];
            let kit = fighter.equipment;
            if kit.owns(gear) {
                format!(
                    "{} {} | {}",
                    if kit.has(gear) { "Unequip" } else { "Equip" },
                    gear.name(),
                    gear.effect()
                )
            } else if let Some(reason) = gear.locked_reason(fighter.mastery) {
                format!("Locked {} | {reason}", gear.name())
            } else {
                let (gold, salvage) = gear.cost();
                format!(
                    "Craft {} ({gold}g/{salvage}s) | {}",
                    gear.name(),
                    gear.effect()
                )
            }
        } else {
            format!("{} | Founders workshop", gear.name())
        };
        for child in children.iter() {
            if let Ok(mut text) = labels.get_mut(child) {
                **text = label.clone();
            }
        }
    }
}

#[cfg(test)]
mod workshop_regressions {
    use super::*;
    #[test]
    fn battlefield_control_previews_rerolls_and_preserves_fighters_without_midfight_changes() {
        let mut session = Session::new(Battle::new(42, false));
        session.autosave = false;
        session.campaign = Campaign::new();
        session.restart_founders(42, Arena::OpenField, Controller::Utility);
        let fighters = serde_json::to_string(&session.campaign.teams).unwrap();
        apply(Control::GenerateArena, &mut session);
        assert!(!session.running && !session.menu);
        assert_eq!(session.battle.seed, 43);
        assert_eq!(session.battle.tick, 0);
        assert_eq!(
            serde_json::to_string(&session.campaign.teams).unwrap(),
            fighters
        );
        let arena = session.battle.arena;
        apply(Control::Reset, &mut session);
        assert_eq!(session.battle.arena, arena);
        session.battle.step();
        apply(Control::GenerateArena, &mut session);
        assert_eq!(session.battle.arena, arena);
        assert_eq!(session.battle.seed, 43);
        assert!(session.save_notice.contains("before starting"));
    }
    #[test]
    fn inspector_controls_retrain_queen_equip_and_reject_mid_battle_changes() {
        // Lab initialization avoids touching the player's campaign save.
        let mut session = Session::new(Battle::new(42, false));
        session.autosave = false;
        session.campaign = Campaign::new();
        let (_, battle) = session
            .campaign
            .begin_battle(42, Arena::OpenField, Controller::Utility)
            .unwrap();
        session.battle = battle;
        session.selected = 1;
        session.campaign.teams[0].salvage = 100;
        apply(Control::Class(FighterClass::Healer), &mut session);
        assert_eq!(session.battle.units[1].class, Some(FighterClass::Healer));
        assert_eq!(
            session.battle.units[1].command_title,
            Some(CommandTitle::Queen)
        );
        let gear = phalanx::equipment::Gear::VitalityCharm;
        apply(Control::Workshop(gear), &mut session);
        assert_eq!(session.battle.units[1].health, 390.0);
        assert!(session.battle.units[1].equipment.has(gear));
        apply(Control::Workshop(gear), &mut session);
        assert_eq!(session.battle.units[1].health, 330.0);
        let gold = session.campaign.teams[0].gold;
        session.battle.tick = 1;
        apply(Control::Workshop(gear), &mut session);
        assert!(!session.battle.units[1].equipment.has(gear));
        assert_eq!(session.campaign.teams[0].gold, gold);
        session.battle.finished = true;
        apply(Control::Workshop(gear), &mut session);
        assert!(session.campaign.teams[0].fighters[1].equipment.has(gear));
        assert!(!session.battle.units[1].equipment.has(gear));
        let (_, next) = session
            .campaign
            .begin_battle(43, Arena::OpenField, Controller::Utility)
            .unwrap();
        assert_eq!(next.units[1].health, 390.0);
    }
    #[test]
    fn rival_selection_control_preserves_squads_and_rejects_mid_battle_swaps() {
        let mut session = Session::new(Battle::new(42, false));
        session.autosave = false;
        session.campaign = Campaign::new();
        let (_, battle) = session
            .campaign
            .begin_battle(42, Arena::OpenField, Controller::Utility)
            .unwrap();
        session.battle = battle;
        let challenger = session.campaign.rivals[0].name.clone();
        apply(Control::ChooseRival(0), &mut session);
        assert_eq!(session.campaign.teams[1].name, challenger);
        assert_eq!(session.battle.tick, 0);
        assert!(!session.running);
        session.battle.step();
        apply(Control::ChooseRival(1), &mut session);
        assert_eq!(session.campaign.teams[1].name, challenger);
    }
    #[test]
    fn weapon_controls_refresh_prebattle_ammo_and_preserve_postbattle_snapshots() {
        use phalanx::equipment::Gear;
        use phalanx::weapons::ProjectileWeapon as W;
        let mut session = Session::new(Battle::new(42, false));
        session.autosave = false;
        session.campaign = Campaign::new();
        session.campaign.teams[0].salvage = 100;
        session.campaign.teams[0].fighters[0]
            .mastery
            .train_weapon(W::Rock, 100);
        session.campaign.teams[0].fighters[0]
            .mastery
            .train_weapon(W::Spear, 100);
        let (_, battle) = session
            .campaign
            .begin_battle(42, Arena::OpenField, Controller::Utility)
            .unwrap();
        session.battle = battle;
        session.selected = 0;
        assert_eq!(session.battle.units[0].role, Role::Fighter);
        apply(Control::Workshop(Gear::ThrowingSpear), &mut session);
        assert_eq!(session.battle.units[0].role, Role::RockThrower);
        assert_eq!(session.battle.units[0].ammunition, 6);
        apply(Control::Workshop(Gear::ThrowingSpear), &mut session);
        assert_eq!(session.battle.units[0].role, Role::Fighter);
        assert_eq!(session.battle.units[0].ammunition, 0);
        apply(Control::Class(FighterClass::RockThrower), &mut session);
        assert_eq!(session.battle.units[0].ammunition, u8::MAX);
        apply(Control::Class(FighterClass::Ranger), &mut session);
        assert_eq!(session.battle.units[0].projectile_weapon(), Some(W::Bow));
        assert_eq!(session.battle.units[0].ammunition, 12);
        session.battle.tick = 1;
        apply(Control::Workshop(Gear::Atlatl), &mut session);
        assert!(
            !session.campaign.teams[0].fighters[0]
                .equipment
                .owns(Gear::Atlatl)
        );
        session.battle.finished = true;
        apply(Control::Workshop(Gear::Atlatl), &mut session);
        assert!(
            session.campaign.teams[0].fighters[0]
                .equipment
                .has(Gear::Atlatl)
        );
        assert!(!session.battle.units[0].equipment.has(Gear::Atlatl));
        let (_, next) = session
            .campaign
            .begin_battle(43, Arena::OpenField, Controller::Utility)
            .unwrap();
        assert_eq!(next.units[0].ammunition, 4);
    }
}
