//! Presentation only: character clips, bounded combat particles, arena dressing.
//! Damage, pathfinding, terrain height and controller decisions remain in Battle.
mod barks;
mod body;
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
        barks::install(app);
        app.add_systems(Update, portrait_camera.after(super::camera_motion));
        if std::env::args().any(|a| a == "--visual-witness") {
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
                    body::update,
                    body::physics,
                )
                    .after(super::sync_pawns),
            );
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
#[derive(Component)]
struct Actor {
    id: usize,
    female: bool,
    clip: usize,
    clock: f32,
    lock: f32,
    health: f32,
    cooldown: f32,
    position: Vec2,
    tick: u32,
    seed: u64,
}
fn female(session: &Session, id: usize) -> bool {
    if let Some(choice) = look(session, id).female {
        return choice;
    }
    session
        .battle
        .units
        .get(id)
        .is_some_and(|u| match u.command_title {
            Some(CommandTitle::Queen) => true,
            Some(_) => false,
            None => id % 2 == 1,
        })
}
fn load_visuals(
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
            (0..7).map(|i| server.load(GltfAssetLabel::Animation(i).from_asset(path))),
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
                .observe(body::ready);
            });
        }
    }
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.82, 0.88, 1.0),
        brightness: 550.0,
        ..default()
    });
    let blood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.29, 0.008, 0.018),
        perceptual_roughness: 0.24,
        ..default()
    });
    let flesh = materials.add(StandardMaterial {
        base_color: Color::srgb(0.64, 0.10, 0.16),
        perceptual_roughness: 0.32,
        ..default()
    });
    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(12, 8));
    let gut = meshes.add(Torus::new(0.05, 0.09));
    for i in 0..1536 {
        commands.spawn((
            Particle(i),
            Mesh3d(if i % 4 == 0 {
                gut.clone()
            } else {
                sphere.clone()
            }),
            MeshMaterial3d(if i % 4 == 0 {
                flesh.clone()
            } else {
                blood.clone()
            }),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    commands.insert_resource(Fx::default());
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
    for i in 0..200 {
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
        if names
            .get(entity)
            .is_ok_and(|name| name.as_str() == "mixamorigHead")
        {
            commands.entity(entity).with_children(|p| {
                for style in 0..6 {
                    p.spawn((
                        Hair {
                            id: model.id,
                            style: (style + 1) as u8,
                        },
                        WorldAssetRoot(hair.0[style].clone()),
                        Transform::from_xyz(0.0, -1.3957, 0.0),
                        Visibility::Hidden,
                    ));
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
                    health: session.battle.units.get(model.id).map_or(0.0, |u| u.health),
                    cooldown: 0.0,
                    position: session
                        .battle
                        .units
                        .get(model.id)
                        .map_or(Vec2::ZERO, |u| Vec2::from_array(u.position)),
                    tick: session.battle.tick,
                    seed: session.battle.seed,
                },
            ));
        }
    }
}
fn animate(
    session: Res<Session>,
    time: Res<Time>,
    assets: Res<CharacterAssets>,
    mut models: Query<(&Model, &mut Visibility)>,
    mut actors: Query<(&mut Actor, &mut AnimationPlayer)>,
) {
    let dt = if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else if session.battle.finished {
        time.delta_secs().min(0.1)
    } else {
        0.0
    };
    for (model, mut visibility) in &mut models {
        *visibility = if model.id < session.battle.units.len()
            && model.female == female(&session, model.id)
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (mut actor, mut player) in &mut actors {
        let Some(unit) = session.battle.units.get(actor.id) else {
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
            actor.position = p;
        }
        actor.lock = (actor.lock - dt).max(0.0);
        let mut clip = actor.clip;
        if unit.health <= 0.0 {
            clip = 6;
        } else if unit.cooldown > actor.cooldown + 0.15 {
            clip = if unit.role == Role::RockThrower { 4 } else { 3 };
            actor.lock = if clip == 4 { 1.1 } else { 0.65 };
        } else if unit.health < actor.health {
            clip = 5;
            actor.lock = 0.38;
        } else if actor.lock == 0.0 {
            let moved = p.distance(actor.position);
            // A simulation tick advances positions every 0.1 s; retain gait between ticks.
            if session.battle.tick != actor.tick || reset {
                clip = if moved > 0.13 {
                    2
                } else if moved > 0.005 {
                    1
                } else {
                    0
                };
            }
        }
        if clip == usize::MAX {
            clip = 0;
        }
        if clip != actor.clip {
            actor.clip = clip;
            actor.clock = 0.0;
            player.stop_all();
            let playing = player.play(assets.clips[usize::from(actor.female)][clip]);
            if clip <= 2 {
                playing.repeat();
            }
            playing.pause();
        } else {
            actor.clock += dt;
        }
        let duration = [1.6, 0.9, 0.6, 0.65, 1.1, 0.38, 1.1][clip];
        let clock = if clip <= 2 {
            actor.clock % duration
        } else {
            actor.clock.min(duration - 0.001)
        };
        if let Some(animation) = player.animation_mut(assets.clips[usize::from(actor.female)][clip])
        {
            animation.set_seek_time(clock);
        }
        actor.health = unit.health;
        actor.cooldown = unit.cooldown;
        actor.tick = session.battle.tick;
        actor.seed = session.battle.seed;
        if session.battle.tick != 0 {
            actor.position = p;
        }
    }
}
#[derive(Component)]
struct Particle(usize);
#[derive(Clone, Default)]
struct Fleck {
    p: Vec3,
    v: Vec3,
    scale: Vec3,
    life: f32,
    age: f32,
    pool: bool,
}
#[derive(Resource)]
struct Fx {
    particles: Vec<Fleck>,
    cursor: usize,
    health: [f32; 20],
    bleeding: [f32; 20],
    pulse: f32,
    tick: u32,
    seed: u64,
    arena: Arena,
    initialized: bool,
}
impl Default for Fx {
    fn default() -> Self {
        Self {
            particles: vec![Fleck::default(); 1536],
            cursor: 0,
            health: [0.0; 20],
            bleeding: [0.0; 20],
            pulse: 0.0,
            tick: 0,
            seed: 0,
            arena: Arena::Ridge,
            initialized: false,
        }
    }
}
impl Fx {
    fn emit(&mut self, p: Vec3, count: usize, death: bool, id: usize) {
        for j in 0..count {
            let index = self.cursor % 1536;
            self.cursor += 1;
            let a = (j * 137 + id * 31) as f32 * 0.17;
            let speed = if death {
                2.0 + (j % 7) as f32 * 0.35
            } else {
                0.6 + (j % 5) as f32 * 0.25
            };
            let guts = death && index.is_multiple_of(4);
            self.particles[index] = Fleck {
                p,
                v: Vec3::new(
                    a.cos() * speed,
                    if death {
                        3.3 + (j % 5) as f32 * 0.65
                    } else {
                        1.8 + (j % 3) as f32 * 0.3
                    },
                    a.sin() * speed,
                ),
                scale: if guts {
                    Vec3::new(1.2, 0.6, 0.8)
                } else if death && j < 3 {
                    Vec3::new(0.11, 0.07, 0.07)
                } else {
                    Vec3::splat(0.025 + (j % 3) as f32 * 0.012)
                },
                life: if death { 180.0 } else { 90.0 },
                age: 0.0,
                pool: false,
            };
        }
        if death {
            // Flattened wet pool and several lobes suggest spilled organs; cosmetic only.
            for j in 0..4 {
                let index = self.cursor % 1536;
                self.cursor += 1;
                self.particles[index] = Fleck {
                    p: p + Vec3::new(j as f32 * 0.11 - 0.16, -0.48, j as f32 * 0.06),
                    v: Vec3::ZERO,
                    scale: if index.is_multiple_of(4) {
                        Vec3::new(1.7, 0.5, 1.0)
                    } else {
                        Vec3::new(0.65, 0.018, 0.47)
                    },
                    life: 240.0,
                    age: 0.0,
                    pool: true,
                };
            }
        }
    }
}
fn effects(
    session: Res<Session>,
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    mut particles: Query<(&Particle, &mut Transform, &mut Visibility)>,
) {
    let reset = !fx.initialized
        || fx.seed != session.battle.seed
        || fx.arena != session.battle.arena
        || session.battle.tick < fx.tick;
    if reset {
        for p in &mut fx.particles {
            p.life = 0.0;
        }
        for u in &session.battle.units {
            fx.health[u.id] = u.health;
        }
        fx.initialized = true;
        fx.bleeding = [0.0; 20];
        fx.pulse = 0.0;
    } else {
        for u in &session.battle.units {
            let old = fx.health[u.id];
            if u.health < old {
                fx.bleeding[u.id] = if u.health <= 0.0 { 1.4 } else { 0.3 };
                let p = session.presented(u.id);
                let pos = Vec3::new(p[0], session.battle.arena.elevation(p) + 0.52, p[1]);
                fx.emit(
                    pos,
                    if u.health <= 0.0 { 110 } else { 15 },
                    u.health <= 0.0,
                    u.id,
                );
            }
            fx.health[u.id] = u.health;
        }
    }
    fx.seed = session.battle.seed;
    fx.tick = session.battle.tick;
    fx.arena = session.battle.arena;
    let dt = if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else if session.battle.finished {
        time.delta_secs().min(0.1)
    } else {
        0.0
    };
    fx.pulse += dt;
    if fx.pulse >= 0.045 {
        fx.pulse = 0.0;
        for u in &session.battle.units {
            if fx.bleeding[u.id] > 0.0 {
                fx.bleeding[u.id] = (fx.bleeding[u.id] - dt.max(0.045)).max(0.0);
                let p = session.presented(u.id);
                fx.emit(
                    Vec3::new(p[0], session.battle.arena.elevation(p) + 0.48, p[1]),
                    3,
                    false,
                    u.id,
                );
            }
        }
    }
    for p in &mut fx.particles {
        if p.life <= 0.0 {
            continue;
        }
        p.age += dt;
        p.life -= dt;
        if !p.pool {
            // Small fixed-size presentation steps prevent fast playback tunnelling.
            let steps = (dt / 0.016).ceil().max(1.0) as usize;
            let step = dt / steps as f32;
            for _ in 0..steps {
                p.v.y -= 9.81 * step;
                p.v *= (-0.18 * step).exp();
                let old = p.p;
                let velocity = p.v;
                p.p += velocity * step;
                let arena = session.battle.arena;
                let size = arena.half_size();
                if p.p.x.abs() > size[0] - 0.1 {
                    p.p.x = p.p.x.clamp(-size[0] + 0.1, size[0] - 0.1);
                    p.v.x *= -0.25;
                }
                if p.p.z.abs() > size[1] - 0.1 {
                    p.p.z = p.p.z.clamp(-size[1] + 0.1, size[1] - 0.1);
                    p.v.z *= -0.25;
                }
                if arena.blocked([p.p.x, p.p.z]) && p.p.y < arena.elevation([p.p.x, p.p.z]) + 2.6 {
                    if arena.blocked([p.p.x, old.z]) {
                        p.p.x = old.x;
                        p.v.x *= -0.25;
                    }
                    if arena.blocked([old.x, p.p.z]) {
                        p.p.z = old.z;
                        p.v.z *= -0.25;
                    }
                }
                let ground = arena.elevation([p.p.x, p.p.z]) + 0.025;
                if p.p.y <= ground {
                    p.p.y = ground;
                    if p.scale.y > 0.1 && p.v.y.abs() > 0.9 {
                        p.v.y = p.v.y.abs() * 0.28;
                        p.v.x *= 0.6;
                        p.v.z *= 0.6;
                    } else {
                        p.v = Vec3::ZERO;
                        p.pool = true;
                        if p.scale.x < 0.1 {
                            p.scale = Vec3::new(0.10, 0.009, 0.075);
                        }
                        break;
                    }
                }
            }
        } else {
            p.p.y = session.battle.arena.elevation([p.p.x, p.p.z]) + 0.025;
            if p.scale.y < 0.03 {
                let target = if p.scale.x > 0.2 { 1.15 } else { 0.18 };
                p.scale.x = (p.scale.x + dt * 0.055).min(target);
                p.scale.z = (p.scale.z + dt * 0.035).min(target * 0.8);
            }
        }
    }
    for (index, mut transform, mut visibility) in &mut particles {
        let p = &fx.particles[index.0];
        *visibility = if p.life > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if p.life > 0.0 {
            transform.translation = p.p;
            transform.scale = p.scale;
            transform.rotation = if p.pool {
                let arena = session.battle.arena;
                let dx =
                    arena.elevation([p.p.x + 0.05, p.p.z]) - arena.elevation([p.p.x - 0.05, p.p.z]);
                let dz =
                    arena.elevation([p.p.x, p.p.z + 0.05]) - arena.elevation([p.p.x, p.p.z - 0.05]);
                Quat::from_rotation_arc(Vec3::Y, Vec3::new(-dx, 0.1, -dz).normalize())
                    * Quat::from_rotation_y(index.0 as f32 * 2.4)
            } else {
                Quat::from_euler(EulerRot::XYZ, p.age * 3., p.age * 2., index.0 as f32)
            };
        }
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
                *visibility = Visibility::Inherited;
                let z = center[1] + (block as f32 / 7.0 - 0.5) * (extent[1] - 0.45);
                let h = arena.elevation(*center) + 2.55;
                *transform = Transform::from_xyz(center[0], h, z).with_scale(Vec3::new(
                    extent[0] * 0.46,
                    0.24,
                    0.29,
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
struct Looks {
    images: [Vec<Handle<Image>>; 4],
}
#[derive(Resource)]
struct HairAssets([Handle<WorldAsset>; 6]);
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
    session
        .battle
        .units
        .get(id)
        .filter(|_| session.battle.founders)
        .map(|u| {
            session.campaign.teams[u.team].fighters[u.id % session.battle.team_size()].appearance
        })
        .unwrap_or_default()
}
fn setup_looks(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut inputs: [Vec<Handle<Image>>; 4] = std::array::from_fn(|_| vec![]);
    for (i, name) in ["faces.png", "eyes.png", "noses.png"].iter().enumerate() {
        inputs[i + 1].push(server.load(format!("customization/{name}")));
    }
    commands.insert_resource(Looks { images: inputs });
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
    mut hair: Query<(&Hair, &mut Visibility)>,
    mut summary: Query<&mut Text, With<LookSummary>>,
) {
    for (part, mut visible) in &mut hair {
        *visible = if look(&session, part.id).hair == part.style
            && session
                .battle
                .units
                .get(part.id)
                .is_some_and(|u| u.health > 0.0)
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let a = look(&session, session.selected);
    for mut text in &mut summary {
        text.0 = format!(
            "{} | Hair {} Face {}\nEyes {} Nose {}",
            if female(&session, session.selected) {
                "Female"
            } else {
                "Male"
            },
            a.hair,
            a.face,
            a.eyes,
            a.nose
        );
    }
}
fn portrait_camera(
    session: Res<Session>,
    pawns: Query<(Entity, &Pawn, &Transform), Without<PortraitCamera>>,
    children: Query<&Children>,
    layers: Query<&RenderLayers>,
    mut camera: Single<&mut Transform, With<PortraitCamera>>,
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

#[derive(Resource, Default)]
struct Witness {
    seen: [bool; 7],
    blood: bool,
    seconds: f32,
    finished: f32,
    gibs: bool,
    wounded: bool,
    captured: bool,
    battle_captured: bool,
}
#[allow(clippy::too_many_arguments)]
fn witness(
    session: Res<Session>,
    time: Res<Time>,
    actors: Query<&Actor>,
    fx: Res<Fx>,
    mut witness: ResMut<Witness>,
    mut exit: MessageWriter<AppExit>,
    gibs: Query<&body::Gib>,
    photo: Res<PortraitPhoto>,
    mut commands: Commands,
) {
    witness.seconds += time.delta_secs();
    if witness.seconds > 1.6 && !witness.captured && !actors.is_empty() {
        witness.captured = true;
        if session.menu {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(
                    if session.ladder {
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
            && actor.clip < 7
        {
            witness.seen[actor.clip] = true;
        }
    }
    witness.blood |= fx.particles.iter().any(|p| p.life > 0.0);
    witness.gibs |= !gibs.is_empty();
    witness.wounded |= session
        .battle
        .units
        .iter()
        .any(|u| u.health > 0.0 && u.lost_limbs > 0);
    if session.battle.finished {
        witness.finished += time.delta_secs();
    }
    if witness.finished > 0.5 && !witness.battle_captured {
        witness.battle_captured = true;
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(
                "outputs/battle-witness.png",
            ));
    }
    if witness.finished > 1.5 || witness.seconds > 40.0 || (session.menu && witness.seconds > 3.0) {
        println!(
            "VISUAL_WITNESS {}",
            serde_json::json!({"clips":witness.seen,"blood":witness.blood,"actors":actors.iter().count(),"finished":session.battle.finished,"seconds":witness.seconds,"particle_capacity":fx.particles.len(),"detached_limbs":witness.gibs,"surviving_wounded":witness.wounded})
        );
        exit.write(AppExit::Success);
    }
}
