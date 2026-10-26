//! Compact Founders play view. The scientific inspector remains available with I.
use super::*;
use phalanx::styles::BattleStyle;
pub const CARDS_PER_PAGE: usize = 6;
pub struct View {
    pub details: bool,
    pub research: bool,
    pub side: usize,
    pub page: usize,
}
impl Default for View {
    fn default() -> Self {
        Self {
            details: false,
            research: false,
            side: 0,
            page: 0,
        }
    }
}
pub fn page_count(fighters: usize) -> usize {
    fighters.div_ceil(CARDS_PER_PAGE).max(1)
}
pub fn card_id(side: usize, page: usize, card: usize, count: usize) -> Option<usize> {
    let slot = page * CARDS_PER_PAGE + card;
    (side < 2 && card < CARDS_PER_PAGE && slot < count).then_some(side * count + slot)
}
#[derive(Component)]
pub struct ResearchHud;
#[derive(Component)]
pub struct PortraitHud;
#[derive(Component, Clone, Copy)]
enum Panel {
    Top,
    Roster,
    Details,
}
#[derive(Component)]
struct Heading;
#[derive(Component)]
struct Detail;
#[derive(Component)]
struct Notice;
#[derive(Component)]
struct RosterHeading;
#[derive(Component)]
struct Card(usize);
#[derive(Component)]
struct CardLabel(usize);
#[derive(Component)]
struct CardHealth(usize);
#[derive(Component)]
struct StyleButton(BattleStyle);
pub struct Player;
impl Plugin for Player {
    fn build(&self, app: &mut App) {
        if std::env::args().any(|a| a == "--player-demo") {
            app.add_systems(Update, record_demo.after(update));
        }

        app.add_systems(Startup, setup_player.after(setup))
            .add_systems(Update, scroll_details)
            .add_systems(
                Update,
                update
                    .after(update_divergence)
                    .after(rewards::tick)
                    .after(battlefield::show_briefing),
            );
    }
}
fn setup_player(mut commands: Commands) {
    commands
        .spawn((
            Panel::Top,
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                right: px(12),
                top: px(10),
                padding: UiRect::all(px(6)),
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.04, 0.055, 0.93)),
        ))
        .with_children(|p| {
            p.spawn((
                Heading,
                Text::new(""),
                font(14.),
                TextColor(Color::srgb(1., 0.81, 0.4)),
            ));
            p.spawn(Node {
                column_gap: px(5),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|row| {
                button(row, "Fight / pause", Control::TogglePlay);
                button(row, "Fighters", Control::BattleView);
                button(row, "Speed", Control::Speed);
                button(row, "Cinema [V]", Control::Cinematic);
                button(row, "Workshop", Control::Camp(camp::Page::Workshop));
                button(row, "Menu", Control::Menu);
            });
        });
    commands
        .spawn((
            Panel::Roster,
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                bottom: px(10),
                padding: UiRect::all(px(6)),
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.04, 0.055, 0.96)),
        ))
        .with_children(|p| {
            p.spawn(Node {
                column_gap: px(4),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                button(row, "Your phalanx", Control::RosterSide(0));
                button(row, "Rivals", Control::RosterSide(1));
                button(row, "<", Control::RosterPage(-1));
                button(row, ">", Control::RosterPage(1));
                row.spawn((RosterHeading, Text::new(""), font(12.)));
            });
            p.spawn(Node {
                column_gap: px(5),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|row| {
                for i in 0..CARDS_PER_PAGE {
                    row.spawn((
                        Card(i),
                        Control::SelectFighter(i),
                        Button,
                        PressLatch::default(),
                        BorderColor::all(Color::srgb(0.22, 0.3, 0.36)),
                        BackgroundColor(Color::srgb(0.09, 0.14, 0.18)),
                        Node {
                            width: px(132),
                            min_height: px(62),
                            border: UiRect::all(px(2)),
                            padding: UiRect::all(px(5)),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(3),
                            ..default()
                        },
                    ))
                    .with_children(|p| {
                        p.spawn((CardLabel(i), Text::new(""), font(12.)));
                        p.spawn((
                            Node {
                                width: percent(100),
                                height: px(5),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.03, 0.04, 0.05)),
                        ))
                        .with_children(|p| {
                            p.spawn((
                                CardHealth(i),
                                Node {
                                    height: percent(100),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.2, 0.8, 0.4)),
                            ));
                        });
                    });
                }
            });
            p.spawn((
                Notice,
                Text::new(""),
                font(12.),
                TextColor(Color::srgb(0.7, 0.82, 0.82)),
            ));
        });
    commands.spawn((Panel::Details,ScrollPosition::default(),Node{position_type:PositionType::Absolute,right:px(16),top:px(130),bottom:px(18),width:px(330),
        padding:UiRect::all(px(17)),flex_direction:FlexDirection::Column,row_gap:px(10),overflow:Overflow::scroll_y(),..default()},
        BackgroundColor(Color::srgba(0.025,0.04,0.055,0.95))))
    .with_children(|p|{
        p.spawn((Detail,Text::new(""),font(15.)));
        p.spawn((Text::new("SEVEN TRAITS"),font(13.),TextColor(Color::srgb(1.,0.78,0.35))));
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
                    font(13.),
                    Node {
                        width: px(180),
                        ..default()
                    },
                ));
                button(row, "-", Control::Trait(i, -0.1));
                button(row, "+", Control::Trait(i, 0.1));
            });
        }
        p.spawn((Text::new("A style below replaces all seven. + and - move one. Styles lock during a fight. Traits do not."),font(12.),TextColor(Color::srgb(0.58,0.72,0.76))));
        button(p,"Follow this fighter",Control::FollowFighter);
        button(p,"Rename this fighter",Control::RenameFighter);
        for style in BattleStyle::ALL {
            p.spawn((StyleButton(style),Control::BattleStyle(style),Button,PressLatch::default(),
                BackgroundColor(Color::srgb(0.12,0.18,0.25)),Node{padding:UiRect::all(px(10)),flex_direction:FlexDirection::Column,..default()}))
            .with_children(|p|{p.spawn((Text::new(format!("{}\n{}",style.name(),style.description())),font(13.)));});
        }
        button(p,"Equipment, skills & class",Control::Camp(camp::Page::Workshop));
        button(p, "Whole battlefield [Home]", Control::FrameBattle);
        button(p, "New battlefield [G]", Control::GenerateArena);
        button(p, "Research [I]", Control::ResearchView);
    });
}
#[allow(clippy::too_many_arguments)]
fn update(
    session: Res<Session>,
    mut panels: Query<
        (&Panel, &mut Node),
        (
            Without<ResearchHud>,
            Without<PortraitHud>,
            Without<Card>,
            Without<CardHealth>,
        ),
    >,
    mut research: Query<
        (&mut Node, Option<&InspectorPanel>),
        (
            With<ResearchHud>,
            Without<Panel>,
            Without<PortraitHud>,
            Without<Card>,
            Without<CardHealth>,
        ),
    >,
    mut portraits: Query<
        &mut Node,
        (
            With<PortraitHud>,
            Without<ResearchHud>,
            Without<Panel>,
            Without<Card>,
            Without<CardHealth>,
        ),
    >,
    mut cards: Query<
        (&Card, &mut Control, &mut Node, &mut BorderColor),
        (
            Without<CardHealth>,
            Without<Panel>,
            Without<ResearchHud>,
            Without<PortraitHud>,
        ),
    >,
    mut health: Query<
        (&CardHealth, &mut Node, &mut BackgroundColor),
        (
            Without<Card>,
            Without<Panel>,
            Without<ResearchHud>,
            Without<PortraitHud>,
        ),
    >,
    mut text: ParamSet<(
        Query<&mut Text, With<Heading>>,
        Query<&mut Text, With<Detail>>,
        Query<&mut Text, With<RosterHeading>>,
        Query<&mut Text, With<Notice>>,
        Query<(&CardLabel, &mut Text)>,
    )>,
    mut styles: Query<(&StyleButton, &mut BackgroundColor), Without<CardHealth>>,
) {
    let play =
        session.battle.founders && !session.player.research && !session.menu && !session.harvesting;
    let map = session.map_briefing
        && session.battle.tick == 0
        && matches!(session.battle.arena, Arena::Pressure(_));
    let cinema = session.cinema_on && !session.cinema_grabbed && !session.camera_follow;
    for (panel, mut node) in &mut panels {
        if matches!(panel, Panel::Top) {
            node.right = px(if session.player.details { 370. } else { 12. });
        }
        let visible = match panel {
            Panel::Top => true,
            Panel::Details => !map && session.player.details,
            Panel::Roster => !session.cinematic_playing(),
        };
        node.display = if play && visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (mut node, inspector) in &mut research {
        node.display = if !session.menu
            && !session.harvesting
            && !play
            && !(inspector.is_some() && map && !session.player.research)
        {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut node in &mut portraits {
        node.display = if play || session.harvesting {
            Display::None
        } else {
            Display::Flex
        };
    }
    if !session.battle.founders {
        return;
    }
    let battle = &session.battle;
    let camera = if cinema {
        session.cinema.shot().name()
    } else {
        "MANUAL CAMERA · V to resume cinema"
    };
    let heading = format!(
        "{}  {} : {}  {}  ·  {}  ×{}  ·  {}{}",
        session.team_name(0),
        battle.survivors(0),
        battle.survivors(1),
        session.team_name(1),
        if session.running {
            "FIGHTING"
        } else {
            "PAUSED"
        },
        session.speed,
        camera,
        if session.player.details {
            format!("\n{}", why_now(&session, session.selected))
        } else {
            String::new()
        }
    );
    for mut t in &mut text.p0() {
        if t.0 != heading {
            t.0 = heading.clone();
        }
    }
    let u = &battle.units[session.selected];
    let f = &session.campaign.teams[u.team].fighters[u.id % battle.team_size()];
    let mind = if battle.tick == 0 || battle.finished {
        f.mind
    } else {
        u.mind
    };
    let style = BattleStyle::ALL
        .into_iter()
        .find(|s| s.matches(mind))
        .map(|s| s.name())
        .unwrap_or("INDIVIDUAL MIND");
    let synergy = match f.class {
        FighterClass::Paladin | FighterClass::BannerBearer => {
            "BUILD IDEA: Keep allies near your aura. Walls interrupt support."
        }
        FighterClass::Healer => {
            "BUILD IDEA: Healing needs a nearby wounded ally and clear sight. Protect that lane."
        }
        FighterClass::Ranger => {
            "BUILD IDEA: Arrows stop at allies. Find a clear firing lane behind your front line."
        }
        FighterClass::Scout => "BUILD IDEA: Speed finds flanks; isolation can leave you exposed.",
        FighterClass::Berserker => {
            "BUILD IDEA: Low HP raises damage. Healing trades some rage for more time alive."
        }
        FighterClass::Guardian => {
            "BUILD IDEA: Anchor an approach; give your heavy fighter ranged support."
        }
        _ => {
            "BUILD IDEA: Class, mind, equipped skills, gear and scars combine. Master a discipline, then try retraining."
        }
    };
    let rules = phalanx::theus_combat::description(u);
    let detail = format!(
        "{}\n{} / {}\n{:.0} / {:.0} HP · {}\n{rules}\n{}\n{synergy}",
        f.name,
        f.title.name(),
        f.class.name(),
        u.health,
        u.max_health(),
        style,
        why_now(&session, session.selected),
    );
    for mut t in &mut text.p1() {
        if t.0 != detail {
            t.0 = detail.clone();
        }
    }
    let count = battle.team_size();
    let page = session.player.page.min(page_count(count) - 1);
    let label = format!("{} fighters | {}/{}", count, page + 1, page_count(count));
    for mut t in &mut text.p2() {
        if t.0 != label {
            t.0 = label.clone();
        }
    }
    let own = &session.campaign.teams[0];
    let recruitment = if count == 3 {
        let price = phalanx::progression::slot_cost(4).unwrap_or(0);
        format!(
            "Next recruit: {} / {price} gold | Buy slot 4 in the menu",
            own.gold
        )
    } else {
        format!(
            "League {count} | Next slot target: {} gold (future league)",
            phalanx::progression::slot_cost(count + 1).unwrap_or(0)
        )
    };
    let lesson = phalanx::tutorial::LESSONS
        .iter()
        .find(|l| Some(l.rival) == session.campaign.teams[1].rival_style);
    let lesson_name = lesson.map(|l| l.title).unwrap_or("Free sparring");
    let notice = format!(
        "Trial {} / 5 · {}\n{} · {} salvage",
        session.campaign.tutorial.completed(),
        lesson_name,
        recruitment,
        own.salvage
    );
    for mut t in &mut text.p3() {
        if t.0 != notice {
            t.0 = notice.clone();
        }
    }
    for (card, mut control, mut node, mut border) in &mut cards {
        let id = card_id(session.player.side, page, card.0, count);
        node.display = if id.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(id) = id {
            *control = Control::SelectFighter(id);
            border.set_all(if id == session.selected {
                Color::srgb(1., 0.75, 0.25)
            } else {
                Color::srgb(0.2, 0.28, 0.35)
            });
        }
    }
    for (card, mut t) in &mut text.p4() {
        if let Some(id) = card_id(session.player.side, page, card.0, count) {
            let u = &battle.units[id];
            let f = &session.campaign.teams[u.team].fighters[id % count];
            let shown = if battle.tick == 0 || battle.finished {
                f.mind
            } else {
                u.mind
            };
            let label = format!(
                "{} · {}\n{}{}{}",
                f.name,
                f.class.name(),
                if u.health <= 0. {
                    "DOWN".into()
                } else {
                    format!("{:.0} HP", u.health)
                },
                card_flag(&session, id)
                    .map(|flag| format!("\n{flag}"))
                    .unwrap_or_default(),
                if session.player.details {
                    format!("\n{}", mind_line(shown))
                } else {
                    String::new()
                }
            );
            if t.0 != label {
                t.0 = label;
            }
        }
    }
    for (card, mut node, mut color) in &mut health {
        if let Some(id) = card_id(session.player.side, page, card.0, count) {
            let hp = battle.units[id].health_fraction().clamp(0., 1.);
            node.width = percent(hp * 100.);
            color.0 = if hp < 0.3 {
                Color::srgb(0.95, 0.2, 0.15)
            } else {
                Color::srgb(0.25, 0.85, 0.4)
            };
        }
    }
    for (button, mut bg) in &mut styles {
        bg.0 = if button.0.matches(mind) {
            Color::srgb(0.31, 0.26, 0.12)
        } else {
            Color::srgb(0.12, 0.18, 0.25)
        };
    }
}

#[derive(Default)]
struct Demo {
    seconds: f32,
    frame: u32,
    next: f32,
    phase: usize,
}
fn record_demo(
    time: Res<Time>,
    mut demo: Local<Demo>,
    mut session: ResMut<Session>,
    fx: Res<visuals::Fx>,
    panic: Res<visuals::PanicAudit>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    demo.seconds += time.delta_secs().min(0.25);
    let t = demo.seconds - 2.;
    if t < 0. {
        return;
    }
    match demo.phase {
        0 => {
            session.founding.name = "THE BAD DECISIONS".into();
            for i in 0..3 {
                session.founding.reveal(i).unwrap();
            }
            demo.phase = 1;
        }
        1 if t > 2. => {
            apply(Control::ConfirmFounders, &mut session);
            apply(Control::SelectFighter(0), &mut session);
            apply(Control::BattleStyle(BattleStyle::Protector), &mut session);
            demo.phase = 2;
        }
        2 if t > 3.5 => {
            apply(Control::SelectFighter(1), &mut session);
            apply(Control::Class(FighterClass::Healer), &mut session);
            apply(Control::BattleStyle(BattleStyle::Skirmisher), &mut session);
            demo.phase = 3;
        }
        3 if t > 5. => {
            apply(Control::SelectFighter(0), &mut session);
            apply(Control::SelectFighter(2), &mut session);
            apply(Control::BattleStyle(BattleStyle::Protector), &mut session);
            apply(Control::SelectFighter(0), &mut session);
            apply(Control::BattleStyle(BattleStyle::Vanguard), &mut session);
            session.speed = 4.;
            apply(Control::TogglePlay, &mut session);
            demo.phase = 4;
        }
        4 if t > 9. => {
            apply(Control::FollowFighter, &mut session);
            session.speed = 1.;
            demo.phase = 5;
        }
        5 if t > 18. => {
            apply(Control::FrameBattle, &mut session);
            session.speed = 8.;
            demo.phase = 6;
        }
        6 if t > 23. => {
            if !session.menu {
                apply(Control::ResearchView, &mut session);
            }
            demo.phase = 7;
        }
        7 if t > 25. => {
            session.player.research = false;
            demo.phase = 8;
        }
        _ => {}
    }
    if t >= demo.next && t < 29. {
        std::fs::create_dir_all("outputs/player-demo").expect("create recording directory");
        let path = format!("outputs/player-demo/{:05}.png", demo.frame);
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(path));
        demo.frame += 1;
        demo.next = t + 0.1;
    }
    if t > 29.8 {
        let audit = serde_json::json!({"scripted_controls":true,"seed":session.battle.seed,"fighters_per_side":session.battle.team_size(),"frames":demo.frame,"controller":session.battle.controller.name(),"finished":session.battle.finished,"tick":session.battle.tick,"fountains_started":fx.streams_started,"peak_fountains":fx.peak_streams,"stained_pixels":fx.stained_pixels(),"near_death_clips_seen":panic.clips_seen,"tutorial_attempts":session.campaign.tutorial.attempts,"tutorial_cleared":session.campaign.tutorial.cleared,"gold":session.campaign.teams[0].gold,"salvage":session.campaign.teams[0].salvage});
        std::fs::write(
            "outputs/player-demo/audit.json",
            serde_json::to_string_pretty(&audit).unwrap(),
        )
        .unwrap();
        println!("PLAYER_DEMO {audit}");
        exit.write(AppExit::Success);
    }
}

fn call_name(session: &Session, id: usize) -> String {
    let unit = &session.battle.units[id];
    if session.battle.founders {
        let fighter = &session.campaign.teams[unit.team].fighters[id % session.battle.team_size()];
        let name = fighter.name.trim();
        if name.is_empty() {
            unit.command_title
                .map(|title| title.name().to_string())
                .unwrap_or_else(|| fighter.class.name().into())
        } else {
            name.to_string()
        }
    } else {
        session.battle.unit_label(id)
    }
}
fn living_allies<'a>(battle: &'a Battle, id: usize) -> Vec<&'a phalanx::Unit> {
    let team = battle.units[id].team;
    battle
        .units
        .iter()
        .filter(|ally| ally.team == team && ally.id != id && ally.health > 0.0)
        .collect()
}
fn away_from_company(battle: &Battle, id: usize) -> bool {
    let here = battle.units[id].position;
    let allies = living_allies(battle, id);
    allies.is_empty()
        || allies
            .iter()
            .map(|ally| phalanx::distance(here, ally.position))
            .fold(f32::MAX, f32::min)
            > 5.0
}
fn help_refused(unit: &phalanx::Unit) -> bool {
    unit.health > 0.0 && unit.action != 8 && unit.scores[8] > 0.0
}
fn route_missing(battle: &Battle, unit: &phalanx::Unit) -> bool {
    battle.tick > 0
        && unit.health > 0.0
        && unit.route.points.is_empty()
        && matches!(unit.action, 0 | 3 | 4 | 5 | 7 | 8)
}
/// One readable line for the selected fighter. It reports the choice already made.
fn why_now(session: &Session, id: usize) -> String {
    let battle = &session.battle;
    let unit = &battle.units[id];
    let name = call_name(session, id);
    let choice = if unit.decision_reason.is_empty() {
        ACTIONS[unit.action].to_string()
    } else {
        unit.decision_reason.clone()
    };
    if unit.health <= 0.0 {
        let killer = battle
            .damage_events
            .iter()
            .rev()
            .find(|event| event.defender == id)
            .map(|event| call_name(session, event.attacker));
        let apart = if away_from_company(battle, id) {
            " They were away from the company."
        } else {
            ""
        };
        return match killer {
            Some(killer) => format!("{name} fell to {killer}. Last choice: {choice}.{apart}"),
            None => format!("{name} is down. Last choice: {choice}.{apart}"),
        };
    }
    let mut notes = Vec::new();
    if away_from_company(battle, id) {
        let aura = unit.has_skill(FighterClass::Paladin)
            || unit.command_title == Some(CommandTitle::General);
        notes.push(if aura {
            "away from the company, so the aura is covering nobody".to_string()
        } else if unit.mind.teamwork < 0.45 {
            "away from the company. Teamwork is too low to pull them back".to_string()
        } else if unit.mind.caution > 0.65 && matches!(unit.action, 5 | 7) {
            "left the fight. Caution outranked staying".to_string()
        } else if unit.mind.aggression > 0.65 && matches!(unit.action, 0 | 3) {
            "walked off. Aggression outranked the company".to_string()
        } else {
            "away from the company".to_string()
        });
    }
    if help_refused(unit) {
        let who = unit
            .support_target
            .map(|ally| call_name(session, ally))
            .unwrap_or_else(|| "an ally".into());
        notes.push(if unit.mind.teamwork < 0.45 {
            format!("could have helped {who}, and teamwork lost that argument")
        } else {
            format!("could have helped {who}, and another urge scored higher")
        });
    }
    if route_missing(battle, unit) {
        notes.push("no walkable route to the goal".into());
    }
    if notes.is_empty() {
        format!("{name}: {choice}")
    } else {
        format!("{name}: {}. {choice}", notes.join(". "))
    }
}
fn card_flag(session: &Session, id: usize) -> Option<&'static str> {
    let unit = &session.battle.units[id];
    if unit.health <= 0.0 {
        Some("DOWN")
    } else if away_from_company(&session.battle, id) {
        Some("ALONE")
    } else if help_refused(unit) {
        Some("WON'T HELP")
    } else if route_missing(&session.battle, unit) {
        Some("NO PATH")
    } else {
        None
    }
}
fn mind_line(mind: Mind) -> String {
    format!(
        "A{:.0} C{:.0} R{:.0} T{:.0}\nP{:.0} S{:.0} D{:.0}",
        mind.aggression * 100.0,
        mind.caution * 100.0,
        mind.courage * 100.0,
        mind.teamwork * 100.0,
        mind.patience * 100.0,
        mind.self_preservation * 100.0,
        mind.discipline * 100.0
    )
}
fn scroll_details(
    session: Res<Session>,
    window: Single<&Window>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut panels: Query<(&Panel, &mut ScrollPosition)>,
) {
    for event in wheel.read() {
        if !session.menu
            && !session.player.research
            && window
                .cursor_position()
                .is_some_and(|p| p.x > window.width() - 365.)
        {
            let delta = match event.unit {
                bevy::input::mouse::MouseScrollUnit::Line => event.y * 30.,
                bevy::input::mouse::MouseScrollUnit::Pixel => event.y,
            };
            for (panel, mut scroll) in &mut panels {
                if matches!(panel, Panel::Details) {
                    scroll.0.y = (scroll.0.y - delta).max(0.);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        let mut s = Session::new(Battle::new(42, false));
        s.autosave = false;
        s.campaign = Campaign::new();
        s.restart_founders(42, Arena::OpenField, Controller::Utility);
        s
    }
    #[test]
    fn paged_roster_covers_twenty_fighters_per_side_without_inventing_slots() {
        assert_eq!(page_count(20), 4);
        for count in [3, 4, 20] {
            for side in 0..2 {
                let ids: Vec<_> = (0..page_count(count))
                    .flat_map(|page| {
                        (0..CARDS_PER_PAGE).filter_map(move |card| card_id(side, page, card, count))
                    })
                    .collect();
                assert_eq!(ids, (side * count..(side + 1) * count).collect::<Vec<_>>());
            }
        }
        assert_eq!(card_id(0, 0, 3, 3), None);
    }
    #[test]
    fn view_and_selection_controls_do_not_change_the_simulation() {
        let mut s = session();
        let before = serde_json::to_string(&s.battle).unwrap();
        for c in [
            Control::ResearchView,
            Control::RosterSide(1),
            Control::RosterPage(1),
            Control::SelectFighter(4),
            Control::FollowFighter,
            Control::FrameBattle,
            Control::ResearchView,
        ] {
            apply(c, &mut s);
        }
        assert_eq!(serde_json::to_string(&s.battle).unwrap(), before);
        assert_eq!(s.battle.controller, Controller::Utility);
        assert_eq!(s.selected, 4);
    }
    #[test]
    fn styles_persist_without_cost_and_cannot_rewrite_live_or_completed_combat() {
        let mut s = session();
        let id = s.campaign.teams[0].fighters[0].fighter_id.clone();
        apply(Control::BattleStyle(BattleStyle::Protector), &mut s);
        assert!(BattleStyle::Protector.matches(s.battle.units[0].mind));
        assert_eq!(s.campaign.teams[0].gold, 100);
        assert_eq!(s.campaign.teams[0].fighters.len(), 3);
        let loaded: Campaign =
            serde_json::from_str(&serde_json::to_string(&s.campaign).unwrap()).unwrap();
        assert!(BattleStyle::Protector.matches(loaded.teams[0].fighters[0].mind));
        assert_eq!(loaded.teams[0].fighters[0].fighter_id, id);
        s.battle.step();
        let live = serde_json::to_string(&s.battle).unwrap();
        apply(Control::BattleStyle(BattleStyle::Vanguard), &mut s);
        assert_eq!(serde_json::to_string(&s.battle).unwrap(), live);
        while !s.battle.finished {
            s.battle.step();
        }
        s.settle_finished();
        let completed = serde_json::to_string(&s.battle).unwrap();
        let report = serde_json::to_string(&s.campaign.last_report).unwrap();
        apply(Control::BattleStyle(BattleStyle::Skirmisher), &mut s);
        assert_eq!(serde_json::to_string(&s.battle).unwrap(), completed);
        assert_eq!(
            serde_json::to_string(&s.campaign.last_report).unwrap(),
            report
        );
        s.restart_founders(43, Arena::OpenField, Controller::Utility);
        assert!(BattleStyle::Skirmisher.matches(s.battle.units[0].mind));
    }
    #[test]
    fn the_play_view_says_why_someone_left_refused_or_fell() {
        let mut s = session();
        let general = s
            .battle
            .units
            .iter()
            .position(|u| u.team == 0 && u.command_title == Some(CommandTitle::General))
            .unwrap();
        let king = s
            .battle
            .units
            .iter()
            .position(|u| u.team == 0 && u.command_title == Some(CommandTitle::King))
            .unwrap();
        s.battle.units[general].position = s.battle.units[king].position;
        s.battle.units[general].decision_reason = "Advance on observed/reported contact".into();
        let together = why_now(&s, general);
        assert!(together.contains("Advance on observed/reported contact"));
        assert!(!together.contains("away from the company"));
        assert_eq!(card_flag(&s, general), None);

        s.battle.units[general].position = [18.0, 10.0];
        s.battle.units[general].action = 7;
        s.battle.units[general].mind.caution = 0.9;
        s.battle.units[general].mind.teamwork = 0.2;
        s.battle.units[general].scores = [-1.0; ACTION_COUNT];
        s.battle.units[general].scores[7] = 1.1;
        s.battle.units[general].scores[8] = 0.4;
        s.battle.units[general].support_target = Some(king);
        s.battle.units[general].decision_reason =
            "Exposure and self-preservation favor cover".into();
        let lost = why_now(&s, general);
        assert!(lost.contains("aura is covering nobody"), "{lost}");
        assert!(lost.contains("could have helped"), "{lost}");
        assert!(lost.contains("teamwork lost"), "{lost}");
        assert_eq!(card_flag(&s, general), Some("ALONE"));

        let killer = s.battle.team_size();
        s.battle.units[general].health = 0.0;
        s.battle.damage_events.push(phalanx::DamageEvent {
            sequence: 1,
            tick: 40,
            attacker: killer,
            defender: general,
            amount: 80.0,
            velocity: [1.0, 0.0],
            position: s.battle.units[general].position,
            severed: 0,
            blunt: true,
        });
        let fallen = why_now(&s, general);
        assert!(fallen.contains("fell to"), "{fallen}");
        assert!(fallen.contains("away from the company"), "{fallen}");
        assert_eq!(card_flag(&s, general), Some("DOWN"));
    }
}
