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
    primitives: Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>)>,
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
fn pixel(
    image: &Image,
    index: usize,
    cols: usize,
    rows: usize,
    u: f32,
    v: f32,
    nose: bool,
) -> Option<Srgba> {
    if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
        return None;
    }
    let x = (index % cols) as f32 / cols as f32 + u / cols as f32;
    let y = if nose {
        let (a, b) = [(200., 395.), (465., 660.), (720., 905.), (970., 1160.)][index / 4];
        (a + v * (b - a)) / 1254.0
    } else if cols == 3 && rows == 5 {
        let (a, b) = [
            (75., 275.),
            (305., 525.),
            (560., 745.),
            (780., 980.),
            (1000., 1205.),
        ][index / 3];
        (a + v * (b - a)) / 1254.0
    } else {
        (index / cols) as f32 / rows as f32 + v / rows as f32
    };
    image
        .get_color_at(
            (x * image.width() as f32) as u32,
            (y * image.height() as f32) as u32,
        )
        .ok()
        .map(|c| c.to_srgba())
        .filter(|c| c.alpha > 0.5)
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

fn texture(
    a: phalanx::progression::Appearance,
    looks: &Looks,
    images: &Assets<Image>,
    expression: u8,
) -> Option<Image> {
    for (kind, choice) in [(1, a.face), (2, a.eyes), (3, a.nose)] {
        if choice > 0 && images.get(&looks.images[kind][0]).is_none() {
            return None;
        }
    }
    let size = if a.face == 0 && a.eyes == 0 && a.nose == 0 && expression == 0 {
        4
    } else {
        512
    };
    let mut data = Vec::with_capacity(size * size * 4);
    let skin = Srgba::new(1.0, 0.82, 0.69, 1.0);
    for y in 0..size {
        for x in 0..size {
            let px = (x as f32 / (size - 1) as f32 - 0.5) * 0.80;
            let py = 2.03 - y as f32 / (size - 1) as f32 * 0.80;
            let mut color = skin;
            // Compose each feature exactly once. Never project a complete illustrated
            // head beneath replacement eyes: that left two separate sets of eyes.
            let eye_u = px / 0.60 + 0.5;
            let eye_v = (1.64 - py) / 0.25;
            let eye_v = if expression == 2 {
                (eye_v - 0.6) * 1.45 + 0.6
            } else {
                eye_v
            };
            let mut layers = Vec::with_capacity(3);
            if a.eyes > 0 {
                layers.push(pixel(
                    images.get(&looks.images[2][0])?,
                    a.eyes as usize - 1,
                    3,
                    5,
                    eye_u,
                    eye_v,
                    false,
                ));
            } else if a.face > 0 && (0.0..1.0).contains(&eye_u) && (0.0..1.0).contains(&eye_v) {
                layers.push(pixel(
                    images.get(&looks.images[1][0])?,
                    a.face as usize - 1,
                    4,
                    4,
                    0.12 + eye_u * 0.76,
                    0.36 + eye_v * 0.43,
                    false,
                ));
            }
            // Mouth is independently cropped from the face sheet, below the eyes.
            let mouth_u = px / 0.12 + 0.5;
            let mouth_v = (1.34 - py) / 0.065;
            if a.face > 0 && (0.0..1.0).contains(&mouth_u) && (0.0..1.0).contains(&mouth_v) {
                layers.push(pixel(
                    images.get(&looks.images[1][0])?,
                    a.face as usize - 1,
                    4,
                    4,
                    0.43 + mouth_u * 0.14,
                    0.84 + mouth_v * 0.09,
                    false,
                ));
            }
            if a.nose > 0 {
                layers.push(pixel(
                    images.get(&looks.images[3][0])?,
                    a.nose as usize - 1,
                    4,
                    4,
                    px / 0.085 + 0.5,
                    (1.405 - py) / 0.07,
                    true,
                ));
            }
            for sample in layers.into_iter().flatten() {
                let f = sample.alpha;
                color = Srgba::new(
                    color.red * (1.0 - f) + sample.red * f,
                    color.green * (1.0 - f) + sample.green * f,
                    color.blue * (1.0 - f) + sample.blue * f,
                    1.0,
                );
            }
            // Simple painted expressions on the same material seen in the portrait.
            if expression > 0 {
                let mouth = match expression {
                    1 => (px / 0.045).powi(2) + ((py - 1.31) / 0.035).powi(2) < 1.0,
                    2 => px.abs() < 0.048 && (py - (1.31 + 0.16 * px)).abs() < 0.008,
                    _ => (px / 0.04).powi(2) + ((py - 1.315) / 0.045).powi(2) < 1.0,
                };
                // Erase the neutral mouth before drawing an open/shouting/grimacing one.
                if px.abs() < 0.065 && (1.26..1.35).contains(&py) {
                    color = skin;
                }
                if mouth {
                    color = Srgba::new(0.21, 0.025, 0.035, 1.0);
                }
                let brow = 1.645
                    + if expression == 1 {
                        0.15 * px.abs()
                    } else {
                        -0.12 * px.abs()
                    };
                if (0.07..0.24).contains(&px.abs()) && (py - brow).abs() < 0.008 {
                    color = Srgba::new(0.22, 0.12, 0.08, 1.0);
                }
            }
            data.extend(
                [color.red, color.green, color.blue, 1.0]
                    .map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8),
            );
        }
    }
    Some(Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}
#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    session: Res<Session>,
    looks: Res<Looks>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<PaintCache>,
    iron: Res<IronAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bodies: Query<(&mut Painted, &Mesh3d, &GlobalTransform)>,
    mut replacements: Query<(&Iron, &mut Visibility)>,
    mut commands: Commands,
    gibs: Query<Entity, With<Gib>>,
) {
    let mut active_gibs = gibs.iter().count();
    for (mut body, mesh, world) in &mut bodies {
        let Some(u) = session.battle.units.get(body.id) else {
            continue;
        };
        let a = look(&session, body.id);
        let expression = if u.health <= 0.0 {
            3
        } else if u.health_fraction() < 0.3 {
            2
        } else if u.action == 1 && u.cooldown > 0.4 {
            1
        } else {
            0
        };
        let choice = [a.face, a.eyes, a.nose, expression];
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
                let Some(image) = texture(a, &looks, &images, expression) else {
                    continue;
                };
                if cache.0.len() >= 128
                    && let Some(key) = cache.0.keys().next().copied()
                {
                    cache.0.remove(&key);
                }
                cache.0.insert(choice, images.add(image));
            }
            if let Some(mut material) = materials.get_mut(&body.material) {
                material.base_color_texture = Some(cache.0[&choice].clone());
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
                    let speed = if bit == 16 { 4.0 } else { 2.7 };
                    commands.spawn((
                        Gib {
                            velocity: Vec3::new(
                                angle.cos() * speed,
                                3.0 + bit as f32 * 0.06,
                                angle.sin() * speed,
                            ),
                            spin: Vec3::new(4., 2., 3.),
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
    for (replacement, mut visible) in &mut replacements {
        *visible = if session
            .battle
            .units
            .get(replacement.id)
            .is_some_and(|u| u.prosthetics & replacement.bit != 0)
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
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
