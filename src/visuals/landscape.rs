//! Authored set dressing around the real, sampled Hollow Procession terrain.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
};
use phalanx::{pressure::PressureField, region};

pub(crate) fn ground(field: PressureField) -> Mesh {
    // Continuous land beyond the sealed boundary replaces the floating board edge.
    let n = 640u32;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uv = Vec::new();
    let mut indices = Vec::new();
    for z in 0..=n {
        for x in 0..=n {
            let p = [x as f32 * 0.25 - 80., z as f32 * 0.25 - 80.];
            positions.push([p[0], region::elevation(field, p), p[1]]);
            let dx = region::elevation(field, [p[0] + 0.1, p[1]])
                - region::elevation(field, [p[0] - 0.1, p[1]]);
            let dz = region::elevation(field, [p[0], p[1] + 0.1])
                - region::elevation(field, [p[0], p[1] - 0.1]);
            normals.push(Vec3::new(-dx, 0.2, -dz).normalize().to_array());
            uv.push([x as f32 / n as f32, z as f32 / n as f32]);
            if x < n && z < n {
                let a = z * (n + 1) + x;
                let b = a + n + 1;
                indices.extend([a, b, a + 1, a + 1, b, b + 1]);
            }
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(indices))
}
#[derive(Component)]
struct Land;
#[derive(Component)]
struct Reed {
    origin: Vec3,
    phase: f32,
}
#[derive(Resource)]
struct Materials {
    ivory: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    ink: Handle<StandardMaterial>,
    gold: Handle<StandardMaterial>,
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup_materials).add_systems(
        Update,
        (
            rebuild,
            sway.after(rebuild),
            paint.after(super::super::sync_terrain),
        ),
    );
    if std::env::args().any(|a| a == "--region-witness" || a == "--individual-witness") {
        app.add_systems(Update, witness.before(super::super::camera_motion));
    }
}
fn setup_materials(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mut make = |color, roughness, metallic| {
        materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: roughness,
            metallic,
            ..default()
        })
    };
    commands.insert_resource(Materials {
        ivory: make(Color::srgb(0.84, 0.79, 0.64), 0.72, 0.),
        red: make(Color::srgb(0.48, 0.035, 0.065), 0.33, 0.12),
        ink: make(Color::srgb(0.022, 0.047, 0.055), 0.78, 0.),
        gold: make(Color::srgb(0.71, 0.48, 0.20), 0.4, 0.55),
    });
}
fn rebuild(
    mut commands: Commands,
    session: Res<Session>,
    materials: Res<Materials>,
    mut meshes: ResMut<Assets<Mesh>>,
    old: Query<Entity, With<Land>>,
    mut key: Local<Option<PressureField>>,
    mut shared: Local<Option<(Handle<Mesh>, Handle<Mesh>)>>,
) {
    let field = match session.battle.arena {
        Arena::Pressure(f) if f.generator == region::GENERATOR => Some(f),
        _ => None,
    };
    if *key == field {
        return;
    }
    *key = field;
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(f) = field else {
        return;
    };
    let (cube, sphere) = shared
        .get_or_insert_with(|| {
            (
                meshes.add(Cuboid::new(1., 1., 1.)),
                meshes.add(Sphere::new(1.).mesh().uv(16, 10)),
            )
        })
        .clone();
    commands
        .spawn((Land, Transform::default(), Visibility::default()))
        .with_children(|root| {
            // Solid masonry follows the sampled ground along each actual wall volume.
            // The outer walls mark sealed gates, rather than an invisible map edge.
            for (p, size) in phalanx::procedural::original_walls(f).iter().skip(4) {
                let along_x = size[0] > size[1];
                let length = size[0].max(size[1]);
                let count = length.ceil() as usize;
                for j in 0..count {
                    let offset = (j as f32 + 0.5) / count as f32 * length - length / 2.;
                    let q = [
                        p[0] + if along_x { offset } else { 0. },
                        p[1] + if along_x { 0. } else { offset },
                    ];
                    root.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(materials.ink.clone()),
                        Transform::from_xyz(q[0], region::elevation(f, q) + 0.7, q[1]).with_scale(
                            if along_x {
                                Vec3::new(length / count as f32, 3.4, size[1])
                            } else {
                                Vec3::new(size[0], 3.4, length / count as f32)
                            },
                        ),
                    ));
                }
            }
            // Pale sediment teeth are confined to the four permanent collision walls.
            for (i, (p, size)) in phalanx::procedural::original_walls(f)
                .iter()
                .enumerate()
                .skip(4)
                .take(4)
            {
                let long_x = size[0] > size[1];
                for j in 0..9 {
                    let along =
                        (j as f32 / 8. - 0.5) * if long_x { size[0] - 0.7 } else { size[1] - 0.7 };
                    let q = [
                        p[0] + if long_x { along } else { 0. },
                        p[1] + if long_x { 0. } else { along },
                    ];
                    let h = 1.7 + 0.45 * (j as f32 * 1.7 + i as f32).sin().abs();
                    root.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(materials.ivory.clone()),
                        Transform::from_xyz(q[0], region::elevation(f, q) + h / 2., q[1])
                            .with_scale(if long_x {
                                Vec3::new(0.5, h, size[1] * 0.8)
                            } else {
                                Vec3::new(size[0] * 0.8, h, 0.5)
                            }),
                    ));
                }
            }
            // Bone reeds mark actual cover. Thin flexible stems do not pretend to be solid walls.
            for (i, a) in region::anchors().iter().enumerate() {
                for j in 0..24 {
                    let angle = j as f32 * 2.399_963;
                    let radius = (j as f32 / 24.).sqrt();
                    let p = [
                        a[0] + angle.cos() * 3.5 * radius,
                        a[1] + angle.sin() * 2.1 * radius,
                    ];
                    if session.battle.arena.blocked(p) {
                        continue;
                    }
                    let h = 1.1 + (angle * 2.).sin().abs() * 0.9;
                    let origin = Vec3::new(p[0], region::elevation(f, p), p[1]);
                    root.spawn((
                        Reed {
                            origin,
                            phase: angle + i as f32,
                        },
                        Transform::from_translation(origin),
                        Visibility::default(),
                    ))
                    .with_children(|r| {
                        r.spawn((
                            Mesh3d(cube.clone()),
                            MeshMaterial3d(materials.ink.clone()),
                            Transform::from_xyz(0., h / 2., 0.)
                                .with_scale(Vec3::new(0.045, h, 0.045)),
                        ));
                        r.spawn((
                            Mesh3d(sphere.clone()),
                            MeshMaterial3d(if j % 4 == 0 {
                                materials.red.clone()
                            } else {
                                materials.ivory.clone()
                            }),
                            Transform::from_xyz(0., h, 0.)
                                .with_scale(Vec3::new(0.09, 0.32, 0.09))
                                .with_rotation(Quat::from_rotation_z(0.25)),
                        ));
                    });
                }
            }
            // Forty-metre ribs grow outside the sealed location. Their arch crowns
            // cross far above all playable trajectories, leaving the road unobstructed.
            for i in 0..7 {
                let z = -44. + i as f32 * 14.;
                let height = 34. + (i as f32 * 1.9).sin() * 5.;
                for side in [-1., 1.] {
                    for j in 0..16 {
                        let t = j as f32 / 15.;
                        let angle = t * 1.25;
                        let pos = Vec3::new(
                            side * (54. - angle.sin() * 43.),
                            10. + angle.sin() * height,
                            z + (t * 3.).sin() * 2.,
                        );
                        root.spawn((
                            Mesh3d(sphere.clone()),
                            MeshMaterial3d(materials.ivory.clone()),
                            Transform::from_translation(pos)
                                .with_rotation(Quat::from_rotation_z(side * angle))
                                .with_scale(Vec3::new(1.1 + 0.6 * (1. - t), 3.1, 0.9)),
                        ));
                    }
                    root.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(materials.red.clone()),
                        Transform::from_xyz(side * 54., 17., z)
                            .with_scale(Vec3::new(0.7, 12., 2.4)),
                    ));
                }
                // Funeral bells; static supports and slow reeds provide scale without flashes.
                root.spawn((
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(materials.gold.clone()),
                    Transform::from_xyz(0., height + 8., z).with_scale(Vec3::new(2.2, 3.8, 2.2)),
                ));
                root.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(materials.ink.clone()),
                    Transform::from_xyz(0., height + 12., z).with_scale(Vec3::new(0.09, 9., 0.09)),
                ));
            }
        });
}
fn sway(
    session: Res<Session>,
    time: Res<Time>,
    mut seconds: Local<f32>,
    mut reeds: Query<(&Reed, &mut Transform)>,
) {
    if session.running {
        *seconds += time.delta_secs().min(0.1) * session.speed;
    }
    for (reed, mut t) in &mut reeds {
        *t = Transform::from_translation(reed.origin).with_rotation(Quat::from_rotation_z(
            (*seconds * 0.9 + reed.phase).sin() * 0.07,
        ));
    }
}

