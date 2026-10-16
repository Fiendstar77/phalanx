//! Surface paint and detachable pieces use the real character meshes.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, VertexAttributeValues},
    render::render_resource::{Extent3d, TextureDimension},
};
use std::collections::BTreeMap;

#[derive(Component)]
pub(super) struct Painted {
    id: usize,
    female: bool,
    source: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    choice: Option<[u8; 4]>,
    mask: u8,
    seed: u64,
    tick: u32,
}
#[derive(Component)]
pub(super) struct Iron {
    id: usize,
    bit: u8,
}
#[derive(Resource)]
pub(super) struct IronAssets {
    bar: Handle<Mesh>,
    joint: Handle<Mesh>,
    metal: Handle<StandardMaterial>,
    wood: Handle<StandardMaterial>,
    wound: Handle<StandardMaterial>,
}
#[derive(Component)]
pub(super) struct Gib {
    velocity: Vec3,
    spin: Vec3,
    life: f32,
    run: u32,
    settled: bool,
}
#[derive(Resource, Default)]
pub(super) struct PaintCache(BTreeMap<[u8; 4], Handle<Image>>);
pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.init_resource::<PaintCache>();
    commands.insert_resource(IronAssets {
        bar: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        joint: meshes.add(Sphere::new(1.0).mesh().uv(12, 8)),
        metal: materials.add(StandardMaterial {
            base_color: Color::srgb(0.20, 0.24, 0.27),
            metallic: 0.9,
            perceptual_roughness: 0.28,
            ..default()
        }),
        wood: materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.28, 0.14),
            metallic: 0.0,
            perceptual_roughness: 0.86,
            ..default()
        }),
        wound: materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.012, 0.027),
            perceptual_roughness: 0.25,
            ..default()
        }),
    });
}
#[allow(clippy::too_many_arguments)]
pub(super) fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    models: Query<&Model>,
    primitives: Query<
        (&Mesh3d, &MeshMaterial3d<StandardMaterial>),
        With<bevy::mesh::skinning::SkinnedMesh>,
    >,
    names: Query<&Name>,
    transforms: Query<&Transform>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    iron: Res<IronAssets>,
    session: Res<Session>,
) {
    let Ok(model) = models.get(event.entity) else {
        return;
    };
    for entity in children.iter_descendants(event.entity) {
        if let Ok((mesh, material)) = primitives.get(entity) {
            let Some(source) = meshes.get(&mesh.0).cloned() else {
                continue;
            };
            let instance = meshes.add(source);
            let mut mat = materials.get(&material.0).cloned().unwrap_or_default();
            mat.base_color = Color::WHITE;
            mat.base_color_texture = None;
            mat.normal_map_texture = None;
            mat.occlusion_texture = None;
            mat.metallic_roughness_texture = None;
            mat.perceptual_roughness = 0.7;
            let material_handle = materials.add(mat);
            commands.entity(entity).insert((
                Mesh3d(instance),
                MeshMaterial3d(material_handle.clone()),
                Painted {
                    id: model.id,
                    female: model.female,
                    source: mesh.0.clone(),
                    material: material_handle,
                    choice: None,
                    mask: 0,
                    seed: session.battle.seed,
                    tick: session.battle.tick,
                },
            ));
        }
        let Ok(name) = names.get(entity) else {
            continue;
        };
        let bit = match name.as_str() {
            "mixamorigLeftArm" | "mixamorigLeftForeArm" => 1,
            "mixamorigRightArm" | "mixamorigRightForeArm" => 2,
            "mixamorigLeftUpLeg" | "mixamorigLeftLeg" => 4,
            "mixamorigRightUpLeg" | "mixamorigRightLeg" => 8,
            _ => continue,
        };
        let endpoint = children
            .get(entity)
            .ok()
            .and_then(|kids| {
                kids.iter()
                    .find_map(|child| transforms.get(child).ok().map(|t| t.translation))
            })
            .unwrap_or(Vec3::Y * 0.25);
        let length = endpoint.length().max(0.08);
        commands.entity(entity).with_children(|p| {
            p.spawn((
                Iron { id: model.id, bit },
                Mesh3d(iron.bar.clone()),
                MeshMaterial3d(iron.metal.clone()),
                Transform::from_translation(endpoint * 0.48)
                    .with_rotation(Quat::from_rotation_arc(
                        Vec3::Y,
                        endpoint.normalize_or_zero(),
                    ))
                    .with_scale(Vec3::new(0.11, length, 0.11)),
                Visibility::Hidden,
            ));
            p.spawn((
                Iron { id: model.id, bit },
                Mesh3d(iron.joint.clone()),
                MeshMaterial3d(iron.metal.clone()),
                Transform::from_translation(endpoint).with_scale(Vec3::splat(0.065)),
                Visibility::Hidden,
            ));
        });
    }
}
fn region(p: Vec3) -> u8 {
    if p.y > 1.26 {
        16
    } else if p.y > 0.82 && p.x > 0.23 {
        1
    } else if p.y > 0.82 && p.x < -0.23 {
        2
    } else if p.y < 0.66 && p.x > 0.0 {
        4
    } else if p.y < 0.66 {
        8
    } else {
        0
    }
}
fn selected_indices(mesh: &Mesh, mask: u8, keep: bool) -> Vec<u32> {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return vec![];
    };
    let indices: Vec<_> = mesh
        .indices()
        .map(|i| i.iter().map(|v| v as u32).collect())
        .unwrap_or_else(|| (0..positions.len() as u32).collect());
    indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|face| {
            let c = (Vec3::from_array(positions[face[0] as usize])
                + Vec3::from_array(positions[face[1] as usize])
                + Vec3::from_array(positions[face[2] as usize]))
                / 3.0;
            (region(c) & mask != 0) == keep
        })
        .flatten()
        .copied()
        .collect()
}
fn paint_uv(mesh: &mut Mesh) {
    // UV discontinuities must be per triangle: shared seam vertices previously
    // interpolated from a facial UV to the skin swatch across an entire cheek.
    mesh.duplicate_vertices();
    let Some(VertexAttributeValues::Float32x3(points)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    let mut uv = Vec::with_capacity(points.len());
    for triangle in points.as_chunks::<3>().0 {
        let center = (Vec3::from_array(triangle[0])
            + Vec3::from_array(triangle[1])
            + Vec3::from_array(triangle[2]))
            / 3.0;
        let front = center.y > 1.23 && center.z > 0.04;
        for p in triangle {
            uv.push(if front {
                [
                    (p[0] / 0.80 + 0.5).clamp(0.001, 0.999),
                    ((2.03 - p[1]) / 0.80).clamp(0.001, 0.999),
                ]
            } else {
                [0.001, 0.001]
            });
        }
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
}

// The new sculpt supplies the face. Do not remap old portrait sheets or source
// normal maps onto its UVs; a mismatched normal map made the face look torn.
fn texture() -> Image {
    Image::new(
        Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        [255, 255, 255, 255].repeat(16),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    session: Res<Session>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<PaintCache>,
    iron: Res<IronAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bodies: Query<(&mut Painted, &Mesh3d, &GlobalTransform)>,
    mut replacements: Query<(
        &Iron,
        &mut Visibility,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    mut commands: Commands,
    gibs: Query<Entity, With<Gib>>,
) {
    let mut active_gibs = gibs.iter().count();
    for (mut body, mesh, world) in &mut bodies {
        let Some(u) = session.battle.units.get(body.id) else {
            continue;
        };
        // Expressions live on the new socketed head, never on the body texture.
        let choice = [look(&session, body.id).skin, 0, 0, 0];
        let reset = body.seed != session.battle.seed || session.battle.tick < body.tick;
        if reset {
            body.mask = 0;
            body.choice = None;
        }
        // Nonfatal limb wounds persist; fatal dismemberment is cosmetic only.
        let mask = u.lost_limbs
            | if u.health <= 0.0 {
                16 | if u.id % 2 == 0 { 3 } else { 12 }
            } else {
                0
            };
        if body.choice != Some(choice) || body.mask != mask {
            let Some(mut painted) = meshes.get(&body.source).cloned() else {
                continue;
            };
            if !cache.0.contains_key(&choice) {
                let image = texture();
                if cache.0.len() >= 128
                    && let Some(key) = cache.0.keys().next().copied()
                {
                    cache.0.remove(&key);
                }
                cache.0.insert(choice, images.add(image));
            }
            if let Some(mut material) = materials.get_mut(&body.material) {
                material.base_color_texture = Some(cache.0[&choice].clone());
                material.base_color = skin_color(&session, body.id);
            }
            paint_uv(&mut painted);
            let detached = mask & !body.mask;
            if detached != 0
                && !reset
                && body.choice.is_some()
                && body.female == female(&session, body.id)
            {
                for bit in [1, 2, 4, 8, 16] {
                    if detached & bit == 0 || active_gibs >= 128 {
                        continue;
                    }
                    let selected = selected_indices(&painted, bit, true);
                    if selected.is_empty() {
                        continue;
                    }
                    let mut piece = painted.clone();
                    piece.insert_indices(Indices::U32(selected));
                    piece.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
                    piece.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
                    let center = match bit {
                        1 => Vec3::new(0.59, 1.10, 0.),
                        2 => Vec3::new(-0.59, 1.10, 0.),
                        4 => Vec3::new(0.20, 0.32, 0.),
                        8 => Vec3::new(-0.20, 0.32, 0.),
                        _ => Vec3::new(0., 1.61, 0.),
                    };
                    if let Some(VertexAttributeValues::Float32x3(points)) =
                        piece.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                    {
                        for p in points {
                            *p = (Vec3::from_array(*p) - center).to_array();
                        }
                    }
                    let transform = world.compute_transform();
                    let origin = world.transform_point(center);
                    let angle = u.id as f32 * 2.1 + bit as f32 * 0.7;
                    let speed = if bit == 16 { 7.5 } else { 5.2 };
                    commands.spawn((
                        Gib {
                            velocity: Vec3::new(
                                angle.cos() * speed,
                                5.6 + bit as f32 * 0.08,
                                angle.sin() * speed,
                            ),
                            spin: Vec3::new(7., 4., 5.),
                            life: 180.0,
                            run: session.run_id,
                            settled: false,
                        },
                        Mesh3d(meshes.add(piece)),
                        MeshMaterial3d(iron.wound.clone()),
                        Transform::from_translation(origin)
                            .with_rotation(transform.rotation)
                            .with_scale(transform.scale),
                    ));
                    active_gibs += 1;
                }
            }
            painted.insert_indices(Indices::U32(selected_indices(&painted, mask, false)));
            if let Some(mut target) = meshes.get_mut(&mesh.0) {
                *target = painted;
            }
            body.choice = Some(choice);
            body.mask = mask;
        }
        body.seed = session.battle.seed;
        body.tick = session.battle.tick;
    }
    for (replacement, mut visible, mut material) in &mut replacements {
        let stand_in = session
            .battle
            .units
            .get(replacement.id)
            .map(|u| {
                phalanx::progression::limb_stand_in(u.lost_limbs, u.prosthetics, replacement.bit)
            })
            .unwrap_or(phalanx::progression::LimbStandIn::None);
        match stand_in {
            phalanx::progression::LimbStandIn::Iron => {
                *visible = Visibility::Inherited;
                material.0 = iron.metal.clone();
            }
            phalanx::progression::LimbStandIn::Wood => {
                *visible = Visibility::Inherited;
                material.0 = iron.wood.clone();
            }
            phalanx::progression::LimbStandIn::None => *visible = Visibility::Hidden,
        }
    }
}
pub(super) fn physics(
    session: Res<Session>,
    time: Res<Time>,
    mut commands: Commands,
    mut gibs: Query<(Entity, &mut Gib, &mut Transform)>,
) {
    let dt = if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else if session.battle.finished {
        time.delta_secs().min(0.1)
    } else {
        0.0
    };
    for (entity, mut gib, mut transform) in &mut gibs {
        if gib.run != session.run_id || gib.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        gib.life -= dt;
        if !gib.settled {
            gib.velocity.y -= 7.0 * dt;
            transform.translation += gib.velocity * dt;
            transform.rotate(Quat::from_scaled_axis(gib.spin * dt));
            let floor = session
                .battle
                .arena
                .elevation([transform.translation.x, transform.translation.z])
                + 0.13;
            if transform.translation.y <= floor {
                transform.translation.y = floor;
                gib.velocity *= 0.22;
                gib.spin *= 0.35;
                if gib.velocity.length() < 0.8 {
                    gib.settled = true;
                }
            }
        }
    }
}
