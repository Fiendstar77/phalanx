//! Ink theatre from resolved impacts and sampled real skeletal poses.
//! These strokes are scenery: they have no collider, damage radius or Battle write.
use super::*;
use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology};
use std::{collections::BTreeMap, f32::consts::TAU};

const BLASTS: usize = 8;
const STROKES: usize = 12;
const ECHOES: usize = 24;
const BONES: [&str; 16] = [
    "Hips",
    "Spine2",
    "Head",
    "Neck",
    "LeftArm",
    "LeftForeArm",
    "LeftHand",
    "RightArm",
    "RightForeArm",
    "RightHand",
    "LeftUpLeg",
    "LeftLeg",
    "LeftFoot",
    "RightUpLeg",
    "RightLeg",
    "RightFoot",
];
// From/to joint, visual radius, missing-limb mask. Heads use their actual joint rotation.
const SEGMENTS: [(usize, usize, f32, u8); 11] = [
    (0, 1, 0.16, 0),
    (1, 3, 0.18, 0),
    (2, 2, 0.29, 0),
    (4, 5, 0.055, 1),
    (5, 6, 0.05, 1),
    (7, 8, 0.055, 2),
    (8, 9, 0.05, 2),
    (10, 11, 0.08, 4),
    (11, 12, 0.065, 4),
    (13, 14, 0.08, 8),
    (14, 15, 0.065, 8),
];
#[derive(Component)]
struct Rig {
    id: usize,
    female: bool,
    bones: [Entity; 16],
}
#[derive(Component)]
struct Stroke(usize, usize);
#[derive(Component)]
struct EchoPart(usize, usize);
#[derive(Clone, Copy, Default)]
struct Blast {
    origin: Vec3,
    direction: Vec3,
    strength: f32,
    age: f32,
    active: bool,
}
#[derive(Clone)]
struct Echo {
    pose: [Transform; 11],
    color: [f32; 3],
    life: f32,
    lost: u8,
}
impl Default for Echo {
    fn default() -> Self {
        Self {
            pose: [Transform::IDENTITY; 11],
            color: [1.; 3],
            life: 0.,
            lost: 0,
        }
    }
}
#[derive(Resource)]
struct Ink {
    run: Option<u32>,
    last_hit: Option<u64>,
    next_blast: usize,
    next_echo: usize,
    last_blast: Option<u32>,
    blasts: [Blast; BLASTS],
    echoes: Vec<Echo>,
    sampled: BTreeMap<usize, (f32, Vec3)>,
    clock: f32,
}
impl Default for Ink {
    fn default() -> Self {
        Self {
            run: None,
            last_hit: None,
            next_blast: 0,
            next_echo: 0,
            last_blast: None,
            blasts: [Blast::default(); BLASTS],
            echoes: vec![Echo::default(); ECHOES],
            sampled: BTreeMap::new(),
            clock: 0.,
        }
    }
}
#[derive(Resource, Default)]
pub(super) struct Audit {
    pub impacts: u32,
    pub echoes: u32,
    pub max_radius: f32,
    pub peak_echoes: usize,
}
#[derive(Resource)]
struct Kit {
    ring: Handle<Mesh>,
    shard: Handle<Mesh>,
    ivory: Handle<StandardMaterial>,
    ink: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    echoes: Vec<Handle<StandardMaterial>>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Ink>()
        .init_resource::<Audit>()
        .add_systems(Startup, setup)
        .add_systems(Update, impacts.after(super::super::sync_pawns))
        .add_systems(
            PostUpdate,
            echoes.after(bevy::transform::TransformSystems::Propagate),
        );
}
// A broken, tapered brush circle. Its edge has authored irregularity, never random flicker.
fn brush() -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for i in 0..64 {
        let edge = |j: usize, outer: bool| {
            let t = j as f32 / 64.;
            let a = t * TAU * 0.92;
            let width = (t * std::f32::consts::PI).sin().max(0.).sqrt();
            let r = 1.
                + if outer {
                    0.10 * width * (1. + (a * 17.).sin() * 0.30)
                } else {
                    -0.035 * width
                };
            [a.cos() * r, 0., a.sin() * r]
        };
        for p in [
            edge(i, false),
            edge(i, true),
            edge(i + 1, true),
            edge(i, false),
            edge(i + 1, true),
            edge(i + 1, false),
        ] {
            positions.push(p);
            normals.push([0., 1., 0.]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let make = |rgb: [f32; 3]| StandardMaterial {
        base_color: Color::srgb(rgb[0], rgb[1], rgb[2]),
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    };
    let ivory = materials.add(make([0.95, 0.86, 0.66]));
    let ink = materials.add(make([0.008, 0.018, 0.026]));
    let red = materials.add(make([0.75, 0.025, 0.07]));
    let ring = meshes.add(brush());
    let shard = meshes.add(Cuboid::new(1., 1., 1.));
    for slot in 0..BLASTS {
        for part in 0..STROKES {
            commands.spawn((
                Stroke(slot, part),
                Mesh3d(if part < 3 {
                    ring.clone()
                } else {
                    shard.clone()
                }),
                MeshMaterial3d(ivory.clone()),
                Transform::default(),
                Visibility::Hidden,
                bevy::light::NotShadowCaster,
            ));
        }
    }
    let sphere = meshes.add(Sphere::new(1.).mesh().uv(12, 8));
    let mut echoes = Vec::new();
    for slot in 0..ECHOES {
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.1, 0.8, 0.8, 0.),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        for part in 0..SEGMENTS.len() {
            commands.spawn((
                EchoPart(slot, part),
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material.clone()),
                Transform::default(),
                Visibility::Hidden,
                bevy::light::NotShadowCaster,
            ));
        }
        echoes.push(material);
    }
    commands.insert_resource(Kit {
        ring,
        shard,
        ivory,
        ink,
        red,
        echoes,
    });
}
pub(super) fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    models: Query<&Model>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let Ok(model) = models.get(event.entity) else {
        return;
    };
    let names: BTreeMap<_, _> = children
        .iter_descendants(event.entity)
        .filter_map(|e| {
            names.get(e).ok().map(|n| {
                // Supplied exports retain Mixamo's prefix; scene APIs expose
                // those original node names rather than animation aliases.
                (bone_name(n.as_str()).to_string(), e)
            })
        })
        .collect();
    let Some(bones) = BONES
        .iter()
        .map(|n| names.get(*n).copied())
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    commands.entity(event.entity).insert(Rig {
        id: model.id,
        female: model.female,
        bones: bones.try_into().unwrap(),
    });
}
fn bone_name(name: &str) -> &str {
    let name = name.rsplit(':').next().unwrap_or(name);
    name.strip_prefix("mixamorig").unwrap_or(name)
}
fn severity(session: &Session, hit: &phalanx::DamageEvent) -> f32 {
    let Some(unit) = session.battle.units.get(hit.defender) else {
        return 0.;
    };
    // Resolved melee only. A missed strike or ranged impact cannot claim this beat.
    let melee = session.battle.theus_events.iter().any(|e| {
        e.tick == hit.tick
            && e.attacker == hit.attacker
            && e.defender == hit.defender
            && !e.ranged
            && e.resolution.damage > 0.
            && e.resolution.damage == hit.amount
    });
    if !melee {
        return 0.;
    }
    let fraction = hit.amount / unit.max_health();
    if fraction < 0.06 && hit.severed == 0 {
        0.
    } else {
        (fraction * 6.).clamp(0.4, 1.6)
    }
}
fn radius(blast: &Blast) -> f32 {
    let t = (blast.age / 1.05).clamp(0., 1.);
    ((1. - (1. - t).powi(3)) * (3.5 + blast.strength * 5.)).max(0.001)
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn impacts(
    session: Res<Session>,
    time: Res<Time>,
    kit: Res<Kit>,
    mut ink: ResMut<Ink>,
    mut audit: ResMut<Audit>,
    mut strokes: Query<(
        &Stroke,
        &mut Transform,
        &mut Visibility,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    if ink.run != Some(session.run_id) {
        *ink = Ink {
            run: Some(session.run_id),
            ..default()
        };
        *audit = Audit::default();
    }
    let dt = if session.menu {
        0.
    } else if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else if session.battle.finished {
        time.delta_secs().min(0.1)
    } else {
        0.
    };
    ink.clock += dt;
    for blast in &mut ink.blasts {
        if blast.active {
            blast.age += dt;
            blast.active = blast.age < 1.05;
        }
    }
    let mut chosen = None;
    for hit in &session.battle.damage_events {
        if ink.last_hit.is_some_and(|s| hit.sequence <= s) {
            continue;
        }
        let strength = severity(&session, hit);
        if strength > 0. && chosen.is_none_or(|(_, old)| strength > old) {
            chosen = Some((hit, strength));
        }
    }
    if let Some(last) = session.battle.damage_events.last() {
        ink.last_hit = Some(last.sequence);
    }
    if let Some((hit, strength)) = chosen
        && ink
            .last_blast
            .is_none_or(|last| hit.tick.saturating_sub(last) >= 6)
    {
        let direction = Vec3::new(hit.velocity[0], 0., hit.velocity[1]).normalize_or(Vec3::Z);
        let origin = Vec3::new(
            hit.position[0],
            session.battle.arena.elevation(hit.position) + 0.08,
            hit.position[1],
        );
        let slot = ink.next_blast % BLASTS;
        ink.next_blast += 1;
        ink.blasts[slot] = Blast {
            origin,
            direction,
            strength,
            age: 0.,
            active: true,
        };
        ink.last_blast = Some(hit.tick);
        audit.impacts += 1;
    }
    for (stroke, mut transform, mut visible, mut mesh, mut material) in &mut strokes {
        let b = ink.blasts[stroke.0];
        *visible = if b.active && !session.menu {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if !b.active {
            continue;
        }
        let t = b.age / 1.05;
        let r = radius(&b);
        audit.max_radius = audit.max_radius.max(r);
        let fade = (1. - t).max(0.);
        let yaw = b.direction.x.atan2(b.direction.z);
        mesh.0 = if stroke.1 < 3 {
            kit.ring.clone()
        } else {
            kit.shard.clone()
        };
        material.0 = if stroke.1 == 0 {
            kit.ink.clone()
        } else if stroke.1.is_multiple_of(3) {
            kit.red.clone()
        } else {
            kit.ivory.clone()
        };
        *transform = match stroke.1 {
            0 => Transform::from_translation(b.origin).with_scale(Vec3::new(r, fade, r)),
            1 | 2 => {
                let delayed = (t - if stroke.1 == 1 { 0.04 } else { 0.14 }).max(0.);
                let size = ((delayed * 3.).min(1.) * (1. + b.strength * 2.2)).max(0.001);
                Transform::from_translation(
                    b.origin + Vec3::Y * (0.9 + t * 1.5) + b.direction * t * 1.8,
                )
                .with_rotation(
                    Quat::from_rotation_y(yaw) * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                )
                .with_scale(Vec3::new(size, fade, size))
            }
            part => {
                let a = part as f32 * 2.399_963;
                let d = Vec3::new(a.cos(), 0.15 + (part % 3) as f32 * 0.30, a.sin());
                Transform::from_translation(b.origin + d * r * 0.75 + Vec3::Y * 0.35)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize()))
                    .with_scale(Vec3::new(
                        0.045 * fade,
                        (0.8 + b.strength * 2.) * fade,
                        0.025 * fade,
                    ))
            }
        };
    }
}
// Sample complete joint positions after animation and transform propagation.
// Echoes remain frozen at those positions, including the real head orientation.
#[allow(clippy::too_many_arguments)]
fn echoes(
    session: Res<Session>,
    time: Res<Time>,
    kit: Res<Kit>,
    mut ink: ResMut<Ink>,
    mut audit: ResMut<Audit>,
    rigs: Query<&Rig>,
    globals: Query<&GlobalTransform>,
    mut parts: Query<(&EchoPart, &mut Transform, &mut Visibility)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = if session.menu {
        0.
    } else if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else if session.battle.finished {
        time.delta_secs().min(0.1)
    } else {
        0.
    };
    for echo in &mut ink.echoes {
        echo.life = (echo.life - dt).max(0.);
    }
    let now = ink.clock;
    for rig in &rigs {
        if dt == 0.
            || rig.id >= session.battle.units.len()
            || rig.female != female(&session, rig.id)
        {
            continue;
        }
        let u = &session.battle.units[rig.id];
        let airborne =
            phalanx::ninjutsu::visual_height(u, session.battle.arena, session.battle.tick as f32)
                > 0.35;
        let burst = u
            .theus
            .as_ref()
            .is_some_and(|a| a.burst_until > session.battle.tick);
        if u.health <= 0. || !(airborne || burst) {
            continue;
        }
        let Some(pose) = rig
            .bones
            .iter()
            .map(|e| globals.get(*e).ok().map(|g| g.compute_transform()))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let root = pose[0].translation;
        if ink
            .sampled
            .get(&rig.id)
            .is_some_and(|(last, p)| now - *last < 0.085 || p.distance(root) < 0.075)
        {
            continue;
        }
        let mut echo = Echo {
            life: 0.36,
            color: session.campaign.teams[u.team].color,
            lost: u.lost_limbs & !u.prosthetics,
            ..default()
        };
        for (part, (a, b, radius, _)) in SEGMENTS.into_iter().enumerate() {
            echo.pose[part] = if a == b {
                Transform::from_translation(pose[a].translation + pose[a].rotation * Vec3::Y * 0.12)
                    .with_rotation(pose[a].rotation)
                    .with_scale(Vec3::new(0.29, 0.31, 0.24))
            } else {
                let d = pose[b].translation - pose[a].translation;
                Transform::from_translation((pose[a].translation + pose[b].translation) * 0.5)
                    .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize_or(Vec3::Y)))
                    .with_scale(Vec3::new(radius, d.length() * 0.58, radius))
            };
        }
        let slot = ink.next_echo % ECHOES;
        ink.next_echo += 1;
        ink.echoes[slot] = echo;
        ink.sampled.insert(rig.id, (now, root));
        audit.echoes += 1;
    }
    for (index, echo) in ink.echoes.iter().enumerate() {
        if let Some(mut material) = materials.get_mut(&kit.echoes[index]) {
            let [r, g, b] = echo.color;
            material.base_color =
                Color::srgba(r * 0.7, g * 0.7, b * 0.7, (echo.life / 0.36) * 0.35);
        }
    }
    for (part, mut transform, mut visible) in &mut parts {
        let echo = &ink.echoes[part.0];
        *visible = if echo.life > 0. && echo.lost & SEGMENTS[part.1].3 == 0 && !session.menu {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        *transform = echo.pose[part.1];
    }
    audit.peak_echoes = audit
        .peak_echoes
        .max(ink.echoes.iter().filter(|e| e.life > 0.).count());
}

