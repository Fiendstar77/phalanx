//! Droplet contacts with scenery and moving equipment/skin. All state is visual.
//! Broad-phase buckets keep collision work local; stains are a fixed decal pool.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    camera::primitives::Aabb,
    core_pipeline::prepass::DepthPrepass,
    math::Affine3A,
    mesh::{Indices, VertexAttributeValues},
    pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt},
    render::render_resource::{Extent3d, TextureDimension},
};
use std::collections::HashMap;

pub(super) const CAPACITY: usize = 512;
const CELL: f32 = 2.0;
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct Receiver(pub Option<usize>);
#[derive(Component)]
struct Skin {
    id: usize,
    center: Vec3,
    radius: f32,
    limb: u8,
}
#[derive(Component)]
struct Decal(usize, bool);
#[derive(Clone)]
struct Mark {
    target: Entity,
    local: Affine3A,
    anchor: Affine3A,
    moving: bool,
    key: (Entity, [i32; 3]),
    geometry: bool,
}
#[derive(Resource)]
pub(super) struct Stains {
    marks: Vec<Option<Mark>>,
    cursor: usize,
    run: Option<u32>,
    coverage: HashMap<(Entity, [i32; 3]), usize>,
    pub rigid_hits: u64,
    pub skin_hits: u64,
    geometry: Vec<Handle<Mesh>>,
}
impl Stains {
    pub fn clear(&mut self) {
        self.marks.fill(None);
        self.cursor = 0;
        self.rigid_hits = 0;
        self.skin_hits = 0;
        self.coverage.clear();
    }
    pub fn contact(&mut self, hit: &Hit, radius: f32, velocity: Vec3, meshes: &mut Assets<Mesh>) {
        if hit.owner.is_some() {
            self.skin_hits += 1;
        } else {
            self.rigid_hits += 1;
        }
        let local_point = hit.world.inverse().transform_point3(hit.point);
        let key = (
            hit.entity,
            local_point.to_array().map(|v| (v / 0.18).floor() as i32),
        );
        if self.coverage.contains_key(&key) {
            return;
        }
        let normal = hit.normal.normalize_or(Vec3::Y);
        let up = (Vec3::Y - normal * normal.y).normalize_or(Vec3::Z);
        let tangent = up.cross(normal).normalize_or(Vec3::X);
        let rotation = Quat::from_mat3(&Mat3::from_cols(tangent, normal, -up));
        let grazing = (1.0 - velocity.normalize_or_zero().dot(normal).abs()).clamp(0., 1.);
        let radius = if hit.mesh.is_none() {
            radius.min(0.10)
        } else {
            radius
        };
        let pose = Transform::from_translation(hit.point + normal * 0.003)
            .with_rotation(rotation)
            .with_scale(Vec3::new(radius * 2.0, 1.0, radius * (2.0 + grazing * 1.6)))
            .compute_affine();
        let index = self.cursor % CAPACITY;
        let geometry = hit.mesh.is_some();
        if let Some(handle) = &hit.mesh {
            let Some(source) = meshes.get(handle) else {
                return;
            };
            let patch = super::stain_patch::clipped(source, hit.world, hit.point, normal, radius);
            if patch.count_vertices() == 0 {
                return;
            }
            if let Some(mut target) = meshes.get_mut(&self.geometry[index]) {
                *target = patch;
            }
        }
        if let Some(old) = &self.marks[index] {
            self.coverage.remove(&old.key);
        }
        self.coverage.insert(key, index);
        self.marks[index] = Some(Mark {
            target: hit.entity,
            local: if geometry {
                Affine3A::IDENTITY
            } else {
                hit.world.inverse() * pose
            },
            anchor: hit.world,
            moving: hit.owner.is_some(),
            key,
            geometry,
        });
        self.cursor += 1;
    }
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, enable_prepass)
        .add_systems(
            PostUpdate,
            follow
                .after(bevy::transform::TransformSystems::Propagate)
                .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
        );
}
fn enable_prepass(
    mut commands: Commands,
    cameras: Query<Entity, (With<Camera3d>, Without<DepthPrepass>)>,
) {
    for entity in &cameras {
        commands.entity(entity).insert(DepthPrepass);
    }
}
fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ForwardDecalMaterial<StandardMaterial>>>,
    mut opaque: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Ragged central splash, satellite spatters and gravity streaks. Transparent
    // padding also prevents projection-edge artifacts at a grazing camera angle.
    let size = 512;
    let mut bytes = vec![0; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / (size - 1) as f32 * 2. - 1.;
            let v = y as f32 / (size - 1) as f32 * 2. - 1.;
            let angle = v.atan2(u);
            let edge = 0.45 + 0.07 * (angle * 7.).sin() + 0.045 * (angle * 13.).cos();
            let mut alpha = ((edge - u.hypot(v)) * 20.).clamp(0., 1.);
            for j in 0..9 {
                let a = j as f32 * 2.39996;
                let r = 0.55 + (j % 3) as f32 * 0.09;
                let q = (u - a.cos() * r).hypot(v - a.sin() * r);
                alpha = alpha.max(((0.025 + (j % 3) as f32 * 0.012 - q) * 60.).clamp(0., 1.));
            }
            for (at, length) in [(-0.23, 0.82), (0.10, 0.68), (0.32, 0.56)] {
                if v > 0. && v < length {
                    let width = 0.014 + (1. - v / length) * 0.016;
                    alpha = alpha.max(
                        ((width - (u - at - (v * 7.).sin() * 0.006).abs()) * 85.).clamp(0., 1.),
                    );
                }
            }
            let i = (y * size + x) * 4;
            bytes[i] = 113;
            bytes[i + 1] = 2;
            bytes[i + 2] = 9;
            bytes[i + 3] = (alpha * 238.) as u8;
        }
    }
    let image = images.add(Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let base = StandardMaterial {
        base_color_texture: Some(image.clone()),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.22,
        reflectance: 0.35,
        double_sided: true,
        cull_mode: None,
        depth_bias: 1.0,
        ..default()
    };
    let rigid_material = opaque.add(base.clone());
    let material = materials.add(ForwardDecalMaterial {
        base,
        extension: ForwardDecalMaterialExt {
            depth_fade_factor: 0.035,
        },
    });
    let mut geometry = Vec::with_capacity(CAPACITY);
    for i in 0..CAPACITY {
        let patch = meshes.add(Rectangle::new(0.001, 0.001));
        geometry.push(patch.clone());
        commands.spawn((
            Decal(i, true),
            Mesh3d(patch),
            MeshMaterial3d(rigid_material.clone()),
            Transform::default(),
            Visibility::Hidden,
            bevy::light::NotShadowCaster,
        ));
        commands.spawn((
            Decal(i, false),
            ForwardDecal,
            MeshMaterial3d(material.clone()),
            Transform::default(),
            Visibility::Hidden,
            bevy::light::NotShadowCaster,
        ));
    }
    commands.insert_resource(Stains {
        marks: vec![None; CAPACITY],
        cursor: 0,
        run: None,
        coverage: HashMap::new(),
        rigid_hits: 0,
        skin_hits: 0,
        geometry,
    });
}
fn follow(
    session: Res<Session>,
    mut stains: ResMut<Stains>,
    targets: Query<(&GlobalTransform, &InheritedVisibility), Without<Decal>>,
    mut decals: Query<(
        &Decal,
        &mut Transform,
        &mut GlobalTransform,
        &mut Visibility,
    )>,
) {
    if stains.run != Some(session.run_id) {
        stains.clear();
        stains.run = Some(session.run_id);
    }
    for (decal, mut pose, mut global, mut visibility) in &mut decals {
        *visibility = Visibility::Hidden;
        let Some(mark) = &stains.marks[decal.0] else {
            continue;
        };
        if decal.1 != mark.geometry {
            continue;
        }
        let Ok((target, inherited)) = targets.get(mark.target) else {
            continue;
        };
        if !inherited.get() || (!mark.moving && target.affine() != mark.anchor) {
            continue;
        }
        let world = target.affine() * mark.local;
        let (scale, rotation, translation) = world.to_scale_rotation_translation();
        *pose = Transform {
            translation,
            rotation,
            scale,
        };
        *global = GlobalTransform::from(*pose);
        *visibility = Visibility::Inherited;
    }
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
    for entity in children.iter_descendants(event.entity) {
        let Ok(name) = names.get(entity) else {
            continue;
        };
        let (center, radius, limb) = match name.as_str() {
            "mixamorigHead" => (Vec3::new(0., 0.20, 0.), 0.25 * super::HEAD_PROPORTION, 0),
            "mixamorigSpine2" => (Vec3::ZERO, 0.21, 0),
            "mixamorigHips" => (Vec3::ZERO, 0.18, 0),
            "mixamorigLeftArm" => (Vec3::new(0., 0.10, 0.), 0.095, 1),
            "mixamorigRightArm" => (Vec3::new(0., 0.10, 0.), 0.095, 2),
            "mixamorigLeftUpLeg" => (Vec3::new(0., 0.12, 0.), 0.12, 4),
            "mixamorigRightUpLeg" => (Vec3::new(0., 0.12, 0.), 0.12, 8),
            _ => continue,
        };
        commands.entity(entity).insert(Skin {
            id: model.id,
            center,
            radius,
            limb,
        });
    }
}
enum Shape {
    Mesh(Handle<Mesh>),
    Sphere { center: Vec3, radius: f32 },
}
struct Target {
    entity: Entity,
    owner: Option<usize>,
    world: Affine3A,
    inverse: Affine3A,
    min: Vec3,
    max: Vec3,
    shape: Shape,
}
pub(super) struct Contacts {
    targets: Vec<Target>,
    cells: HashMap<(i32, i32), Vec<usize>>,
}
pub(super) struct Hit {
    pub entity: Entity,
    pub point: Vec3,
    pub normal: Vec3,
    pub owner: Option<usize>,
    pub world: Affine3A,
    t: f32,
    mesh: Option<Handle<Mesh>>,
}
impl Hit {
    pub fn fraction(&self) -> f32 {
        self.t
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Receivers<'w, 's> {
    rigid: Query<
        'w,
        's,
        (
            Entity,
            &'static Receiver,
            &'static Mesh3d,
            &'static Aabb,
            &'static GlobalTransform,
            &'static InheritedVisibility,
        ),
    >,
    skin: Query<
        'w,
        's,
        (
            Entity,
            &'static Skin,
            &'static GlobalTransform,
            &'static InheritedVisibility,
        ),
    >,
    parents: Query<'w, 's, &'static ChildOf>,
    models: Query<'w, 's, &'static Model>,
}
impl Receivers<'_, '_> {
    pub fn contacts(&self, session: &Session) -> Contacts {
        let mut contacts = Contacts {
            targets: Vec::new(),
            cells: HashMap::new(),
        };
        for (entity, receiver, mesh, bounds, world, visible) in &self.rigid {
            if !visible.get() {
                continue;
            }
            let owner = receiver.0.or_else(|| {
                self.parents
                    .iter_ancestors(entity)
                    .find_map(|e| self.models.get(e).ok().map(|m| m.id))
            });
            contacts.push(Target {
                entity,
                owner,
                world: world.affine(),
                inverse: world.affine().inverse(),
                min: bounds.min().into(),
                max: bounds.max().into(),
                shape: Shape::Mesh(mesh.0.clone()),
            });
        }
        for (entity, skin, world, visible) in &self.skin {
            if !visible.get()
                || session
                    .visual_unit(skin.id)
                    .is_none_or(|u| u.lost_limbs & !u.prosthetics & skin.limb != 0)
            {
                continue;
            }
            let center = world.transform_point(skin.center);
            let radius = skin.radius;
            contacts.push(Target {
                entity,
                owner: Some(skin.id),
                world: world.affine(),
                inverse: world.affine().inverse(),
                min: world
                    .affine()
                    .inverse()
                    .transform_point3(center - Vec3::splat(radius)),
                max: world
                    .affine()
                    .inverse()
                    .transform_point3(center + Vec3::splat(radius)),
                shape: Shape::Sphere { center, radius },
            });
        }
        contacts
    }
}
impl Contacts {
    fn push(&mut self, target: Target) {
        let (min, max) = match target.shape {
            Shape::Sphere { center, radius } => {
                (center - Vec3::splat(radius), center + Vec3::splat(radius))
            }
            _ => {
                let mut min = Vec3::splat(f32::INFINITY);
                let mut max = Vec3::splat(f32::NEG_INFINITY);
                for x in [target.min.x, target.max.x] {
                    for y in [target.min.y, target.max.y] {
                        for z in [target.min.z, target.max.z] {
                            let p = target.world.transform_point3(Vec3::new(x, y, z));
                            min = min.min(p);
                            max = max.max(p);
                        }
                    }
                }
                (min, max)
            }
        };
        for x in (min.x / CELL).floor() as i32..=(max.x / CELL).floor() as i32 {
            for z in (min.z / CELL).floor() as i32..=(max.z / CELL).floor() as i32 {
                self.cells
                    .entry((x, z))
                    .or_default()
                    .push(self.targets.len());
            }
        }
        self.targets.push(target);
    }
    pub fn hit(&self, from: Vec3, to: Vec3, source: usize, meshes: &Assets<Mesh>) -> Option<Hit> {
        if from.distance_squared(to) < 1e-12 {
            return None;
        }
        let mut best: Option<Hit> = None;
        let min = from.min(to);
        let max = from.max(to);
        for x in (min.x / CELL).floor() as i32..=(max.x / CELL).floor() as i32 {
            for z in (min.z / CELL).floor() as i32..=(max.z / CELL).floor() as i32 {
                let Some(candidates) = self.cells.get(&(x, z)) else {
                    continue;
                };
                for &index in candidates {
                    let target = &self.targets[index];
                    if target.owner == Some(source) {
                        continue;
                    }
                    let hit = match &target.shape {
                        Shape::Sphere { center, radius } => {
                            segment_sphere(from, to, *center, *radius)
                        }
                        Shape::Mesh(handle) => {
                            let a = target.inverse.transform_point3(from);
                            let b = target.inverse.transform_point3(to);
                            if !segment_box(a, b, target.min, target.max) {
                                continue;
                            }
                            let Some(mesh) = meshes.get(handle) else {
                                continue;
                            };
                            mesh_hit(mesh, a, b).map(|(t, n)| {
                                (
                                    t,
                                    target
                                        .inverse
                                        .matrix3
                                        .transpose()
                                        .mul_vec3(n)
                                        .normalize_or(Vec3::Y),
                                )
                            })
                        }
                    };
                    if let Some((t, normal)) = hit
                        && best.as_ref().is_none_or(|old| t < old.t)
                    {
                        best = Some(Hit {
                            entity: target.entity,
                            point: from.lerp(to, t),
                            normal,
                            owner: target.owner,
                            world: target.world,
                            t,
                            mesh: match &target.shape {
                                Shape::Mesh(handle) => Some(handle.clone()),
                                _ => None,
                            },
                        });
                    }
                }
            }
        }
        best
    }
}
fn segment_box(a: Vec3, b: Vec3, min: Vec3, max: Vec3) -> bool {
    let d = b - a;
    let mut near = 0.0_f32;
    let mut far = 1.0_f32;
    for axis in 0..3 {
        if d[axis].abs() < 1e-8 {
            if a[axis] < min[axis] || a[axis] > max[axis] {
                return false;
            }
            continue;
        }
        let p = (min[axis] - a[axis]) / d[axis];
        let q = (max[axis] - a[axis]) / d[axis];
        near = near.max(p.min(q));
        far = far.min(p.max(q));
        if near > far {
            return false;
        }
    }
    true
}
fn segment_sphere(a: Vec3, b: Vec3, center: Vec3, radius: f32) -> Option<(f32, Vec3)> {
    let d = b - a;
    let p = a - center;
    let aa = d.length_squared();
    let bb = p.dot(d);
    let disc = bb * bb - aa * (p.length_squared() - radius * radius);
    if disc < 0. || aa < 1e-12 {
        return None;
    }
    let first = (-bb - disc.sqrt()) / aa;
    let second = (-bb + disc.sqrt()) / aa;
    let t = if first >= 0. { first } else { second };
    (0.0..=1.0)
        .contains(&t)
        .then(|| (t, (a + d * t - center).normalize_or(Vec3::Y)))
}
fn mesh_hit(mesh: &Mesh, a: Vec3, b: Vec3) -> Option<(f32, Vec3)> {
    let Some(VertexAttributeValues::Float32x3(points)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return None;
    };
    let mut hit = None;
    let d = b - a;
    let count = mesh.indices().map_or(points.len(), Indices::len);
    let at = |i| match mesh.indices() {
        Some(Indices::U16(v)) => v[i] as usize,
        Some(Indices::U32(v)) => v[i] as usize,
        None => i,
    };
    for i in (0..count.saturating_sub(2)).step_by(3) {
        let p = Vec3::from_array(points[at(i)]);
        let e = Vec3::from_array(points[at(i + 1)]) - p;
        let f = Vec3::from_array(points[at(i + 2)]) - p;
        let h = d.cross(f);
        let det = e.dot(h);
        if det.abs() < 1e-9 {
            continue;
        }
        let inv = 1. / det;
        let s = a - p;
        let u = s.dot(h) * inv;
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let q = s.cross(e);
        let v = d.dot(q) * inv;
        if v < 0. || u + v > 1. {
            continue;
        }
        let t = f.dot(q) * inv;
        if (0.0..=1.0).contains(&t) && hit.is_none_or(|(old, _)| t < old) {
            let mut n = e.cross(f).normalize_or(Vec3::Y);
            if n.dot(d) > 0. {
                n = -n;
            }
            hit = Some((t, n));
        }
    }
    hit
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stains_follow_the_receiver_hide_with_it_and_clear_at_the_next_battle() {
        let mut app = App::new();
        let mut session = Session::new(phalanx::Battle::new(42, false));
        session.autosave = false;
        let before = serde_json::to_string(&session.battle).unwrap();
        let target = app
            .world_mut()
            .spawn((GlobalTransform::IDENTITY, InheritedVisibility::VISIBLE))
            .id();
        let decal = app
            .world_mut()
            .spawn((
                Decal(0, false),
                Transform::default(),
                GlobalTransform::IDENTITY,
                Visibility::Hidden,
            ))
            .id();
        let mut stains = Stains {
            marks: vec![None; CAPACITY],
            cursor: 0,
            run: Some(session.run_id),
            coverage: HashMap::new(),
            rigid_hits: 0,
            skin_hits: 0,
            geometry: Vec::new(),
        };
        let hit = Hit {
            entity: target,
            point: Vec3::new(0., 0.5, 0.),
            normal: Vec3::Z,
            owner: Some(1),
            world: Affine3A::IDENTITY,
            t: 0.5,
            mesh: None,
        };
        stains.contact(&hit, 0.2, Vec3::NEG_Z, &mut Assets::<Mesh>::default());
        app.insert_resource(session)
            .insert_resource(stains)
            .add_systems(Update, follow);
        app.update();
        let first = app.world().get::<Transform>(decal).unwrap().translation;
        *app.world_mut().get_mut::<GlobalTransform>(target).unwrap() =
            GlobalTransform::from_translation(Vec3::X * 2.);
        app.update();
        assert!(
            (app.world().get::<Transform>(decal).unwrap().translation - first - Vec3::X * 2.)
                .length()
                < 1e-5
        );
        *app.world_mut()
            .get_mut::<InheritedVisibility>(target)
            .unwrap() = InheritedVisibility::HIDDEN;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(decal).unwrap(),
            Visibility::Hidden
        );
        app.world_mut().resource_mut::<Session>().run_id += 1;
        app.update();
        assert!(
            app.world()
                .resource::<Stains>()
                .marks
                .iter()
                .all(Option::is_none)
        );
        assert_eq!(
            serde_json::to_string(&app.world().resource::<Session>().battle).unwrap(),
            before
        );
    }
    #[test]
    fn repeated_contacts_merge_and_stain_storage_is_bounded() {
        let mut stains = Stains {
            marks: vec![None; CAPACITY],
            cursor: 0,
            run: None,
            coverage: HashMap::new(),
            rigid_hits: 0,
            skin_hits: 0,
            geometry: Vec::new(),
        };
        let mut hit = Hit {
            entity: Entity::from_bits(7),
            point: Vec3::ZERO,
            normal: Vec3::Z,
            owner: None,
            world: Affine3A::IDENTITY,
            t: 0.5,
            mesh: None,
        };
        for _ in 0..100 {
            stains.contact(&hit, 0.2, Vec3::NEG_Z, &mut Assets::<Mesh>::default());
        }
        assert_eq!(stains.cursor, 1);
        assert_eq!(stains.rigid_hits, 100);
        for i in 1..CAPACITY * 3 {
            hit.point.x = i as f32 * 0.3;
            stains.contact(&hit, 0.2, Vec3::NEG_Z, &mut Assets::<Mesh>::default());
        }
        assert_eq!(stains.marks.len(), CAPACITY);
        assert_eq!(stains.coverage.len(), CAPACITY);
        stains.clear();
        assert!(stains.coverage.is_empty());
    }
    #[test]
    fn swept_contacts_hit_thin_rotated_surfaces_and_the_nearest_object() {
        let mut meshes = Assets::default();
        let handle = meshes.add(Cuboid::new(1., 1., 1.));
        let pose = Transform::from_xyz(1., 0.5, 0.)
            .with_rotation(Quat::from_rotation_y(0.35))
            .with_scale(Vec3::new(0.12, 1., 2.))
            .compute_affine();
        let mut contacts = Contacts {
            targets: Vec::new(),
            cells: HashMap::new(),
        };
        let entity = Entity::from_bits(7);
        contacts.push(Target {
            entity,
            owner: None,
            world: pose,
            inverse: pose.inverse(),
            min: Vec3::splat(-0.5),
            max: Vec3::splat(0.5),
            shape: Shape::Mesh(handle),
        });
        let hit = contacts
            .hit(Vec3::new(-2., 0.5, 0.), Vec3::new(3., 0.5, 0.), 0, &meshes)
            .unwrap();
        assert_eq!(hit.entity, entity);
        assert!(hit.normal.x < -0.8);
        assert!(hit.point.x > 0.8 && hit.point.x < 1.1);
        assert!(
            contacts
                .hit(Vec3::new(-2., 2., 0.), Vec3::new(3., 2., 0.), 0, &meshes)
                .is_none()
        );
        contacts.push(Target {
            entity: Entity::from_bits(8),
            owner: Some(1),
            world: Affine3A::IDENTITY,
            inverse: Affine3A::IDENTITY,
            min: Vec3::splat(-0.2),
            max: Vec3::splat(0.2),
            shape: Shape::Sphere {
                center: Vec3::new(0., 0.5, 0.),
                radius: 0.2,
            },
        });
        assert_eq!(
            contacts
                .hit(Vec3::new(-2., 0.5, 0.), Vec3::new(3., 0.5, 0.), 0, &meshes)
                .unwrap()
                .owner,
            Some(1)
        );
        assert_eq!(
            contacts
                .hit(Vec3::new(-2., 0.5, 0.), Vec3::new(3., 0.5, 0.), 1, &meshes)
                .unwrap()
                .entity,
            entity
        );
    }
}
