//! Bounded sword ribbons, impact sparks and landing dust, driven by real poses/events.
use super::*;
use std::collections::HashMap;

const CAPACITY: usize = 384;

#[derive(Component)]
pub(super) struct Tip {
    pub id: usize,
    pub female: bool,
}
#[derive(Component)]
struct Particle(usize);
#[derive(Clone, Copy, Default)]
enum Kind {
    #[default]
    Spark,
    Trail,
    Dust,
    Ring,
}
#[derive(Clone, Copy, Default)]
struct Mote {
    kind: Kind,
    position: Vec3,
    velocity: Vec3,
    end: Vec3,
    life: f32,
    duration: f32,
    size: f32,
}
#[derive(Resource)]
struct Effects {
    motes: Vec<Mote>,
    cursor: usize,
    run: Option<u32>,
    event: Option<u64>,
    tips: HashMap<Entity, Vec3>,
    landings: HashMap<usize, (usize, f32)>,
}
impl Default for Effects {
    fn default() -> Self {
        Self {
            motes: vec![Mote::default(); CAPACITY],
            cursor: 0,
            run: None,
            event: None,
            tips: HashMap::new(),
            landings: HashMap::new(),
        }
    }
}
impl Effects {
    fn emit(&mut self, mote: Mote) {
        let index = self.cursor % CAPACITY;
        self.motes[index] = mote;
        self.cursor += 1;
    }
    fn landing(&mut self, position: Vec3) {
        self.emit(Mote {
            kind: Kind::Ring,
            position: position + Vec3::Y * 0.04,
            life: 0.38,
            duration: 0.38,
            size: 0.7,
            ..default()
        });
        for i in 0..10 {
            let angle = i as f32 * std::f32::consts::TAU / 10.;
            self.emit(Mote {
                kind: Kind::Dust,
                position: position + Vec3::Y * 0.08,
                velocity: Vec3::new(angle.cos() * 1.6, 0.5, angle.sin() * 1.6),
                life: 0.48,
                duration: 0.48,
                size: 0.10,
                ..default()
            });
        }
    }
}
#[derive(Resource, Default)]
struct Kit {
    cube: Handle<Mesh>,
    sphere: Handle<Mesh>,
    ring: Handle<Mesh>,
    light: Handle<StandardMaterial>,
    dust: Handle<StandardMaterial>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn impact_pool_is_bounded_and_paused_motion_preserves_resolved_combat() {
        let mut fx = Effects::default();
        for _ in 0..1000 {
            fx.landing(Vec3::ZERO);
        }
        assert_eq!(fx.motes.len(), CAPACITY);
        assert!(fx.motes.iter().all(|m| m.life > 0.));
        let mut session = Session::new(phalanx::Battle::new(42, false));
        session.autosave = false;
        session.menu = false;
        session.running = false;
        session.battle.damage_events.push(phalanx::DamageEvent {
            sequence: 0,
            tick: 1,
            attacker: 0,
            defender: 5,
            amount: 10.,
            velocity: [1., 0.],
            position: [0., 0.],
            severed: 0,
            blunt: false,
        });
        let before = serde_json::to_string(&session.battle).unwrap();
        let mut app = App::new();
        app.insert_resource(session)
            .insert_resource(Time::<()>::default())
            .init_resource::<Effects>()
            .init_resource::<Audit>()
            .init_resource::<Kit>()
            .add_systems(Update, update);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));
        app.update();
        let life = app.world().resource::<Effects>().motes[0].life;
        app.update();
        assert_eq!(app.world().resource::<Effects>().motes[0].life, life);
        assert_eq!(app.world().resource::<Audit>().impacts, 1);
        assert_eq!(
            serde_json::to_string(&app.world().resource::<Session>().battle).unwrap(),
            before
        );
        app.world_mut().resource_mut::<Session>().running = true;
        app.update();
        assert!(app.world().resource::<Effects>().motes[0].life < life);
    }
}
#[derive(Resource, Default)]
pub(super) struct Audit {
    pub trails: u32,
    pub impacts: u32,
    pub landings: u32,
    pub air_clips: [bool; 3],
    pub peak_particles: usize,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Effects>()
        .init_resource::<Audit>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            update.after(animate).after(super::super::sync_pawns),
        );
    if std::env::args().any(|a| a == "--wirefu-demo") {
        app.add_systems(Startup, demo_setup.after(load_visuals))
            .add_systems(Update, demo.after(update));
    }
}

