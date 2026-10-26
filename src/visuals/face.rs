//! Socketed head presentation: additive sculpt shapes and independent eye pivots.
//! Reads combat state and barks; never changes simulation or consumes its RNG.
//!
//! The folder sheets are the target: a round, smug, adorable face that turns
//! into one extreme Gotlib expression the moment the fight gets ugly.
use super::*;
use bevy::mesh::morph::MorphWeights;

// Must match expressive-manifest.json and the Blender shape-key order.
const RAGE: usize = 0;
const PAIN: usize = 1;
const FEAR: usize = 2;
const GRIN: usize = 3;
const SHOUT: usize = 4;
const BLINK: usize = 5;
/// Uniform scale on the head bone. Hair and eyes are parented there, so they
/// stay seated. Enough to read chibi at battle distance; the neck still holds.
const HEAD_CARICATURE: f32 = 1.22;
const MOUTH_CAP: f32 = 1.35;

#[derive(Component)]
struct FaceState {
    id: usize,
    health: f32,
    cooldown: f32,
    run: u32,
    tick: u32,
    clock: f32,
    hurt: f32,
    attack: f32,
    laugh: f32,
    seen_kill: u64,
    weights: [f32; 10],
    gaze: Vec2,
    iris: u8,
}
#[derive(Component)]
struct Caricature {
    root: Entity,
    base: Vec3,
}
/// One frame of mood. Presentation only.
struct Mood {
    alive: bool,
    finished: bool,
    attacking: bool,
    attack: f32,
    hurt: f32,
    laugh: f32,
    retreat: bool,
    critical: bool,
    aggression: f32,
    courage: f32,
    speaking: bool,
    blink: f32,
}
#[derive(Component)]
struct Feature {
    root: Entity,
    rest: Transform,
    eye: bool,
}
#[derive(Component)]
struct IrisMaterial {
    root: Entity,
    material: Handle<StandardMaterial>,
    shade: f32,
}

