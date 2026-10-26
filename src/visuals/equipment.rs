//! Body-fitted skinned garments and bone-mounted class silhouettes.
//! Starter class silhouettes plus visible crafted weapon, armor and charm fittings.
use super::*;
use bevy::mesh::{Indices, VertexAttributeValues, skinning::SkinnedMesh};
use phalanx::{
    FighterClass as Class,
    equipment::{Equipment, Gear},
    weapons::ProjectileWeapon as W,
};

#[derive(Resource)]
pub(super) struct Kit {
    cube: Handle<Mesh>,
    sphere: Handle<Mesh>,
    cone: Handle<Mesh>,
    ring: Handle<Mesh>,
    steel: Handle<StandardMaterial>,
    leather: Handle<StandardMaterial>,
    gold: Handle<StandardMaterial>,
    cloth: Handle<StandardMaterial>,
    magic: Handle<StandardMaterial>,
}
#[derive(Component)]
struct Item {
    limb: u8,
    hand: bool,
    id: usize,
    class: Class,
}
#[derive(Component)]
struct LimbFitting {
    id: usize,
    limb: u8,
}
#[derive(Component)]
struct Technology {
    id: usize,
    weapon: W,
}
#[derive(Component)]
struct Crafted {
    id: usize,
    gear: Gear,
    limb: u8,
}
#[derive(Component)]
struct Garment {
    equipment: Equipment,
    id: usize,
    layer: u8,
    indices: Vec<u32>,
    mask: u8,
    class: Option<Class>,
    color: [f32; 3],
    material: Handle<StandardMaterial>,
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, update.after(super::body::update));
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(Kit {
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        sphere: meshes.add(Sphere::new(1.0).mesh().uv(16, 12)),
        cone: meshes.add(Cone::new(1.0, 1.0)),
        ring: meshes.add(Torus::new(0.82, 1.0)),
        steel: materials.add(StandardMaterial {
            base_color: Color::srgb(0.59, 0.64, 0.67),
            metallic: 0.7,
            perceptual_roughness: 0.44,
            ..default()
        }),
        leather: materials.add(StandardMaterial {
            base_color: Color::srgb(0.21, 0.10, 0.055),
            perceptual_roughness: 0.85,
            ..default()
        }),
        gold: materials.add(StandardMaterial {
            base_color: Color::srgb(0.73, 0.48, 0.13),
            metallic: 0.65,
            ..default()
        }),
        cloth: materials.add(StandardMaterial {
            base_color: Color::srgb(0.47, 0.055, 0.035),
            perceptual_roughness: 0.95,
            ..default()
        }),
        magic: materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.78, 0.50),
            emissive: LinearRgba::new(0.05, 0.25, 0.10, 1.0),
            ..default()
        }),
    });
}
fn part(
    p: &mut ChildSpawnerCommands,
    mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    at: Vec3,
    scale: Vec3,
    rotation: Quat,
) {
    p.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(at)
            .with_scale(scale)
            .with_rotation(rotation),
    ));
}
fn block(
    p: &mut ChildSpawnerCommands,
    kit: &Kit,
    material: &Handle<StandardMaterial>,
    at: [f32; 3],
    scale: [f32; 3],
) {
    part(
        p,
        &kit.cube,
        material,
        Vec3::from_array(at),
        Vec3::from_array(scale),
        Quat::IDENTITY,
    );
}
fn dart(p: &mut ChildSpawnerCommands, kit: &Kit, length: f32, x: f32, z: f32, fletched: bool) {
    let center = if fletched { 1.05 } else { 0.18 };
    block(p, kit, &kit.leather, [x, center, z], [0.032, length, 0.032]);
    part(
        p,
        &kit.cone,
        &kit.steel,
        Vec3::new(x, center + length * 0.5 + 0.07, z),
        Vec3::new(0.065, 0.16, 0.035),
        Quat::IDENTITY,
    );
    if fletched {
        for angle in [0.0, std::f32::consts::FRAC_PI_2] {
            part(
                p,
                &kit.cube,
                &kit.cloth,
                Vec3::new(x, center + 0.07 - length * 0.5, z),
                Vec3::new(0.12, 0.18, 0.015),
                Quat::from_rotation_y(angle),
            );
        }
    }
}
fn technology(p: &mut ChildSpawnerCommands, kit: &Kit, weapon: W) {
    if weapon == W::Spear {
        dart(p, kit, 1.25, 0., 0., false);
    } else {
        // Short hooked throwing lever supports a longer, visibly fletched dart.
        block(p, kit, &kit.leather, [0., 0.12, 0.], [0.06, 0.54, 0.055]);
        block(p, kit, &kit.gold, [0.055, 0.37, 0.], [0.15, 0.05, 0.065]);
        block(p, kit, &kit.gold, [0., -0.10, 0.], [0.075, 0.09, 0.07]);
        dart(p, kit, 1.55, 0.11, 0., true);
    }
}
fn grip(female: bool, left: bool, shield: bool) -> Transform {
    let palm = Vec3::new(0.0, if female { 0.057 } else { 0.080 }, -0.038);
    let rotation = if shield {
        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
    } else {
        Quat::from_rotation_z(if left {
            -std::f32::consts::FRAC_PI_2
        } else {
            std::f32::consts::FRAC_PI_2
        })
    };
    // The handle crosses the closed palm. Its center is 3 cm above the mesh
    // origin; attaching it at the wrist made the blade float beside an open hand.
    Transform::from_translation(palm - rotation * Vec3::Y * 0.03).with_rotation(rotation)
}

