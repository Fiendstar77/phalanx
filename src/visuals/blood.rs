//! Bounded directional droplets and a persistent terrain-conforming stain canvas.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
const COUNT: usize = 1024;
const SIZE: usize = 512;
#[derive(Component)]
pub(super) struct Particle(usize);
#[derive(Component)]
pub(super) struct StainSurface;
#[derive(Clone, Default)]
pub(super) struct Droplet {
    p: Vec3,
    v: Vec3,
    scale: Vec3,
    pub(super) life: f32,
}
#[derive(Resource)]
pub(super) struct Fx {
    pub(super) particles: Vec<Droplet>,
    cursor: usize,
    run: Option<u32>,
    last_event: Option<u64>,
    upload: f32,
    drip: f32,
    image: Handle<Image>,
    pixels: Vec<u8>,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup.after(super::load_visuals));
}
fn image() -> Image {
    Image::new(
        Extent3d {
            width: SIZE as u32,
            height: SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; SIZE * SIZE * 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
fn surface(arena: Arena) -> Mesh {
    let half = arena.half_size();
    let nx = (half[0] * 4.0) as usize;
    let nz = (half[1] * 4.0) as usize;
    let mut p = Vec::new();
    let mut uv = Vec::new();
    let mut indices = Vec::new();
    for z in 0..=nz {
        for x in 0..=nx {
            let u = x as f32 / nx as f32;
            let v = z as f32 / nz as f32;
            let pos = [(u * 2.0 - 1.0) * half[0], (v * 2.0 - 1.0) * half[1]];
            p.push([pos[0], arena.elevation(pos) + 0.018, pos[1]]);
            uv.push([u, v]);
        }
    }
    for z in 0..nz {
        for x in 0..nx {
            let i = (z * (nx + 1) + x) as u32;
            let n = i + (nx + 1) as u32;
            indices.extend([i, n, i + 1, i + 1, n, n + 1]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    mesh
}
fn setup(
    mut commands: Commands,
    session: Res<Session>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let image = images.add(image());
    let stain = materials.add(StandardMaterial {
        base_color_texture: Some(image.clone()),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.65,
        reflectance: 0.1,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        StainSurface,
        Mesh3d(meshes.add(surface(session.battle.arena))),
        MeshMaterial3d(stain),
        Transform::default(),
    ));
    let drop = meshes.add(Sphere::new(1.0).mesh().uv(6, 4));
    let wet = materials.add(StandardMaterial {
        base_color: Color::srgb(0.46, 0.006, 0.014),
        perceptual_roughness: 0.17,
        reflectance: 0.35,
        ..default()
    });
    for i in 0..COUNT {
        commands.spawn((
            Particle(i),
            Mesh3d(drop.clone()),
            MeshMaterial3d(wet.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    commands.insert_resource(Fx {
        particles: vec![Droplet::default(); COUNT],
        cursor: 0,
        run: None,
        last_event: None,
        upload: 0.,
        drip: 0.,
        image,
        pixels: vec![0; SIZE * SIZE * 4],
        dirty: false,
    });
}
impl Fx {
    pub(super) fn stained_pixels(&self) -> usize {
        self.pixels.chunks_exact(4).filter(|p| p[3] != 0).count()
    }
    fn stamp(
        &mut self,
        arena: Arena,
        p: Vec3,
        radius: f32,
        stretch: f32,
        angle: f32,
        strength: f32,
    ) {
        let half = arena.half_size();
        let cx = (p.x / half[0] + 1.0) * 0.5 * SIZE as f32;
        let cy = (p.z / half[1] + 1.0) * 0.5 * SIZE as f32;
        let r = (radius * SIZE as f32 / (half[0] * 2.0)).max(0.8);
        let bound = (r * stretch + 2.0).ceil() as i32;
        let (sin, cos) = angle.sin_cos();
        for y in (cy as i32 - bound).max(0)..=(cy as i32 + bound).min(SIZE as i32 - 1) {
            for x in (cx as i32 - bound).max(0)..=(cx as i32 + bound).min(SIZE as i32 - 1) {
                let dx = x as f32 - cx;
                let dy = (y as f32 - cy) * half[1] / half[0];
                let u = (dx * cos + dy * sin) / (r * stretch);
                let v = (-dx * sin + dy * cos) / r;
                let noise = ((x * 73 + y * 131) % 19) as f32 / 19.0;
                let q = u * u + v * v;
                if q > 0.82 + noise * 0.28 {
                    continue;
                }
                let i = (y as usize * SIZE + x as usize) * 4;
                let alpha = ((1.0 - q).clamp(0.0, 1.0) * strength * 180.0 + 20.0).min(220.0) as u8;
                let old = self.pixels[i + 3] as u32;
                self.pixels[i] = if old > 150 { 65 } else { 96 };
                self.pixels[i + 1] = 3;
                self.pixels[i + 2] = 10;
                self.pixels[i + 3] = (old + ((255 - old) * alpha as u32) / 255).min(245) as u8;
            }
        }
        self.dirty = true;
    }
    fn emit(&mut self, p: Vec3, direction: Vec3, amount: f32, severe: bool, blunt: bool) {
        let count = if severe {
            90
        } else {
            (amount * 2.0).clamp(12.0, 50.0) as usize
        };
        let forward = direction.normalize_or_zero();
        let side = Vec3::new(-forward.z, 0., forward.x);
        for j in 0..count {
            let index = self.cursor % COUNT;
            self.cursor += 1;
            let n = ((self.cursor * 73 + j * 131) % 997) as f32 / 997.0;
            let fan = if severe {
                1.2
            } else if blunt {
                0.24
            } else {
                0.7
            };
            let a = (n - 0.5) * fan;
            let speed = if severe {
                3.5 + n * 3.0
            } else if blunt {
                1.1 + n * 1.6
            } else {
                2.0 + n * 2.5
            };
            let radius = if blunt {
                0.025 + n * 0.025
            } else {
                0.012 + n * 0.023
            };
            self.particles[index] = Droplet {
                p,
                v: (forward * a.cos() + side * a.sin()) * speed
                    + Vec3::Y * (0.5 + n * if severe { 1.6 } else { 2.0 }),
                scale: Vec3::new(radius, radius * if blunt { 1.2 } else { 2.5 }, radius),
                life: 3.0,
            };
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn effects(
    session: Res<Session>,
    time: Res<Time>,
    mut fx: ResMut<Fx>,
    mut particles: Query<(&Particle, &mut Transform, &mut Visibility)>,
    surface_mesh: Single<&Mesh3d, With<StainSurface>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
) {
    let arena = session.battle.arena;
    if fx.run != Some(session.run_id) {
        fx.run = Some(session.run_id);
        fx.last_event = None;
        fx.pixels.fill(0);
        fx.dirty = true;
        for p in &mut fx.particles {
            p.life = 0.;
        }
        if let Some(mut mesh) = meshes.get_mut(&surface_mesh.0) {
            *mesh = surface(arena);
        }
    }
    for event in &session.battle.damage_events {
        if fx.last_event.is_some_and(|last| event.sequence <= last) {
            continue;
        }
        let pos = Vec3::new(
            event.position[0],
            arena.elevation(event.position) + if event.severed & 12 != 0 { 0.36 } else { 0.72 },
            event.position[1],
        );
        let direction = Vec3::new(event.velocity[0], 0., event.velocity[1]);
        fx.emit(
            pos,
            direction,
            event.amount,
            event.severed != 0 || event.amount >= 25.0,
            event.blunt,
        );
        if event.severed != 0 || session.battle.units[event.defender].health <= 0.0 {
            fx.stamp(
                arena,
                pos,
                0.3,
                1.3,
                event.velocity[1].atan2(event.velocity[0]),
                0.65,
            );
        }
        fx.last_event = Some(event.sequence);
    }
    let dt = if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else if session.battle.finished && !session.menu {
        time.delta_secs().min(0.1)
    } else {
        0.0
    };
    fx.drip += dt;
    if fx.drip >= 0.28 {
        fx.drip = 0.;
        for u in &session.battle.units {
            if u.health > 0.0
                && u.lost_limbs & !u.prosthetics != 0
                && session.running
                && session
                    .battle
                    .motion
                    .position(u.id, 0.0)
                    .zip(session.battle.motion.position(u.id, 1.0))
                    .is_some_and(|(a, b)| phalanx::distance(a, b) > 0.001)
            {
                let p = session.presented(u.id);
                let pos = Vec3::new(p[0], 0., p[1]);
                fx.stamp(
                    arena,
                    pos,
                    0.025 + 0.015 * u.lost_limbs.count_ones() as f32,
                    1.1,
                    u.id as f32,
                    0.35,
                );
            }
        }
    }
    let mut landed = Vec::new();
    for p in &mut fx.particles {
        if p.life <= 0.0 {
            continue;
        }
        p.life -= dt;
        let steps = (dt / 0.016).ceil().max(1.0) as usize;
        let step = dt / steps as f32;
        for _ in 0..steps {
            p.v.y -= 9.81 * step;
            p.v *= (-0.15 * step).exp();
            let next = p.p + p.v * step;
            if arena.blocked([next.x, next.z]) {
                p.v.x *= -0.12;
                p.v.z *= -0.12;
            } else {
                p.p.x = next.x;
                p.p.z = next.z;
            }
            p.p.y = next.y;
            if p.p.y <= arena.elevation([p.p.x, p.p.z]) + 0.025 {
                let speed = p.v.xz().length();
                landed.push((
                    p.p,
                    (p.scale.x * 2.0).max(0.025),
                    1.0 + (speed * 0.2).min(2.0),
                    p.v.z.atan2(p.v.x),
                ));
                p.life = 0.;
                break;
            }
        }
    }
    for (p, r, stretch, angle) in landed {
        fx.stamp(arena, p, r, stretch, angle, 0.65);
    }
    fx.upload += time.delta_secs();
    if fx.dirty && fx.upload >= 0.1 {
        let bytes = fx.pixels.clone();
        if let Some(mut image) = images.get_mut(&fx.image) {
            image.data = Some(bytes);
        }
        fx.dirty = false;
        fx.upload = 0.;
    }
    for (particle, mut transform, mut visibility) in &mut particles {
        let p = &fx.particles[particle.0];
        *visibility = if p.life > 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if p.life > 0. {
            transform.translation = p.p;
            transform.scale = p.scale;
            transform.rotation = Quat::from_rotation_arc(Vec3::Y, p.v.normalize_or_zero());
        }
    }
}
