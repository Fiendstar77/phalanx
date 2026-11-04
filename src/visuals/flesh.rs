//! Bounded layered lesions/scars attached to actual skin joints and injury records.
//! This is surface presentation, not a tissue-volume simulation.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues, skinning::SkinnedMesh},
};
use phalanx::injuries::Region;
#[derive(Resource)]
pub(super) struct Surfaces {
    open: Handle<Mesh>,
    scar: Handle<Mesh>,
    wet: Handle<StandardMaterial>,
    healed: Handle<StandardMaterial>,
}
#[derive(Component)]
struct Mark {
    id: usize,
    region: Region,
    rest: Transform,
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, update.after(super::super::sync_pawns));
}
fn lesion(scar: bool) -> Mesh {
    let count = 28;
    let mut points = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for ring in 0..2 {
        for i in 0..count {
            let angle = i as f32 / count as f32 * std::f32::consts::TAU;
            let ragged = 1. + (i % 3) as f32 * 0.10;
            let x = if scar {
                0.006
            } else if ring == 0 {
                0.065
            } else {
                0.025
            };
            points.push([
                angle.cos() * x * ragged,
                angle.sin() * if ring == 0 { 0.13 } else { 0.10 },
                if ring == 0 { 0.012 } else { -0.002 },
            ]);
            colors.push(if scar {
                [0.68, 0.36, 0.35, 1.]
            } else if ring == 0 {
                [0.21, 0.008, 0.018, 1.]
            } else {
                [0.76, 0.09, 0.15, 1.]
            });
        }
    }
    points.push([0., 0., -0.008]);
    colors.push([0.16, 0.003, 0.008, 1.]);
    for i in 0..count {
        let next = (i + 1) % count;
        indices.extend_from_slice(&[
            i as u32,
            next as u32,
            (i + count) as u32,
            next as u32,
            (next + count) as u32,
            (i + count) as u32,
            (i + count) as u32,
            (next + count) as u32,
            (2 * count) as u32,
        ]);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    mesh
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(Surfaces {
        open: meshes.add(lesion(false)),
        scar: meshes.add(lesion(true)),
        wet: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.28,
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
        healed: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.85,
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
    });
}
fn model_matrix(
    entity: Entity,
    root: Entity,
    locals: &Query<&Transform>,
    parents: &Query<&ChildOf>,
) -> Mat4 {
    let mut chain = Vec::new();
    let mut cursor = entity;
    while cursor != root && chain.len() < 40 {
        if let Ok(t) = locals.get(cursor) {
            chain.push(t.to_matrix());
        }
        let Ok(parent) = parents.get(cursor) else {
            break;
        };
        cursor = parent.parent();
    }
    chain.into_iter().rev().fold(Mat4::IDENTITY, |a, b| a * b)
}
#[allow(clippy::too_many_arguments)]
pub(super) fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    models: Query<&Model>,
    children: Query<&Children>,
    primitives: Query<(&Mesh3d, &SkinnedMesh)>,
    locals: Query<&Transform>,
    parents: Query<&ChildOf>,
    meshes: Res<Assets<Mesh>>,
    surfaces: Res<Surfaces>,
) {
    let Ok(model) = models.get(event.entity) else {
        return;
    };
    for primitive in children.iter_descendants(event.entity) {
        let Ok((handle, skin)) = primitives.get(primitive) else {
            continue;
        };
        let Some(mesh) = meshes.get(&handle.0) else {
            continue;
        };
        let Some(VertexAttributeValues::Float32x3(points)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            continue;
        };
        let Some(VertexAttributeValues::Float32x4(weights)) =
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        else {
            continue;
        };
        let Some(VertexAttributeValues::Uint16x4(joints)) =
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
        else {
            continue;
        };
        let mesh_pose = model_matrix(primitive, event.entity, &locals, &parents);
        for region in [
            Region::Torso,
            Region::LeftArm,
            Region::RightArm,
            Region::LeftLeg,
            Region::RightLeg,
        ] {
            let desired = match region {
                Region::Torso => Vec3::new(0., 1.02, 0.20),
                Region::LeftArm => Vec3::new(0.40, 1.04, 0.12),
                Region::RightArm => Vec3::new(-0.40, 1.04, 0.12),
                Region::LeftLeg => Vec3::new(0.13, 0.40, 0.13),
                Region::RightLeg => Vec3::new(-0.13, 0.40, 0.13),
            };
            let Some((index, p)) = points
                .iter()
                .enumerate()
                .filter(|(_, p)| super::body::region(Vec3::from_array(**p)) == region.limb())
                .min_by(|(_, a), (_, b)| {
                    Vec3::from_array(**a)
                        .distance_squared(desired)
                        .total_cmp(&Vec3::from_array(**b).distance_squared(desired))
                })
            else {
                continue;
            };
            let dominant = (0..4)
                .max_by(|a, b| weights[index][*a].total_cmp(&weights[index][*b]))
                .unwrap();
            let Some(&joint) = skin.joints.get(joints[index][dominant] as usize) else {
                continue;
            };
            let bone = model_matrix(joint, event.entity, &locals, &parents);
            let normal = mesh_pose
                .inverse()
                .transpose()
                .transform_vector3(Vec3::from_array(normals[index]))
                .normalize_or(Vec3::Z);
            // Clear the fitted clothing shell (up to 0.024m) as well as skin.
            // This is a readable surface lesion, not a simulated cloth tear.
            let at = mesh_pose.transform_point3(Vec3::from_array(*p)) + normal * 0.045;
            let local_normal = bone
                .transpose()
                .transform_vector3(normal)
                .normalize_or(Vec3::Z);
            let rest = Transform::from_translation(bone.inverse().transform_point3(at))
                .with_rotation(Quat::from_rotation_arc(Vec3::Z, local_normal));
            commands.entity(joint).with_children(|p| {
                p.spawn((
                    Mark {
                        id: model.id,
                        region,
                        rest,
                    },
                    Mesh3d(surfaces.open.clone()),
                    MeshMaterial3d(surfaces.wet.clone()),
                    rest,
                    Visibility::Hidden,
                ));
            });
        }
        break;
    }
}
#[allow(clippy::type_complexity)]
fn update(
    session: Res<Session>,
    surfaces: Res<Surfaces>,
    mut marks: Query<(
        &Mark,
        &mut Transform,
        &mut Visibility,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    mut pawns: Query<(&Pawn, &mut Visibility), Without<Mark>>,
) {
    if std::env::args().any(|a| a == "--flesh-witness") {
        // Isolate the subject for art inspection without moving or altering
        // any simulated fighter or the hit's original receipt.
        for (pawn, mut visible) in &mut pawns {
            *visible = if pawn.0 == session.selected {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    for (mark, mut transform, mut visible, mut mesh, mut material) in &mut marks {
        let Some(unit) = session.visual_unit(mark.id) else {
            *visible = Visibility::Hidden;
            continue;
        };
        let wound = unit
            .flesh
            .marks
            .iter()
            .filter(|w| w.region == mark.region)
            .max_by_key(|w| (w.recovery_bouts > 0, w.severity));
        let Some(wound) = wound.filter(|_| unit.lost_limbs & mark.region.limb() == 0) else {
            *visible = Visibility::Hidden;
            continue;
        };
        *visible = Visibility::Inherited;
        *transform = mark.rest;
        transform.scale = Vec3::splat(0.7 + f32::from(wound.severity) * 0.20);
        if wound.recovery_bouts > 0 {
            mesh.0 = surfaces.open.clone();
            material.0 = surfaces.wet.clone();
        } else {
            mesh.0 = surfaces.scar.clone();
            material.0 = surfaces.healed.clone();
        }
    }
}