fn demo_setup(mut session: ResMut<Session>) {
    use phalanx::{FighterClass as C, learned::Controller, progression::Campaign};
    session.autosave = false;
    session.campaign = Campaign::new();
    for (team, classes) in [
        [C::Warrior, C::Scout, C::Paladin],
        [C::Guardian, C::Warrior, C::Scout],
    ]
    .into_iter()
    .enumerate()
    {
        for (fighter, class) in session.campaign.teams[team]
            .fighters
            .iter_mut()
            .zip(classes)
        {
            fighter.class = class;
        }
    }
    let seed = session.battle.seed;
    session.restart_founders(seed, Arena::OpenField, Controller::Utility);
    for unit in &mut session.battle.units {
        unit.position = [
            if unit.team == 0 { -4.5 } else { 4.5 },
            (unit.id % 3) as f32 * 1.7 - 1.7,
        ];
        unit.target = [-unit.position[0], unit.position[1]];
    }
    session.remember_positions();
    session.founding.open = false;
    session.menu = false;
    session.map_briefing = false;
    session.running = true;
    session.speed = 1.;
    session.cinema_on = true;
    session.cinema_grabbed = false;
    session.player.details = false;
    session.player.research = false;
}
#[derive(Default)]
struct Demo {
    seconds: f32,
    next: f32,
    frame: u32,
    shots: [bool; 6],
    ended: f32,
}
fn demo(
    time: Res<Time>,
    session: Res<Session>,
    audit: Res<Audit>,
    music: Res<super::super::audio::Audit>,
    mut state: Local<Demo>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    state.seconds += time.delta_secs();
    state.shots[session.cinema.shot() as usize] = true;
    if session.battle.finished {
        state.ended += time.delta_secs();
    }
    if state.seconds > 2. && state.seconds < 33. && state.ended < 1.0 && state.seconds >= state.next
    {
        std::fs::create_dir_all("outputs/wirefu-demo").expect("create wire-fu review directory");
        let path = format!("outputs/wirefu-demo/{:05}.png", state.frame);
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(path));
        if state.frame == 0 {
            state.next = state.seconds;
        }
        state.frame += 1;
        state.next += 1.0 / 24.0;
    }
    if state.seconds > 34. || state.ended > 2. {
        let record = serde_json::json!({
            "scripted_fixture":true,"real_combat":true,"campaign_autosave":session.autosave,
            "seed":session.battle.seed,"tick":session.battle.tick,"finished":session.battle.finished,
            "frames":state.frame,"camera_shots":state.shots,"cinema_on":session.cinema_on,
            "cinema_grabbed":session.cinema_grabbed,"trails":audit.trails,"impacts":audit.impacts,
            "landings":audit.landings,"air_clips":audit.air_clips,"peak_particles":audit.peak_particles,
            "particle_capacity":CAPACITY,"music_layer_peaks":music.music_peak,
        });
        std::fs::write(
            "outputs/wirefu-demo/audit.json",
            serde_json::to_string_pretty(&record).unwrap(),
        )
        .unwrap();
        println!("WIREFU_DEMO {record}");
        exit.write(AppExit::Success);
    }
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube = meshes.add(Cuboid::new(1., 1., 1.));
    let light = materials.add(StandardMaterial {
        base_color: Color::srgb(1., 0.86, 0.54),
        emissive: LinearRgba::new(3.0, 1.8, 0.55, 1.),
        unlit: true,
        ..default()
    });
    for i in 0..CAPACITY {
        commands.spawn((
            Particle(i),
            Mesh3d(cube.clone()),
            MeshMaterial3d(light.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    commands.insert_resource(Kit {
        cube,
        sphere: meshes.add(Sphere::new(1.).mesh().ico(1).unwrap()),
        ring: meshes.add(Torus::new(0.48, 0.52)),
        light,
        dust: materials.add(StandardMaterial {
            base_color: Color::srgba(0.55, 0.47, 0.32, 0.35),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
    });
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update(
    session: Res<Session>,
    time: Res<Time>,
    kit: Res<Kit>,
    mut fx: ResMut<Effects>,
    mut audit: ResMut<Audit>,
    tips: Query<(Entity, &Tip, &GlobalTransform, &InheritedVisibility)>,
    actors: Query<&Actor>,
    mut particles: Query<(
        &Particle,
        &mut Transform,
        &mut Visibility,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    if fx.run != Some(session.run_id) {
        *fx = Effects {
            run: Some(session.run_id),
            ..default()
        };
        *audit = Audit::default();
    }
    let dt = if session.running {
        time.delta_secs().min(0.1) * session.speed
    } else if session.battle.finished {
        time.delta_secs().min(0.1)
    } else {
        0.
    };
    for mote in &mut fx.motes {
        mote.life = (mote.life - dt).max(0.);
        if mote.life == 0. {
            continue;
        }
        if matches!(mote.kind, Kind::Spark | Kind::Dust) {
            mote.velocity.y -= dt * 4.;
            let velocity = mote.velocity;
            mote.position += velocity * dt;
        }
    }
    for actor in &actors {
        if actor.female != female(&session, actor.id) {
            continue;
        }
        if matches!(actor.clip, 17..=19) {
            audit.air_clips[actor.clip - 17] = true;
            let phase = actor.clock / CLIP_DURATIONS[actor.clip];
            let previous = fx.landings.insert(actor.id, (actor.clip, phase));
            if previous
                .is_some_and(|(clip, before)| clip == actor.clip && before < 0.97 && phase >= 0.97)
            {
                let p = session.presented(actor.id);
                fx.landing(Vec3::new(p[0], session.battle.arena.elevation(p), p[1]));
                audit.landings += 1;
            }
        } else if let Some((_, phase)) = fx.landings.remove(&actor.id)
            && (0.4..0.97).contains(&phase)
            && session.visual_unit(actor.id).is_some_and(|u| u.health > 0.)
        {
            // A hit can interrupt an aerial clip. Its return to a grounded pose
            // still needs the same recovery dust as an uninterrupted landing.
            let p = session.presented(actor.id);
            fx.landing(Vec3::new(p[0], session.battle.arena.elevation(p), p[1]));
            audit.landings += 1;
        }
    }
    for (entity, tip, transform, visible) in &tips {
        if !visible.get() || tip.female != female(&session, tip.id) {
            fx.tips.remove(&entity);
            continue;
        }
        let p = transform.translation();
        let previous = fx.tips.insert(entity, p);
        let swinging = actors.iter().any(|a| {
            a.id == tip.id
                && a.female == tip.female
                && matches!(a.clip, 3 | 7..=9 | 17..=19)
                && a.clock > 0.03
                && a.clock < CLIP_DURATIONS[a.clip] * 0.85
        });
        if dt > 0.
            && swinging
            && let Some(old) = previous
        {
            let distance = old.distance(p);
            if (0.015..1.5).contains(&distance) {
                fx.emit(Mote {
                    kind: Kind::Trail,
                    position: old,
                    end: p,
                    life: 0.13,
                    duration: 0.13,
                    size: 0.045,
                    ..default()
                });
                audit.trails += 1;
            }
        }
    }
    for event in &session.battle.damage_events {
        if fx.event.is_some_and(|sequence| event.sequence <= sequence) {
            continue;
        }
        fx.event = Some(event.sequence);
        let position = Vec3::new(
            event.position[0],
            session.battle.arena.elevation(event.position) + 0.75,
            event.position[1],
        );
        for i in 0..8 {
            let angle = i as f32 * std::f32::consts::TAU / 8.;
            fx.emit(Mote {
                position,
                velocity: Vec3::new(
                    angle.cos() * 2.3 + event.velocity[0] * 0.08,
                    1.2 + (i % 3) as f32 * 0.35,
                    angle.sin() * 2.3 + event.velocity[1] * 0.08,
                ),
                life: 0.22,
                duration: 0.22,
                size: 0.035,
                ..default()
            });
        }
        audit.impacts += 1;
    }
    let mut active = 0;
    for (particle, mut transform, mut visible, mut mesh, mut material) in &mut particles {
        let mote = fx.motes[particle.0];
        *visible = if mote.life > 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if mote.life <= 0. {
            continue;
        }
        active += 1;
        let fraction = mote.life / mote.duration;
        *transform = Transform::from_translation(mote.position);
        mesh.0 = kit.cube.clone();
        material.0 = kit.light.clone();
        match mote.kind {
            Kind::Trail => {
                let delta = mote.end - mote.position;
                transform.translation = (mote.position + mote.end) * 0.5;
                transform.rotation = Quat::from_rotation_arc(Vec3::Y, delta.normalize_or_zero());
                transform.scale = Vec3::new(mote.size * fraction, delta.length(), 0.018 * fraction);
            }
            Kind::Spark => {
                transform.rotation =
                    Quat::from_rotation_arc(Vec3::Y, mote.velocity.normalize_or_zero());
                transform.scale =
                    Vec3::new(mote.size * fraction, 0.12 * fraction, mote.size * fraction);
            }
            Kind::Dust => {
                mesh.0 = kit.sphere.clone();
                material.0 = kit.dust.clone();
                transform.scale = Vec3::splat(mote.size * fraction * (3. - 2. * fraction));
            }
            Kind::Ring => {
                mesh.0 = kit.ring.clone();
                transform.scale = Vec3::new(
                    mote.size * (3. - 2. * fraction),
                    0.10 * fraction,
                    mote.size * (3. - 2. * fraction),
                );
            }
        }
    }
    audit.peak_particles = audit.peak_particles.max(active);
}
