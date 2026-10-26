//! Bounded presentation of authoritative ninja arts and physical razor wheels.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::PrimitiveTopology,
    render::render_resource::{Extent3d, TextureDimension},
};
use phalanx::ninjutsu::{self as rules, Art};
#[derive(Component)]
struct Cloud(usize, usize);
#[derive(Component)]
struct Disc(usize);
#[derive(Component)]
struct Link(usize);
#[derive(Component)]
struct Seal(usize, usize);
#[derive(Resource, Default)]
pub(super) struct Audit {
    pub arts: std::collections::BTreeMap<&'static str, u32>,
    pub max_wall_height: f32,
    pub smoke_frames: u32,
    pub hidden_contacts: bool,
    pub last: Option<u64>,
    pub run: Option<u32>,
}
pub(super) fn razor_mesh() -> Mesh {
    let mut p = Vec::new();
    let mut n = Vec::new();
    let rim: Vec<_> = (0..16)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 16.;
            let r = if i % 2 == 0 { 0.48 } else { 0.20 };
            Vec3::new(a.cos() * r, 0., a.sin() * r)
        })
        .collect();
    let mut triangle = |a: Vec3, b: Vec3, c: Vec3| {
        let normal = (b - a).cross(c - a).normalize_or(Vec3::Y);
        for v in [a, b, c] {
            p.push(v.to_array());
            n.push(normal.to_array());
        }
    };
    for i in 0..16 {
        let a = rim[i];
        let b = rim[(i + 1) % 16];
        let up = Vec3::Y * 0.025;
        triangle(up, b + up, a + up);
        triangle(-up, a - up, b - up);
        triangle(a + up, b + up, b - up);
        triangle(a + up, b - up, a - up);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, p)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, n)
}
pub(super) fn sickle_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut triangle = |a: Vec3, b: Vec3, c: Vec3| {
        let n = (b - a).cross(c - a).normalize_or(Vec3::Z);
        for v in [a, b, c] {
            positions.push(v.to_array());
            normals.push(n.to_array());
        }
    };
    let point = |t: f32, outer: bool| {
        let a = -std::f32::consts::PI * 0.75 + t * std::f32::consts::PI * 1.5;
        let r = if outer {
            0.30
        } else {
            0.19 + 0.085 * (t * 2. - 1.).abs().powi(6)
        };
        Vec3::new(0.08 + a.cos() * r, 0.34 + a.sin() * r, 0.)
    };
    let z = Vec3::Z * 0.014;
    for i in 0..24 {
        let t = i as f32 / 24.;
        let u = (i + 1) as f32 / 24.;
        let [a, b, c, d] = [
            point(t, true),
            point(u, true),
            point(u, false),
            point(t, false),
        ];
        triangle(a + z, b + z, c + z);
        triangle(a + z, c + z, d + z);
        triangle(a - z, c - z, b - z);
        triangle(a - z, d - z, c - z);
        triangle(a + z, a - z, b - z);
        triangle(a + z, b - z, b + z);
        triangle(d + z, c - z, d - z);
        triangle(d + z, c + z, c - z);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}
