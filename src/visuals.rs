//! Presentation only: character clips, bounded combat particles, arena dressing.
//! Damage, pathfinding, terrain height and controller decisions remain in Battle.
mod archery;
mod barks;
mod blood;
pub(super) use blood::Fx;
use blood::effects;
mod body;
mod equipment;
mod face;
mod health;
mod standards;
use super::{Pawn, Session, setup};
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::TextureFormat,
    world_serialization::WorldInstanceReady,
};
use phalanx::{CommandTitle, Role, terrain::Arena};

pub struct BattleVisuals;
impl Plugin for BattleVisuals {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanicAudit>();
        barks::install(app);
        archery::install(app);
        blood::install(app);
        face::install(app);
        health::install(app);
        standards::install(app);
        equipment::install(app);
        app.add_systems(Update, portrait_camera.after(super::camera_motion));
        if std::env::args().any(|a| a == "--visual-witness")
            && !std::env::args().any(|a| a == "--player-demo")
        {
            app.init_resource::<Witness>()
                .add_systems(Update, witness.after(animate).after(effects));
        }
        app.add_systems(Startup, load_visuals.after(setup))
            .add_systems(Startup, (setup_looks, body::setup).after(load_visuals))
            .add_systems(
                Update,
                (
                    animate,
                    effects,
                    scenery,
                    update_looks,
                    tint_hair,
                    body::update,
                    body::physics,
                )
                    .after(super::sync_pawns),
            );
    }
}
#[derive(Resource, Default)]
pub(super) struct PanicAudit {
    run: Option<u32>,
    pub(super) clips_seen: [bool; 2],
}
const CLIP_NAMES: [&str; 17] = [
    "Idle",
    "Walk",
    "Run",
    "Attack",
    "Throw",
    "Hit",
    "Death",
    "AttackQuick",
    "AttackHeavy",
    "AttackDesperate",
    "ThrowUnderhand",
    "HitHeavy",
    "LimbRecoil",
    "DeathBackward",
    "DeathDelayed",
    "NearDeathPanic",
    "WoundedStagger",
];
const CLIP_DURATIONS: [f32; 17] = [
    1.6, 0.9, 0.6, 0.65, 1.1, 0.38, 1.1, 0.42, 0.95, 0.60, 0.85, 0.70, 0.85, 1.35, 1.90, 1.4, 1.1,
];
fn loop_clip(clip: usize) -> bool {
    matches!(clip, 0..=2 | 15 | 16)
}
fn locomotion_clip(unit: &phalanx::Unit, moved: f32, finished: bool) -> usize {
    if !finished && unit.health > 0. && unit.health_fraction() <= 0.30 {
        if moved > 0.005 { 16 } else { 15 }
    } else if moved > 0.13 {
        2
    } else if moved > 0.005 {
        1
    } else {
        0
    }
}
fn death_clip(clip: usize) -> bool {
    matches!(clip, 6 | 13 | 14)
}
fn attack_clip(unit: &phalanx::Unit, ordinal: u32) -> usize {
    use phalanx::FighterClass as Class;
    if unit.projectile_weapon() == Some(phalanx::weapons::ProjectileWeapon::Bow) && unit.is_ranged()
    {
        return 0;
    }
    if unit.is_ranged() || unit.windup.is_some() {
        if unit.equipment.projectile_weapon().is_some() {
            return 4;
        }
        return if unit.mind.patience > 0.65 || ordinal.is_multiple_of(2) {
            4
        } else {
            10
        };
    }
    if (unit.health_fraction() < 0.30 && unit.mind.discipline < 0.70)
        || (unit.mind.courage < 0.35 && unit.mind.aggression > 0.6)
    {
        return 9;
    }
    if unit.class == Some(Class::Scout) {
        return 7;
    }
    if matches!(
        unit.class,
        Some(Class::Guardian | Class::Berserker | Class::Paladin)
    ) && !ordinal.is_multiple_of(3)
    {
        return 8;
    }
    if unit.mind.discipline > 0.70 {
        return if ordinal.is_multiple_of(2) { 3 } else { 7 };
    }
    match (ordinal as usize + unit.id) % 3 {
        0 => 3,
        1 => 7,
        _ => 8,
    }
}
#[derive(Resource)]
struct CharacterAssets {
    graphs: [Handle<AnimationGraph>; 2],
    clips: [Vec<AnimationNodeIndex>; 2],
}
#[derive(Component)]
struct Model {
    id: usize,
    female: bool,
}
#[derive(Component, Default)]
struct Actor {
    id: usize,
    female: bool,
    clip: usize,
    clock: f32,
    lock: f32,
    health: f32,
    cooldown: f32,
    winding: bool,
    shots: u32,
    position: Vec2,
    tick: u32,
    seed: u64,
    replays: u32,
    attacks: u32,
    lost_limbs: u8,
}
pub(super) fn female(session: &Session, id: usize) -> bool {
    if let Some(choice) = look(session, id).female {
        return choice;
    }
    session
        .visual_unit(id)
        .is_some_and(|u| match u.command_title {
            Some(CommandTitle::Queen) => true,
            Some(_) => false,
            None => id % 2 == 1,
        })
}
pub(crate) fn load_visuals(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    pawns: Query<(Entity, &Pawn)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let paths = ["characters/male.glb", "characters/female.glb"];
    let loaded = paths.map(|path| {
        let (graph, nodes) = AnimationGraph::from_clips(
            (0..CLIP_NAMES.len())
                .map(|i| server.load(GltfAssetLabel::Animation(i).from_asset(path))),
        );
        (graphs.add(graph), nodes)
    });
    commands.insert_resource(CharacterAssets {
        graphs: [loaded[0].0.clone(), loaded[1].0.clone()],
        clips: [loaded[0].1.clone(), loaded[1].1.clone()],
    });
    for (entity, pawn) in &pawns {
        for (sex, path) in paths.iter().enumerate() {
            commands.entity(entity).with_children(|p| {
                p.spawn((
                    Model {
                        id: pawn.0,
                        female: sex == 1,
                    },
                    WorldAssetRoot(server.load(GltfAssetLabel::Scene(0).from_asset(*path))),
                    Transform::from_xyz(0.0, -0.64, 0.0).with_scale(Vec3::splat(0.7)),
                    Visibility::Hidden,
                ))
                .observe(ready)
                .observe(body::ready)
                .observe(face::ready)
                .observe(equipment::ready)
                .observe(standards::ready)
                .observe(archery::ready);
            });
        }
    }
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.82, 0.88, 1.0),
        brightness: 550.0,
        ..default()
    });
    let rock = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    let trunk = meshes.add(Cylinder::new(0.10, 0.7));
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.33, 0.28),
        perceptual_roughness: 1.0,
        ..default()
    });
    let leaf = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.31, 0.12),
        perceptual_roughness: 1.0,
        ..default()
    });
    let wood = materials.add(Color::srgb(0.25, 0.16, 0.09));
    // Shrubs sit only in authored cover; ruin stones only in blocked wall cells.
    // No decorative solid obstacle is introduced into a walkable corridor.
    let battlement = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for i in 0..160 + phalanx::procedural::MAX_WALLS * 8 {
        commands.spawn((
            Dressing(i),
            Mesh3d(if i >= 160 {
                battlement.clone()
            } else if i < 120 {
                rock.clone()
            } else {
                trunk.clone()
            }),
            MeshMaterial3d(if i < 80 {
                leaf.clone()
            } else if !(120..160).contains(&i) {
                stone.clone()
            } else {
                wood.clone()
            }),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}
// Bevy injects independent scene, animation and asset parameters.
#[allow(clippy::too_many_arguments)]
fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    models: Query<&Model>,
    assets: Res<CharacterAssets>,
    mut players: Query<&mut AnimationPlayer>,
    session: Res<Session>,
    names: Query<&Name>,
    hair: Res<HairAssets>,
) {
    let Ok(model) = models.get(event.entity) else {
        return;
    };
    for entity in children.iter_descendants(event.entity) {
        if names.get(entity).is_ok_and(|name| {
            matches!(
                name.as_str(),
                "Eye_L"
                    | "Eye_R"
                    | "Face_Sculpt"
                    | "Teeth_Upper"
                    | "Teeth_Lower"
                    | "Tongue"
                    | "Mouth_Interior"
                    | "Brow_L"
                    | "Brow_R"
                    | "Lid_L"
                    | "Lid_R"
                    | "Lip_Rim"
            )
        }) {
            commands.entity(entity).insert(FacePart { id: model.id });
        }
        if names
            .get(entity)
            .is_ok_and(|name| matches!(name.as_str(), "mixamorigRightArm" | "mixamorigLeftUpLeg"))
        {
            commands.entity(entity).insert(JointWitness {
                id: model.id,
                female: model.female,
            });
        }
        if names
            .get(entity)
            .is_ok_and(|name| name.as_str() == "mixamorigHead")
        {
            commands.entity(entity).with_children(|p| {
                for style in 0..10 {
                    p.spawn((
                        Hair {
                            id: model.id,
                            style: (style + 1) as u8,
                        },
                        WorldAssetRoot(hair.0[style].clone()),
                        Transform::from_xyz(
                            0.0,
                            if model.female { -1.34286 } else { -1.32460 },
                            0.0,
                        ),
                        Visibility::Hidden,
                    ))
                    .observe(hair_ready);
                }
            });
        }
        if let Ok(mut player) = players.get_mut(entity) {
            let sex = usize::from(model.female);
            player.play(assets.clips[sex][0]).repeat().pause();
            commands.entity(entity).insert((
                AnimationGraphHandle(assets.graphs[sex].clone()),
                Actor {
                    id: model.id,
                    female: model.female,
                    clip: 0,
                    clock: 0.0,
                    lock: 0.0,
                    health: session.visual_unit(model.id).map_or(0.0, |u| u.health),
                    cooldown: 0.0,
                    winding: false,
                    shots: 0,
                    position: session
                        .visual_unit(model.id)
                        .map_or(Vec2::ZERO, |u| Vec2::from_array(u.position)),
                    tick: session.battle.tick,
                    seed: session.battle.seed,
                    replays: 0,
                    attacks: 0,
                    lost_limbs: session.visual_unit(model.id).map_or(0, |u| u.lost_limbs),
                },
            ));
        }
    }
}
fn animate(
    session: Res<Session>,
    time: Res<Time>,
    assets: Res<CharacterAssets>,
    mut audit: ResMut<PanicAudit>,
    mut models: Query<(&Model, &mut Visibility)>,
    mut actors: Query<(&mut Actor, &mut AnimationPlayer)>,
) {
    if audit.run != Some(session.run_id) {
        audit.run = Some(session.run_id);
        audit.clips_seen = [false; 2];
    }
    let dt = if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else if session.battle.finished {
        time.delta_secs().min(0.1)
    } else {
        0.0
    };
    for (model, mut visibility) in &mut models {
        *visibility = if session.visual_unit(model.id).is_some()
            && model.female == female(&session, model.id)
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (mut actor, mut player) in &mut actors {
        let Some(unit) = session.visual_unit(actor.id) else {
            continue;
        };
        if actor.female != female(&session, actor.id) {
            continue;
        }
        let p = Vec2::from_array(unit.position);
        let reset = actor.seed != session.battle.seed
            || session.battle.tick < actor.tick
            || (unit.health > 0.0 && actor.health <= 0.0);
        if reset {
            actor.clip = usize::MAX;
            actor.clock = 0.0;
            actor.lock = 0.0;
            actor.health = unit.health;
            actor.cooldown = unit.cooldown;
            actor.winding = unit.windup.is_some();
            actor.shots = unit.rocks_thrown;
            actor.attacks = 0;
            actor.lost_limbs = unit.lost_limbs;
            actor.position = p;
        }
        let dt = if dt > 0.0 {
            dt
        } else {
            session.battle.tick.saturating_sub(actor.tick).min(4) as f32 * phalanx::DT
        };
        actor.lock = (actor.lock - dt).max(0.0);
        let damage = (actor.health - unit.health).max(0.0);
        let limb_loss = unit.lost_limbs & !actor.lost_limbs != 0;
        let bow = unit.projectile_weapon() == Some(phalanx::weapons::ProjectileWeapon::Bow);
        let advanced = unit.equipment.projectile_weapon().is_some() || bow;
        let release = advanced && unit.rocks_thrown > actor.shots;
        let attack = if advanced {
            unit.windup.is_some() && !actor.winding
                || (unit.ammunition == 0 && !release && unit.cooldown > actor.cooldown + 0.15)
        } else {
            unit.cooldown > actor.cooldown + 0.15
        };
        if advanced && actor.winding && unit.windup.is_none() && !release {
            actor.lock = 0.0;
        }
        let mut retrigger = false;
        let mut clip = actor.clip;
        if unit.health <= 0.0 {
            if !death_clip(clip) {
                clip = [6, 13, 14][unit.id.wrapping_add(session.battle.seed as usize) % 3];
            }
        } else if limb_loss || (damage > 0.0 && (!attack || damage / unit.max_health() >= 0.16)) {
            clip = if limb_loss {
                12
            } else if damage / unit.max_health() >= 0.16 {
                11
            } else {
                5
            };
            actor.lock = CLIP_DURATIONS[clip];
            retrigger = true;
        } else if attack {
            actor.attacks += 1;
            clip = attack_clip(unit, actor.attacks);
            actor.lock = CLIP_DURATIONS[clip];
            retrigger = true;
        } else if actor.lock == 0.0 {
            let moved = p.distance(actor.position);
            // A simulation tick advances positions every 0.1 s; retain gait between ticks.
            if session.battle.tick != actor.tick || reset {
                clip = locomotion_clip(unit, moved, session.battle.finished);
            }
        }
        if clip == usize::MAX {
            clip = 0;
        }
        // Opt-in animation review, isolated from normal clip selection and combat.
        if let Ok(name) = std::env::var("PHALANX_ANIMATION")
            && let Some(index) = CLIP_NAMES.iter().position(|clip| *clip == name)
        {
            clip = index;
            if let Ok(time) = std::env::var("PHALANX_ANIMATION_TIME") {
                actor.clock = time
                    .parse::<f32>()
                    .unwrap_or(0.0)
                    .clamp(0.0, CLIP_DURATIONS[clip]);
                retrigger = false;
            }
        }
        // Match the authored throw pose to simulation windup, then finish the follow-through.
        if let Some(draw) = &unit.windup {
            if !matches!(clip, 5 | 11 | 12) {
                clip = if bow { 0 } else { 4 };
                actor.clock = (1.0 - (draw.remaining - session.accumulator) / draw.weapon.windup())
                    .clamp(0.0, 1.0)
                    * 0.72;
                actor.lock = draw.remaining + 0.35;
            }
        } else if release && unit.health > 0.0 && !matches!(clip, 5 | 11 | 12) {
            clip = if bow { 0 } else { 4 };
            actor.clock = 0.72;
            actor.lock = 0.35;
        }
        if matches!(clip, 15 | 16) {
            audit.clips_seen[clip - 15] = true;
        }
        let node = assets.clips[usize::from(actor.female)][clip];
        let clip_dt = if actor.id == super::rpg::PREVIEW_ID {
            time.delta_secs()
        } else {
            dt
        };
        advance_clip(&mut actor, &mut player, clip, node, clip_dt, retrigger);
        actor.health = unit.health;
        actor.lost_limbs = unit.lost_limbs;
        actor.cooldown = unit.cooldown;
        actor.winding = unit.windup.is_some();
        actor.shots = unit.rocks_thrown;
        actor.tick = session.battle.tick;
        actor.seed = session.battle.seed;
        if session.battle.tick != 0 {
            actor.position = p;
        }
    }
}
fn advance_clip(
    actor: &mut Actor,
    player: &mut AnimationPlayer,
    clip: usize,
    node: AnimationNodeIndex,
    dt: f32,
    retrigger: bool,
) {
    if clip != actor.clip || (retrigger && clip > 2 && !death_clip(clip)) {
        if clip == actor.clip {
            actor.replays += 1;
        }
        actor.clip = clip;
        actor.clock = 0.0;
        player.stop_all();
        let playing = player.play(node);
        if loop_clip(clip) {
            playing.repeat();
        }
        playing.pause();
    } else {
        actor.clock += dt;
    }
    let duration = CLIP_DURATIONS[clip];
    let clock = if loop_clip(clip) {
        actor.clock % duration
    } else {
        actor.clock.min(duration - 0.001)
    };
    if let Some(animation) = player.animation_mut(node) {
        animation.set_seek_time(clock);
    }
}

#[derive(Component)]
struct Dressing(usize);
fn scenery(
    session: Res<Session>,
    mut pieces: Query<(&Dressing, &mut Transform, &mut Visibility)>,
    mut rendered: Local<Option<Arena>>,
) {
    let arena = session.battle.arena;
    if *rendered == Some(arena) {
        return;
    }
    *rendered = Some(arena);
    let size = arena.half_size();
    for (piece, mut transform, mut visibility) in &mut pieces {
        let i = piece.0;
        if i >= 160 {
            let wall = (i - 160) / 8;
            let block = (i - 160) % 8;
            if let Some((center, extent)) = arena.walls().get(wall) {
                let timber = if let Arena::Pressure(f) = arena {
                    f.layout > 0 && wall < 4 - (f.destroyed & 15).count_ones() as usize
                } else {
                    false
                };
                *visibility = Visibility::Inherited;
                let along_x = extent[0] > extent[1];
                let t = block as f32 / 7.0 - 0.5;
                let x = center[0] + if along_x { t * (extent[0] - 0.25) } else { 0. };
                let z = center[1] + if along_x { 0. } else { t * (extent[1] - 0.25) };
                let h = arena.elevation(*center) + if timber { 1.58 } else { 2.53 };
                *transform = Transform::from_xyz(x, h, z).with_scale(Vec3::new(
                    if along_x { 0.24 } else { extent[0] * 0.75 },
                    if timber { 0.12 } else { 0.26 },
                    if along_x { extent[1] * 0.75 } else { 0.24 },
                ));
            } else {
                *visibility = Visibility::Hidden;
            }
            continue;
        }
        let mut chosen = None;
        for attempt in 0..100 {
            let k = i * 97 + attempt * 43;
            let p = [
                ((k * 37 % 997) as f32 / 997.0 * 2.0 - 1.0) * (size[0] - 0.7),
                ((k * 67 % 991) as f32 / 991.0 * 2.0 - 1.0) * (size[1] - 0.7),
            ];
            if (i < 80 && arena.cover(p) > 0.0 && arena.bridge_height(p) == 0.0)
                || ((80..120).contains(&i) && arena.blocked(p))
                || (i >= 120 && arena.cover(p) > 0.0 && arena.bridge_height(p) == 0.0)
            {
                chosen = Some(p);
                break;
            }
        }
        if let Some(p) = chosen {
            *visibility = Visibility::Inherited;
            let h = arena.ground_elevation(p);
            let s = 0.20 + (i % 7) as f32 * 0.025;
            let next = Transform::from_xyz(p[0], h + if i >= 120 { 0.22 } else { s * 0.55 }, p[1])
                .with_scale(if i < 80 {
                    Vec3::new(s * 1.6, s, s * 1.4)
                } else if i < 120 {
                    Vec3::new(s * 1.3, s * 0.8, s)
                } else {
                    Vec3::splat(0.6)
                })
                .with_rotation(Quat::from_rotation_y(i as f32 * 1.7));
            if *transform != next {
                *transform = next;
            }
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[derive(Resource)]
struct HairAssets([Handle<WorldAsset>; 10]);
#[derive(Component)]
struct FacePart {
    id: usize,
}
#[derive(Component)]
struct Hair {
    id: usize,
    style: u8,
}
#[derive(Component)]
struct LookSummary;
#[derive(Resource)]
struct PortraitPhoto(Handle<Image>);
#[derive(Component)]
pub(super) struct PortraitCamera;
fn look(session: &Session, id: usize) -> phalanx::progression::Appearance {
    let id = if id == super::rpg::PREVIEW_ID {
        session.selected
    } else {
        id
    };
    session
        .visual_unit(id)
        .filter(|_| session.battle.founders)
        .map(|u| {
            let slot = u.id % session.battle.team_size();
            session.campaign.teams[u.team].fighters[slot]
                .appearance
                .resolved(u.team, slot)
        })
        .unwrap_or_else(|| {
            let team = session.visual_unit(id).map_or(0, |u| u.team);
            phalanx::progression::Appearance::default()
                .resolved(team, id % session.battle.team_size())
        })
}
pub(super) fn skin_color(session: &Session, id: usize) -> Color {
    let colors = [
        [1.0, 0.82, 0.69],
        [0.91, 0.68, 0.51],
        [0.75, 0.49, 0.32],
        [0.52, 0.31, 0.20],
        [0.34, 0.19, 0.13],
        [0.94, 0.75, 0.66],
        [0.73, 0.57, 0.44],
        [0.63, 0.43, 0.34],
    ];
    Color::srgb_from_array(colors[look(session, id).skin as usize % colors.len()])
}
#[derive(Component)]
struct HairTint {
    id: usize,
    material: Handle<StandardMaterial>,
    shade: f32,
}
fn hair_ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    roots: Query<&Hair>,
    children: Query<&Children>,
    primitives: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(hair) = roots.get(event.entity) else {
        return;
    };
    for entity in children.iter_descendants(event.entity) {
        if let Ok(old) = primitives.get(entity)
            && let Some(source) = materials.get(&old.0).cloned()
        {
            let shade = (source.base_color.to_linear().red / 0.16)
                .sqrt()
                .clamp(0.65, 1.3);
            let material = materials.add(source);
            commands.entity(entity).insert((
                MeshMaterial3d(material.clone()),
                HairTint {
                    id: hair.id,
                    material,
                    shade,
                },
            ));
        }
    }
}
fn tint_hair(
    session: Res<Session>,
    tints: Query<&HairTint>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let colors = [
        [0.085, 0.065, 0.055],
        [0.73, 0.21, 0.065],
        [0.28, 0.12, 0.055],
        [0.87, 0.67, 0.25],
        [0.66, 0.67, 0.70],
        [0.30, 0.12, 0.37],
    ];
    for tint in &tints {
        let choice = look(&session, tint.id).hair_color;
        let index = if choice == 0 {
            usize::from(female(&session, tint.id))
        } else {
            (choice as usize - 1) % colors.len()
        };
        if let Some(mut material) = materials.get_mut(&tint.material) {
            let c = colors[index];
            let color = Color::srgb(c[0] * tint.shade, c[1] * tint.shade, c[2] * tint.shade);
            if material.base_color != color {
                material.base_color = color;
            }
        }
    }
}
fn setup_looks(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.insert_resource(HairAssets(std::array::from_fn(|i| {
        server.load(GltfAssetLabel::Scene(0).from_asset(format!("characters/hair/{}.glb", i + 1)))
    })));
    let photo = images.add(Image::new_target_texture(
        320,
        320,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    ));
    commands.insert_resource(PortraitPhoto(photo.clone()));
    commands.spawn((
        PortraitCamera,
        Camera3d::default(),
        Camera {
            order: -1,
            clear_color: Color::srgb(0.045, 0.06, 0.075).into(),
            ..default()
        },
        RenderTarget::Image(photo.clone().into()),
        RenderLayers::layer(1),
        Transform::default(),
    ));
    commands
        .spawn((
            super::player::PortraitHud,
            Node {
                position_type: PositionType::Absolute,
                left: px(26),
                bottom: px(170),
                width: px(180),
                height: px(205),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.055, 0.075, 0.96)),
        ))
        .with_children(|p| {
            p.spawn((
                ImageNode::new(photo),
                Node {
                    width: px(180),
                    height: px(180),
                    ..default()
                },
            ));
            p.spawn((
                LookSummary,
                Text::new(""),
                super::font(11.0),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(0),
                    left: px(4),
                    ..default()
                },
            ));
        });
}
fn update_looks(
    session: Res<Session>,
    mut hair: Query<(&Hair, &mut Visibility), Without<FacePart>>,
    mut face: Query<(&FacePart, &mut Visibility), Without<Hair>>,
    mut summary: Query<&mut Text, With<LookSummary>>,
) {
    for (part, mut visible) in &mut hair {
        *visible = if look(&session, part.id).hair == part.style
            && session.visual_unit(part.id).is_some_and(|u| u.health > 0.0)
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (part, mut visible) in &mut face {
        *visible = if session.visual_unit(part.id).is_some_and(|u| u.health > 0.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let a = look(&session, session.selected);
    for mut text in &mut summary {
        text.0 = format!(
            "{} | Hair {} Face {}\nEyes {} Nose {} | Skin {} Dye {}",
            if female(&session, session.selected) {
                "Female"
            } else {
                "Male"
            },
            a.hair,
            a.face,
            a.eyes,
            a.nose,
            a.skin,
            a.hair_color
        );
    }
}
fn portrait_camera(
    session: Res<Session>,
    pawns: Query<(Entity, &Pawn, &Transform), Without<PortraitCamera>>,
    children: Query<&Children>,
    layers: Query<&RenderLayers>,
    mut camera: Single<&mut Transform, (With<PortraitCamera>, Without<super::rpg::CampCamera>)>,
    mut commands: Commands,
    mut old: Local<Option<Entity>>,
) {
    let Some((entity, _, pawn)) = pawns.iter().find(|(_, p, _)| p.0 == session.selected) else {
        return;
    };
    if old.is_some_and(|previous| previous != entity) {
        for child in children.iter_descendants(old.unwrap()) {
            if layers
                .get(child)
                .is_ok_and(|l| l.intersects(&RenderLayers::layer(1)))
            {
                commands.entity(child).insert(RenderLayers::layer(0));
            }
        }
    }
    *old = Some(entity);
    for child in children.iter_descendants(entity) {
        if !layers
            .get(child)
            .is_ok_and(|l| l.intersects(&RenderLayers::layer(1)))
        {
            commands
                .entity(child)
                .insert(RenderLayers::layer(0).with(1));
        }
    }
    let target = pawn.translation + Vec3::Y * 0.50;
    **camera = Transform::from_translation(target + pawn.rotation * Vec3::new(0.10, 0.08, 0.93))
        .looking_at(target, Vec3::Y);
}

#[derive(Component)]
struct JointWitness {
    id: usize,
    female: bool,
}
#[derive(Resource, Default)]
struct Witness {
    seen: [bool; 17],
    joints: std::collections::BTreeMap<Entity, Quat>,
    skeleton_motion: bool,
    model_motion: [bool; 2],
    blood: bool,
    seconds: f32,
    finished: f32,
    gibs: bool,
    wounded: bool,
    captured: bool,
    battle_captured: bool,
    refinement_captured: bool,
    weapon_captured: bool,
    winding_seen: bool,
    shaft_seen: bool,
}
#[allow(clippy::too_many_arguments)]
fn witness(
    session: Res<Session>,
    time: Res<Time>,
    actors: Query<&Actor>,
    joints: Query<(Entity, &JointWitness, &Transform)>,
    fx: Res<Fx>,
    bow_audit: Res<archery::Audit>,
    mut witness: ResMut<Witness>,
    mut exit: MessageWriter<AppExit>,
    gibs: Query<&body::Gib>,
    photo: Res<PortraitPhoto>,
    face_audit: Res<face::Audit>,
    audio_audit: Res<super::audio::Audit>,
    mut commands: Commands,
) {
    witness.seconds += time.delta_secs();
    for (entity, joint, transform) in &joints {
        if joint.id < session.battle.units.len()
            && joint.female == female(&session, joint.id)
            && let Some(old) = witness.joints.insert(entity, transform.rotation)
        {
            let moved = old.angle_between(transform.rotation) > 0.005;
            witness.skeleton_motion |= moved;
            witness.model_motion[usize::from(joint.female)] |= moved;
        }
    }
    if witness.seconds > 1.6 && !witness.captured && !actors.is_empty() {
        witness.captured = true;
        if session.menu
            || std::env::args().any(|a| {
                a == "--battlefield-witness"
                    || a == "--face-witness"
                    || a == "--standard-witness"
                    || a == "--weapon-loadout-witness"
                    || a == "--bow-draw-witness"
            })
        {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(
                    if std::env::args().any(|a| a == "--battlefield-witness") {
                        "outputs/battlefield-witness.png"
                    } else if std::env::args().any(|a| a == "--bow-draw-witness") {
                        if std::env::args().any(|a| a == "--male-bow-witness") {
                            "outputs/bow-male-draw-witness.png"
                        } else {
                            "outputs/bow-draw-witness.png"
                        }
                    } else if std::env::args().any(|a| a == "--weapon-report-witness") {
                        "outputs/weapon-report-witness.png"
                    } else if std::env::args().any(|a| a == "--weapon-workshop-witness") {
                        if std::env::args().any(|a| a == "--camp-scroll-witness") {
                            "outputs/weapon-recipes-witness.png"
                        } else {
                            "outputs/weapon-workshop-witness.png"
                        }
                    } else if std::env::args()
                        .any(|a| a == "--weapon-loadout-witness" || a == "--bow-draw-witness")
                    {
                        if std::env::args().any(|a| a == "--spear-witness") {
                            "outputs/spear-loadout-witness.png"
                        } else {
                            "outputs/atlatl-loadout-witness.png"
                        }
                    } else if std::env::args().any(|a| a == "--standard-witness") {
                        if std::env::args().any(|a| a == "--standard-back-witness") {
                            "outputs/standard-back-witness.png"
                        } else {
                            "outputs/standard-witness.png"
                        }
                    } else if std::env::args().any(|a| a == "--rival-witness") {
                        "outputs/rivals-witness.png"
                    } else if std::env::args().any(|a| a == "--camp-witness") {
                        if std::env::args().any(|a| a == "--workshop-witness") {
                            if std::env::args().any(|a| a == "--camp-scroll-witness") {
                                "outputs/workshop-scrolled-witness.png"
                            } else {
                                "outputs/workshop-witness.png"
                            }
                        } else {
                            "outputs/report-witness.png"
                        }
                    } else if std::env::args().any(|a| a == "--face-witness") {
                        "outputs/face-witness.png"
                    } else if std::env::args().any(|a| a == "--snapshot-witness") {
                        if session.battle.finished {
                            "outputs/snapshot-report-witness.png"
                        } else {
                            "outputs/snapshot-witness.png"
                        }
                    } else if session.ladder {
                        "outputs/ladder-witness.png"
                    } else {
                        "outputs/menu-witness.png"
                    },
                ));
        }
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::image(
                photo.0.clone(),
            ))
            .observe(bevy::render::view::screenshot::save_to_disk(
                "outputs/portrait-witness.png",
            ));
    }
    for actor in &actors {
        if actor.female == female(&session, actor.id)
            && actor.id < session.battle.units.len()
            && actor.clip < CLIP_NAMES.len()
        {
            witness.seen[actor.clip] = true;
        }
    }
    witness.blood |= fx.particles.iter().any(|p| p.life > 0.0);
    witness.winding_seen |= session.battle.units.iter().any(|u| u.windup.is_some());
    let airborne = session
        .battle
        .rocks
        .iter()
        .any(|p| p.weapon != phalanx::weapons::ProjectileWeapon::Rock);
    witness.shaft_seen |= airborne;
    if std::env::args().any(|a| a == "--weapon-witness")
        && airborne
        && witness.seconds > 1.6
        && !witness.weapon_captured
        && !session.menu
    {
        witness.weapon_captured = true;
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(
                if std::env::args().any(|a| a == "--bow-witness") {
                    "outputs/bow-battle-witness.png"
                } else if std::env::args().any(|a| a == "--spear-witness") {
                    "outputs/spear-battle-witness.png"
                } else {
                    "outputs/atlatl-battle-witness.png"
                },
            ));
    }

    witness.gibs |= !gibs.is_empty();
    witness.wounded |= session
        .battle
        .units
        .iter()
        .any(|u| u.health > 0.0 && u.lost_limbs > 0);
    if session.battle.finished {
        witness.finished += time.delta_secs();
    }

    if std::env::args().any(|a| a == "--refinement-witness")
        && !witness.refinement_captured
        && session.battle.tick > 200
        && session
            .battle
            .units
            .iter()
            .map(|u| u.damage_dealt)
            .sum::<f32>()
            > 80.0
        && !session.battle.damage_events.is_empty()
        && !session.menu
    {
        witness.refinement_captured = true;
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(
                "outputs/refinement-witness.png",
            ));
    }
    if witness.finished > 0.5 && !witness.battle_captured {
        witness.battle_captured = true;
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(
                "outputs/battle-witness.png",
            ));
    }
    if witness.finished
        > if std::env::args().any(|a| a == "--camp-witness" || a == "--weapon-report-witness") {
            3.0
        } else {
            1.5
        }
        || witness.seconds > 40.0
        || (((session.menu && !session.battle.finished)
            || std::env::args().any(|a| {
                a == "--battlefield-witness"
                    || a == "--face-witness"
                    || a == "--standard-witness"
                    || a == "--weapon-loadout-witness"
                    || a == "--bow-draw-witness"
            }))
            && witness.seconds
                > if std::env::args().any(|a| a == "--rpg-witness") {
                    9.0
                } else {
                    3.0
                })
    {
        println!(
            "VISUAL_WITNESS {}",
            serde_json::json!({"clip_names":CLIP_NAMES,"clips":witness.seen,"blood":witness.blood,"actors":actors.iter().count(),"finished":session.battle.finished,"seconds":witness.seconds,"bow_poses":bow_audit.poses,"bow_grip_error":bow_audit.max_grip_error,"bow_string_error":bow_audit.max_string_error,"bow_draw":bow_audit.max_draw,"arrows_blocked":session.battle.units.iter().map(|u|u.arrow_blocks).sum::<u32>(),"weapon_windup":witness.winding_seen,"shaft_projectile":witness.shaft_seen,"selected_weapon":session.battle.units[session.selected].weapon_status(),"selected_shots":session.battle.units[session.selected].weapon_shots,"selected_healing":session.battle.units[session.selected].healing_done,"particle_capacity":fx.particles.len(),"stained_pixels":fx.stained_pixels(),"detached_limbs":witness.gibs,"surviving_wounded":witness.wounded,"face_expressions":face_audit.seen,"eye_rotation":face_audit.eye_rotation,"face_morph_meshes":face_audit.meshes,"repeated_gestures":actors.iter().map(|a|a.replays).sum::<u32>(),"skeleton_motion":witness.skeleton_motion,"male_female_motion":witness.model_motion,"sound_effects":audio_audit.emitted})
        );
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod animation_regressions {
    use super::*;
    #[test]
    fn critical_health_loops_panic_and_stagger_without_changing_tactics() {
        let b = phalanx::Battle::with_founders(42, false, phalanx::terrain::Arena::OpenField);
        let mut u = b.units[0].clone();
        assert_eq!(locomotion_clip(&u, 0., false), 0);
        u.health = u.max_health() * 0.25;
        let before = serde_json::to_string(&u).unwrap();
        assert_eq!(locomotion_clip(&u, 0., false), 15);
        assert_eq!(locomotion_clip(&u, 0.1, false), 16);
        assert_eq!(locomotion_clip(&u, 0., true), 0);
        let (_, nodes) = AnimationGraph::from_clips(
            (0..CLIP_NAMES.len()).map(|_| Handle::<AnimationClip>::default()),
        );
        for clip in [15, 16] {
            let mut actor = Actor {
                clip: usize::MAX,
                ..default()
            };
            let mut player = AnimationPlayer::default();
            advance_clip(&mut actor, &mut player, clip, nodes[clip], 0., false);
            advance_clip(
                &mut actor,
                &mut player,
                clip,
                nodes[clip],
                CLIP_DURATIONS[clip] + 0.3,
                false,
            );
            assert!((player.animation(nodes[clip]).unwrap().seek_time() - 0.3).abs() < 0.001);
        }
        assert_eq!(serde_json::to_string(&u).unwrap(), before);
    }
    #[test]
    fn injuries_and_class_change_gestures_without_mutating_combat() {
        let battle = phalanx::Battle::with_founders(42, false, phalanx::terrain::Arena::OpenField);
        let mut unit = battle.units[0].clone();
        unit.role = Role::Fighter;
        unit.class = Some(phalanx::FighterClass::Scout);
        assert_eq!(attack_clip(&unit, 1), 7);
        unit.class = Some(phalanx::FighterClass::Guardian);
        assert_eq!(attack_clip(&unit, 1), 8);
        unit.health = unit.max_health() * 0.20;
        assert_eq!(attack_clip(&unit, 1), 9);
        unit.role = Role::RockThrower;
        unit.mind.patience = 0.1;
        assert_eq!(attack_clip(&unit, 1), 10);
        assert_eq!(attack_clip(&unit, 2), 4);
    }
    #[test]
    fn death_variants_advance_and_hold_instead_of_restarting_on_hits() {
        let (_, nodes) = AnimationGraph::from_clips(
            (0..CLIP_NAMES.len()).map(|_| Handle::<AnimationClip>::default()),
        );
        for clip in [6, 13, 14] {
            let mut actor = Actor {
                clip: usize::MAX,
                ..default()
            };
            let mut player = AnimationPlayer::default();
            advance_clip(&mut actor, &mut player, clip, nodes[clip], 0.0, true);
            advance_clip(&mut actor, &mut player, clip, nodes[clip], 0.3, true);
            assert!(player.animation(nodes[clip]).unwrap().seek_time() > 0.29);
            advance_clip(&mut actor, &mut player, clip, nodes[clip], 10.0, true);
            assert!(
                (player.animation(nodes[clip]).unwrap().seek_time()
                    - (CLIP_DURATIONS[clip] - 0.001))
                    .abs()
                    < 0.001
            );
            assert_eq!(actor.replays, 0);
        }
    }
    #[test]
    fn repeated_attacks_throws_and_hits_rewind_and_then_advance() {
        let (_, nodes) = AnimationGraph::from_clips(
            (0..CLIP_NAMES.len()).map(|_| Handle::<AnimationClip>::default()),
        );
        for clip in [3, 4, 5, 7, 8, 9, 10, 11, 12] {
            let node = nodes[clip];
            let mut actor = Actor {
                clip: usize::MAX,
                ..default()
            };
            let mut player = AnimationPlayer::default();
            advance_clip(&mut actor, &mut player, clip, node, 0.0, true);
            advance_clip(&mut actor, &mut player, clip, node, 0.30, false);
            assert!(player.animation(node).unwrap().seek_time() > 0.29);
            advance_clip(&mut actor, &mut player, clip, node, 0.1, true);
            assert_eq!(player.animation(node).unwrap().seek_time(), 0.0);
            advance_clip(&mut actor, &mut player, clip, node, 0.1, false);
            assert!(player.animation(node).unwrap().seek_time() > 0.09);
            assert_eq!(actor.replays, 1);
        }
    }
}
