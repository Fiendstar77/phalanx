//! Bounded, persistent organ meshes emitted by real fatal damage events.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
};
#[derive(Component)]
struct Chunk {
    velocity: Vec3,
    spin: Vec3,
    settled: bool,
}
#[derive(Resource)]
struct Assets {
    meshes: [Handle<Mesh>; 3],
    materials: [Handle<StandardMaterial>; 3],
}
#[derive(Resource, Default)]
pub(crate) struct Audit {
    run: Option<u32>,
    seen: Vec<bool>,
    pub(crate) emitted: [u32; 3],
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Audit>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (emit, physics).chain().after(super::super::sync_pawns),
        );
}
fn guts() -> Mesh {
    let mut p = Vec::new();
    let mut indices = Vec::new();
    let count = 96;
    let sides = 8;
    for i in 0..count {
        let t = i as f32 / (count - 1) as f32;
        let a = t * std::f32::consts::TAU * 3.2;
        let center = Vec3::new(
            a.sin() * (0.17 + 0.08 * t),
            0.04 * (a * 0.7).cos(),
            (a * 0.75).cos() * 0.15,
        );
        for j in 0..sides {
            let b = j as f32 / sides as f32 * std::f32::consts::TAU;
            p.push((center + Vec3::new(b.cos() * 0.035, b.sin() * 0.035, 0.)).to_array());
        }
        if i + 1 < count {
            for j in 0..sides {
                let a = (i * sides + j) as u32;
                let b = (i * sides + (j + 1) % sides) as u32;
                let n = sides as u32;
                indices.extend([a, b, a + n, b, b + n, a + n]);
            }
        }
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    m.insert_indices(Indices::U32(indices));
    m.compute_smooth_normals();
    m
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<bevy::asset::Assets<Mesh>>,
    mut materials: ResMut<bevy::asset::Assets<StandardMaterial>>,
) {
    let sphere = Sphere::new(1.).mesh().uv(20, 14);
    commands.insert_resource(Assets {
        meshes: [
            meshes.add(sphere.clone()),
            meshes.add(sphere),
            meshes.add(guts()),
        ],
        materials: [
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.34, 0.012, 0.027),
                perceptual_roughness: 0.18,
                reflectance: 0.5,
                ..default()
            }),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.018, 0.034),
                perceptual_roughness: 0.20,
                reflectance: 0.5,
                ..default()
            }),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.56, 0.16, 0.18),
                perceptual_roughness: 0.23,
                reflectance: 0.45,
                ..default()
            }),
        ],
    });
}
fn emit(
    mut commands: Commands,
    session: Res<Session>,
    assets: Res<Assets>,
    mut audit: ResMut<Audit>,
    chunks: Query<Entity, With<Chunk>>,
) {
    if audit.run != Some(session.run_id) {
        audit.run = Some(session.run_id);
        audit.seen = vec![false; session.battle.units.len()];
        audit.emitted = [0; 3];
        for entity in &chunks {
            commands.entity(entity).despawn();
        }
    }
    let mut count = chunks.iter().count();
    for event in &session.battle.damage_events {
        let id = event.defender;
        if audit.seen[id] || session.battle.units[id].health > 0. || count >= 64 {
            continue;
        }
        audit.seen[id] = true;
        for kind in 0..3 {
            let bits = phalanx::procedural::hash(
                session.battle.seed ^ event.sequence ^ (kind as u64 * 131),
            );
            let angle = (bits & 65535) as f32 / 65535. * std::f32::consts::TAU;
            let p = session.presented(id);
            let scale = [
                Vec3::new(0.14, 0.19, 0.12),
                Vec3::new(0.25, 0.08, 0.14),
                Vec3::ONE,
            ][kind];
            commands.spawn((
                Chunk {
                    velocity: Vec3::new(
                        angle.cos() * 2.1 + event.velocity[0] * 0.12,
                        2.2 + kind as f32 * 0.5,
                        angle.sin() * 2.1 + event.velocity[1] * 0.12,
                    ),
                    spin: Vec3::new(3.4, 1.7, 4.2),
                    settled: false,
                },
                Mesh3d(assets.meshes[kind].clone()),
                MeshMaterial3d(assets.materials[kind].clone()),
                Transform::from_xyz(p[0], session.battle.arena.elevation(p) + 0.7, p[1])
                    .with_scale(scale),
            ));
            audit.emitted[kind] += 1;
            count += 1;
        }
    }
}
fn physics(
    session: Res<Session>,
    time: Res<Time>,
    mut chunks: Query<(&mut Chunk, &mut Transform)>,
) {
    let dt = if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else if session.battle.finished && !session.menu {
        time.delta_secs().min(0.1)
    } else {
        0.
    };
    for (mut chunk, mut transform) in &mut chunks {
        if chunk.settled {
            continue;
        }
        let steps = (dt / 0.016).ceil().max(1.) as usize;
        let dt = dt / steps as f32;
        for _ in 0..steps {
            chunk.velocity.y -= 9.81 * dt;
            let next = transform.translation + chunk.velocity * dt;
            if session.battle.arena.blocked([next.x, next.z]) {
                chunk.velocity.x *= -0.2;
                chunk.velocity.z *= -0.2;
            } else {
                transform.translation = next;
            }
            transform.rotate(Quat::from_scaled_axis(chunk.spin * dt));
            let floor = session
                .battle
                .arena
                .elevation([transform.translation.x, transform.translation.z])
                + 0.065;
            if transform.translation.y < floor {
                transform.translation.y = floor;
                chunk.velocity.y = chunk.velocity.y.abs() * 0.16;
                chunk.velocity.x *= 0.55;
                chunk.velocity.z *= 0.55;
                chunk.spin *= 0.5;
                if chunk.velocity.length() < 0.45 {
                    chunk.settled = true;
                }
            }
        }
    }
}
