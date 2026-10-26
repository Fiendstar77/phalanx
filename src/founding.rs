//! Word-seeded founder reveals and editable names. Initial choices do not spend resources.
use super::*;
use bevy::input::keyboard::{Key, KeyboardInput};
use phalanx::genesis::{self, Founder, Origin};
pub struct Draft {
    pub open: bool,
    pub name: String,
    pub word: String,
    pub classes: [FighterClass; 3],
    pub cards: [Founder; 3],
    pub revealed: [bool; 3],
    pub rolls: [u32; 3],
    pub stamps: [u32; 3],
    pub focus: usize,
    /// 0 is the opening title, 1 is appearance, 2 is class.
    pub stage: u8,
    /// The first three faces appear on a timer once. A seed edit waits for Reveal.
    pub introduced: bool,
    pub closeup: [bool; 3],
    pub angle: [u8; 3],
}
impl Draft {
    pub fn new(c: &Campaign, open: bool) -> Self {
        let classes = std::array::from_fn(|i| c.teams[0].fighters[i].class);
        Self {
            open,
            name: String::new(),
            word: "BONES".into(),
            cards: std::array::from_fn(|i| genesis::reveal("BONES", i, 0, classes[i]).unwrap()),
            classes,
            revealed: [false; 3],
            rolls: [0; 3],
            stamps: [0; 3],
            focus: 0,
            stage: 0,
            introduced: false,
            closeup: [true; 3],
            angle: [0; 3],
        }
    }
    pub fn reveal(&mut self, i: usize) -> Result<(), String> {
        if i >= 3 {
            return Err("Unknown founder".into());
        }
        if self.revealed[i] {
            self.rolls[i] = (self.rolls[i] + 1) % 1_000_000;
        }
        self.cards[i] = genesis::reveal(&self.word, i, self.rolls[i], self.classes[i])?;
        self.revealed[i] = true;
        self.stamps[i] = self.stamps[i].wrapping_add(1);
        Ok(())
    }
    fn text(&mut self) -> &mut String {
        match self.focus {
            0 => &mut self.name,
            1 => &mut self.word,
            i => &mut self.cards[(i - 2).min(2)].name,
        }
    }
    fn changed(&mut self) {
        if self.focus == 1 {
            self.revealed = [false; 3];
            self.rolls = [0; 3];
            self.introduced = true;
        }
    }
    pub fn origin(&self) -> Origin {
        Origin {
            version: 1,
            word: self.word.trim().into(),
            rolls: self.rolls,
        }
    }
}
pub struct Founding;
#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Scroll;
#[derive(Component)]
struct Card(usize);
#[derive(Component)]
struct Portrait(usize);
/// Off-screen card camera. Kept out of the main battlefield camera query.
#[derive(Component)]
pub struct CardCamera;
#[derive(Component)]
struct RenamePanel;
#[derive(Component)]
struct RenameText;
#[derive(Component)]
enum Label {
    Name,
    Seed,
    Fighter(usize),
    Slot(usize),
    Notice,
    Theus(usize),
    Attribute(usize, usize),
}
#[derive(Component)]
struct Opening;
#[derive(Component)]
struct OpeningTitle;
#[derive(Component)]
struct OpeningLine;
#[derive(Component)]
struct LookRow;
#[derive(Component)]
struct LookChoice(usize, usize);
#[derive(Component)]
struct ClassRow;
#[derive(Component)]
struct Advance;
#[derive(Component)]
struct Accept;
impl Plugin for Founding {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_founding.after(setup))
            .add_systems(
                Update,
                (
                    type_name,
                    play_opening,
                    present_founders,
                    refresh,
                    scroll,
                    portraits,
                    refresh_choices,
                )
                    .chain()
                    .after(controls)
                    .after(super::keyboard),
            );
        if std::env::args().any(|a| a == "--creator-review") {
            app.add_systems(Update, review.before(controls));
        }
    }
}
fn label(p: &mut ChildSpawnerCommands, kind: Label, size: f32) {
    p.spawn((kind, Text::new(""), font(size)));
}
fn field(p: &mut ChildSpawnerCommands, kind: Label, focus: usize) {
    p.spawn((
        Button,
        Control::FocusFounderField(focus),
        PressLatch::default(),
        BackgroundColor(Color::srgb(0.10, 0.14, 0.23)),
        Node {
            padding: UiRect::all(px(9)),
            ..default()
        },
    ))
    .with_children(|p| label(p, kind, 18.));
}
fn look_arrow(p: &mut ChildSpawnerCommands, slot: usize, feature: usize, back: bool) {
    p.spawn((
        Button,
        PressLatch::default(),
        if back {
            Control::FounderLookBack(slot, feature)
        } else {
            Control::FounderLook(slot, feature)
        },
        Node {
            width: px(24),
            height: px(26),
            flex_shrink: 0.,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.12, 0.18, 0.25)),
    ))
    .with_children(|p| {
        p.spawn((Text::new(if back { "<" } else { ">" }), font(13.)));
    });
}
fn setup_founding(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let portraits: [Handle<Image>; 3] = std::array::from_fn(|i| {
        let image = images.add(Image::new_target_texture(
            384,
            320,
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            Some(bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb),
        ));
        commands.spawn((
            Portrait(i),
            CardCamera,
            Camera3d::default(),
            Camera {
                order: -5 + i as isize,
                is_active: false,
                ..default()
            },
            bevy::camera::RenderTarget::Image(image.clone().into()),
            Transform::default(),
        ));
        image
    });
    commands
        .spawn((
            Opening,
            GlobalZIndex(160),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: px(14),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgb(0.012, 0.016, 0.028)),
        ))
        .with_children(|p| {
            p.spawn((
                OpeningTitle,
                Text::new("PHALANX"),
                font(68.),
                TextColor(Color::srgba(1., 0.78, 0.28, 0.)),
            ));
            p.spawn((
                OpeningLine,
                Text::new("Three lives. You do not command them."),
                font(22.),
                TextColor(Color::srgba(0.78, 0.86, 0.9, 0.)),
            ));
            p.spawn((
                OpeningLine,
                Text::new(
                    "A word decides who they are. You decide how they look, then how they fight.",
                ),
                font(16.),
                TextColor(Color::srgba(0.78, 0.86, 0.9, 0.)),
            ));
            button(p, "Meet them", Control::FoundingStep);
            button(p, "Back to main menu", Control::CreatorMenu);
        });
    commands.spawn((Panel,GlobalZIndex(150),Interaction::default(),Node {position_type:PositionType::Absolute,width:percent(100),height:percent(100),align_items:AlignItems::Center,justify_content:JustifyContent::Center,display:Display::None,..default()},BackgroundColor(Color::srgb(0.018,0.022,0.045))))
    .with_children(|root| {root.spawn((Scroll,ScrollPosition::default(),Node {width:px(1220),max_width:vw(96),height:vh(97),min_height:px(0),flex_shrink:0.,overflow:Overflow::scroll_y(),padding:UiRect::all(px(18)),flex_direction:FlexDirection::Column,row_gap:px(8),..default()})).with_children(|p| {
        p.spawn((Text::new("A WORD. THREE LIVES. YOUR PHALANX."),font(29.),TextColor(Color::srgb(0.85,0.7,1.))));
        button(p,"Back to main menu",Control::CreatorMenu);
        field(p,Label::Name,0);field(p,Label::Seed,1);
        p.spawn((Text::new("Appearance first. Click the company name or the seed word to type. Reveal each founder, then shape the body under the portrait. Classes come after the faces."),font(15.)));
        p.spawn(Node {column_gap:px(14),..default()}).with_children(|row| {for (i,photo) in portraits.into_iter().enumerate() {
            row.spawn((Card(i),BackgroundColor(Color::srgb(0.055,0.06,0.10)),Node {flex_basis:percent(33.),min_width:px(0),padding:UiRect::all(px(10)),flex_direction:FlexDirection::Column,row_gap:px(5),..default()})).with_children(|card| {
                card.spawn((ImageNode::new(photo),Node {height:px(190),width:percent(100),..default()}));
                card.spawn(Node {column_gap:px(4),..default()}).with_children(|r| {
                    button(r,"Face / body",Control::FounderView(i));
                    button(r,"Turn portrait",Control::FounderTurn(i));
                });
                field(card,Label::Fighter(i),i+2);label(card,Label::Slot(i),14.);
                button(card,"Reveal / reroll this founder",Control::RevealFounder(i));
                card.spawn((LookRow,Node {flex_wrap:FlexWrap::Wrap,row_gap:px(4),display:Display::None,..default()})).with_children(|r| {
                    for choice in 0..phalanx::appearance::FEATURES.len() {
                        r.spawn(Node {width:percent(50),padding:UiRect::right(px(4)),align_items:AlignItems::Center,column_gap:px(2),height:px(34),..default()}).with_children(|row| {
                            look_arrow(row,i,choice,true);
                            row.spawn((LookChoice(i,choice),Text::new(""),font(12.),Node {flex_grow:1.,min_width:px(0),..default()}));
                            look_arrow(row,i,choice,false);
                        });
                    }
                });
                card.spawn((ClassRow,Node {flex_direction:FlexDirection::Column,row_gap:px(3),display:Display::None,..default()})).with_children(|classes| {
                    label(classes,Label::Theus(i),14.);
                    classes.spawn(Node {flex_wrap:FlexWrap::Wrap,..default()}).with_children(|stats| {
                        for attribute in 0..4 {
                            stats.spawn(Node {width:percent(50),align_items:AlignItems::Center,..default()}).with_children(|row| {
                                button(row,"-",Control::FounderAttribute(i,attribute,-1));
                                label(row,Label::Attribute(i,attribute),12.);
                                button(row,"+",Control::FounderAttribute(i,attribute,1));
                            });
                        }
                    });
                    classes.spawn((Text::new("NOW THE CLASS"),font(13.),TextColor(Color::srgb(1.,0.78,0.35))));
                    for pair in FighterClass::ALL.chunks(2) {classes.spawn(Node {flex_wrap:FlexWrap::Wrap,column_gap:px(3),..default()}).with_children(|r| {
                        for class in pair {button(r,class.name(),Control::StarterClass(i,*class));}
                    });}
                });
            });
        }});
        let fourth = phalanx::progression::slot_cost(4).unwrap();
        p.spawn((Text::new(format!("Initial looks and classes are free. Later retraining: 10 gold. A limb is 15 gold. Fourth fighter: {fourth} gold. Names stay editable.")),font(14.)));
        label(p,Label::Notice,14.);
        p.spawn((Advance,Node {display:Display::None,..default()})).with_children(|p| button(p,"Their faces are set. Choose classes.",Control::FoundingStep));
        p.spawn((Accept,Node {display:Display::None,..default()})).with_children(|p| button(p,"Accept these lives / enter the first trial",Control::ConfirmFounders));
        button(p,"Quit",Control::Quit);
        button(p,"Back to main menu",Control::CreatorMenu);
    });});
    commands
        .spawn((
            RenamePanel,
            GlobalZIndex(170),
            Node {
                position_type: PositionType::Absolute,
                left: vw(25),
                top: vh(30),
                width: vw(50),
                padding: UiRect::all(px(24)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.05, 0.10)),
        ))
        .with_children(|p| {
            p.spawn((Text::new("NAME YOUR FIGHTER"), font(25.)));
            p.spawn((RenameText, Text::new(""), font(22.)));
            p.spawn((
                Text::new("Type to edit. Backspace deletes. Enter saves. Escape cancels."),
                font(14.),
            ));
            button(p, "Save name", Control::FinishRename);
            button(p, "Cancel", Control::CancelRename);
        });
}
fn portraits(
    session: Res<Session>,
    pawns: Query<(&Pawn, &Transform), Without<Portrait>>,
    mut cameras: Query<(&Portrait, &mut Camera, &mut Transform)>,
) {
    for (portrait, mut camera, mut transform) in &mut cameras {
        camera.is_active = session.founding.open && session.founding.revealed[portrait.0];
        if let Some((_, pawn)) = pawns.iter().find(|(p, _)| p.0 == portrait.0) {
            let close = session.founding.closeup[portrait.0];
            let target = pawn.translation + Vec3::Y * if close { 0.36 } else { 0.10 };
            let yaw = [0.0, 0.55, 1.15, -1.15, -0.55][session.founding.angle[portrait.0] as usize];
            let offset = Quat::from_rotation_y(yaw)
                * if close {
                    Vec3::new(0.0, 0.025, 1.12)
                } else {
                    Vec3::new(0.0, 0.12, 1.65)
                };
            *transform = Transform::from_translation(target + pawn.rotation * offset)
                .looking_at(target, Vec3::Y);
        }
    }
}
fn refresh_choices(session: Res<Session>, mut labels: Query<(&LookChoice, &mut Text)>) {
    if !session.founding.open {
        return;
    }
    for (choice, mut text) in &mut labels {
        let look = session.founding.cards[choice.0].appearance;
        let value = format!(
            "{}: {}",
            phalanx::appearance::FEATURES[choice.1],
            look.choice_name(choice.1)
        );
        if text.0 != value {
            text.0 = value;
        }
    }
}
#[derive(Default)]
struct Review {
    started: u8,
    saved_name: Option<String>,
    saved_gold: u32,
    seconds: f32,
    next: f32,
    step: u8,
    base: Option<phalanx::progression::Appearance>,
    edited: Option<phalanx::progression::Appearance>,
    sheet: Option<phalanx::theus::Sheet>,
}
// Opt-in scripted render review: the actual visible UI buttons go through
// controls(), then the accepted cosmetics round-trip through the campaign save.
fn review(
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut state: Local<Review>,
    mut buttons: Query<(&Control, &ComputedNode, &mut Interaction)>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    state.seconds += time.delta_secs();
    for (_, _, mut interaction) in &mut buttons {
        *interaction = Interaction::None;
    }
    if state.seconds < 3.5 || state.seconds < state.next {
        return;
    }
    std::fs::create_dir_all("outputs/creator-review").unwrap();
    let starting = state.started < 3;
    let wanted = if starting {
        match state.started {
            1 => Some(Control::NewGame),
            2 => Some(Control::FoundingStep),
            _ => None,
        }
    } else {
        match state.step {
            1 | 3 | 5 => Some(Control::FounderLook(1, 2)),
            7 | 9 => Some(Control::FounderLookBack(1, 4)),
            11 => Some(Control::FounderTurn(1)),
            13 => Some(Control::FoundingStep),
            14 => Some(Control::FounderAttribute(1, 0, 1)),
            15 => Some(Control::StarterClass(1, FighterClass::Healer)),
            17 => Some(Control::ConfirmFounders),
            19 if session.save_path.exists() => Some(Control::Menu),
            21 => Some(Control::Load),
            _ => None,
        }
    };
    if let Some(wanted) = wanted {
        let mut found = false;
        for (control, node, mut interaction) in &mut buttons {
            if format!("{control:?}") == format!("{wanted:?}") && node.size().x > 0. {
                *interaction = Interaction::Pressed;
                found = true;
                break;
            }
        }
        assert!(
            found,
            "creator review could not find visible button {wanted:?}"
        );
    }
    if starting {
        if state.started == 0 {
            state.saved_name = Some(session.campaign.teams[0].name.clone());
            state.saved_gold = session.campaign.teams[0].gold;
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(
                    "outputs/creator-review/00-menu.png",
                ));
        }
        if state.started == 2 {
            session.founding.name = "Creator QA".into();
        }
        state.started += 1;
        state.next = state.seconds + 0.6;
        return;
    }
    let snapshot = match state.step {
        0 => {
            state.base = Some(session.founding.cards[1].appearance);
            Some("01-before")
        }
        6 => {
            assert_eq!(
                session.founding.cards[1].appearance.face,
                (state.base.unwrap().face + 3) % 17
            );
            Some("02-face")
        }
        10 => {
            assert_eq!(
                session.founding.cards[1].appearance.nose,
                (state.base.unwrap().nose + 15) % 17
            );
            state.edited = Some(session.founding.cards[1].appearance);
            Some("03-nose")
        }
        12 => {
            state.sheet = session.founding.cards[1].theus;
            Some("04-profile")
        }
        16 => {
            let sheet = session.founding.cards[1].theus.unwrap();
            assert_eq!(sheet.attributes[0], state.sheet.unwrap().attributes[0] + 1);
            state.sheet = Some(sheet);
            Some("04-theus")
        }
        18 => {
            assert!(!session.founding.open && !session.menu);
            assert_eq!(
                session.campaign.teams[0].fighters[1].appearance,
                state.edited.unwrap()
            );
            let encoded = serde_json::to_string_pretty(&session.campaign).unwrap();
            let restored: Campaign = serde_json::from_str(&encoded).unwrap();
            restored.validate().unwrap();
            assert_eq!(restored.teams[0].fighters[1].theus, state.sheet);
            assert_eq!(
                restored.teams[0].fighters[1].appearance,
                state.edited.unwrap()
            );
            std::fs::write("outputs/creator-review/campaign.json", encoded).unwrap();
            std::fs::write("outputs/creator-review/audit.json",serde_json::to_string_pretty(&serde_json::json!({
                "scripted_ui":true,"human_playtest":false,"autosave":session.autosave,
                "before":state.base,"accepted":state.edited,"class":session.battle.units[1].class,
                "head_scale_relative_to_previous":super::visuals::HEAD_PROPORTION,"save_round_trip":true,
                "theus_edited_and_persisted":state.sheet,
            })).unwrap()).unwrap();
            Some("05-entered")
        }
        22 => {
            assert_eq!(
                Some(&session.campaign.teams[0].name),
                state.saved_name.as_ref()
            );
            assert_eq!(session.campaign.teams[0].gold, state.saved_gold);
            assert!(!session.founding.open && !session.running);
            std::fs::write(
                "outputs/creator-review/load-audit.json",
                serde_json::to_string_pretty(&serde_json::json!({
                    "new_game_button":true,"load_game_button":true,"restored_name":state.saved_name,
                    "restored_gold":state.saved_gold,"scripted_ui":true,
                }))
                .unwrap(),
            )
            .unwrap();
            Some("06-loaded")
        }
        19 if !session.save_path.exists() => {
            exit.write(AppExit::Success);
            None
        }
        23 => {
            exit.write(AppExit::Success);
            None
        }
        _ => None,
    };
    if let Some(name) = snapshot {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(format!(
                "outputs/creator-review/{name}.png"
            )));
    }
    state.step += 1;
    state.next = state.seconds + if snapshot.is_some() { 0.8 } else { 0.20 };
}
fn scroll(
    session: Res<Session>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut panels: Query<&mut ScrollPosition, With<Scroll>>,
) {
    for event in wheel.read() {
        if session.founding.open {
            let delta = match event.unit {
                bevy::input::mouse::MouseScrollUnit::Line => event.y * 30.,
                bevy::input::mouse::MouseScrollUnit::Pixel => event.y,
            };
            for mut p in &mut panels {
                p.0.y = (p.0.y - delta).max(0.);
            }
        }
    }
}
fn type_name(mut input: MessageReader<KeyboardInput>, mut session: ResMut<Session>) {
    for event in input.read() {
        if !event.state.is_pressed() {
            continue;
        }
        if session.renaming.is_some() {
            match &event.logical_key {
                Key::Enter => apply(Control::FinishRename, &mut session),
                Key::Escape => apply(Control::CancelRename, &mut session),
                _ => {
                    let (_, text) = session.renaming.as_mut().unwrap();
                    if event.logical_key == Key::Backspace {
                        text.pop();
                    } else if let Some(value) = &event.text {
                        for c in value.chars().filter(|c| !c.is_control()) {
                            if text.chars().count() < 40 {
                                text.push(c);
                            }
                        }
                    }
                }
            };
            continue;
        }
        if !session.founding.open {
            continue;
        }
        if event.logical_key == Key::Escape {
            apply(Control::CreatorMenu, &mut session);
            continue;
        }
        if session.founding.stage == 0 {
            if event.logical_key == Key::Enter {
                apply(Control::FoundingStep, &mut session);
            }
            continue;
        }
        match &event.logical_key {
            Key::Enter => apply(
                if session.founding.stage >= 2 {
                    Control::ConfirmFounders
                } else {
                    Control::FoundingStep
                },
                &mut session,
            ),
            _ => {
                let text = session.founding.text();
                if event.logical_key == Key::Backspace {
                    text.pop();
                } else if let Some(value) = &event.text {
                    for c in value.chars().filter(|c| !c.is_control()) {
                        if text.chars().count() < 40 {
                            text.push(c);
                        }
                    }
                }
                session.founding.changed();
            }
        }
    }
}
fn play_opening(
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut clock: Local<f32>,
    mut opening: Query<&mut Node, With<Opening>>,
    mut titles: Query<&mut TextColor, With<OpeningTitle>>,
    mut lines: Query<&mut TextColor, (With<OpeningLine>, Without<OpeningTitle>)>,
) {
    let show = session.founding.open && session.founding.stage == 0;
    for mut node in &mut opening {
        node.display = if show { Display::Flex } else { Display::None };
    }
    if !show {
        *clock = 0.;
        return;
    }
    *clock += time.delta_secs();
    let title = (*clock / 0.55).clamp(0., 1.);
    let line = ((*clock - 0.4) / 0.5).clamp(0., 1.);
    for mut color in &mut titles {
        color.0 = Color::srgba(1., 0.78, 0.28, title);
    }
    for mut color in &mut lines {
        color.0 = Color::srgba(0.78, 0.86, 0.9, line);
    }
    if *clock > 1.8 {
        session.founding.stage = 1;
    }
}
fn present_founders(time: Res<Time>, mut session: ResMut<Session>, mut wait: Local<f32>) {
    if !session.founding.open || session.founding.stage != 1 || session.founding.introduced {
        *wait = 0.;
        return;
    }
    if session.founding.revealed.iter().all(|shown| *shown) {
        session.founding.introduced = true;
        *wait = 0.;
        return;
    }
    *wait += time.delta_secs();
    if *wait < 0.42 {
        return;
    }
    *wait = 0.;
    if let Some(slot) = session.founding.revealed.iter().position(|shown| !*shown) {
        let _ = session.founding.reveal(slot);
        if session.founding.revealed.iter().all(|shown| *shown) {
            session.founding.introduced = true;
        }
    }
}
fn refresh(
    session: Res<Session>,
    time: Res<Time>,
    mut gates: Query<
        (
            &mut Node,
            Option<&LookRow>,
            Option<&ClassRow>,
            Option<&Advance>,
            Option<&Accept>,
            Option<&Panel>,
        ),
        (
            Or<(
                With<LookRow>,
                With<ClassRow>,
                With<Advance>,
                With<Accept>,
                With<Panel>,
            )>,
            Without<RenamePanel>,
        ),
    >,
    mut rename: Query<&mut Node, (With<RenamePanel>, Without<Panel>)>,
    mut rename_text: Query<&mut Text, With<RenameText>>,
    mut labels: Query<(&Label, &mut Text), Without<RenameText>>,
    mut buttons: Query<(&Control, &mut BackgroundColor), Without<Card>>,
    mut cards: Query<(&Card, &mut BackgroundColor)>,
    mut animation: Local<([u32; 3], [f32; 3])>,
) {
    let open = session.founding.open;
    let stage = session.founding.stage;
    for (mut node, look, class, advance, accept, panel) in &mut gates {
        let show = if panel.is_some() || look.is_some() {
            open && stage >= 1
        } else if class.is_some() || accept.is_some() {
            open && stage >= 2
        } else if advance.is_some() {
            open && stage == 1
        } else {
            false
        };
        node.display = if show { Display::Flex } else { Display::None };
    }
    for mut p in &mut rename {
        p.display = if session.renaming.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Some((_, name)) = &session.renaming {
        for mut t in &mut rename_text {
            t.0 = format!("{name}_");
        }
    }
    if !open {
        return;
    }
    for (kind, mut text) in &mut labels {
        let content = match kind {
            Label::Name => format!(
                "PHALANX: {}{}",
                session.founding.name,
                if session.founding.focus == 0 { "_" } else { "" }
            ),
            Label::Seed => format!(
                "SEED WORD: {}{}",
                session.founding.word,
                if session.founding.focus == 1 { "_" } else { "" }
            ),
            Label::Fighter(i) => format!(
                "{} / {}{}",
                session.campaign.teams[0].fighters[*i].title.name(),
                if session.founding.revealed[*i] {
                    &session.founding.cards[*i].name
                } else {
                    "UNREVEALED"
                },
                if session.founding.focus == i + 2 {
                    "_"
                } else {
                    ""
                }
            ),
            Label::Slot(i) => {
                let f = &session.founding.cards[*i];
                if !session.founding.revealed[*i] {
                    "Waiting to appear.".into()
                } else if stage < 2 {
                    let body = if f.appearance.female.unwrap_or(false) {
                        "Woman"
                    } else {
                        "Man"
                    };
                    format!(
                        "{body}\nShape the face. The class comes next.\nCourage {:.0} / Teamwork {:.0} / Caution {:.0}",
                        f.mind.courage * 100.,
                        f.mind.teamwork * 100.,
                        f.mind.caution * 100.
                    )
                } else {
                    format!(
                        "{}\n{}\nCourage {:.0} / Teamwork {:.0} / Caution {:.0}",
                        session.founding.classes[*i].name(),
                        session.founding.classes[*i].description(),
                        f.mind.courage * 100.,
                        f.mind.teamwork * 100.,
                        f.mind.caution * 100.
                    )
                }
            }
            Label::Theus(i) => format!(
                "THEUS | {} attribute points left",
                session.founding.cards[*i]
                    .theus
                    .unwrap_or_default()
                    .unspent()
            ),
            Label::Attribute(i, a) => format!(
                "{} {}",
                phalanx::theus::ATTRIBUTE_NAMES[*a],
                session.founding.cards[*i]
                    .theus
                    .unwrap_or_default()
                    .attributes[*a]
            ),
            Label::Notice => session.save_notice.clone(),
        };
        if text.0 != content {
            text.0 = content;
        }
    }
    for (c, mut color) in &mut buttons {
        if let Control::StarterClass(i, class) = c
            && session.founding.classes[*i] == *class
        {
            color.0 = Color::srgb(0.3, 0.23, 0.1);
        }
    }
    for (card, mut color) in &mut cards {
        let i = card.0;
        if animation.0[i] != session.founding.stamps[i] {
            animation.0[i] = session.founding.stamps[i];
            animation.1[i] = 0.;
        }
        animation.1[i] += time.delta_secs();
        let glow = if session.founding.revealed[i] {
            (-animation.1[i] * 2.).exp()
        } else {
            0.
        };
        color.0 = Color::srgb(0.055 + glow * 0.25, 0.06 + glow * 0.10, 0.10 + glow * 0.35);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn button_app() -> App {
        // Start in the Lab so Session::new cannot autosave the player's campaign.
        let mut session = Session::new(Battle::new(42, false));
        session.autosave = false;
        session.campaign = Campaign::new();
        session.restart_founders(41, Arena::OpenField, Controller::Utility);
        session.founding = Draft::new(&session.campaign, true);
        let mut app = App::new();
        app.insert_resource(session)
            .insert_resource(Time::<()>::default())
            .add_systems(Update, (controls, simulate).chain());
        app
    }

    fn click(app: &mut App, control: Control) {
        let button = app
            .world_mut()
            .spawn((
                Button,
                control,
                Interaction::Pressed,
                BackgroundColor::default(),
                PressLatch::default(),
            ))
            .id();
        app.update();
        app.world_mut().despawn(button);
    }

    #[test]
    fn escape_returns_to_the_menu_without_falling_through_to_game_controls() {
        let mut app = button_app();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Escape);
        app.insert_resource(keys)
            .add_message::<KeyboardInput>()
            .add_systems(
                Update,
                (super::super::keyboard, type_name).chain().after(controls),
            );
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::Escape,
                logical_key: Key::Escape,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        let session = app.world().resource::<Session>();
        assert!(session.menu && !session.founding.open && !session.running);
    }

    #[test]
    fn creator_buttons_advance_from_opening_through_appearance_to_combat() {
        let mut app = button_app();
        assert!(app.world().resource::<Session>().menu);
        click(&mut app, Control::FoundingStep);
        assert_eq!(app.world().resource::<Session>().founding.stage, 1);

        let look = app.world().resource::<Session>().founding.cards[0].appearance;
        for choice in 0..phalanx::appearance::FEATURES.len() {
            click(&mut app, Control::FounderLook(0, choice));
        }
        let changed = app.world().resource::<Session>().founding.cards[0].appearance;
        assert_ne!(changed.female, look.female);
        assert_eq!(changed.hair, (look.hair + 1) % 11);
        assert_eq!(changed.face, (look.face + 1) % 17);
        assert_eq!(changed.eyes, (look.eyes + 1) % 16);
        assert_eq!(changed.nose, (look.nose + 1) % 17);
        assert_eq!(changed.skin, (look.skin + 1) % 8);
        assert_eq!(changed.hair_color, (look.hair_color + 1) % 7);
        assert_eq!(changed.cheeks, (look.cheeks + 1) % 5);
        assert_eq!(changed.mouth, (look.mouth + 1) % 5);
        assert_eq!(changed.brows, (look.brows + 1) % 5);
        click(&mut app, Control::FounderLookBack(0, 2));
        assert_eq!(
            app.world().resource::<Session>().founding.cards[0]
                .appearance
                .face,
            look.face
        );
        click(&mut app, Control::FounderLook(0, 2));
        click(&mut app, Control::FounderView(0));
        click(&mut app, Control::FounderTurn(0));
        assert!(!app.world().resource::<Session>().founding.closeup[0]);
        assert_eq!(app.world().resource::<Session>().founding.angle[0], 1);

        assert_eq!(
            app.world().resource::<Session>().founding.cards[1]
                .appearance
                .female,
            Some(true)
        );
        click(&mut app, Control::FounderLook(1, 0));
        assert_eq!(
            app.world().resource::<Session>().founding.cards[1]
                .appearance
                .female,
            Some(false)
        );

        click(&mut app, Control::FocusFounderField(1));
        assert_eq!(app.world().resource::<Session>().founding.focus, 1);
        click(&mut app, Control::RevealFounder(1));
        assert_eq!(app.world().resource::<Session>().founding.rolls[1], 1);
        assert_eq!(
            app.world().resource::<Session>().founding.cards[1]
                .appearance
                .female,
            Some(true)
        );
        click(&mut app, Control::FoundingStep);
        assert_eq!(app.world().resource::<Session>().founding.stage, 2);
        let original = app.world().resource::<Session>().founding.cards[0]
            .theus
            .unwrap();
        click(&mut app, Control::FounderAttribute(0, 0, -1));
        click(&mut app, Control::FounderAttribute(0, 1, 1));
        let sheet = app.world().resource::<Session>().founding.cards[0]
            .theus
            .unwrap();
        assert_eq!(sheet.attributes[0], original.attributes[0] - 1);
        assert_eq!(sheet.attributes[1], original.attributes[1] + 1);
        assert_eq!(sheet.unspent(), original.unspent());
        click(&mut app, Control::StarterClass(0, FighterClass::Guardian));
        click(&mut app, Control::StarterClass(1, FighterClass::Healer));
        app.world_mut().resource_mut::<Session>().founding.name = "Button Company".into();
        click(&mut app, Control::ConfirmFounders);
        let session = app.world().resource::<Session>();
        assert!(!session.founding.open && !session.menu && !session.running);
        assert_eq!(session.campaign.teams[0].name, "Button Company");
        assert_eq!(session.campaign.teams[0].gold, 100);
        assert_eq!(session.battle.units[0].class, Some(FighterClass::Guardian));
        assert_eq!(session.battle.units[1].class, Some(FighterClass::Healer));
        assert_eq!(session.campaign.teams[0].fighters[0].appearance, changed);
        assert_eq!(session.campaign.teams[0].fighters[0].theus, Some(sheet));
        assert_eq!(session.battle.units[0].theus.as_ref().unwrap().sheet, sheet);
        let loaded: Campaign =
            serde_json::from_str(&serde_json::to_string(&session.campaign).unwrap()).unwrap();
        loaded.validate().unwrap();
        assert_eq!(loaded.teams[0].fighters[0].theus, Some(sheet));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));
        click(&mut app, Control::TogglePlay);
        assert!(app.world().resource::<Session>().battle.tick > 0);
        click(&mut app, Control::TogglePlay);
        assert!(!app.world().resource::<Session>().running);
    }

    #[test]
    fn creator_buttons_keep_modal_guards_and_fire_once_per_press() {
        let mut app = button_app();
        click(&mut app, Control::Continue);
        click(&mut app, Control::Run);
        assert!(app.world().resource::<Session>().menu);
        assert!(!app.world().resource::<Session>().running);
        click(&mut app, Control::FoundingStep);
        let hair = app.world().resource::<Session>().founding.cards[0]
            .appearance
            .hair;
        let button = app
            .world_mut()
            .spawn((
                Button,
                Control::FounderLook(0, 1),
                Interaction::Pressed,
                BackgroundColor::default(),
                PressLatch::default(),
            ))
            .id();
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<Session>().founding.cards[0]
                .appearance
                .hair,
            (hair + 1) % 11
        );
        *app.world_mut().get_mut::<Interaction>(button).unwrap() = Interaction::None;
        app.update();
        *app.world_mut().get_mut::<Interaction>(button).unwrap() = Interaction::Pressed;
        app.update();
        assert_eq!(
            app.world().resource::<Session>().founding.cards[0]
                .appearance
                .hair,
            (hair + 2) % 11
        );
    }

    #[test]
    fn typing_selection_and_confirmation_start_the_chosen_team_once() {
        let mut s = Session::new(Battle::new(42, false));
        s.autosave = false;
        s.campaign = Campaign::new();
        s.restart_founders(41, Arena::OpenField, Controller::Utility);
        s.founding = Draft::new(&s.campaign, true);
        for i in 0..3 {
            s.founding.reveal(i).unwrap();
        }
        s.founding.stage = 1;
        apply(Control::Run, &mut s);
        assert!(!s.running);
        apply(Control::ConfirmFounders, &mut s);
        assert!(s.founding.open);
        let mut app = App::new();
        app.add_message::<KeyboardInput>()
            .insert_resource(s)
            .add_systems(Update, type_name);
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::KeyP,
                logical_key: Key::Character("P".into()),
                state: bevy::input::ButtonState::Pressed,
                text: Some("Penelope's Phalanx".into()),
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        let mut s = app.world_mut().resource_mut::<Session>();
        apply(Control::StarterClass(0, FighterClass::Guardian), &mut s);
        apply(Control::StarterClass(1, FighterClass::Healer), &mut s);
        apply(Control::ConfirmFounders, &mut s);
        assert!(!s.founding.open);
        assert!(s.campaign.founders_prepared);
        assert_eq!(s.campaign.teams[0].name, "Penelope's Phalanx");
        assert_eq!(s.battle.units[0].class, Some(FighterClass::Guardian));
        assert_eq!(s.battle.units[1].class, Some(FighterClass::Healer));
        assert_eq!(s.campaign.teams[0].gold, 100);
        assert_eq!(s.campaign.teams[0].fighters.len(), 3);
        assert_eq!(
            s.campaign.teams[1].rival_style,
            Some(phalanx::rivals::RivalStyle::Bloodhound)
        );
        let before = serde_json::to_string(&s.campaign).unwrap();
        apply(Control::ConfirmFounders, &mut s);
        assert_eq!(serde_json::to_string(&s.campaign).unwrap(), before);
    }
    #[test]
    fn faces_come_before_classes_and_a_seed_edit_stops_the_stagger() {
        let mut s = Session::new(Battle::new(42, false));
        s.autosave = false;
        s.campaign = Campaign::new();
        s.restart_founders(41, Arena::OpenField, Controller::Utility);
        s.founding = Draft::new(&s.campaign, true);
        assert_eq!(s.founding.stage, 0);
        assert!(s.founding.revealed.iter().all(|shown| !*shown));
        apply(Control::FoundingStep, &mut s);
        assert_eq!(s.founding.stage, 1);
        assert!(s.founding.open);
        assert!(s.founding.revealed.iter().all(|shown| *shown));
        let hair = s.founding.cards[0].appearance.hair;
        apply(Control::FounderLook(0, 1), &mut s);
        assert_eq!(s.founding.cards[0].appearance.hair, (hair + 1) % 11);
        apply(Control::FoundingStep, &mut s);
        assert_eq!(s.founding.stage, 2);
        assert!(s.founding.open);

        s.founding.stage = 1;
        s.founding.introduced = false;
        s.founding.revealed = [false; 3];
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(s)
            .add_systems(Update, present_founders);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));
        app.update();
        assert_eq!(
            app.world().resource::<Session>().founding.revealed,
            [true, false, false]
        );
        {
            let founding = &mut app.world_mut().resource_mut::<Session>().founding;
            founding.focus = 1;
            founding.word.push('Z');
            founding.changed();
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));
        app.update();
        let founding = &app.world().resource::<Session>().founding;
        assert!(founding.introduced);
        assert!(founding.revealed.iter().all(|shown| !*shown));
    }
}
