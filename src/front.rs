//! Compact local-game front end. Combat is paused while this overlay is open.
use super::*;
pub struct FrontEnd;
#[derive(Component)]
struct MainMenu(bool);
#[derive(Component)]
struct MenuSummary;
#[derive(Component)]
struct MenuScroll;
impl Plugin for FrontEnd {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_menu.after(setup))
            .add_systems(Update, (menu, quit, scroll_menu).after(controls));
    }
}
fn setup_menu(mut commands: Commands) {
    commands.spawn((MainMenu(false), GlobalZIndex(100), Interaction::default(),
        Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100),
            align_items: AlignItems::Center, justify_content: JustifyContent::Center, ..default() },
        BackgroundColor(Color::srgba(0.018,0.026,0.035,0.98))))
        .with_children(|root| {
            root.spawn((MenuScroll, ScrollPosition::default(), Node { width: px(630), max_width: vw(94), max_height: vh(95), overflow: Overflow::scroll_y(), padding: UiRect::all(px(28)),
                flex_direction: FlexDirection::Column, row_gap: px(12), ..default() }))
                .with_children(|p| {
                    p.spawn((Text::new("PHALANX"),font(48.0),TextColor(Color::srgb(1.0,0.72,0.08))));
                    p.spawn((Text::new("Build their minds. Watch them fight. Bring them home."),font(16.0)));
                    p.spawn((MenuSummary,Text::new(""),font(15.0),TextColor(Color::srgb(0.65,0.82,0.84))));
                    button(p,"Enter arena / manage your fighters",Control::Continue);
                    button(p,"Squad workshop",Control::Camp(camp::Page::Workshop));
                    button(p,"Last battle report",Control::Camp(camp::Page::Report));
                    button(p,"Choose a rival phalanx",Control::Camp(camp::Page::Rivals));
                    button(p,"Snapshot challenges / exchange teams",Control::Camp(camp::Page::Challenges));
                    button(p,"Unlock fourth fighter / League 4 - 1000 gold",Control::Recruit);
                    button(p,"View League 3 -> 20 gold ladder",Control::Ladder);
                    button(p,"Change arena",Control::Arena);
                    button(p,"Create a procedural battlefield",Control::GenerateArena);
                    button(p,"Switch Game / AI Lab",Control::Setup);
                    button(p,"Save squads",Control::Save);
                    button(p,"Load saved squads",Control::Load);
                    super::audio::menu(p);
                    button(p,"Quit",Control::Quit);
                    p.spawn((Text::new("Select a fighter to change class, appearance and mind.\nRetrain: 10 gold | Iron limb: 15 gold + 5 salvage\nLeague 3: 30 gold win / 15 otherwise | rewards rise by league\nBattles scar minds. The same event marks two personalities differently.\nFounders fields are generated for both squads. Arena returns to the Lab maps.\n\nRMB orbit | MMB pan | Wheel zoom | F follow\nSpace pause / run | Tab next fighter | Esc menu\n\nLocal prototype | no account or connection required"),font(14.0)));
                });
        });
    let mut ladder = String::from(
        "LEAGUE    SLOT COST    TOTAL INVESTED    WIN GOLD\n3         Starter      0                 30\n",
    );
    let mut total = 0;
    for league in 4..=20 {
        let price = phalanx::progression::slot_cost(league).unwrap();
        total += price;
        ladder.push_str(&format!(
            "{league:<10}{price:<13}{total:<18}{}\n",
            phalanx::progression::gold_reward(league, true)
        ));
    }
    commands.spawn((MainMenu(true),GlobalZIndex(101),Interaction::default(),
        Node { position_type:PositionType::Absolute,width:percent(100),height:percent(100),
            flex_direction:FlexDirection::Column,align_items:AlignItems::Center,justify_content:JustifyContent::Center,row_gap:px(12),..default()},
        BackgroundColor(Color::srgb(0.018,0.026,0.035))))
        .with_children(|p| {
            p.spawn((Text::new("THE ROAD TO TWENTY"),font(32.),TextColor(Color::srgb(1.,0.72,0.08))));
            p.spawn((Text::new(ladder),font(16.)));
            p.spawn((Text::new("Earned gold only | losses / draws pay half | no passive income\nStarter purse: 100 gold | crafting and retraining compete with savings\nPlayable now: leagues 3-4 | leagues 5-20 are planned content\nPrices are first-pass tuning; a long grind still needs worthwhile battles."),font(14.)));
            button(p,"Back",Control::Ladder);
        });
}
fn menu(
    session: Res<Session>,
    mut panels: Query<(&MainMenu, &mut Node)>,
    mut summaries: Query<&mut Text, With<MenuSummary>>,
) {
    for (page, mut panel) in &mut panels {
        panel.display = if session.menu && session.camp_page.is_none() && session.ladder == page.0 {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut text in &mut summaries {
        let team = &session.campaign.teams[0];
        let content = format!(
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
        );
        if text.0 != content {
            text.0 = content;
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
