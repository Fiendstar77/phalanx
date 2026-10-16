//! Back-mounted sashimono-style standards. Presentation never grants rally skills.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use phalanx::heraldry::Heraldry;
const COLS: usize = 16;
const ROWS: usize = 28;
const WIDTH: f32 = 0.76;
const HEIGHT: f32 = 1.42;
#[derive(Resource)]
pub(super) struct Standards {
    textures: [Handle<Image>; 2],
    materials: [Handle<StandardMaterial>; 2],
    saved: [([f32; 3], Heraldry); 2],
    pole: Handle<Mesh>,
    wood: Handle<StandardMaterial>,
    iron: Handle<StandardMaterial>,
    leather: Handle<StandardMaterial>,
}
#[derive(Component)]
struct Standard {
    id: usize,
}
#[derive(Component)]
struct Cloth {
    id: usize,
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, update.after(super::body::update));
}
fn texture(primary: [f32; 3], design: Heraldry) -> Image {
    let mut bytes = Vec::with_capacity(128 * 256 * 4);
    for y in 0..256 {
        for x in 0..128 {
            bytes.extend(design.flag_pixel(primary, x as f32 / 127.0, y as f32 / 255.0));
        }
    }
    Image::new(
        Extent3d {
            width: 128,
            height: 256,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
fn setup(
    mut commands: Commands,
    session: Res<Session>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let saved = std::array::from_fn(|team| {
        (
            session.campaign.teams[team].color,
            session.campaign.teams[team].heraldry,
        )
    });
    let textures = saved.map(|(color, design)| images.add(texture(color, design)));
    let cloth = textures.clone().map(|handle| {
        materials.add(StandardMaterial {
            base_color_texture: Some(handle),
            cull_mode: None,
            double_sided: true,
            perceptual_roughness: 0.98,
            ..default()
        })
    });
    commands.insert_resource(Standards {
        textures,
        materials: cloth,
        saved,
        pole: meshes.add(Cuboid::new(1., 1., 1.)),
        wood: materials.add(StandardMaterial {
            base_color: Color::srgb(0.22, 0.12, 0.045),
            perceptual_roughness: 0.8,
            ..default()
        }),
        iron: materials.add(StandardMaterial {
            base_color: Color::srgb(0.30, 0.34, 0.37),
            metallic: 0.7,
            ..default()
        }),
        leather: materials.add(Color::srgb(0.16, 0.075, 0.035)),
    });
}
fn cloth_points(clock: f32, phase: f32, speed: f32) -> Vec<[f32; 3]> {
    (0..=ROWS)
        .flat_map(|row| {
            (0..=COLS).map(move |col| {
                let (u, v) = (col as f32 / COLS as f32, row as f32 / ROWS as f32);
                let wave = clock * 3.2 - u * 5.0 + v * 2.8 + phase;
                let flutter = (0.10 + speed.min(2.0) * 0.05)
                    * u
                    * (0.2 + 0.8 * v)
                    * (wave.sin() + 0.35 * (wave * 1.9).sin());
                [0.045 + u * WIDTH, 1.98 - v * HEIGHT - 0.08 * u * u, flutter]
            })
        })
        .collect()
}
fn cloth_mesh() -> Mesh {
    let mut indices = Vec::new();
    for row in 0..ROWS {
        for col in 0..COLS {
            let i = (row * (COLS + 1) + col) as u32;
            let next = i + (COLS + 1) as u32;
            indices.extend([i, next, i + 1, i + 1, next, next + 1]);
        }
    }
    let uv = (0..=ROWS)
        .flat_map(|row| {
            (0..=COLS).map(move |col| [col as f32 / COLS as f32, row as f32 / ROWS as f32])
        })
        .collect::<Vec<_>>();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, cloth_points(0., 0., 0.));
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    mesh
}
#[allow(clippy::too_many_arguments)]
pub(super) fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    models: Query<&Model>,
    children: Query<&Children>,
    names: Query<&Name>,
    session: Res<Session>,
    kit: Res<Standards>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(model) = models.get(event.entity) else {
        return;
    };
    let Some(unit) = session.battle.units.get(model.id) else {
        return;
    };
    let cloth = meshes.add(cloth_mesh());
    for entity in children.iter_descendants(event.entity) {
        if !names
            .get(entity)
            .is_ok_and(|name| name.as_str() == "mixamorigSpine2")
        {
            continue;
        }
        commands.entity(entity).with_children(|p| {
            p.spawn((
                Standard { id: model.id },
                Transform::from_xyz(-0.16, -0.12, -0.24),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                // Armor socket and two iron clamps fix the pole to the back, leaving both hands free.
                for (at, size, material) in [
                    ([0., -0.10, 0.055], [0.19, 0.42, 0.08], &kit.leather),
                    ([0., -0.23, 0.005], [0.12, 0.045, 0.12], &kit.iron),
                    ([0., 0.12, 0.005], [0.12, 0.045, 0.12], &kit.iron),
                    ([0., 0.85, 0.], [0.028, 2.60, 0.028], &kit.wood),
                    ([0., 2.17, 0.], [0.045, 0.08, 0.045], &kit.iron),
                ] {
                    p.spawn((
                        Mesh3d(kit.pole.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_translation(Vec3::from_array(at))
                            .with_scale(Vec3::from_array(size)),
                    ));
                }
                // An angled crossbar presents the cloth during side-on fighting without camera billboarding.
                p.spawn((
                    Transform::from_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_4)),
                    Visibility::Inherited,
                ))
                .with_children(|p| {
                    p.spawn((
                        Mesh3d(kit.pole.clone()),
                        MeshMaterial3d(kit.wood.clone()),
                        Transform::from_xyz(0.40, 2.02, 0.0)
                            .with_scale(Vec3::new(0.85, 0.023, 0.023)),
                    ));
                    p.spawn((
                        Cloth { id: model.id },
                        Mesh3d(cloth.clone()),
                        MeshMaterial3d(kit.materials[unit.team].clone()),
                        Transform::default(),
                    ));
                });
            });
        });
        break;
    }
}
fn update(
    session: Res<Session>,
    time: Res<Time>,
    mut kit: ResMut<Standards>,
    mut images: ResMut<Assets<Image>>,
    mut standards: Query<(&Standard, &mut Visibility)>,
    cloths: Query<(&Cloth, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for team in 0..2 {
        let squad = &session.campaign.teams[team];
        let color = session.team_color(team).to_srgba();
        let design = ([color.red, color.green, color.blue], squad.heraldry);
        if design != kit.saved[team] {
            if let Some(mut image) = images.get_mut(&kit.textures[team]) {
                *image = texture(design.0, design.1);
            }
            kit.saved[team] = design;
        }
    }
    let carries = |id: usize| {
        session.battle.founders
            && session
                .battle
                .units
                .get(id)
                .is_some_and(|u| u.displays_standard())
    };
    for (standard, mut visibility) in &mut standards {
        *visibility = if carries(standard.id) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (cloth, mesh) in &cloths {
        if !carries(cloth.id) {
            continue;
        }
        let unit = &session.battle.units[cloth.id];
        let moving = session.running && unit.health > 0.0 && !matches!(unit.action, 1 | 6);
        if let Some(mut mesh) = meshes.get_mut(&mesh.0) {
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                cloth_points(
                    time.elapsed_secs(),
                    unit.team as f32 * 1.7,
                    if moving {
                        unit.movement_multiplier()
                    } else {
                        0.0
                    },
                ),
            );
            mesh.compute_smooth_normals();
        }
    }
}
