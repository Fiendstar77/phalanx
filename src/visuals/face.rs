//! Socketed head presentation: additive sculpt shapes and independent eye pivots.
//! Reads combat state and barks; never changes simulation or consumes its RNG.
use super::*;
use bevy::mesh::morph::MorphWeights;

// Must match expressive-manifest.json and the Blender shape-key order.
const RAGE: usize = 0;
const PAIN: usize = 1;
const FEAR: usize = 2;
const GRIN: usize = 3;
const SHOUT: usize = 4;
const BLINK: usize = 5;

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
    weights: [f32; 10],
    gaze: Vec2,
    iris: u8,
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

pub(super) fn install(app: &mut App) {
    app.init_resource::<Audit>()
        .add_systems(Update, drive.after(barks::bubbles))
        .add_systems(
            PostUpdate,
            apply
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
        state.health = unit.health;
        state.cooldown = unit.cooldown;
        let mut target = [0.0; 6];
        if unit.health > 0.0 {
            target[PAIN] = state.hurt;
            target[RAGE] = if unit.action == 1 {
                0.40 + state.attack * 0.50
            } else {
                unit.mind.aggression * 0.20
            };
            let critical = !session.battle.finished && unit.health_fraction() <= 0.30;
            target[FEAR] = if critical {
                0.95
            } else if unit.action == 5 {
                0.8
            } else if unit.health_fraction() < 0.3 {
                0.55
            } else {
                0.0
            };
            target[GRIN] = if session.battle.finished {
                0.95
            } else {
                state.attack * unit.mind.courage * 0.35
            };
            let speaking = barks::speaking(&voices, state.id, session.battle.tick);
            target[SHOUT] = (state.attack * 0.75).max(if speaking {
                0.25 + (state.clock * 19.0 + state.id as f32).sin().abs() * 0.65
            } else {
                0.0
            });
            // Deliberate, staggered blink, with full pause preservation.
            let phase = (state.clock + state.id as f32 * 0.79) % 4.1;
            target[BLINK] = (1.0 - (phase - 3.88).abs() / 0.095).clamp(0.0, 1.0);
        }
        // Limit simultaneous mouth poses so additive keys cannot tear the lips.
        let mouth_sum = target[..5].iter().sum::<f32>();
        if mouth_sum > 1.15 {
            for weight in &mut target[..5] {
                *weight *= 1.15 / mouth_sum;
            }
        }
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
        state.weights[6] = (appearance.nose % 4) as f32 / 3.0;
        state.weights[7] = (appearance.nose / 4) as f32 / 3.0;
        state.weights[8] = (appearance.face % 4) as f32 / 3.0;
        state.weights[9] = (appearance.face / 4) as f32 / 3.0;
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

fn apply(
    states: Query<&FaceState>,
    mut features: Query<(&Feature, &mut Transform, Option<&mut MorphWeights>)>,
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
    audit.meshes = meshes;
    for skin in &skins {
        if let Ok(state) = states.get(skin.root)
            && let Some(mut material) = materials.get_mut(&skin.material)
        {
            let color = skin_color(&session, state.id);
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
