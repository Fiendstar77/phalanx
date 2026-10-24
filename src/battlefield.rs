//! Pre-battle map briefing. Its controls use the same dispatcher as the main menu.
use super::*;
#[derive(Resource)]
pub struct WallMaterials {
    pub stone: Handle<StandardMaterial>,
    pub timber: Handle<StandardMaterial>,
}
pub struct Battlefield;
#[derive(Component)]
struct Briefing;
#[derive(Component)]
struct BriefingText;
impl Plugin for Battlefield {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_briefing.after(setup))
            .add_systems(Update, show_briefing.after(controls));
    }
}
fn setup_briefing(mut commands: Commands) {
    commands
        .spawn((
            Briefing,
            Node {
                position_type: PositionType::Absolute,
                right: px(16),
                top: px(16),
                bottom: px(96),
                width: px(330),
                padding: UiRect::all(px(16)),
                row_gap: px(10),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.045, 0.06, 0.94)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("BATTLEFIELD FORGE"),
                font(19.),
                TextColor(Color::srgb(0.96, 0.76, 0.35)),
            ));
            p.spawn((BriefingText, Text::new(""), font(14.)));
            button(p, "New battlefield [G]", Control::GenerateArena);
            button(p, "Terrain / tactical overlays [O]", Control::Overlay);
            button(p, "Fight on this battlefield", Control::Run);
            button(
                p,
                "Inspect / prepare a fighter",
                Control::BattlefieldBriefing,
            );
        });
}
fn show_briefing(
    session: Res<Session>,
    mut panels: Query<&mut Node, (With<Briefing>, Without<InspectorPanel>)>,
    mut inspector: Query<&mut Node, (With<InspectorPanel>, Without<Briefing>)>,
    mut text: Query<&mut Text, With<BriefingText>>,
) {
    let field = match session.battle.arena {
        Arena::Pressure(f) if f.generator > 0 => Some(f),
        _ => None,
    };
    let show = field.is_some()
        && session.map_briefing
        && session.exchange.live.is_none()
        && !session.menu
        && !session.running
        && session.battle.tick == 0;
    for mut node in &mut panels {
        node.display = if show { Display::Flex } else { Display::None };
    }
    for mut node in &mut inspector {
        node.display = if show { Display::None } else { Display::Flex };
    }
    if let Some(f) = field {
        let size = f.half();
        let content = format!(
            "{}\nSeed {} | {:.0} x {:.0}\n\n{}\n\n{}\n\nGreen: cover\nDark earth: slow ground\nPale stone: heights\nTimber: breachable",
            f.name(),
            f.seed,
            size[0] * 2.,
            size[1] * 2.,
            phalanx::procedural::briefing(f),
            f.lesson.name()
        );
        for mut t in &mut text {
            if t.0 != content {
                t.0 = content.clone();
            }
        }
    }
}