#[derive(Default)]
struct Witness {
    seconds: f32,
    stage: u8,
    paused: Option<String>,
}
fn witness(
    time: Res<Time>,
    mut state: Local<Witness>,
    mut session: ResMut<Session>,
    mut commands: Commands,
    audit: Res<super::balance::Audit>,
    faces: Res<super::face::Audit>,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    state.seconds += time.delta_secs().min(0.1);
    let stage = state.stage;
    session.cinema_grabbed = true;
    session.cinema_on = false;
    if (2..=5).contains(&stage) {
        session.camera_follow = true;
    }
    if stage == 0 {
        session.camera_follow = false;
        session.zoom = 96.;
        session.pitch = 0.72;
    }
    if stage == 2 || stage == 3 {
        session.zoom = 3.3;
        session.pitch = 0.18;
        session.orbit = 2.2;
    }
    if state.seconds < 4. + stage as f32 * 4. {
        return;
    }
    std::fs::create_dir_all("outputs/hollow-procession").unwrap();
    match stage {
        0 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("outputs/hollow-procession/overview.png"));
            session.running = true;
        }
        1 => {
            session.camera_follow = true;
            session.selected = 0;
            session.zoom = 4.5;
            session.pitch = 0.22;
            session.orbit = 2.2;
        }
        2 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("outputs/hollow-procession/scout.png"));
        }
        3 => {
            session.zoom = 18.;
            session.pitch = 0.36;
            session.orbit = 0.8;
        }
        4 => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk("outputs/hollow-procession/movement.png"));
        }
        5 => {
            session.running = false;
            state.paused = Some(serde_json::to_string(&session.battle).unwrap());
            std::fs::write("outputs/hollow-procession/witness.json",serde_json::to_vec_pretty(&serde_json::json!({"fixture":"isolated four-fighter campaign; no player save writes","tick":session.battle.tick,"plants":audit.plants,"max_correction_metres":audit.max_correction,"invalid_poses":audit.invalid,"arena":session.battle.arena,"expression_names":super::individual::EXPRESSIONS,"expression_variants":faces.variants,
                "hound_actors":session.battle.kennel.actors.len(),"hound_events":session.battle.kennel.events,"aerial_events":session.battle.aerial_events,
                "social_events":session.battle.social_events,"neural_inputs":phalanx::telemetry::FEATURE_NAMES.len()})).unwrap()).unwrap();
        }
        _ => {
            assert_eq!(
                state.paused.as_deref(),
                Some(serde_json::to_string(&session.battle).unwrap().as_str()),
                "paused presentation changed Battle"
            );
            exit.write(AppExit::Success);
        }
    }
    state.stage += 1;
}