#[cfg(test)]
mod grip_tests {
    use super::*;
    #[test]
    fn handle_centers_stay_in_both_palms_under_arbitrary_wrist_motion() {
        for female in [false, true] {
            for left in [false, true] {
                let mount = grip(female, left, false);
                let palm = Vec3::new(0., if female { 0.057 } else { 0.080 }, -0.038);
                for angle in [0., 0.8, 2.7] {
                    let wrist = Transform::from_xyz(1., 2., -3.).with_rotation(Quat::from_euler(
                        EulerRot::XYZ,
                        angle,
                        angle * 0.4,
                        -angle,
                    ));
                    let handle = wrist.transform_point(mount.transform_point(Vec3::Y * 0.03));
                    assert!(handle.distance(wrist.transform_point(palm)) < 1e-5);
                }
            }
        }
    }
}
fn weapon(p: &mut ChildSpawnerCommands, kit: &Kit, class: Class, left: bool) {
    if class == Class::BannerBearer {
        return;
    }
    if left && class != Class::Scout {
        part(
            p,
            &kit.sphere,
            &kit.steel,
            Vec3::new(0.0, 0.09, 0.06),
            Vec3::new(0.16, 0.22, 0.035),
            Quat::IDENTITY,
        );
        part(
            p,
            &kit.sphere,
            &kit.gold,
            Vec3::new(0.0, 0.09, 0.10),
            Vec3::splat(0.047),
            Quat::IDENTITY,
        );
        block(p, kit, &kit.gold, [0.0, 0.09, 0.10], [0.025, 0.31, 0.012]);
        return;
    }
    block(p, kit, &kit.leather, [0.0, 0.03, 0.0], [0.035, 0.16, 0.035]);
    match class {
        Class::Warrior | Class::Scout => {
            let length = if class == Class::Scout { 0.22 } else { 0.42 };
            block(
                p,
                kit,
                &kit.steel,
                [0.0, 0.13 + length / 2.0, 0.0],
                [0.055, length, 0.022],
            );
            part(
                p,
                &kit.cone,
                &kit.steel,
                Vec3::new(0.0, 0.13 + length + 0.045, 0.0),
                Vec3::new(0.035, 0.09, 0.018),
                Quat::IDENTITY,
            );
            block(p, kit, &kit.gold, [0.0, 0.12, 0.0], [0.17, 0.035, 0.045]);
        }
        Class::Guardian | Class::Paladin | Class::Berserker => {
            block(p, kit, &kit.leather, [0.0, 0.22, 0.0], [0.04, 0.44, 0.04]);
            match class {
                Class::Guardian => {
                    part(
                        p,
                        &kit.sphere,
                        &kit.steel,
                        Vec3::new(0.0, 0.46, 0.0),
                        Vec3::splat(0.095),
                        Quat::IDENTITY,
                    );
                    for x in [-0.12, 0.12] {
                        block(p, kit, &kit.gold, [x, 0.46, 0.0], [0.10, 0.035, 0.035]);
                    }
                }
                Class::Paladin => {
                    block(p, kit, &kit.steel, [0.0, 0.46, 0.0], [0.25, 0.14, 0.12]);
                    block(p, kit, &kit.gold, [0.0, 0.46, 0.065], [0.035, 0.10, 0.018]);
                }
                _ => {
                    for side in [-1.0, 1.0] {
                        part(
                            p,
                            &kit.cube,
                            &kit.steel,
                            Vec3::new(side * 0.10, 0.46, 0.0),
                            Vec3::new(0.17, 0.19, 0.045),
                            Quat::from_rotation_z(-side * 0.20),
                        );
                    }
                }
            }
        }
        Class::Ranger => {
            let mut previous = None;
            for i in 0..15 {
                let angle = -1.30 + i as f32 / 14.0 * 2.60;
                let point = Vec3::new(0.22 * angle.cos() - 0.12, 0.22 + 0.28 * angle.sin(), 0.0);
                if let Some(old) = previous {
                    let d: Vec3 = point - old;
                    part(
                        p,
                        &kit.cube,
                        &kit.leather,
                        (point + old) * 0.5,
                        Vec3::new(0.025, d.length() + 0.008, 0.025),
                        Quat::from_rotation_arc(Vec3::Y, d.normalize()),
                    );
                }
                previous = Some(point);
            }
            block(p, kit, &kit.gold, [-0.061, 0.22, 0.0], [0.008, 0.54, 0.008]);
            block(p, kit, &kit.steel, [0.02, 0.22, 0.02], [0.34, 0.010, 0.010]);
        }
        Class::Healer | Class::Hexer | Class::BannerBearer => {
            block(p, kit, &kit.leather, [0.0, 0.28, 0.0], [0.035, 0.78, 0.035]);
            if class == Class::BannerBearer {
                block(p, kit, &kit.cloth, [0.13, 0.59, 0.0], [0.25, 0.27, 0.018]);
                block(p, kit, &kit.gold, [0.13, 0.59, 0.015], [0.025, 0.18, 0.012]);
            } else if class == Class::Hexer {
                part(
                    p,
                    &kit.sphere,
                    &kit.magic,
                    Vec3::new(0.0, 0.72, 0.0),
                    Vec3::splat(0.09),
                    Quat::IDENTITY,
                );
            } else {
                part(
                    p,
                    &kit.ring,
                    &kit.gold,
                    Vec3::new(0.0, 0.72, 0.0),
                    Vec3::splat(0.08),
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                );
                block(p, kit, &kit.gold, [0.0, 0.72, 0.0], [0.025, 0.16, 0.025]);
            }
        }
        Class::RockThrower => {
            for x in [-0.05, 0.05] {
                part(
                    p,
                    &kit.cube,
                    &kit.leather,
                    Vec3::new(x, 0.16, 0.0),
                    Vec3::new(0.012, 0.25, 0.012),
                    Quat::from_rotation_z(x * 2.0),
                );
            }
            part(
                p,
                &kit.sphere,
                &kit.leather,
                Vec3::new(0.0, 0.30, 0.0),
                Vec3::new(0.07, 0.035, 0.05),
                Quat::IDENTITY,
            );
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    models: Query<&Model>,
    children: Query<&Children>,
    names: Query<&Name>,
    primitives: Query<(&Mesh3d, &SkinnedMesh)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    kit: Res<Kit>,
) {
    let Ok(model) = models.get(event.entity) else {
        return;
    };
    for entity in children.iter_descendants(event.entity) {
        if let Ok((mesh, skin)) = primitives.get(entity)
            && let Some(source) = meshes.get(&mesh.0).cloned()
        {
            let Some(VertexAttributeValues::Float32x3(positions)) =
                source.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                continue;
            };
            let indices: Vec<u32> = source
                .indices()
                .map(|i| i.iter().map(|n| n as u32).collect())
                .unwrap_or_default();
            for layer in 0..2 {
                let mut garment = source.clone();
                let selected = indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .filter(|tri| {
                        let c = (Vec3::from_array(positions[tri[0] as usize])
                            + Vec3::from_array(positions[tri[1] as usize])
                            + Vec3::from_array(positions[tri[2] as usize]))
                            / 3.0;
                        if layer == 0 {
                            c.y > 0.70 && c.y < 1.22 && c.x.abs() < 0.32
                        } else {
                            c.y > -0.02 && c.y <= 0.74
                        }
                    })
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>();
                garment.insert_indices(Indices::U32(selected.clone()));
                garment.remove_attribute(Mesh::ATTRIBUTE_COLOR);
                if let Some(VertexAttributeValues::Float32x3(points)) =
                    garment.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                {
                    for p in points {
                        p[0] *= 1.07;
                        p[2] = p[2] * 1.08 + p[2].signum() * 0.008;
                    }
                }
                let material = materials.add(StandardMaterial {
                    perceptual_roughness: 0.85,
                    ..default()
                });
                commands.entity(entity).with_children(|p| {
                    p.spawn((
                        Garment {
                            equipment: Equipment::default(),
                            id: model.id,
                            layer,
                            indices: selected,
                            mask: u8::MAX,
                            class: None,
                            color: [-1.0; 3],
                            material: material.clone(),
                        },
                        Mesh3d(meshes.add(garment)),
                        skin.clone(),
                        MeshMaterial3d(material),
                        Transform::default(),
                    ));
                });
            }
        }
        if let Ok(name) = names.get(entity) {
            let left = name.as_str() == "mixamorigLeftHand";
            if left || name.as_str() == "mixamorigRightHand" {
                for class in Class::ALL {
                    if left
                        && !matches!(
                            class,
                            Class::Warrior | Class::Guardian | Class::Paladin | Class::Scout
                        )
                    {
                        continue;
                    }
                    commands.entity(entity).with_children(|p| {
                        p.spawn((
                            Item {
                                limb: if left { 1 } else { 2 },
                                hand: true,
                                id: model.id,
                                class,
                            },
                            grip(model.female, left, left && class != Class::Scout),
                            Visibility::Hidden,
                        ))
                        .with_children(|p| {
                            weapon(p, &kit, class, left);
                            if !left
                                && matches!(
                                    class,
                                    Class::Warrior
                                        | Class::Scout
                                        | Class::Guardian
                                        | Class::Paladin
                                        | Class::Berserker
                                )
                            {
                                p.spawn((
                                    super::spectacle::Tip {
                                        id: model.id,
                                        female: model.female,
                                    },
                                    Transform::from_xyz(
                                        0.0,
                                        if class == Class::Scout { 0.39 } else { 0.64 },
                                        0.0,
                                    ),
                                    Visibility::Inherited,
                                ));
                            }
                        });
                    });
                }
            }
            if name.as_str() == "mixamorigRightHand" {
                for weapon in [W::Spear, W::Atlatl] {
                    commands.entity(entity).with_children(|p| {
                        p.spawn((
                            Technology {
                                id: model.id,
                                weapon,
                            },
                            grip(model.female, false, false),
                            Visibility::Hidden,
                        ))
                        .with_children(|p| technology(p, &kit, weapon));
                    });
                }
                for gear in [Gear::TemperedWeapon, Gear::BalancedWeapon] {
                    commands.entity(entity).with_children(|p| {
                        p.spawn((
                            Crafted {
                                id: model.id,
                                gear,
                                limb: 2,
                            },
                            Transform::default(),
                            Visibility::Hidden,
                        ))
                        .with_children(|p| {
                            // A forged cuff identifies upgraded class weapons without replacing their silhouette.
                            part(
                                p,
                                &kit.ring,
                                if gear == Gear::TemperedWeapon {
                                    &kit.gold
                                } else {
                                    &kit.steel
                                },
                                Vec3::new(0.0, 0.035, 0.0),
                                Vec3::splat(if gear == Gear::TemperedWeapon {
                                    0.07
                                } else {
                                    0.052
                                }),
                                Quat::IDENTITY,
                            );
                        });
                    });
                }
            }
            if name.as_str() == "mixamorigSpine2" {
                for gear in [Gear::IronArmor, Gear::TrailArmor, Gear::VitalityCharm] {
                    commands.entity(entity).with_children(|p| {
                        p.spawn((
                            Crafted {
                                id: model.id,
                                gear,
                                limb: 0,
                            },
                            Transform::default(),
                            Visibility::Hidden,
                        ))
                        .with_children(|p| match gear {
                            Gear::IronArmor => block(
                                p,
                                &kit,
                                &kit.steel,
                                [0.0, -0.03, 0.115],
                                [0.32, 0.30, 0.065],
                            ),
                            Gear::TrailArmor => {
                                for x in [-0.12, 0.12] {
                                    block(
                                        p,
                                        &kit,
                                        &kit.leather,
                                        [x, -0.03, 0.13],
                                        [0.045, 0.35, 0.035],
                                    );
                                }
                            }
                            _ => part(
                                p,
                                &kit.sphere,
                                &kit.magic,
                                Vec3::new(0.0, 0.09, 0.17),
                                Vec3::splat(0.05),
                                Quat::IDENTITY,
                            ),
                        });
                    });
                }
            }
            if matches!(
                name.as_str(),
                "mixamorigLeftForeArm" | "mixamorigRightForeArm"
            ) {
                commands.entity(entity).with_children(|p| {
                    p.spawn((
                        LimbFitting {
                            id: model.id,
                            limb: if name.as_str() == "mixamorigLeftForeArm" {
                                1
                            } else {
                                2
                            },
                        },
                        Transform::default(),
                        Visibility::Hidden,
                    ))
                    .with_children(|p| {
                        part(
                            p,
                            &kit.sphere,
                            &kit.leather,
                            Vec3::new(0.0, 0.13, 0.0),
                            Vec3::new(0.070, 0.085, 0.070),
                            Quat::IDENTITY,
                        );
                    });
                });
            }
            if matches!(name.as_str(), "mixamorigLeftArm" | "mixamorigRightArm") {
                commands.entity(entity).with_children(|p| {
                    for class in [Class::Guardian, Class::Paladin, Class::Warrior] {
                        p.spawn((
                            Item {
                                limb: if name.as_str() == "mixamorigLeftArm" {
                                    1
                                } else {
                                    2
                                },
                                hand: false,
                                id: model.id,
                                class,
                            },
                            Transform::default(),
                            Visibility::Hidden,
                        ))
                        .with_children(|p| {
                            part(
                                p,
                                &kit.sphere,
                                &kit.steel,
                                Vec3::new(0.0, 0.05, 0.0),
                                Vec3::new(0.105, 0.09, 0.105),
                                Quat::IDENTITY,
                            );
                        });
                    }
                });
            }
            if name.as_str() == "mixamorigSpine2" {
                commands.entity(entity).with_children(|p| {
                    block(
                        p,
                        &kit,
                        &kit.leather,
                        [0.0, -0.18, 0.11],
                        [0.40, 0.055, 0.20],
                    );
                    block(p, &kit, &kit.gold, [0.0, -0.18, 0.22], [0.065, 0.05, 0.018]);
                });
            }
        }
    }
}
fn update(
    session: Res<Session>,
    mut items: Query<(&Item, &mut Visibility), (Without<Crafted>, Without<Technology>)>,
    mut technologies: Query<(&Technology, &mut Visibility), (Without<Item>, Without<Crafted>)>,
    mut crafted: Query<(&Crafted, &mut Visibility), (Without<Item>, Without<Technology>)>,
    mut fittings: Query<
        (&LimbFitting, &mut Visibility),
        (Without<Item>, Without<Crafted>, Without<Technology>),
    >,
    mut garments: Query<(&mut Garment, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (item, mut visible) in &mut technologies {
        *visible = if session.visual_unit(item.id).is_some_and(|u| {
            u.health > 0.0
                && u.ammunition > 0
                && u.projectile_weapon() == Some(item.weapon)
                && u.lost_limbs & !u.prosthetics & 2 == 0
        }) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (piece, mut visible) in &mut crafted {
        *visible = if session.visual_unit(piece.id).is_some_and(|u| {
            u.health > 0.0
                && u.equipment.has(piece.gear)
                && (u.lost_limbs & !u.prosthetics & piece.limb == 0)
        }) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (fitting, mut visible) in &mut fittings {
        *visible = if session
            .visual_unit(fitting.id)
            .is_some_and(|u| u.health > 0. && u.lost_limbs & !u.prosthetics & fitting.limb == 0)
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (item, mut visible) in &mut items {
        *visible = if session.visual_unit(item.id).is_some_and(|u| {
            u.health > 0.0
                && u.lost_limbs & !u.prosthetics & item.limb == 0
                && (!item.hand
                    || u.projectile_weapon() != Some(W::Bow)
                    || (!u.is_ranged() && !u.bow_follow_through()))
                && (!item.hand || u.equipment.projectile_weapon().is_none() || !u.is_ranged())
                && (if item.hand && u.projectile_weapon().is_some() && !u.is_ranged() {
                    Class::Warrior
                } else {
                    u.class.unwrap_or(if u.role == Role::RockThrower {
                        Class::RockThrower
                    } else {
                        Class::Warrior
                    })
                }) == item.class
        }) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (mut garment, mesh) in &mut garments {
        let Some(unit) = session.visual_unit(garment.id) else {
            continue;
        };
        let class = unit.class.unwrap_or(Class::Warrior);
        let c = session.team_color(unit.team).to_srgba();
        let color = [c.red, c.green, c.blue];
        if garment.class != Some(class)
            || garment.color != color
            || garment.equipment != unit.equipment
        {
            if let Some(mut material) = materials.get_mut(&garment.material) {
                let heavy = unit.equipment.has(Gear::IronArmor)
                    || (!unit.equipment.has(Gear::TrailArmor)
                        && matches!(class, Class::Guardian | Class::Paladin | Class::Warrior));
                let base = if unit.equipment.has(Gear::IronArmor) {
                    [0.48, 0.54, 0.60]
                } else if unit.equipment.has(Gear::TrailArmor) {
                    [0.26, 0.19, 0.10]
                } else {
                    match class {
                        Class::Warrior => [0.40, 0.48, 0.56],
                        Class::Guardian => [0.20, 0.31, 0.48],
                        Class::Ranger => [0.21, 0.36, 0.16],
                        Class::Scout => [0.25, 0.20, 0.18],
                        Class::Healer => [0.84, 0.78, 0.60],
                        Class::Berserker => [0.56, 0.13, 0.10],
                        Class::Hexer => [0.32, 0.13, 0.43],
                        Class::BannerBearer => [0.65, 0.39, 0.12],
                        Class::RockThrower => [0.43, 0.33, 0.22],
                        Class::Paladin => [0.72, 0.62, 0.34],
                    }
                };
                let mix = if garment.layer == 0 { 0.18 } else { 0.05 };
                let dark = if garment.layer == 0 { 1.0 } else { 0.45 };
                material.base_color = Color::srgb(
                    base[0] * dark * (1.0 - mix) + color[0] * mix,
                    base[1] * dark * (1.0 - mix) + color[1] * mix,
                    base[2] * dark * (1.0 - mix) + color[2] * mix,
                );
                material.metallic = if heavy && garment.layer == 0 {
                    0.65
                } else {
                    0.0
                };
                material.perceptual_roughness = if heavy && garment.layer == 0 {
                    0.42
                } else {
                    0.86
                };
            }
            garment.equipment = unit.equipment;
            garment.class = Some(class);
            garment.color = color;
        }
        let mask = unit.lost_limbs
            | if unit.health <= 0.0 {
                if unit.id % 2 == 0 { 3 } else { 12 }
            } else {
                0
            };
        if garment.mask != mask
            && let Some(mut mesh) = meshes.get_mut(&mesh.0)
        {
            let Some(VertexAttributeValues::Float32x3(points)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                continue;
            };
            let selected = garment
                .indices
                .as_chunks::<3>()
                .0
                .iter()
                .filter(|tri| {
                    let c = (Vec3::from_array(points[tri[0] as usize])
                        + Vec3::from_array(points[tri[1] as usize])
                        + Vec3::from_array(points[tri[2] as usize]))
                        / 3.0;
                    let bit = if c.y < 0.66 {
                        if c.x > 0.0 { 4 } else { 8 }
                    } else if c.y > 0.82 && c.x.abs() > 0.23 {
                        if c.x > 0.0 { 1 } else { 2 }
                    } else {
                        0
                    };
                    mask & bit == 0
                })
                .flatten()
                .copied()
                .collect();
            mesh.insert_indices(Indices::U32(selected));
            garment.mask = mask;
        }
    }
}