#[derive(Component)]
struct SkinMaterial {
    root: Entity,
    material: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
pub(super) struct Audit {
    pub seen: [bool; 6],
    pub eye_rotation: bool,
    pub meshes: usize,
}

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Pose;

pub(super) fn install(app: &mut App) {
    app.init_resource::<Audit>()
        .add_systems(Update, drive.after(barks::bubbles))
        .add_systems(
            PostUpdate,
            apply
                .in_set(Pose)
                .after(bevy::app::AnimationSystems)
                .before(bevy::transform::TransformSystems::Propagate),
        );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    roots: Query<&Model>,
    children: Query<&Children>,
    nodes: Query<(&Name, &Transform, Option<&MorphWeights>)>,
    primitives: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    session: Res<Session>,
) {
    let Ok(part) = roots.get(event.entity) else {
        return;
    };
    // Models are pooled for future roster slots too. Bind every loaded head;
    // an unused slot starts animating as soon as the campaign recruits it.
    let unit = session.visual_unit(part.id);
    commands.entity(event.entity).insert(FaceState {
        id: part.id,
        health: unit.map_or(0.0, |unit| unit.health),
        cooldown: unit.map_or(0.0, |unit| unit.cooldown),
        run: session.run_id,
        tick: session.battle.tick,
        clock: 0.0,
        hurt: 0.0,
        attack: 0.0,
        laugh: 0.0,
        seen_kill: caught_up(&session.battle.damage_events),
        weights: [0.0; 10],
        gaze: Vec2::ZERO,
        iris: u8::MAX,
    });
    for entity in children.iter_descendants(event.entity) {
        let Ok((name, transform, morph)) = nodes.get(entity) else {
            continue;
        };
        if morph.is_some()
            || name.as_str().starts_with("Eye_")
            || name.as_str().starts_with("Iris_")
            || name.as_str().starts_with("Pupil_")
        {
            commands.entity(entity).insert(Feature {
                root: event.entity,
                rest: *transform,
                eye: name.as_str().starts_with("Eye_"),
            });
        }
        if name.as_str() == "Face_Sculpt"
            || name.as_str() == "Lip_Rim"
            || name.as_str().starts_with("Lid_")
        {
            for primitive in children.iter_descendants(entity) {
                if let Ok(old) = primitives.get(primitive)
                    && let Some(source) = materials.get(&old.0).cloned()
                {
                    let material = materials.add(source);
                    commands.entity(primitive).insert((
                        MeshMaterial3d(material.clone()),
                        SkinMaterial {
                            root: event.entity,
                            material,
                        },
                    ));
                }
            }
        }
        if name.as_str() == "mixamorigHead" {
            commands.entity(entity).insert(Caricature {
                root: event.entity,
                base: transform.scale,
            });
        }
        if name.as_str().starts_with("Iris_") {
            for primitive in children.iter_descendants(entity) {
                if let Ok(material) = primitives.get(primitive)
                    && let Some(source) = materials.get(&material.0).cloned()
                {
                    if source.base_color.to_linear().red < 0.02 {
                        continue;
                    }
                    let shade = (source.base_color.to_linear().red / 0.25)
                        .sqrt()
                        .clamp(0.35, 1.25);
                    let handle = materials.add(source);
                    commands.entity(primitive).insert((
                        MeshMaterial3d(handle.clone()),
                        IrisMaterial {
                            root: event.entity,
                            material: handle,
                            shade,
                        },
                    ));
                }
            }
        }
    }
}

fn drive(
    session: Res<Session>,
    time: Res<Time>,
    voices: Res<barks::Voices>,
    mut states: Query<&mut FaceState>,
) {
    let dt = if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else {
        0.0
    };
    for mut state in &mut states {
        let Some(unit) = session.visual_unit(state.id) else {
            continue;
        };
        if state.run != session.run_id {
            state.clock = 0.0;
            state.hurt = 0.0;
            state.attack = 0.0;
            state.laugh = 0.0;
            state.seen_kill = caught_up(&session.battle.damage_events);
            state.health = unit.health;
            state.cooldown = unit.cooldown;
            state.run = session.run_id;
            state.weights = [0.0; 10];
            state.gaze = Vec2::ZERO;
        }
        let dt = if dt > 0.0 {
            dt
        } else {
            session.battle.tick.saturating_sub(state.tick).min(4) as f32 * phalanx::DT
        };
        state.tick = session.battle.tick;
        state.clock += dt;
        if unit.health < state.health {
            state.hurt = 1.0;
        }
        if unit.cooldown > state.cooldown && unit.health > 0.0 {
            state.attack = 1.0;
        }
        state.hurt = (state.hurt - dt * 2.6).max(0.0);
        state.attack = (state.attack - dt * 2.0).max(0.0);
        state.laugh = (state.laugh - dt * 0.65).max(0.0);
        for event in session.battle.damage_events.iter().rev() {
            // u64::MAX means nothing has been consumed yet, so sequence 0 still counts.
            if state.seen_kill != u64::MAX && event.sequence <= state.seen_kill {
                break;
            }
            let killed = event.attacker == state.id
                && session
                    .battle
                    .units
                    .get(event.defender)
                    .is_some_and(|defender| defender.health <= 0.0);
            if killed {
                state.laugh = 1.0;
            }
        }
        if let Some(event) = session.battle.damage_events.last() {
            state.seen_kill = state.seen_kill.max(event.sequence);
        }
        state.health = unit.health;
        state.cooldown = unit.cooldown;
        // Deliberate, staggered blink, with full pause preservation.
        let phase = (state.clock + state.id as f32 * 0.79) % 4.1;
        let blink = (1.0 - (phase - 3.88).abs() / 0.095).clamp(0.0, 1.0);
        let speaking = barks::speaking(&voices, state.id, session.battle.tick);
        let target = caricature_pose(Mood {
            alive: unit.health > 0.0,
            finished: session.battle.finished,
            attacking: unit.action == 1 || state.attack > 0.12,
            attack: state.attack,
            hurt: state.hurt,
            laugh: state.laugh,
            retreat: unit.action == 5,
            critical: !session.battle.finished && unit.health_fraction() <= 0.30,
            aggression: unit.mind.aggression,
            courage: unit.mind.courage,
            speaking,
            blink,
        });
        let alpha = 1.0 - (-dt * 22.0).exp();
        for (weight, target) in state.weights.iter_mut().zip(target) {
            *weight += (target - *weight) * alpha;
        }
        let mut gaze = unit
            .contacts
            .iter()
            .find(|contact| Some(contact.enemy) == unit.enemy)
            .map(|enemy| {
                let d = Vec2::from_array(enemy.position) - Vec2::from_array(unit.position);
                let facing = Vec2::from_array(unit.target) - Vec2::from_array(unit.position);
                let yaw = if facing.length_squared() > 0.001 {
                    facing.perp_dot(d).atan2(facing.dot(d))
                } else {
                    0.0
                };
                Vec2::new(yaw.clamp(-0.30, 0.30), 0.0)
            })
            .unwrap_or_else(|| Vec2::new((state.clock * 0.70 + state.id as f32).sin() * 0.10, 0.0));
        if unit.health > 0. && unit.health_fraction() <= 0.30 && !session.battle.finished {
            gaze.x += (state.clock * 11. + state.id as f32).sin() * 0.19;
            gaze.y += (state.clock * 7.).sin() * 0.05;
        }
        state.gaze = state.gaze.lerp(gaze, alpha);
        let appearance = look(&session, state.id);
        state.iris = appearance.eyes;
        state.weights[6..10].copy_from_slice(&round_face(appearance.nose, appearance.face));
        if let Some(pose) = witness_pose() {
            state.weights[..6].copy_from_slice(&pose);
            state.gaze = Vec2::new(0.10, 0.0);
        }
    }
}

// Opt-in render witness, also useful for taking consistent art-review shots.
fn witness_pose() -> Option<[f32; 6]> {
    let pose = std::env::var("PHALANX_FACE_POSE").ok()?;
    let mut weights = [0.0; 6];
    if pose != "neutral" {
        let index = match pose.as_str() {
            "rage" => RAGE,
            "pain" => PAIN,
            "fear" => FEAR,
            "grin" => GRIN,
            "shout" => SHOUT,
            "blink" => BLINK,
            _ => return None,
        };
        weights[index] = 1.0;
    }
    Some(weights)
}

fn caught_up(events: &[phalanx::DamageEvent]) -> u64 {
    events
        .last()
        .map(|event| event.sequence)
        .unwrap_or(u64::MAX)
}

/// One readable expression. Cute at rest, one savage shape when something happens.
fn caricature_pose(mood: Mood) -> [f32; 6] {
    let mut target = [0.0; 6];
    if !mood.alive {
        return target;
    }
    target[BLINK] = mood.blink;
    if mood.laugh > 0.2 {
        // Killing blow: the adorable face comes apart laughing.
        target[GRIN] = 1.0;
        target[SHOUT] = 0.48 * mood.laugh;
        target[RAGE] = 0.18;
    } else if mood.finished {
        target[GRIN] = 1.0;
        target[SHOUT] = 0.32;
    } else if mood.attacking {
        target[PAIN] = mood.hurt * 0.28;
        if mood.courage >= 0.62 {
            // Swinging and enjoying it.
            target[GRIN] = 0.92;
            target[RAGE] = 0.48 + mood.attack * 0.20;
            target[SHOUT] = 0.22 + mood.attack * 0.30;
        } else {
            target[RAGE] = 1.0;
            target[SHOUT] = 0.35 + mood.attack * 0.55;
        }
    } else if mood.hurt > 0.35 {
        if mood.aggression >= 0.55 && mood.courage >= 0.40 {
            target[RAGE] = 0.95;
            target[PAIN] = 0.70 * mood.hurt;
        } else if mood.courage < 0.40 {
            target[FEAR] = 1.0;
            target[PAIN] = 0.30 * mood.hurt;
        } else {
            target[PAIN] = 1.0;
            target[FEAR] = 0.22;
        }
    } else if mood.retreat && mood.courage < 0.55 {
        target[FEAR] = if mood.critical { 1.0 } else { 0.88 };
        target[PAIN] = mood.hurt * 0.25;
    } else if mood.critical && mood.aggression >= 0.50 {
        // Last of their blood, and they look delighted about it.
        target[RAGE] = 0.85;
        target[GRIN] = 0.55;
        target[SHOUT] = 0.28;
    } else if mood.critical || (mood.retreat && mood.courage < 0.70) {
        target[FEAR] = if mood.critical { 0.72 } else { 0.28 };
        target[PAIN] = if mood.critical { 0.48 } else { 0.0 };
        target[GRIN] = 0.16;
    } else if mood.retreat {
        // A calm walk backward still wears the little smug face.
        target[GRIN] = 0.36;
        target[RAGE] = mood.aggression * 0.16;
        target[FEAR] = 0.12;
    } else {
        // Resting sheet: round cheeks, lopsided confidence, a mean little brow.
        target[GRIN] = 0.40 + mood.courage * 0.16;
        target[RAGE] = mood.aggression * 0.24;
    }
    if mood.speaking && target[SHOUT] < 0.55 {
        target[SHOUT] = 0.55;
        if target[GRIN] < 0.2 {
            target[GRIN] = 0.2;
        }
    }
    let mouth_sum = target[..5].iter().sum::<f32>();
    if mouth_sum > MOUTH_CAP {
        for weight in &mut target[..5] {
            *weight *= MOUTH_CAP / mouth_sum;
        }
    }
    target
}

/// Short round nose and soft cheeks, with the saved face still visible inside that.
fn round_face(nose: u8, face: u8) -> [f32; 4] {
    let nose_long = (nose % 4) as f32 / 3.0;
    let nose_wide = (nose / 4) as f32 / 3.0;
    let jaw = (face % 4) as f32 / 3.0;
    let cheek = (face / 4) as f32 / 3.0;
    [
        nose_long * 0.45,
        nose_wide * 0.40 + 0.48,
        jaw * 0.85,
        cheek * 0.35 + 0.62,
    ]
}

fn blushed(color: Color, amount: f32) -> Color {
    let tint = color.to_srgba();
    Color::srgba(
        (tint.red + 0.10 * amount).clamp(0.0, 1.0),
        (tint.green - 0.04 * amount).clamp(0.0, 1.0),
        (tint.blue - 0.03 * amount).clamp(0.0, 1.0),
        tint.alpha,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply(
    states: Query<&FaceState>,
    mut features: Query<(&Feature, &mut Transform, Option<&mut MorphWeights>), Without<Caricature>>,
    mut heads: Query<(&Caricature, &mut Transform), Without<Feature>>,
    irises: Query<&IrisMaterial>,
    skins: Query<&SkinMaterial>,
    session: Res<Session>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut audit: ResMut<Audit>,
) {
    let mut meshes = 0;
    for (feature, mut transform, morph) in &mut features {
        let Ok(state) = states.get(feature.root) else {
            continue;
        };
        if let Some(mut morph) = morph {
            meshes += 1;
            for (seen, weight) in audit.seen.iter_mut().zip(state.weights) {
                *seen |= weight > 0.15;
            }
            for (weight, value) in morph.weights_mut().iter_mut().zip(state.weights) {
                *weight = value;
            }
        }
        if feature.eye {
            audit.eye_rotation |= state.gaze.length() > 0.01;
            // glTF converts Blender -Y-forward into +Z-forward.
            transform.rotation = feature.rest.rotation
                * Quat::from_rotation_z(state.gaze.x)
                * Quat::from_rotation_x(state.gaze.y);
        }
    }
    for (head, mut transform) in &mut heads {
        let Ok(state) = states.get(head.root) else {
            continue;
        };
        // Animation rewrites the bone first. Set scale absolutely so it cannot compound.
        transform.scale = head.base * HEAD_CARICATURE;
        let wobble = if state.weights[RAGE] > 0.55 || state.weights[SHOUT] > 0.55 {
            Quat::from_rotation_z((state.clock * 21.0 + state.id as f32).sin() * 0.055)
        } else if state.weights[GRIN] > 0.30 && state.weights[FEAR] < 0.25 {
            Quat::from_rotation_x((state.clock * 2.3 + state.id as f32).sin() * 0.04)
        } else {
            Quat::IDENTITY
        };
        transform.rotation *= wobble;
    }
    audit.meshes = meshes;
    for skin in &skins {
        if let Ok(state) = states.get(skin.root)
            && let Some(mut material) = materials.get_mut(&skin.material)
        {
            let cute = (state.weights[GRIN]
                * (1.0
                    - state.weights[PAIN]
                        .max(state.weights[FEAR])
                        .max(state.weights[RAGE])))
            .clamp(0.0, 1.0);
            let color = blushed(skin_color(&session, state.id), cute * 0.65);
            if material.base_color != color {
                material.base_color = color;
            }
        }
    }

    for iris in &irises {
        let Ok(state) = states.get(iris.root) else {
            continue;
        };
        let palette = [
            Color::srgb(0.36, 0.18, 0.06),
            Color::srgb(0.16, 0.36, 0.49),
            Color::srgb(0.22, 0.36, 0.13),
            Color::srgb(0.48, 0.28, 0.08),
            Color::srgb(0.32, 0.20, 0.39),
        ];
        if let Some(mut material) = materials.get_mut(&iris.material) {
            let base = palette[state.iris as usize % palette.len()].to_linear();
            let color = Color::linear_rgb(
                base.red * iris.shade,
                base.green * iris.shade,
                base.blue * iris.shade,
            );
            if material.base_color != color {
                material.base_color = color;
            }
        }
    }
}

#[cfg(test)]
mod caricature_rules {
    use super::*;

    fn mood() -> Mood {
        Mood {
            alive: true,
            finished: false,
            attacking: false,
            attack: 0.0,
            hurt: 0.0,
            laugh: 0.0,
            retreat: false,
            critical: false,
            aggression: 0.4,
            courage: 0.6,
            speaking: false,
            blink: 0.0,
        }
    }

    fn dominant(pose: [f32; 6]) -> usize {
        pose[..5]
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .unwrap()
            .0
    }

    #[test]
    fn rest_is_a_smug_little_grin() {
        let pose = caricature_pose(mood());
        assert_eq!(dominant(pose), GRIN);
        assert!(pose[GRIN] > 0.35);
        assert!(pose[RAGE] < 0.30);
        assert!(pose[..5].iter().sum::<f32>() <= MOUTH_CAP + 1e-4);
    }

    #[test]
    fn a_coward_hit_panics_and_a_brute_gets_mean() {
        let mut scared = mood();
        scared.hurt = 1.0;
        scared.courage = 0.2;
        scared.aggression = 0.3;
        assert_eq!(dominant(caricature_pose(scared)), FEAR);

        let mut mean = mood();
        mean.hurt = 1.0;
        mean.courage = 0.7;
        mean.aggression = 0.9;
        let pose = caricature_pose(mean);
        assert_eq!(dominant(pose), RAGE);
        assert!(pose[PAIN] > 0.3);
    }

    #[test]
    fn a_brave_swing_laughs_and_a_grim_swing_screams() {
        let mut happy = mood();
        happy.attacking = true;
        happy.attack = 1.0;
        happy.courage = 0.8;
        assert_eq!(dominant(caricature_pose(happy)), GRIN);

        let mut grim = mood();
        grim.attacking = true;
        grim.attack = 1.0;
        grim.courage = 0.3;
        let pose = caricature_pose(grim);
        assert_eq!(dominant(pose), RAGE);
        assert!(pose[SHOUT] > 0.3);
    }

    #[test]
    fn a_kill_laughs_even_while_hurt() {
        let mut killing = mood();
        killing.laugh = 1.0;
        killing.hurt = 1.0;
        killing.attacking = true;
        killing.critical = true;
        let pose = caricature_pose(killing);
        assert_eq!(dominant(pose), GRIN);
        assert!(pose[GRIN] > 0.7);
        assert!(pose[..5].iter().sum::<f32>() <= MOUTH_CAP + 1e-4);
    }

    #[test]
    fn the_last_of_a_mean_fighter_stays_savage() {
        let mut brute = mood();
        brute.critical = true;
        brute.aggression = 0.8;
        brute.courage = 0.5;
        let pose = caricature_pose(brute);
        assert!(pose[RAGE] > pose[FEAR]);
        assert!(pose[GRIN] > 0.2);

        let mut coward = mood();
        coward.critical = true;
        coward.aggression = 0.2;
        coward.courage = 0.2;
        let pose = caricature_pose(coward);
        assert!(pose[FEAR] > pose[RAGE]);
    }

    #[test]
    fn cheeks_and_noses_stay_round_and_still_differ() {
        let plain = round_face(0, 0);
        let strong = round_face(15, 15);
        assert!(plain[1] >= 0.45);
        assert!(plain[3] >= 0.60);
        assert!(plain[0] < 0.2);
        assert!(strong[1] > plain[1]);
        assert!(strong[3] > plain[3]);
        assert!(strong[0] > plain[0]);
        assert!(strong[2] > plain[2]);
    }

    #[test]
    fn a_dead_face_does_not_keep_performing() {
        let mut dead = mood();
        dead.alive = false;
        dead.laugh = 1.0;
        dead.attacking = true;
        assert_eq!(caricature_pose(dead), [0.0; 6]);
    }
}
