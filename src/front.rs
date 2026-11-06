//! Compact local-game front end. Combat is paused while this overlay is open.
use super::*;
pub struct FrontEnd;
#[derive(Component)]
struct MainMenu(bool);
#[derive(Component)]
struct MenuSummary;
#[derive(Component)]
struct MenuScroll;
#[derive(Component)]
struct CampaignOptions;
impl Plugin for FrontEnd {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_menu.after(setup))
            .add_systems(Update, (menu, quit, scroll_menu).after(controls))
            .add_systems(PostUpdate, opening_controls);
    }
}
fn opening_controls(session: Res<Session>, mut nodes: Query<(&Control, &mut Node)>) {
    for (control, mut node) in &mut nodes {
        if matches!(
            control,
            Control::Camp(camp::Page::Training | camp::Page::Rivals | camp::Page::Challenges)
                | Control::Title(_)
                | Control::Ladder
                | Control::Setup
                | Control::GenerateArena
                | Control::Arena
                | Control::ChooseRival(_)
        ) {
            node.display = if session.campaign.prologue_active() {
                Display::None
            } else {
                Display::Flex
            };
        }
    }
}
fn setup_menu(mut commands: Commands) {
    commands.spawn((MainMenu(false), GlobalZIndex(100), Interaction::default(),
        Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100),
            align_items: AlignItems::Center, justify_content: JustifyContent::Center, ..default() },
        BackgroundColor(Color::srgba(0.018,0.026,0.035,0.98))))
        .with_children(|root| {
            root.spawn((MenuScroll, ScrollPosition::default(), Node { width: px(630), max_width: vw(94), height: vh(95),min_height:px(0),flex_shrink:0., overflow: Overflow::scroll_y(), padding: UiRect::all(px(28)),
                flex_direction: FlexDirection::Column, row_gap: px(12), ..default() }))
                .with_children(|p| {
                    p.spawn((Text::new("PHALANX"),font(48.0),TextColor(Color::srgb(1.0,0.72,0.08))));
                    p.spawn((Text::new("A game by Guillaume Heinerth"),font(13.0),TextColor(Color::srgb(0.65,0.75,0.80))));
                    p.spawn((Text::new("Build their minds. Watch them fight. Bring them home."),font(16.0)));
                    p.spawn((MenuSummary,Text::new(""),font(15.0),TextColor(Color::srgb(0.65,0.82,0.84))));
                    button(p,"New Game / Before the crown",Control::NewGame);
                    button(p,"Classic three-founder start",Control::LegacyNewGame);
                    button(p,"Load Game",Control::Load);
                    button(p,"Continue",Control::Continue);
                    button(p,"Opening chapters / meet your companions",Control::Camp(camp::Page::Prologue));
                    p.spawn((CampaignOptions,Node {flex_direction:FlexDirection::Column,row_gap:px(6),..default()})).with_children(|p| {
                    button(p,"Begin / continue NPC tutorials",Control::NextTutorial);
                    button(p,"Tutorial teams & tactical lessons",Control::Camp(camp::Page::Rivals));
                    button(p,"Enter arena / manage your fighters",Control::Continue);
                    button(p,"Squad workshop",Control::Camp(camp::Page::Workshop));
                    button(p,"Last battle report",Control::Camp(camp::Page::Report));
                    button(p,"Choose a rival phalanx",Control::Camp(camp::Page::Rivals));
                    button(p,"Snapshot challenges / exchange teams",Control::Camp(camp::Page::Challenges));
                    let fourth = phalanx::progression::slot_cost(4).unwrap();
                    button(p,&format!("Unlock fourth fighter / League 4 - {fourth} gold"),Control::Recruit);
                    button(p,"View League 3 -> 20 gold ladder",Control::Ladder);
                    button(p,"Change arena",Control::Arena);
                    button(p,"Create a procedural battlefield",Control::GenerateArena);
                    button(p,"Switch Game / AI Lab",Control::Setup);
                    button(p,"Save Game",Control::Save);
                    });
                    super::audio::menu(p);
                    button(p,"Quit",Control::Quit);
                    p.spawn((Text::new("Select a fighter to change class, appearance and mind.\nRetrain: 10 gold | Iron limb: 15 gold + 5 salvage\nLeague 3: 30 gold per completed battle | rewards rise by league\nBattles scar minds. The same event marks two personalities differently.\nFounders fields are generated for both squads. Arena returns to the Lab maps.\n\nRMB orbit | MMB pan | Wheel zoom | F follow | V cinema\nSpace pause / run | Tab next fighter | Esc menu\n\nLocal prototype | no account or connection required"),font(14.0)));
                });
        });
    let mut ladder = String::from(
        "LEAGUE    SLOT COST    TOTAL INVESTED    BATTLE GOLD\n3         Starter      0                 30\n",
    );
    let mut total = 0;
    for league in 4..=20 {
        let price = phalanx::progression::slot_cost(league).unwrap();
        total += price;
        ladder.push_str(&format!(
            "{league:<10}{price:<13}{total:<18}{}\n",
            phalanx::progression::gold_reward(league)
        ));
    }
    commands.spawn((MainMenu(true),GlobalZIndex(101),Interaction::default(),
        Node { position_type:PositionType::Absolute,width:percent(100),height:percent(100),
            flex_direction:FlexDirection::Column,align_items:AlignItems::Center,justify_content:JustifyContent::Center,row_gap:px(12),..default()},
        BackgroundColor(Color::srgb(0.018,0.026,0.035))))
        .with_children(|p| {
            p.spawn((Text::new("THE ROAD TO TWENTY"),font(32.),TextColor(Color::srgb(1.,0.72,0.08))));
            p.spawn((Text::new(ladder),font(16.)));
            p.spawn((Text::new("Earned gold only | equal battle credit for every result | no passive income\nStarter purse: 100 gold | crafting and retraining compete with savings\nPlayable now: leagues 3-4 | leagues 5-20 are planned content\nPrices are first-pass tuning; a long grind still needs worthwhile battles."),font(14.)));
            button(p,"Back",Control::Ladder);
        });
}
fn menu(
    session: Res<Session>,
    mut panels: Query<(&MainMenu, &mut Node), Without<CampaignOptions>>,
    mut options: Query<&mut Node, (With<CampaignOptions>, Without<MainMenu>)>,
    mut summaries: Query<&mut Text, With<MenuSummary>>,
) {
    for mut node in &mut options {
        node.display = if !session.campaign.prologue_active()
            && (session.campaign.founders_prepared || !session.battle.founders)
        {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (page, mut panel) in &mut panels {
        panel.display = if !session.founding.open
            && session.menu
            && session.camp_page.is_none()
            && session.ladder == page.0
        {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut text in &mut summaries {
        let team = &session.campaign.teams[0];
        let content = if let Some(p) = session.campaign.prologue.as_ref().filter(|p| p.active()) {
            format!(
                "{}\n{} | {} companions | {} gold\n{}",
                p.title(),
                team.fighters[0].name,
                team.fighters.len() - 1,
                team.gold,
                session.save_notice
            )
        } else if !session.campaign.founders_prepared && session.battle.founders {
            format!(
                "Start a new phalanx, load a saved game, or continue creating your founders.\n{}",
                session.save_notice
            )
        } else {
            format!(
                "{} | League {} | {} gold | {} salvage\n{} | {}\n{}",
                team.name,
                team.fighters.len(),
                team.gold,
                team.salvage,
                if session.battle.founders {
                    "GAME"
                } else {
                    "AI LAB"
                },
                session.battle.arena.name(),
                session.save_notice
            )
        };
        if text.0 != content {
            text.0 = content;
        }
    }
}

/// Build the replacement first, so an invalid load leaves the active game intact.
fn install_campaign(
    session: &mut Session,
    mut campaign: Campaign,
    seed: u64,
    controller: Controller,
) -> Result<(), String> {
    let arena = campaign.battlefield(seed);
    let (id, battle) = campaign.begin_battle(seed, arena, controller)?;
    let arena = battle.arena;
    session.campaign = campaign;
    session.battle = battle;
    session.active_battle_id = Some(id);
    session.rewarded = false;
    session.founding = founding::Draft::new(&session.campaign, false);
    session.renaming = None;
    session.harvesting = false;
    session.exchange = exchange::State::default();
    session.player = player::View::default();
    session.menu = true;
    session.ladder = false;
    session.camp_page = None;
    session.running = false;
    session.accumulator = 0.;
    session.selected = 0;
    session.speed = 1.;
    session.paths = 2;
    session.overlay = 0;
    session.disagreement_pair = None;
    session.previous_positions = session.battle.units.iter().map(|u| u.position).collect();
    session.orbit = 0.;
    session.pitch = 0.72;
    session.camera_target = Vec3::new(3.5, 0., 0.);
    session.zoom = frame_zoom(arena);
    session.camera_follow = false;
    session.cinema = phalanx::cinema::Director::default();
    session.cinema_on = true;
    session.cinema_grabbed = false;
    session.logged_disagreements = 0;
    session.run_id += 1;
    session.refresh_preview();
    if session.campaign.prologue_active() {
        session.zoom = 13.;
        session.camera_follow = true;
        session.map_briefing = false;
    }
    Ok(())
}
pub(super) fn new_game(session: &mut Session) {
    match install_campaign(session, Campaign::new_prologue(), 42, Controller::Utility) {
        Ok(()) => {
            session.campaign.founders_prepared = false;
            session.founding = founding::Draft::new(&session.campaign, true);
            session.founding.stage = 1;
            session.save_notice =
                "Create your King at age 16. Your current save stays available until you accept."
                    .into();
        }
        Err(e) => session.save_notice = format!("New game failed: {e}"),
    }
}
pub(super) fn legacy_new_game(session: &mut Session) {
    match install_campaign(session, Campaign::new(), 42, Controller::Utility) {
        Ok(()) => {
            session.founding.open = true;
            session.save_notice =
                "Create your new founders. Your saved game stays available until you accept them."
                    .into();
        }
        Err(e) => session.save_notice = format!("New game failed: {e}"),
    }
}
pub(super) fn next_prologue(session: &mut Session) {
    if !session.campaign.founders_prepared {
        session.founding.open = true;
        session.menu = true;
        session.save_notice = "Finish creating your King before beginning the opening.".into();
        return;
    }
    if !session.campaign.prologue_active() {
        session.camp_page = Some(camp::Page::Rivals);
        session.menu = true;
        return;
    }
    if session.battle.tick > 0 && !session.battle.finished {
        session.save_notice = "Finish the current chapter encounter before advancing.".into();
        return;
    }
    if session.battle.finished
        || session
            .campaign
            .prologue
            .as_ref()
            .is_some_and(|p| p.cleared())
    {
        match session.campaign.advance_prologue() {
            Ok(message) => session.save_notice = message,
            Err(error) => {
                session.save_notice = error;
                return;
            }
        }
        let seed = session.battle.seed.wrapping_add(1);
        session.restart_founders(seed, Arena::OpenField, session.battle.controller);
        session.previous_positions = session.battle.units.iter().map(|u| u.position).collect();
        session.selected = 0;
        session.accumulator = 0.;
        session.run_id += 1;
        session.refresh_preview();
        session.zoom = if session.campaign.prologue_active() {
            13. + 4. * session.campaign.prologue.as_ref().unwrap().chapter as f32
        } else {
            frame_zoom(session.battle.arena)
        };
    }
    session.menu = true;
    session.camp_page = Some(if session.campaign.prologue_active() {
        camp::Page::Prologue
    } else {
        camp::Page::Rivals
    });
    session.running = false;
    session.map_briefing = false;
    if session.autosave {
        session.save_campaign();
    }
}
pub(super) fn load_game(session: &mut Session) -> bool {
    let path = session.save_path.clone();
    if !path.exists() {
        session.save_notice = "No saved game yet. Choose New Game to create your phalanx.".into();
        return false;
    }
    let result = Campaign::load(&path).and_then(|campaign| {
        let seed = campaign.teams[0].fighters[0]
            .history
            .last()
            .map_or(42, |h| h.seed.wrapping_add(1));
        install_campaign(session, campaign, seed, session.battle.controller)
    });
    match result {
        Ok(()) => {
            session.autosave = true;
            session.founding.open = !session.campaign.founders_prepared;
            session.menu = session.founding.open || session.campaign.prologue_active();
            if session.campaign.prologue_active() {
                session.camp_page = Some(camp::Page::Prologue);
            }
            session.save_notice =
                "Loaded your saved phalanx and campaign progress. Ready for battle.".into();
            true
        }
        Err(e) => {
            session.save_notice = format!("Load failed: {e}");
            false
        }
    }
}
fn quit(
    session: Res<Session>,
    buttons: Query<(&Interaction, &Control), Changed<Interaction>>,
    mut exit: MessageWriter<AppExit>,
) {
    if session.menu
        && buttons
            .iter()
            .any(|(i, c)| *i == Interaction::Pressed && matches!(c, Control::Quit))
    {
        exit.write(AppExit::Success);
    }
}

fn scroll_menu(
    session: Res<Session>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut panels: Query<&mut ScrollPosition, With<MenuScroll>>,
) {
    for event in wheel.read() {
        if session.menu && !session.ladder && session.camp_page.is_none() {
            let delta = match event.unit {
                bevy::input::mouse::MouseScrollUnit::Line => event.y * 32.0,
                bevy::input::mouse::MouseScrollUnit::Pixel => event.y,
            };
            for mut position in &mut panels {
                position.0.y = (position.0.y - delta).max(0.);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    fn fixture() -> (App, PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "phalanx-new-load-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("campaign.json");
        let mut campaign = Campaign::new();
        campaign
            .choose_founders(
                "Veterans",
                [
                    FighterClass::Warrior,
                    FighterClass::Healer,
                    FighterClass::Paladin,
                ],
            )
            .unwrap();
        campaign.teams[0].gold = 777;
        campaign.unlock_fourth().unwrap();
        campaign.teams[0].fighters[1].appearance.nose = 12;
        campaign.teams[0].fighters[1].appearance.mouth = 4;
        campaign.save(&path).unwrap();
        let mut session = Session::new(Battle::new(42, false));
        session.autosave = true;
        session.save_path = path;
        install_campaign(&mut session, campaign, 42, Controller::Utility).unwrap();
        let mut app = App::new();
        app.insert_resource(session).add_systems(Update, controls);
        (app, directory)
    }
    fn click(app: &mut App, control: Control) {
        let entity = app
            .world_mut()
            .spawn((
                control,
                Interaction::Pressed,
                BackgroundColor::default(),
                PressLatch::default(),
            ))
            .id();
        app.update();
        app.world_mut().despawn(entity);
    }
    #[test]
    fn new_game_opens_one_adolescent_and_saves_story_progress_without_rebuilding_the_roster() {
        let (mut app, directory) = fixture();
        let path = directory.join("campaign.json");
        let previous = std::fs::read(&path).unwrap();
        click(&mut app, Control::NewGame);
        let s = app.world().resource::<Session>();
        assert_eq!(s.battle.units.len(), 2);
        assert_eq!(s.camp_page, None);
        assert!(s.founding.open && s.founding.solo && !s.running);
        assert_eq!(s.founding.count(), 1);
        assert_eq!(s.campaign.teams[0].fighters[0].appearance.age_years, 16);
        assert_eq!(s.campaign.teams[0].fighters[0].title, CommandTitle::Member);
        assert_eq!(std::fs::read(&path).unwrap(), previous);
        let identity = s.campaign.teams[0].fighters[0].fighter_id.clone();
        let waiting = serde_json::to_value(&s.campaign.prologue.as_ref().unwrap().waiting).unwrap();
        click(&mut app, Control::CreatorMenu);
        click(&mut app, Control::Save);
        assert_eq!(std::fs::read(&path).unwrap(), previous);
        click(&mut app, Control::Continue);
        click(&mut app, Control::RevealFounder(0));
        app.world_mut().resource_mut::<Session>().founding.cards[0].name = "Aurel".into();
        click(&mut app, Control::FounderLook(0, 2));
        let look = app.world().resource::<Session>().founding.cards[0].appearance;
        click(&mut app, Control::FoundingStep);
        click(&mut app, Control::StarterClass(0, FighterClass::Guardian));
        click(&mut app, Control::FounderAttribute(0, 0, 1));
        let sheet = app.world().resource::<Session>().founding.cards[0].theus;
        click(&mut app, Control::ConfirmFounders);
        let s = app.world().resource::<Session>();
        assert!(!s.founding.open && !s.running);
        assert_eq!(s.camp_page, Some(camp::Page::Prologue));
        assert_eq!(s.campaign.teams[0].fighters[0].name, "Aurel");
        assert_eq!(
            s.campaign.teams[0].fighters[0].class,
            FighterClass::Guardian
        );
        assert_eq!(s.campaign.teams[0].fighters[0].appearance, look);
        assert_eq!(s.campaign.teams[0].fighters[0].theus, sheet);
        assert_eq!(
            serde_json::to_value(&s.campaign.prologue.as_ref().unwrap().waiting).unwrap(),
            waiting
        );
        click(&mut app, Control::StandardBearer);
        assert!(
            app.world()
                .resource::<Session>()
                .battle
                .units
                .iter()
                .all(|u| !u.carries_standard)
        );
        click(&mut app, Control::PrologueNext);
        click(&mut app, Control::Continue);
        assert!(!app.world().resource::<Session>().menu);
        assert_eq!(
            Campaign::load(&path).unwrap().teams[0].fighters[0].fighter_id,
            identity
        );
        click(&mut app, Control::Load);
        let s = app.world().resource::<Session>();
        assert_eq!(s.battle.team_size(), 1);
        assert_eq!(s.camp_page, Some(camp::Page::Prologue));
        assert_eq!(s.campaign.teams[0].fighters[0].fighter_id, identity);
        assert_eq!(s.campaign.teams[0].fighters[0].appearance, look);
        assert_eq!(s.campaign.teams[0].fighters[0].theus, sheet);
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn chapter_buttons_expand_live_snapshots_and_unlock_the_existing_tutorials() {
        let (mut app, directory) = fixture();
        click(&mut app, Control::NewGame);
        click(&mut app, Control::RevealFounder(0));
        click(&mut app, Control::FoundingStep);
        click(&mut app, Control::ConfirmFounders);
        click(
            &mut app,
            Control::BattleStyle(phalanx::styles::BattleStyle::Protector),
        );
        let identity = app.world().resource::<Session>().campaign.teams[0].fighters[0]
            .fighter_id
            .clone();
        for _ in 0..24 {
            {
                let mut session = app.world_mut().resource_mut::<Session>();
                while !session.battle.finished {
                    session.battle.step();
                }
                session.settle_finished();
                assert_eq!(session.camp_page, Some(camp::Page::Report));
                assert!(session.rewarded);
            }
            // This UI-flow witness repairs lasting limb loss using actual
            // earned resources. Repeating combat with four missing limbs is
            // not a test of chapter-button wiring or opening balance.
            for _ in 0..4 {
                let repair = {
                    let s = app.world().resource::<Session>();
                    let team = &s.campaign.teams[0];
                    let f = &team.fighters[0];
                    f.lost_limbs & !f.prosthetics != 0 && team.gold >= 15 && team.salvage >= 5
                };
                if !repair {
                    break;
                }
                click(&mut app, Control::Prosthetic);
            }
            click(&mut app, Control::PrologueNext);
            let session = app.world().resource::<Session>();
            assert_eq!(session.previous_positions.len(), session.battle.units.len());
            assert_eq!(session.selected, 0);
            assert_eq!(session.campaign.teams[0].fighters[0].fighter_id, identity);
            assert_eq!(
                session.battle.team_size(),
                session.campaign.teams[0].fighters.len()
            );
            if !session.campaign.prologue_active() {
                break;
            }
        }
        assert!(!app.world().resource::<Session>().campaign.prologue_active());
        assert_eq!(app.world().resource::<Session>().battle.team_size(), 3);
        click(&mut app, Control::NextTutorial);
        assert_eq!(
            app.world().resource::<Session>().campaign.teams[1].rival_style,
            Some(phalanx::rivals::RivalStyle::Bloodhound)
        );
        assert_eq!(
            app.world()
                .resource::<Session>()
                .campaign
                .tutorial
                .completed(),
            0
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn classic_new_game_preserves_the_old_save_until_new_founders_are_accepted() {
        let (mut app, directory) = fixture();
        let path = directory.join("campaign.json");
        let original = std::fs::read(&path).unwrap();
        app.world_mut().resource_mut::<Session>().selected = 7;
        app.world_mut().resource_mut::<Session>().battle.step();
        click(&mut app, Control::LegacyNewGame);
        let session = app.world().resource::<Session>();
        assert!(session.menu && session.founding.open && !session.running);
        assert_eq!(session.founding.stage, 0);
        assert_eq!(session.campaign.teams[0].fighters.len(), 3);
        assert_eq!(session.campaign.teams[0].gold, 100);
        assert_eq!(session.selected, 0);
        assert_eq!(session.battle.tick, 0);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        click(&mut app, Control::FoundingStep);
        click(&mut app, Control::FounderLook(1, 4));
        click(&mut app, Control::FoundingStep);
        app.world_mut().resource_mut::<Session>().founding.name = "New beginnings".into();
        click(&mut app, Control::ConfirmFounders);
        let saved = Campaign::load(&path).unwrap();
        assert!(saved.founders_prepared);
        assert_eq!(saved.teams[0].name, "New beginnings");
        assert_eq!(saved.teams[0].fighters.len(), 3);
        assert_eq!(saved.teams[0].gold, 100);
        assert_eq!(saved.teams[0].fighters[1].appearance.female, Some(true));
        assert_eq!(
            std::fs::read(path.with_extension("json.bak")).unwrap(),
            original
        );
        assert!(!app.world().resource::<Session>().founding.open);
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn the_creator_can_return_to_the_menu_resume_or_load_the_saved_campaign() {
        let (mut app, directory) = fixture();
        let path = directory.join("campaign.json");
        let bytes = std::fs::read(&path).unwrap();
        click(&mut app, Control::LegacyNewGame);
        click(&mut app, Control::FoundingStep);
        click(&mut app, Control::FounderLook(0, 2));
        let edited = app.world().resource::<Session>().founding.cards[0].appearance;
        click(&mut app, Control::CreatorMenu);
        assert!(!app.world().resource::<Session>().founding.open);
        click(&mut app, Control::Continue);
        assert!(app.world().resource::<Session>().founding.open);
        assert_eq!(
            app.world().resource::<Session>().founding.cards[0].appearance,
            edited
        );
        click(&mut app, Control::Load);
        let session = app.world().resource::<Session>();
        assert!(!session.menu && !session.founding.open && !session.running);
        assert_eq!(session.campaign.teams[0].name, "Veterans");
        assert_eq!(session.campaign.teams[0].fighters.len(), 4);
        assert_eq!(session.campaign.teams[0].fighters[1].appearance.nose, 12);
        assert_eq!(session.campaign.teams[0].fighters[1].appearance.mouth, 4);
        assert_eq!(session.battle.units.len(), 8);
        assert_eq!(session.selected, 0);
        assert_eq!(
            std::fs::read(path).unwrap(),
            bytes,
            "loading must not overwrite the save"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn missing_and_invalid_saves_leave_the_current_game_intact() {
        let (mut app, directory) = fixture();
        let path = directory.join("campaign.json");
        let before = serde_json::to_string(&app.world().resource::<Session>().battle).unwrap();
        let run = app.world().resource::<Session>().run_id;
        std::fs::remove_file(&path).unwrap();
        click(&mut app, Control::Load);
        assert!(
            app.world()
                .resource::<Session>()
                .save_notice
                .contains("No saved game")
        );
        assert!(!path.exists());
        std::fs::write(&path, b"broken save").unwrap();
        click(&mut app, Control::Load);
        let session = app.world().resource::<Session>();
        assert!(session.save_notice.contains("Load failed"));
        assert_eq!(serde_json::to_string(&session.battle).unwrap(), before);
        assert_eq!(session.run_id, run);
        assert_eq!(std::fs::read(path).unwrap(), b"broken save");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