fn smoke_image() -> Image {
    const SIZE: usize = 96;
    let mut pixels = vec![0; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let u = (x as f32 + 0.5) / SIZE as f32 * 2. - 1.;
            let v = (y as f32 + 0.5) / SIZE as f32 * 2. - 1.;
            let r2 = u * u + v * v;
            let grain = ((u * 13. + v * 7.).sin() * (v * 17. - u * 5.).cos() * 0.22
                + (u * 29. - v * 19.).cos() * 0.08
                + 0.70)
                .clamp(0., 1.);
            let alpha = (1. - r2).clamp(0., 1.).powi(2) * grain;
            let at = (y * SIZE + x) * 4;
            pixels[at..at + 4].copy_from_slice(&[150, 156, 168, (alpha * 180.) as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: SIZE as u32,
            height: SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Audit>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            update.after(animate).after(super::super::sync_pawns),
        );
    if std::env::args().any(|a| a == "--ninja-demo") {
        app.add_systems(Startup, demo_setup.after(load_visuals))
            .add_systems(Update, demo.after(update));
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let puff = meshes.add(Rectangle::new(1., 1.));
    let ring = meshes.add(Torus::new(0.82, 1.));
    let paper = meshes.add(Cuboid::new(0.12, 0.28, 0.012));
    let blade = meshes.add(razor_mesh());
    let cloud = materials.add(StandardMaterial {
        base_color: Color::srgba(0.75, 0.78, 0.84, 0.75),
        base_color_texture: Some(images.add(smoke_image())),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let steel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.39, 0.48),
        metallic: 0.85,
        perceptual_roughness: 0.24,
        ..default()
    });
    let ink = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.012, 0.13),
        emissive: LinearRgba::new(0.65, 0.035, 1.1, 1.),
        ..default()
    });
    let talisman = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.25, 0.44),
        emissive: LinearRgba::new(0.40, 0.02, 0.09, 1.),
        ..default()
    });
    for slot in 0..rules::SMOKE_CAPACITY {
        for part in 0..12 {
            commands.spawn((
                Cloud(slot, part),
                Mesh3d(puff.clone()),
                MeshMaterial3d(cloud.clone()),
                Transform::default(),
                Visibility::Hidden,
                bevy::light::NotShadowCaster,
            ));
        }
    }
    for slot in 0..rules::WHEEL_CAPACITY {
        commands.spawn((
            Disc(slot),
            Mesh3d(blade.clone()),
            MeshMaterial3d(steel.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    for slot in 0..32 {
        commands.spawn((
            Link(slot),
            Mesh3d(ring.clone()),
            MeshMaterial3d(steel.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    for id in 0..20 {
        for part in 0..9 {
            commands.spawn((
                Seal(id, part),
                Mesh3d(if part == 0 {
                    ring.clone()
                } else {
                    paper.clone()
                }),
                MeshMaterial3d(if part == 0 {
                    ink.clone()
                } else {
                    talisman.clone()
                }),
                Transform::default(),
                Visibility::Hidden,
            ));
        }
    }
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update(
    session: Res<Session>,
    mut audit: ResMut<Audit>,
    cameras: Query<&GlobalTransform, (With<Camera3d>, With<IsDefaultUiCamera>)>,
    mut clouds: Query<(&Cloud, &mut Transform, &mut Visibility)>,
    mut discs: Query<(&Disc, &mut Transform, &mut Visibility), Without<Cloud>>,
    mut links: Query<(&Link, &mut Transform, &mut Visibility), (Without<Cloud>, Without<Disc>)>,
    mut seals: Query<
        (&Seal, &mut Transform, &mut Visibility),
        (Without<Cloud>, Without<Disc>, Without<Link>),
    >,
) {
    let battle = &session.battle;
    if audit.run != Some(session.run_id) {
        *audit = Audit {
            run: Some(session.run_id),
            ..default()
        };
    }
    let tick = if session.running {
        battle.tick.saturating_sub(1) as f32 + session.accumulator / phalanx::DT
    } else {
        battle.tick as f32
    };
    for event in &battle.ninjutsu.events {
        if audit.last.is_none_or(|old| event.sequence > old) {
            audit.last = Some(event.sequence);
            let name = if event.art == Art::GraveBind && !event.success {
                "Binding resisted"
            } else {
                event.art.name()
            };
            *audit.arts.entry(name).or_default() += 1;
        }
    }
    for u in &battle.units {
        audit.max_wall_height =
            audit
                .max_wall_height
                .max(rules::physical_height(u, battle.arena, tick));
    }
    if !battle.ninjutsu.smoke.is_empty() {
        audit.smoke_frames += 1;
        audit.hidden_contacts |= battle
            .units
            .iter()
            .any(|u| u.health > 0. && u.visible_enemies == 0 && !u.contacts.is_empty());
    }
    for (cloud, mut transform, mut visible) in &mut clouds {
        let Some(s) = battle.ninjutsu.smoke.get(cloud.0) else {
            *visible = Visibility::Hidden;
            continue;
        };
        let r = s.radius(tick);
        *visible = if r > 0.02 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let angle = cloud.1 as f32 * 2.399_963 + tick * 0.014;
        let layer = (cloud.1 % 3) as f32;
        let radial = if cloud.1 < 3 { 0.1 } else { 0.52 };
        let p = Vec3::from_array(s.position)
            + Vec3::new(
                angle.cos() * r * radial,
                0.35 + r * (0.18 + layer * 0.30),
                angle.sin() * r * radial,
            );
        let rotation = cameras
            .iter()
            .next()
            .map_or(Quat::IDENTITY, |c| c.compute_transform().rotation);
        *transform = Transform::from_translation(p)
            .with_rotation(rotation * Quat::from_rotation_z(angle * 0.3))
            .with_scale(Vec3::new(r * 2.1, r * (1.4 + layer * 0.12), 1.));
    }
    for (disc, mut transform, mut visible) in &mut discs {
        let Some(w) = battle.ninjutsu.wheels.get(disc.0) else {
            *visible = Visibility::Hidden;
            continue;
        };
        *visible = Visibility::Inherited;
        let position = if session.running {
            Vec3::from_array(w.previous).lerp(
                Vec3::from_array(w.position),
                (session.accumulator / phalanx::DT).clamp(0., 1.),
            )
        } else {
            Vec3::from_array(w.position)
        };
        *transform = Transform::from_translation(position)
            .with_rotation(Quat::from_rotation_y(tick * 1.8) * Quat::from_rotation_z(0.22));
    }
    let hook = battle
        .ninjutsu
        .events
        .iter()
        .rev()
        .find(|e| e.art == Art::ChainSnare && battle.tick.saturating_sub(e.tick) < 7);
    for (link, mut transform, mut visible) in &mut links {
        let Some(e) = hook else {
            *visible = Visibility::Hidden;
            continue;
        };
        *visible = Visibility::Inherited;
        let a = Vec3::from_array(rules::body(&battle.units[e.caster], battle.arena, tick));
        let b = Vec3::from_array(rules::body(
            &battle.units[e.target.unwrap()],
            battle.arena,
            tick,
        ));
        let t = link.0 as f32 / 31.;
        let delta = b - a;
        *transform = Transform::from_translation(
            a.lerp(b, t) + Vec3::Y * (std::f32::consts::PI * t).sin() * 0.12,
        )
        .with_rotation(
            Quat::from_rotation_arc(Vec3::Z, delta.normalize_or(Vec3::Z))
                * Quat::from_rotation_z(if link.0 % 2 == 0 {
                    0.
                } else {
                    std::f32::consts::FRAC_PI_2
                }),
        )
        .with_scale(Vec3::new(0.04, 0.04, delta.length() / 32. * 0.60));
    }
    for (seal, mut transform, mut visible) in &mut seals {
        let Some(u) = battle.units.get(seal.0).filter(|u| u.health > 0.) else {
            *visible = Visibility::Hidden;
            continue;
        };
        let active = battle.ninjutsu.events.iter().rev().any(|e| {
            e.art == Art::GraveBind
                && e.success
                && e.target == Some(u.id)
                && battle.tick.saturating_sub(e.tick) < 22
        });
        *visible = if active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if !active {
            continue;
        }
        let p = session.presented(u.id);
        let at = Vec3::new(p[0], battle.arena.elevation(p) + 0.05, p[1]);
        if seal.1 == 0 {
            *transform = Transform::from_translation(at).with_scale(Vec3::new(0.85, 0.12, 0.85));
        } else {
            let angle = seal.1 as f32 * std::f32::consts::TAU / 8. + tick * 0.09;
            *transform = Transform::from_translation(
                at + Vec3::new(
                    angle.cos() * 0.75,
                    0.4 + (tick * 0.2 + seal.1 as f32).sin() * 0.17,
                    angle.sin() * 0.75,
                ),
            )
            .with_rotation(Quat::from_rotation_y(-angle));
        }
    }
}

fn demo_setup(mut session: ResMut<Session>) {
    use phalanx::{FighterClass as C, equipment::Gear, learned::Controller, progression::Campaign};
    session.autosave = false;
    session.campaign = Campaign::new();
    for team in &mut session.campaign.teams {
        for (f, class) in team
            .fighters
            .iter_mut()
            .zip([C::Scout, C::Hexer, C::Paladin])
        {
            f.class = class;
            f.mind = phalanx::Mind::disciplined();
        }
    }
    for team in 0..2 {
        session
            .campaign
            .workshop(team, 0, Gear::Kusarigama)
            .unwrap();
        session
            .campaign
            .workshop(team, 1, Gear::RazorWheel)
            .unwrap();
    }
    let seed = session.battle.seed;
    session.restart_founders(seed, Arena::Choke, Controller::Utility);
    for (u, p) in session.battle.units.iter_mut().zip([
        [-2., -5.],
        [-3., -0.8],
        [-2., 1.2],
        [2., -5.],
        [3., -0.8],
        [2., 1.2],
    ]) {
        u.position = p;
        u.target = [-p[0], p[1]];
    }
    session.remember_positions();
    session.founding.open = false;
    session.menu = false;
    session.map_briefing = false;
    session.running = false;
    session.speed = 1.;
    session.cinema_on = true;
    session.cinema_grabbed = false;
    session.player.details = false;
    session.player.research = false;
}
#[derive(Default)]
struct Demo {
    seconds: f32,
    started: bool,
    next: f32,
    frame: u32,
    ended: f32,
}
#[allow(clippy::too_many_arguments)]
fn demo(
    time: Res<Time>,
    mut session: ResMut<Session>,
    audit: Res<Audit>,
    stains: Res<super::stains::Stains>,
    mut state: Local<Demo>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    state.seconds += time.delta_secs();
    if state.seconds > 3. && !state.started {
        state.started = true;
        session.running = true;
        state.next = state.seconds + 0.1;
    }
    if session.battle.finished {
        state.ended += time.delta_secs();
    }
    if state.started && state.seconds < 35.5 && state.seconds >= state.next && state.ended < 1. {
        std::fs::create_dir_all("outputs/ninja-demo").unwrap();
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(format!(
                "outputs/ninja-demo/{:05}.png",
                state.frame
            )));
        state.frame += 1;
        state.next += 1. / 24.;
    }
    if state.seconds > 37. || state.ended > 2. {
        let record = serde_json::json!({"scripted_fixture":true,"human_playtest":false,"real_combat":true,"autosave":session.autosave,"seed":session.battle.seed,"tick":session.battle.tick,"finished":session.battle.finished,"frames":state.frame,"arts":audit.arts,"physical_wall_height":audit.max_wall_height,"smoke_render_frames":audit.smoke_frames,"smoke_hid_contacts":audit.hidden_contacts,"rigid_blood_contacts":stains.rigid_hits,"smoke_capacity":rules::SMOKE_CAPACITY,"wheel_capacity":rules::WHEEL_CAPACITY});
        std::fs::create_dir_all("outputs/ninja-demo").unwrap();
        std::fs::write(
            "outputs/ninja-demo/audit.json",
            serde_json::to_string_pretty(&record).unwrap(),
        )
        .unwrap();
        std::fs::write(
            "outputs/ninja-demo/events.json",
            serde_json::to_string_pretty(&session.battle.ninjutsu.events).unwrap(),
        )
        .unwrap();
        println!("NINJA_DEMO {record}");
        exit.write(AppExit::Success);
    }
}