/// Four-centimetre texels, bilinear magnification and a complete mip chain keep
/// pigment edges smooth at the feet and stable from the tabletop camera.
fn surface(field: PressureField) -> Image {
    use bevy::{
        image::{ImageSampler, ImageSamplerDescriptor},
        render::render_resource::{Extent3d, TextureDimension},
    };
    let size = 4096usize;
    let mut data = Vec::with_capacity(size * size * 4 * 4 / 3 + 16);
    for z in 0..size {
        for x in 0..size {
            let p = [
                (x as f32 + 0.5) / size as f32 * 160. - 80.,
                (z as f32 + 0.5) / size as f32 * 160. - 80.,
            ];
            let c = region::pigment(field, p);
            data.extend([
                (c[0] * 255.).round() as u8,
                (c[1] * 255.).round() as u8,
                (c[2] * 255.).round() as u8,
                255,
            ]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let linear: [f32; 256] = std::array::from_fn(|i| {
        let s = i as f32 / 255.;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    });
    let data = image.data.as_mut().unwrap();
    let mut width = size;
    let mut offset = 0;
    let mut levels = 1;
    while width > 1 {
        let next_offset = data.len();
        let next = width / 2;
        for z in 0..next {
            for x in 0..next {
                let ids = [
                    offset + ((z * 2) * width + x * 2) * 4,
                    offset + ((z * 2) * width + x * 2 + 1) * 4,
                    offset + ((z * 2 + 1) * width + x * 2) * 4,
                    offset + ((z * 2 + 1) * width + x * 2 + 1) * 4,
                ];
                let pixel: [u8; 4] = std::array::from_fn(|c| {
                    if c == 3 {
                        255
                    } else {
                        let l = ids
                            .iter()
                            .map(|i| linear[data[i + c] as usize])
                            .sum::<f32>()
                            * 0.25;
                        let s = if l <= 0.0031308 {
                            l * 12.92
                        } else {
                            1.055 * l.powf(1. / 2.4) - 0.055
                        };
                        (s * 255.).round() as u8
                    }
                });
                data.extend(pixel);
            }
        }
        width = next;
        offset = next_offset;
        levels += 1;
    }
    image.texture_descriptor.mip_level_count = levels;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        anisotropy_clamp: 8,
        ..ImageSamplerDescriptor::linear()
    });
    image
}
#[allow(clippy::type_complexity)]
fn paint(
    session: Res<Session>,
    ground: Query<&MeshMaterial3d<StandardMaterial>, With<super::super::GroundArena>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut current: Local<Option<(Option<u64>, Option<Handle<Image>>)>>,
) {
    let field = match session.battle.arena {
        Arena::Pressure(f) if f.generator == region::GENERATOR => Some(f),
        _ => None,
    };
    let seed = field.map(|f| f.seed);
    if current.as_ref().is_some_and(|(old, _)| *old == seed) {
        return;
    }
    let handle = field.map(|f| images.add(surface(f)));
    for mesh_material in &ground {
        if let Some(mut material) = materials.get_mut(&mesh_material.0) {
            material.base_color_texture = handle.clone();
            material.perceptual_roughness = 0.92;
        }
    }
    if let Some((_, Some(old))) = current.take() {
        images.remove(old.id());
    }
    *current = Some((seed, handle));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_uses_filtered_mips_and_ground_has_continuous_normals_and_uvs() {
        let field = PressureField {
            generator: region::GENERATOR,
            seed: 42,
            ..default()
        };
        let mesh = ground(field);
        assert!(
            mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_none(),
            "paint must not be quantized to mesh vertices"
        );
        assert_eq!(mesh.count_vertices(), 641 * 641);
        let bevy::mesh::VertexAttributeValues::Float32x3(points) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        for p in points.iter().step_by(509) {
            assert_eq!(p[1], region::elevation(field, [p[0], p[2]]));
        }
        let image = surface(field);
        assert_eq!(image.texture_descriptor.mip_level_count, 13);
        let expected: usize = (0..13).map(|m| (4096usize >> m).pow(2) * 4).sum();
        assert_eq!(image.data.as_ref().unwrap().len(), expected);
        let bevy::image::ImageSampler::Descriptor(sampler) = image.sampler else {
            panic!()
        };
        assert_eq!(sampler.mag_filter, bevy::image::ImageFilterMode::Linear);
        assert_eq!(sampler.anisotropy_clamp, 8);
    }
}
