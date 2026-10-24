//! One-time starter selection. Existing saves keep their squad and economy.
use super::*;
use bevy::input::keyboard::{Key, KeyboardInput};

pub struct Draft {
    pub open: bool,
    pub name: String,
    pub classes: [FighterClass; 3],
}
impl Draft {
    pub fn new(c: &Campaign, open: bool) -> Self {
        Self {
            open,
            name: String::new(),
            classes: std::array::from_fn(|i| c.teams[0].fighters[i].class),
        }
    }
}
pub struct Founding;
#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Scroll;
#[derive(Component)]
enum Label {
    Name,
    Slot(usize),
    Notice,
}
impl Plugin for Founding {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_founding.after(setup))
            .add_systems(Update, (type_name, refresh, scroll).chain().after(controls));
    }
}
fn label(p: &mut ChildSpawnerCommands, kind: Label, size: f32) {
    p.spawn((kind, Text::new(""), font(size)));
}
fn setup_founding(mut commands: Commands) {
    commands.spawn((Panel, GlobalZIndex(150), Interaction::default(),
        Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), align_items: AlignItems::Center, justify_content: JustifyContent::Center, display: Display::None, ..default() },
        BackgroundColor(Color::srgb(0.018,0.026,0.035))))
        .with_children(|root| {
            root.spawn((Scroll, ScrollPosition::default(), Node { width: px(1200), max_width: vw(95), max_height: vh(96), overflow: Overflow::scroll_y(), padding: UiRect::all(px(22)), flex_direction: FlexDirection::Column, row_gap: px(12), ..default() }))
                .with_children(|p| {
                    p.spawn((Text::new("YOUR PHALANX STARTS HERE"),font(32.),TextColor(Color::srgb(1.,0.72,0.08))));
                    p.spawn((Text::new("Choose your three founders. Crowns are titles; every title can train any class."),font(16.)));
                    label(p, Label::Name, 24.);
                    p.spawn((Text::new("Type your phalanx name. Backspace edits; Enter confirms when ready."),font(14.)));
                    button(p,"Clear name",Control::ClearPhalanxName);
                    p.spawn(Node { column_gap: px(18), ..default() }).with_children(|row| {
                        for slot in 0..3 {
                            row.spawn(Node { flex_basis: percent(33.), min_width: px(0), flex_direction: FlexDirection::Column, row_gap: px(5), ..default() }).with_children(|card| {
                                label(card,Label::Slot(slot),17.);
                                for class in FighterClass::ALL {
                                    button(card,class.name(),Control::StarterClass(slot,class));
                                }
                            });
                        }
                    });
                    p.spawn((Text::new("Three starters are yours. Initial class choices are free. Later retraining costs 10 gold; slot four costs 1,000 gold."),font(14.)));
                    label(p,Label::Notice,14.);
                    button(p,"Found my Phalanx / enter the first trial",Control::ConfirmFounders);
                    button(p,"Quit",Control::Quit);
                });
        });
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
            for mut position in &mut panels {
                position.0.y = (position.0.y - delta).max(0.);
            }
        }
    }
}
fn type_name(mut input: MessageReader<KeyboardInput>, mut session: ResMut<Session>) {
    for event in input.read() {
        if !session.founding.open || !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Backspace => {
                session.founding.name.pop();
            }
            Key::Enter => apply(Control::ConfirmFounders, &mut session),
            _ => {
                if let Some(text) = &event.text {
                    for c in text.chars().filter(|c| !c.is_control()) {
                        if session.founding.name.chars().count() < 40 {
                            session.founding.name.push(c);
                        }
                    }
                }
            }
        }
    }
}
fn refresh(
    session: Res<Session>,
    mut panels: Query<&mut Node, With<Panel>>,
    mut labels: Query<(&Label, &mut Text)>,
    mut buttons: Query<(&Control, &mut BackgroundColor)>,
) {
    for mut node in &mut panels {
        node.display = if session.founding.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !session.founding.open {
        return;
    }
    for (kind, mut text) in &mut labels {
        let content = match kind {
            Label::Name => format!("PHALANX NAME: {}_", session.founding.name),
            Label::Slot(i) => {
                let c = session.founding.classes[*i];
                format!(
                    "{} / {}\n{}",
                    session.campaign.teams[0].fighters[*i].title.name(),
                    c.name(),
                    c.description()
                )
            }
            Label::Notice => session.save_notice.clone(),
        };
        if text.0 != content {
            text.0 = content;
        }
    }
    for (control, mut color) in &mut buttons {
        if let Control::StarterClass(slot, class) = control
            && session.founding.classes[*slot] == *class
        {
            color.0 = Color::srgb(0.32, 0.25, 0.08);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typing_selection_and_confirmation_start_the_chosen_team_once() {
        let mut s = Session::new(Battle::new(42, false));
        s.autosave = false;
        s.campaign = Campaign::new();
        s.restart_founders(41, Arena::OpenField, Controller::Utility);
        s.founding = Draft::new(&s.campaign, true);
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
}
