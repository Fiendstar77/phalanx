//! Event-driven text bubbles; no network service, inference call or voice dependency.
use super::*;
#[derive(Clone, Default)]
struct Voice {
    health: f32,
    limbs: u8,
    action: usize,
    cooldown: f32,
    until: u32,
    last: u32,
    text: String,
}
#[derive(Resource, Default)]
pub(super) struct Voices {
    units: Vec<Voice>,
    run: Option<u32>,
}
#[derive(Component)]
pub(super) struct Bubble(usize);
pub(super) fn install(app: &mut App) {
    app.init_resource::<Voices>()
        .add_systems(Startup, setup_bubbles.after(super::setup))
        .add_systems(Update, bubbles.after(super::super::simulate));
}
fn setup_bubbles(mut commands: Commands) {
    for id in 0..20 {
        commands.spawn((
            Bubble(id),
            Text::new(""),
            super::super::font(15.),
            TextColor(Color::srgb(1., 0.94, 0.75)),
            BackgroundColor(Color::srgba(0.05, 0.025, 0.03, 0.9)),
            GlobalZIndex(5),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                padding: UiRect::axes(px(7), px(4)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
        ));
    }
}
pub(super) fn bubbles(
    session: Res<Session>,
    mut voices: ResMut<Voices>,
    camera: Single<(&Camera, &GlobalTransform), super::super::MainCameraFilter>,
    mut bubbles: Query<(&Bubble, &mut Node, &mut Text)>,
) {
    let b = &session.battle;
    if voices.run != Some(session.run_id) || voices.units.len() != b.units.len() {
        voices.units = b
            .units
            .iter()
            .map(|u| Voice {
                health: u.health,
                limbs: u.lost_limbs,
                action: u.action,
                cooldown: u.cooldown,
                ..default()
            })
            .collect();
        voices.run = Some(session.run_id);
    }
    for u in &b.units {
        let v = &mut voices.units[u.id];
        let pain = u.health < v.health;
        let new_wound = u.lost_limbs != v.limbs;
        let mut line = None;
        if u.health <= 0. && v.health > 0. {
            line = Some("I'LL BE BACK!");
        } else if new_wound {
            line = Some(if u.lost_limbs & 3 != v.limbs & 3 {
                "THAT WAS MY ARM!"
            } else {
                "I NEEDED THAT LEG!"
            });
        } else if pain && b.tick.saturating_sub(v.last) > 35 {
            line = Some(if u.mind.courage > 0.75 {
                "IS THAT ALL?"
            } else {
                "I'M HIT!"
            });
        } else if b.tick.saturating_sub(v.last) > 80 {
            if let Some(text) = scar_bark(b, u) {
                line = Some(text);
            } else if u.action == 5 && v.action != 5 {
                line = Some(if u.mind.courage > 0.7 {
                    "TACTICAL RETREAT!"
                } else {
                    "GET ME OUT!"
                });
            } else if u.cooldown > v.cooldown && u.health > 0. {
                line = Some(match u.class {
                    Some(phalanx::FighterClass::Healer) if u.support_target.is_some() => {
                        "HOLD STILL!"
                    }
                    Some(phalanx::FighterClass::Berserker) => "BREAK THEM!",
                    Some(phalanx::FighterClass::Guardian) => "GET BEHIND ME!",
                    Some(phalanx::FighterClass::Hexer) => "FEEL WEAK YET?",
                    Some(phalanx::FighterClass::Paladin) => "STAND WITH ME!",
                    Some(phalanx::FighterClass::BannerBearer) => "WITH ME!",
                    _ if u.role == Role::RockThrower => "INCOMING!",
                    _ if u.mind.aggression > 0.7 => "CHAAARGE!",
                    _ => "HOLD THE LINE!",
                });
            }
        }
        if let Some(line) = line {
            v.text = line.into();
            v.until = b.tick + 28;
            v.last = b.tick;
        }
        v.health = u.health;
        v.limbs = u.lost_limbs;
        v.action = u.action;
        v.cooldown = u.cooldown;
    }
    let mut active: Vec<_> = voices
        .units
        .iter()
        .enumerate()
        .filter(|(_, v)| v.until > b.tick)
        .map(|(i, v)| (i, v.last))
        .collect();
    active.sort_by_key(|(i, t)| {
        (
            std::cmp::Reverse(*i == session.selected),
            std::cmp::Reverse(*t),
            *i,
        )
    });
    active.truncate(4);
    for (bubble, mut node, mut text) in &mut bubbles {
        node.display = Display::None;
        if session.menu || !active.iter().any(|(i, _)| *i == bubble.0) {
            continue;
        }
        let p = session.presented(bubble.0);
        if let Ok(screen) = camera
            .0
            .world_to_viewport(camera.1, Vec3::new(p[0], b.arena.elevation(p) + 2.15, p[1]))
            && screen.x > 15.
            && screen.x
                < camera
                    .0
                    .logical_viewport_size()
                    .map(|v| v.x - 365.)
                    .unwrap_or(1000.)
            && screen.y > 40.
        {
            node.display = Display::Flex;
            node.left = px(screen.x - 55.);
            node.top = px(screen.y - 30.);
            let label = &voices.units[bubble.0].text;
            if &text.0 != label {
                text.0 = label.clone();
            }
        }
    }
}
fn scar_bark(battle: &phalanx::Battle, unit: &phalanx::Unit) -> Option<&'static str> {
    if !battle.founders || unit.health <= 0.0 || unit.psyche.is_blank() {
        return None;
    }
    let allies: Vec<[f32; 2]> = battle
        .units
        .iter()
        .filter(|ally| ally.team == unit.team && ally.id != unit.id && ally.health > 0.0)
        .map(|ally| ally.position)
        .collect();
    let enemies: Vec<[f32; 2]> = battle
        .units
        .iter()
        .filter(|enemy| enemy.team != unit.team && enemy.health > 0.0)
        .map(|enemy| enemy.position)
        .collect();
    let ally_hurt = battle.units.iter().any(|ally| {
        ally.team == unit.team
            && ally.id != unit.id
            && phalanx::distance(unit.position, ally.position) <= 8.0
            && (ally.health <= 0.0 || ally.health_fraction() < 0.6)
    });
    let closing = unit.enemy.is_some_and(|id| {
        unit.contacts.iter().any(|contact| {
            contact.enemy == id
                && phalanx::distance(unit.position, contact.position) > unit.attack_range()
        })
    });
    phalanx::psyche::bark(
        unit.psyche,
        phalanx::psyche::Moment {
            isolated: phalanx::psyche::isolated(unit.position, &allies),
            encircled: phalanx::psyche::encircled(unit.position, &enemies),
            ally_hurt,
            closing,
        },
    )
}

// Facial animation uses the same active bark interval as the text bubble.
pub(super) fn speaking(voices: &Voices, id: usize, tick: u32) -> bool {
    voices.units.get(id).is_some_and(|voice| voice.until > tick)
}