// Only the automatic cinema uses these choices. Manual/tactical camera stays in the patron's hands.
pub(crate) fn impact_focus(session: &Session) -> Option<(Vec3, f32)> {
    let hit =
        session.battle.damage_events.iter().rev().find(|h| {
            session.battle.tick.saturating_sub(h.tick) < 9 && severity(session, h) >= 0.6
        })?;
    let age = session.battle.tick.saturating_sub(hit.tick) as f32 * phalanx::DT;
    let p = hit.position;
    let lift = phalanx::ninjutsu::visual_height(
        &session.battle.units[hit.defender],
        session.battle.arena,
        session.battle.tick as f32,
    );
    // Compress into contact, then pull out to read the expanding ink and consequences.
    Some((
        Vec3::new(
            p[0],
            session.battle.arena.elevation(p) + 0.85 + lift * 0.6,
            p[1],
        ),
        5.2 + age * 7.0,
    ))
}
pub(crate) fn camera_bank(session: &Session) -> f32 {
    let height = session
        .battle
        .units
        .iter()
        .filter(|u| u.health > 0.)
        .map(|u| {
            phalanx::ninjutsu::visual_height(u, session.battle.arena, session.battle.tick as f32)
        })
        .fold(0., f32::max);
    (height * 0.025).min(0.09)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_actual_body_exports_supply_every_echo_joint_without_name_collisions() {
        for body in ["male", "female"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("assets/characters/{body}.glb"));
            let bytes = std::fs::read(path).unwrap();
            let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
            let document: serde_json::Value =
                serde_json::from_slice(&bytes[20..20 + length]).unwrap();
            let names: Vec<_> = document["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|n| n["name"].as_str())
                .map(bone_name)
                .collect();
            for bone in BONES {
                assert_eq!(
                    names.iter().filter(|name| **name == bone).count(),
                    1,
                    "{body}: {bone}"
                );
            }
        }
    }
    fn real_impact() -> Session {
        use phalanx::{FighterClass, learned::Controller, progression::Campaign};
        let mut campaign = Campaign::new();
        for team in &mut campaign.teams {
            for f in &mut team.fighters {
                f.class = FighterClass::Warrior;
                f.mind = phalanx::styles::BattleStyle::Vanguard.mind();
            }
        }
        let (_, mut battle) = campaign
            .begin_battle(42, Arena::OpenField, Controller::Utility)
            .unwrap();
        while !battle.finished {
            battle.step();
            if battle
                .damage_events
                .iter()
                .any(|h| h.amount / battle.units[h.defender].max_health() >= 0.06)
            {
                break;
            }
        }
        let mut session = Session::new(phalanx::Battle::new(42, false));
        assert!(
            !battle.finished,
            "fixture must stop at a live actual heavy hit"
        );
        session.autosave = false;
        session.battle = battle;
        session.campaign = campaign;
        session.menu = false;
        assert!(
            session
                .battle
                .damage_events
                .iter()
                .any(|h| severity(&session, h) > 0.)
        );
        session
    }
    #[test]
    fn missed_and_ranged_hits_cannot_create_heavy_melee_beats() {
        let mut session = real_impact();
        let hit = session
            .battle
            .damage_events
            .iter()
            .find(|h| severity(&session, h) > 0.)
            .unwrap()
            .clone();
        for e in &mut session.battle.theus_events {
            e.ranged = true;
        }
        assert_eq!(severity(&session, &hit), 0.);
        session.battle.theus_events.clear();
        assert_eq!(severity(&session, &hit), 0.);
    }
    #[test]
    fn ink_consumes_real_hits_once_and_pause_preserves_battle_and_effect_age() {
        let session = real_impact();
        let before = serde_json::to_string(&session.battle).unwrap();
        let mut app = App::new();
        app.insert_resource(session)
            .insert_resource(Time::<()>::default())
            .init_resource::<Ink>()
            .init_resource::<Audit>()
            .insert_resource(Kit {
                ring: default(),
                shard: default(),
                ivory: default(),
                ink: default(),
                red: default(),
                echoes: Vec::new(),
            })
            .add_systems(Update, impacts);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));
        app.update();
        assert_eq!(app.world().resource::<Audit>().impacts, 1);
        app.update();
        assert_eq!(app.world().resource::<Audit>().impacts, 1);
        assert_eq!(app.world().resource::<Ink>().blasts[0].age, 0.);
        assert_eq!(
            serde_json::to_string(&app.world().resource::<Session>().battle).unwrap(),
            before
        );
        app.world_mut().resource_mut::<Session>().running = true;
        app.update();
        assert!(app.world().resource::<Ink>().blasts[0].age > 0.);
        assert_eq!(
            serde_json::to_string(&app.world().resource::<Session>().battle).unwrap(),
            before
        );
    }
    #[test]
    fn graphic_shockwaves_are_bounded_and_finish_without_persistent_geometry() {
        for strength in [0.4, 1., 1.6] {
            for i in 0..106 {
                let b = Blast {
                    strength,
                    age: i as f32 * 0.01,
                    ..default()
                };
                assert!(radius(&b).is_finite());
                assert!((0.0..=11.51).contains(&radius(&b)));
            }
        }
        let ink = Ink::default();
        assert_eq!(ink.echoes.len(), ECHOES);
        assert_eq!(ink.blasts.len(), BLASTS);
        assert!(brush().count_vertices() > 300);
    }
}
