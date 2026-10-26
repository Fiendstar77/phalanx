//! Temporary captions and camera framing for real Theus combinations.
use super::*;
#[derive(Component)]
struct Banner;
#[derive(Component)]
struct Caption;
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, update);
}
fn setup(mut commands: Commands) {
    commands
        .spawn((
            Banner,
            Node {
                position_type: PositionType::Absolute,
                top: px(92),
                left: percent(20),
                right: percent(20),
                padding: UiRect::all(px(8)),
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.025, 0.015, 0.82)),
        ))
        .with_children(|p| {
            p.spawn((
                Caption,
                Text::new(""),
                super::super::font(20.),
                TextColor(Color::srgb(1., 0.82, 0.35)),
            ));
        });
}
fn latest(session: &Session) -> Option<&phalanx::theus::Event> {
    session.battle.theus_events.iter().rev().find(|event| {
        event.technique.combination()
            && event.partner.is_some()
            && event.resolution.damage > 0.
            && session.battle.tick.saturating_sub(event.tick) < 22
    })
}
fn update(
    session: Res<Session>,
    mut panel: Query<&mut Node, With<Banner>>,
    mut text: Query<&mut Text, With<Caption>>,
) {
    let combat = latest(&session);
    let art = wall_event(&session)
        .or_else(|| latest_art(&session).filter(|e| combat.is_none_or(|c| e.tick >= c.tick)));
    let caption = if session.menu || session.founding.open {
        None
    } else if let Some(e) = art {
        Some(format!(
            "{}{}",
            e.art.name().to_uppercase(),
            if e.success { "" } else { " · RESISTED" }
        ))
    } else {
        combat.map(|event| {
            let name = |id: usize| {
                let u = &session.battle.units[id];
                session.campaign.teams[u.team].fighters[id % session.battle.team_size()]
                    .name
                    .as_str()
            };
            format!(
                "{}\n{} + {}",
                event.technique.name().to_uppercase(),
                name(event.attacker),
                name(event.partner.unwrap())
            )
        })
    };
    for mut node in &mut panel {
        node.display = if caption.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Some(caption) = caption {
        for mut text in &mut text {
            if text.0 != caption {
                text.0 = caption.clone();
            }
        }
    }
}
fn latest_art(session: &Session) -> Option<&phalanx::ninjutsu::Event> {
    session
        .battle
        .ninjutsu
        .events
        .iter()
        .rev()
        .find(|e| session.battle.tick.saturating_sub(e.tick) < 14)
}
fn wall_event(session: &Session) -> Option<&phalanx::ninjutsu::Event> {
    session.battle.ninjutsu.events.iter().rev().find(|e| {
        e.art == phalanx::ninjutsu::Art::WallVault
            && session.battle.units[e.caster]
                .ninja
                .as_ref()
                .and_then(|n| n.leap.as_ref())
                .is_some_and(|l| l.start == e.tick)
    })
}
pub(crate) fn focus(session: &Session) -> Option<(Vec3, f32)> {
    if let Some(e) = wall_event(session) {
        let u = &session.battle.units[e.caster];
        let p = session.presented(e.caster);
        let height =
            phalanx::ninjutsu::visual_height(u, session.battle.arena, session.battle.tick as f32);
        return Some((
            Vec3::new(p[0], session.battle.arena.elevation(p) + height + 1., p[1]),
            9.5,
        ));
    }
    if let Some(e) = latest_art(session).filter(|e| {
        e.art == phalanx::ninjutsu::Art::GraveBind
            && e.success
            && session.battle.tick.saturating_sub(e.tick) < 10
    }) {
        return Some((
            (Vec3::from_array(e.from) + Vec3::from_array(e.to)) * 0.5 + Vec3::Y * 0.15,
            7.6,
        ));
    }
    let event = latest(session)?;
    let a = session.presented(event.attacker);
    let b = session.presented(event.defender);
    let height = [event.attacker, event.defender]
        .into_iter()
        .filter_map(|id| session.battle.units[id].theus.as_ref())
        .map(|a| a.height(session.battle.tick))
        .fold(0., f32::max);
    let point = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
    Some((
        Vec3::new(
            point[0],
            session.battle.arena.elevation(point) + 1.25 + height * 0.65,
            point[1],
        ),
        8.8 + height * 0.70,
    ))
}
