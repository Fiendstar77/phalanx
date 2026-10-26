//! Saved-state character sheet, physical inventory, skill selection and animated forge.
use super::*;
use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::render::render_resource::TextureFormat;
use phalanx::{
    equipment::{Gear, Item},
    inventory::{Destination, Location},
    skills::{Skill, SkillLoadout},
};
pub const PREVIEW_ID: usize = 20;
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Craft(Gear),
    Forge(Location),
    Transfer(Location, Destination),
    Skill(bool, usize, Option<Skill>),
}
pub fn apply(action: Action, session: &mut Session) {
    if session.exchange.live.is_some() {
        session.save_notice =
            "Challenge teams are frozen. Return to your campaign to change equipment or skills."
                .into();
        return;
    }
    if !session.battle.founders || (session.battle.tick > 0 && !session.battle.finished) {
        session.save_notice = "Prepare equipment and skills before or after battle".into();
        return;
    }
    let u = &session.battle.units[session.selected];
    let (team, slot) = (u.team, u.id % session.battle.team_size());
    let result = match action {
        Action::Craft(g) => session.campaign.craft_item(team, slot, g),
        Action::Forge(l) => session.campaign.forge_item(team, slot, l),
        Action::Transfer(l, d) => session.campaign.transfer_item(team, slot, l, d),
        Action::Skill(p, i, s) => session.campaign.set_skill(team, slot, p, i, s),
    };
    match result {
        Ok(message) => {
            if session.battle.tick == 0 && !session.battle.finished {
                let f = &session.campaign.teams[team].fighters[slot];
                let u = &mut session.battle.units[session.selected];
                u.equipment = f.equipment;
                u.skills = f.skills;
                u.reset_weapon();
                u.health = u.max_health();
            }
            if session.autosave && !session.save_campaign() {
                session.save_notice = format!("{message}. {}", session.save_notice);
            } else {
                session.save_notice = message;
            }
        }
        Err(e) => session.save_notice = e,
    }
    session.refresh_preview();
}
pub struct Rpg;
impl Plugin for Rpg {
    fn build(&self, app: &mut App) {
        if std::env::args().any(|a| a == "--rpg-pointer-witness") {
            app.add_systems(Update, witness_input.before(input).after(refresh));
        }
        app.init_resource::<SheetState>()
            .add_systems(
                Startup,
                preview_setup.after(setup).before(visuals::load_visuals),
            )
            .add_systems(Startup, ui_setup.after(preview_setup))
            .add_systems(PreUpdate, sync_saved_preview)
            .add_systems(
                Update,
                (
                    refresh.after(controls),
                    input.after(refresh),
                    preview_update,
                    forge_animation,
                    scroll,
                ),
            );
    }
}
#[derive(Resource)]
struct Photo(Handle<Image>);
#[derive(Component)]
pub struct CampCamera;
#[derive(Component)]
struct PreviewRoot;
#[derive(Component)]
struct Screen(camp::Page);
#[derive(Component, Clone, Copy)]
enum Label {
    Header,
    Identity,
    Limbs,
    Notice,
    Forge,
    Description,
}
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Tile {
    Personal(usize),
    Shared(usize),
    Equipped(usize),
    Catalog(Skill),
    Active(usize),
    Passive(usize),
    Recipe(Gear),
    Workbench,
    ForgeButton,
    Turn(i8),
}
#[derive(Component)]
struct TileText(Tile);
#[derive(Component)]
struct DropArea(Destination);
#[derive(Component)]
struct Scroller;
#[derive(Component)]
struct ForgeGlow;
#[derive(Component)]
struct Hammer;
#[derive(Component)]
struct Spark(usize);
#[derive(Component)]
struct DragGhost;
#[derive(Clone, Copy)]
enum Payload {
    Item(Location),
    Skill(Skill),
}
#[derive(Resource, Default)]
struct SheetState {
    selected: Option<usize>,
    focus: Option<Location>,
    drag: Option<Payload>,
    start: Vec2,
    angle: f32,
    notice: String,
    forge_clock: f32,
}
fn sync_saved_preview(mut session: ResMut<Session>) {
    session.refresh_preview();
}
fn preview_setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Res<PawnAssets>,
) {
    let photo = images.add(Image::new_target_texture(
        600,
        800,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    ));
    commands.insert_resource(Photo(photo.clone()));
    let band = meshes.add(Cylinder::new(0.25, 0.10));
    let tip = meshes.add(Cone::new(0.07, 0.17));
    commands
        .spawn((
            Pawn(PREVIEW_ID),
            PreviewRoot,
            Transform::from_xyz(1000., 0.64, 0.),
            Visibility::Inherited,
        ))
        .with_children(|p| {
            p.spawn((
                Insignia(PREVIEW_ID, true),
                visuals::CrownAnchor {
                    id: PREVIEW_ID,
                    up: visuals::CROWN_BAND_UP,
                    ring: 0.,
                    angle: 0.,
                },
                Mesh3d(band),
                MeshMaterial3d(assets.gold.clone()),
                Transform::from_xyz(0., 0.71, 0.),
                Visibility::Hidden,
            ));
            for point in 0..5 {
                let angle = point as f32 * std::f32::consts::TAU / 5.;
                p.spawn((
                    Insignia(PREVIEW_ID, true),
                    visuals::CrownAnchor {
                        id: PREVIEW_ID,
                        up: visuals::CROWN_TIP_UP,
                        ring: visuals::CROWN_RING,
                        angle,
                    },
                    Mesh3d(tip.clone()),
                    MeshMaterial3d(assets.gold.clone()),
                    Transform::from_xyz(angle.cos() * 0.19, 0.81, angle.sin() * 0.19),
                    Visibility::Hidden,
                ));
            }
        });
    commands.spawn((
        CampCamera,
        visuals::PortraitCamera,
        Camera3d::default(),
        Camera {
            order: -2,
            clear_color: Color::srgb(0.028, 0.042, 0.057).into(),
            ..default()
        },
        RenderTarget::Image(photo.into()),
        RenderLayers::layer(2),
        Transform::from_xyz(1000., 0.95, 2.9).looking_at(Vec3::new(1000., 0.82, 0.), Vec3::Y),
    ));
    commands.spawn((
        RenderLayers::layer(2),
        DirectionalLight {
            illuminance: 13000.,
            ..default()
        },
        Transform::from_xyz(1002., 4., 3.).looking_at(Vec3::new(1000., 0.7, 0.), Vec3::Y),
    ));
}
fn preview_update(
    session: Res<Session>,
    state: Res<SheetState>,
    mut roots: Query<(Entity, &mut Transform), With<PreviewRoot>>,
    children: Query<&Children>,
    layers: Query<&RenderLayers>,
    mut commands: Commands,
) {
    for (root, mut t) in &mut roots {
        t.rotation = Quat::from_rotation_y(state.angle);
        for child in children.iter_descendants(root) {
            if !layers
                .get(child)
                .is_ok_and(|l| *l == RenderLayers::layer(2))
            {
                commands.entity(child).insert(RenderLayers::layer(2));
            }
        }
    }
    let _ = &session;
}
fn forge_rect(p: &mut ChildSpawnerCommands, x: f32, y: f32, w: f32, h: f32, c: Color) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: percent(x),
            top: px(y),
            width: px(w),
            height: px(h),
            ..default()
        },
        BackgroundColor(c),
    ));
}
fn heading(p: &mut ChildSpawnerCommands, text: &str, size: f32) {
    p.spawn((
        Text::new(text),
        font(size),
        TextColor(Color::srgb(0.94, 0.78, 0.43)),
    ));
}
fn label(p: &mut ChildSpawnerCommands, kind: Label, size: f32) {
    p.spawn((
        kind,
        Text::new(""),
        font(size),
        TextColor(Color::srgb(0.79, 0.85, 0.89)),
    ));
}
fn tile(p: &mut ChildSpawnerCommands, kind: Tile, height: f32) {
    p.spawn((
        kind,
        Button,
        Node {
            width: if matches!(kind, Tile::Turn(_)) {
                percent(48)
            } else {
                percent(100)
            },
            min_height: px(height),
            padding: UiRect::all(px(8)),
            border: UiRect::all(px(1)),
            flex_shrink: 0.,
            ..default()
        },
        BorderColor::all(Color::srgb(0.19, 0.28, 0.34)),
        BackgroundColor(Color::srgb(0.065, 0.10, 0.13)),
    ))
    .with_children(|p| {
        p.spawn((
            TileText(kind),
            Text::new(""),
            font(13.),
            TextColor(Color::srgb(0.81, 0.86, 0.88)),
        ));
    });
}
fn bag(p: &mut ChildSpawnerCommands, shared: bool) {
    let dest = if shared {
        Destination::Shared
    } else {
        Destination::Personal
    };
    p.spawn((
        DropArea(dest),
        Node {
            width: if shared { px(245) } else { px(210) },
            flex_shrink: 0.,
            height: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.035, 0.055, 0.073)),
    ))
    .with_children(|p| {
        heading(
            p,
            if shared {
                "PHALANX CHEST"
            } else {
                "PERSONAL INVENTORY"
            },
            15.,
        );
        heading(
            p,
            if shared {
                "Drop here to share equipment"
            } else {
                "Drop here to carry / unequip"
            },
            11.,
        );
        p.spawn((
            Scroller,
            ScrollPosition::default(),
            Node {
                flex_grow: 1.,
                min_height: px(0),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                ..default()
            },
        ))
        .with_children(|p| {
            for i in 0..if shared { 128 } else { 24 } {
                tile(
                    p,
                    if shared {
                        Tile::Shared(i)
                    } else {
                        Tile::Personal(i)
                    },
                    55.,
                );
            }
        });
    });
}
fn roster(p: &mut ChildSpawnerCommands) {
    p.spawn(Node {
        width: px(155),
        height: percent(100),
        flex_shrink: 0.,
        flex_direction: FlexDirection::Column,
        row_gap: px(6),
        ..default()
    })
    .with_children(|p| {
        heading(p, "PHALANX MEMBERS", 14.);
        for id in 0..8 {
            p.spawn((
                Button,
                Control::SelectFighter(id),
                PressLatch::default(),
                Node {
                    padding: UiRect::all(px(10)),
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BorderColor::all(Color::srgb(0.19, 0.28, 0.34)),
                BackgroundColor(Color::srgb(0.065, 0.10, 0.13)),
            ))
            .with_children(|p| {
                p.spawn((RosterText(id), Text::new(""), font(13.)));
            });
        }
    });
}
#[derive(Component)]
struct RosterText(usize);
fn ui_setup(mut commands: Commands, photo: Res<Photo>) {
    for page in [camp::Page::Workshop, camp::Page::Skills, camp::Page::Forge] {
        commands.spawn((Screen(page),GlobalZIndex(110),Node{position_type:PositionType::Absolute,width:percent(100),height:percent(100),display:Display::None,justify_content:JustifyContent::Center,align_items:AlignItems::Center,..default()},BackgroundColor(Color::srgba(0.012,0.021,0.03,0.99)))).with_children(|root|{
            root.spawn((Node{width:vw(96),max_width:px(1540),height:vh(96),padding:UiRect::all(px(18)),flex_direction:FlexDirection::Column,row_gap:px(10),..default()},BackgroundColor(Color::srgb(0.025,0.04,0.055)))).with_children(|p|{
                label(p,Label::Header,23.);
                p.spawn(Node{flex_wrap:FlexWrap::Wrap,..default()}).with_children(|p|{
                    for (name,page) in [("Character",camp::Page::Workshop),("Skills",camp::Page::Skills),("Forge",camp::Page::Forge),("Training & titles",camp::Page::Training),("Battle report",camp::Page::Report),("Rivals",camp::Page::Rivals)]{button(p,name,Control::Camp(page));}
                    button(p,"Return to arena",Control::Continue);
                });
                p.spawn(Node{flex_grow:1.,min_height:px(0),column_gap:px(14),..default()}).with_children(|body|{
                    roster(body);
                    if page==camp::Page::Workshop {
                        bag(body,false);
                        body.spawn(Node{flex_grow:1.,min_width:px(260),flex_direction:FlexDirection::Column,row_gap:px(8),..default()}).with_children(|p|{
                            label(p,Label::Identity,17.);
                            p.spawn(Node{flex_grow:1.,min_height:px(120),column_gap:px(10),..default()}).with_children(|row|{
                                row.spawn(Node{flex_grow:1.,min_width:px(0),justify_content:JustifyContent::Center,..default()}).with_children(|p|{p.spawn((ImageNode::new(photo.0.clone()),Node{height:percent(100),aspect_ratio:Some(0.75),max_width:percent(100),..default()}));});
                                row.spawn(Node{width:px(185),flex_shrink:0.,flex_direction:FlexDirection::Column,row_gap:px(10),justify_content:JustifyContent::Center,..default()}).with_children(|p|{for slot in 0..3{tile(p,Tile::Equipped(slot),85.);}});
                            });
                            p.spawn(Node{column_gap:px(8),..default()}).with_children(|p|{tile(p,Tile::Turn(-1),28.);tile(p,Tile::Turn(1),28.);});
                            label(p,Label::Limbs,13.);
                            button(p,"Fit missing limb | 15 gold + 5 salvage",Control::Prosthetic);
                        });
                        bag(body,true);
                    } else if page==camp::Page::Skills {
                        body.spawn(Node{width:px(420),flex_shrink:0.,flex_direction:FlexDirection::Column,row_gap:px(6),..default()}).with_children(|p|{
                            heading(p,"10 ACTIVE SLOTS / 3 PASSIVES",17.);
                            heading(p,"Drag a learned skill into a slot. Right-click a slot to clear it.",12.);
                            p.spawn((Scroller,ScrollPosition::default(),Node{flex_grow:1.,min_height:px(0),overflow:Overflow::scroll_y(),flex_direction:FlexDirection::Column,row_gap:px(5),..default()})).with_children(|p|{
                                for i in 0..10{tile(p,Tile::Active(i),40.);} heading(p,"PASSIVES | choose three",15.);for i in 0..3{tile(p,Tile::Passive(i),45.);}
                            });
                        });
                        body.spawn(Node{flex_grow:1.,flex_direction:FlexDirection::Column,row_gap:px(8),..default()}).with_children(|p|{
                            heading(p,"LEARNED DISCIPLINES",20.);
                            heading(p,"Current discipline is available now. At 100 practice, its skills stay unlocked across retraining. Empty slots leave room for future skills.",13.);
                            p.spawn((Scroller,ScrollPosition::default(),Node{flex_grow:1.,min_height:px(0),overflow:Overflow::scroll_y(),flex_direction:FlexDirection::Column,row_gap:px(6),..default()})).with_children(|p|{for s in Skill::ALL{tile(p,Tile::Catalog(s),65.);}});
                        });
                    } else {
                        bag(body,false);
                        body.spawn(Node{flex_grow:1.,min_width:px(280),flex_direction:FlexDirection::Column,row_gap:px(10),..default()}).with_children(|p|{
                            heading(p,"THE CRUCIBLE",24.);
                            heading(p,"Select an item in your inventory or drag it onto the anvil.\nUpgrade to level 5, then melt it into a stronger alloy.\nIron -> Steel -> Crucible steel -> Meteoric alloy",13.);
                            p.spawn((ForgeGlow,Node{height:px(135),position_type:PositionType::Relative,justify_content:JustifyContent::Center,align_items:AlignItems::Center,overflow:Overflow::clip(),..default()},BackgroundColor(Color::srgb(0.20,0.065,0.018)))).with_children(|p|{
                                forge_rect(p,15.,20.,88.,100.,Color::srgb(0.16,0.10,0.08));
                                forge_rect(p,17.,38.,64.,68.,Color::srgb(0.94,0.27,0.025));
                                forge_rect(p,18.,62.,44.,40.,Color::srgb(1.,0.72,0.10));
                                forge_rect(p,55.,85.,102.,20.,Color::srgb(0.40,0.45,0.47));
                                forge_rect(p,61.,105.,30.,24.,Color::srgb(0.26,0.30,0.32));
                                forge_rect(p,53.,125.,122.,9.,Color::srgb(0.30,0.33,0.35));
                                forge_rect(p,57.,79.,74.,6.,Color::srgb(1.,0.52,0.08));
                                p.spawn((Hammer,Node{position_type:PositionType::Absolute,left:percent(62),top:px(10),width:px(45),height:px(55),..default()})).with_children(|p|{
                                    p.spawn((Node{position_type:PositionType::Absolute,left:px(19),top:px(15),width:px(7),height:px(38),..default()},BackgroundColor(Color::srgb(0.35,0.19,0.08))));
                                    p.spawn((Node{width:px(45),height:px(22),..default()},BackgroundColor(Color::srgb(0.65,0.71,0.73))));
                                });
                                for i in 0..16{p.spawn((Spark(i),Node{position_type:PositionType::Absolute,width:px(3),height:px(3),..default()},BackgroundColor(Color::srgb(1.,0.65,0.12))));}
                            });
                            tile(p,Tile::Workbench,70.);label(p,Label::Forge,14.);tile(p,Tile::ForgeButton,48.);
                            label(p,Label::Description,13.);
                        });
                        body.spawn(Node{width:px(290),flex_shrink:0.,flex_direction:FlexDirection::Column,row_gap:px(8),..default()}).with_children(|p|{
                            heading(p,"CRAFT NEW EQUIPMENT",17.);
                            p.spawn((Scroller,ScrollPosition::default(),Node{flex_grow:1.,min_height:px(0),overflow:Overflow::scroll_y(),flex_direction:FlexDirection::Column,row_gap:px(6),..default()})).with_children(|p|{for gear in Gear::ALL{tile(p,Tile::Recipe(gear),80.);}});
                        });
                    }
                });
                label(p,Label::Notice,13.);
                p.spawn(Node{flex_wrap:FlexWrap::Wrap,..default()}).with_children(|p|{button(p,"Next battle",Control::NextBattle);button(p,"Save squads",Control::Save);});
            });
        });
    }
    commands.spawn((
        DragGhost,
        GlobalZIndex(500),
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.12, 0.17, 0.21, 0.96)),
        Text::new(""),
        font(14.),
        TextColor(Color::srgb(1., 0.8, 0.3)),
    ));
}
fn selected(session: &Session) -> (usize, usize) {
    let u = &session.battle.units[session.selected];
    (u.team, u.id % session.battle.team_size())
}
fn object(session: &Session, location: Location) -> Option<Item> {
    let (t, f) = selected(session);
    session.campaign.inventory_item(t, f, location).ok()
}
fn location(tile: Tile) -> Option<Location> {
    match tile {
        Tile::Personal(i) => Some(Location::Personal(i)),
        Tile::Shared(i) => Some(Location::Shared(i)),
        Tile::Equipped(i) => Some(Location::Equipped(i)),
        _ => None,
    }
}
fn skill_slots(session: &Session) -> SkillLoadout {
    let (t, f) = selected(session);
    let f = &session.campaign.teams[t].fighters[f];
    f.skills
        .unwrap_or_else(|| SkillLoadout::defaults(f.class, f.mastery))
}
fn skill_at(session: &Session, tile: Tile) -> Option<Skill> {
    let s = skill_slots(session);
    match tile {
        Tile::Active(i) => s.active[i],
        Tile::Passive(i) => s.passive[i],
        Tile::Catalog(s) => Some(s),
        _ => None,
    }
}
fn refresh(
    session: Res<Session>,
    mut state: ResMut<SheetState>,
    mut screens: Query<(&Screen, &mut Node)>,
    mut labels: Query<(&Label, &mut Text)>,
    mut texts: Query<(&TileText, &mut Text), Without<Label>>,
    mut roster: Query<(&RosterText, &mut Text), (Without<Label>, Without<TileText>)>,
    mut tiles: Query<(&Tile, &mut Node, &mut BackgroundColor, &mut BorderColor), Without<Screen>>,
) {
    for (s, mut node) in &mut screens {
        node.display = if session.menu && session.camp_page == Some(s.0) {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !session.battle.founders {
        return;
    }
    if state.selected != Some(session.selected) {
        state.selected = Some(session.selected);
        state.focus = None;
        state.drag = None;
        state.angle = 0.;
    }
    if state.notice != session.save_notice {
        if session.save_notice.starts_with("Forged")
            || session.save_notice.starts_with("Upgraded")
            || session.save_notice.starts_with("Melted")
        {
            state.forge_clock = 1.7;
        }
        state.notice = session.save_notice.clone();
    }
    let (team, slot) = selected(&session);
    let squad = &session.campaign.teams[team];
    let f = &squad.fighters[slot];
    let unit = camp::prepared(&session, session.selected);
    for (kind, mut text) in &mut labels {
        let value=match kind{
        Label::Header=>format!("{} / {} [{}]     |     {} gold    {} salvage",squad.name,f.name,f.title.name(),squad.gold,squad.salvage),
        Label::Identity=>format!("{} / {}   Level {}\n{:.0} HP   Damage x{:.2}   Protection x{:.2}",f.title.name(),f.class.name(),unit.level(),unit.max_health(),unit.damage_multiplier(),unit.incoming_multiplier()),
        Label::Limbs=>format!("Missing: {}\nProsthetics: {}",limb_names(f.lost_limbs),limb_names(f.prosthetics)),
        Label::Notice=>format!("{}{}",if session.battle.tick>0&&!session.battle.finished{"Battle in progress: loadout changes are locked. "}else{"Drag items between the chest, personal inventory and matching equipment slots. "},session.save_notice),
        Label::Forge=>state.focus.and_then(|l|object(&session,l)).map(|i|match i.forged(){Some(next)=>{let(g,s)=i.forge_cost().unwrap();format!("{}\n-> {}\nCost: {g} gold + {s} salvage\nPower x{:.2} -> x{:.2}",i.label(),next.label(),i.power(),next.power())},None=>"Maximum level and alloy reached".into()}).unwrap_or_else(||"No item selected".into()),
        Label::Description=>state.focus.and_then(|l|object(&session,l)).map(|i|format!("{}\nWeapon upgrades increase damage; armor upgrades improve mitigation; vitality upgrades increase HP; banner upgrades strengthen learned rally auras. Alloy fittings preserve the original item type. Reforging consumes the old item and retains its inventory/equipment slot.",i.gear.effect())).unwrap_or_default(),
    };
        if text.0 != value {
            text.0 = value;
        }
    }
    for (rt, mut text) in &mut roster {
        if let Some(u) = session.battle.units.get(rt.0) {
            let f = &session.campaign.teams[u.team].fighters[rt.0 % session.battle.team_size()];
            text.0 = format!(
                "{}\n{}\n{}",
                session.team_name(u.team),
                f.title.name(),
                f.class.name()
            );
        }
    }
    for (kind, mut text) in &mut texts {
        let value = match kind.0 {
            Tile::Personal(i) => unit.equipment.items[i]
                .filter(|_| !unit.equipment.equipped.contains(&Some(i)))
                .map(Item::label)
                .unwrap_or_else(|| "Empty".into()),
            Tile::Shared(i) => squad
                .inventory
                .get(i)
                .map(|i| i.label())
                .unwrap_or_else(|| "Empty chest - drop items here".into()),
            Tile::Equipped(s) => format!(
                "{}\n{}",
                ["WEAPON", "ARMOR", "CHARM / STANDARD"][s],
                unit.equipment
                    .worn(s)
                    .map(Item::label)
                    .unwrap_or_else(|| "Starter / no crafted item".into())
            ),
            Tile::Catalog(s) => format!(
                "{}{} | {} {}\n{}",
                if s.unlocked(f.class, f.mastery) {
                    ""
                } else {
                    "LOCKED / "
                },
                s.name(),
                s.class().name(),
                if s.passive() { "PASSIVE" } else { "ACTIVE" },
                s.description()
            ),
            Tile::Active(i) => format!(
                "{:02}  {}",
                i + 1,
                skill_slots(&session).active[i]
                    .map(Skill::name)
                    .unwrap_or("Empty active slot")
            ),
            Tile::Passive(i) => format!(
                "P{}  {}",
                i + 1,
                skill_slots(&session).passive[i]
                    .map(Skill::name)
                    .unwrap_or("Empty passive slot")
            ),
            Tile::Recipe(g) => {
                let (gold, salvage) = g.cost();
                format!(
                    "CRAFT {}\n{}g + {} salvage\n{}",
                    g.name(),
                    gold,
                    salvage,
                    g.locked_reason(f.mastery)
                        .unwrap_or_else(|| g.effect().into())
                )
            }
            Tile::Workbench => state
                .focus
                .and_then(|l| object(&session, l))
                .map(Item::label)
                .unwrap_or_else(|| "ANVIL | drop or select an item".into()),
            Tile::ForgeButton => state
                .focus
                .and_then(|l| object(&session, l))
                .map(|i| {
                    if i.level == 5 {
                        "MELT & REFORGE"
                    } else {
                        "UPGRADE EQUIPMENT"
                    }
                    .into()
                })
                .unwrap_or_else(|| "SELECT EQUIPMENT FIRST".into()),
            Tile::Turn(sign) => if sign < 0 {
                "Rotate left"
            } else {
                "Rotate right"
            }
            .into(),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    for (tile, mut node, mut color, mut border) in &mut tiles {
        let hide = match *tile {
            Tile::Personal(i) => {
                unit.equipment.equipped.contains(&Some(i))
                    || (unit.equipment.items[i].is_none()
                        && i > unit
                            .equipment
                            .items
                            .iter()
                            .rposition(Option::is_some)
                            .map_or(1, |i| i + 1))
            }
            Tile::Shared(i) => i > squad.inventory.len(),
            _ => false,
        };
        node.display = if hide { Display::None } else { Display::Flex };
        let disabled = match *tile {
            Tile::Catalog(s) => !s.unlocked(f.class, f.mastery),
            Tile::Recipe(g) => {
                let (gold, salvage) = g.cost();
                g.locked_reason(f.mastery).is_some() || squad.gold < gold || squad.salvage < salvage
            }
            Tile::ForgeButton => state
                .focus
                .and_then(|l| object(&session, l))
                .and_then(Item::forge_cost)
                .is_none_or(|(g, s)| squad.gold < g || squad.salvage < s),
            _ => false,
        };
        color.0 = if disabled {
            Color::srgb(0.034, 0.045, 0.053)
        } else {
            Color::srgb(0.065, 0.10, 0.13)
        };
        let focused = location(*tile).is_some_and(|l| state.focus == Some(l));
        let target = match (state.drag, *tile) {
            (Some(Payload::Item(l)), Tile::Equipped(s)) => {
                object(&session, l).is_some_and(|i| i.gear.slot() == s)
            }
            (Some(Payload::Skill(s)), Tile::Active(_)) => !s.passive(),
            (Some(Payload::Skill(s)), Tile::Passive(_)) => s.passive(),
            _ => false,
        };
        *border = BorderColor::all(if focused {
            Color::srgb(0.95, 0.67, 0.22)
        } else if target {
            Color::srgb(0.3, 0.8, 0.65)
        } else {
            Color::srgb(0.19, 0.28, 0.34)
        });
    }
}
fn hit(
    point: Vec2,
    node: &ComputedNode,
    t: &UiGlobalTransform,
    clip: Option<&CalculatedClip>,
) -> bool {
    node.size.x > 0.
        && node.size.y > 0.
        && Rect::from_center_size(t.translation, node.size).contains(point)
        && clip.is_none_or(|c| c.clip.contains(point))
}
#[allow(clippy::too_many_arguments)]
fn input(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    mut session: ResMut<Session>,
    mut state: ResMut<SheetState>,
    tiles: Query<(
        &Tile,
        &ComputedNode,
        &UiGlobalTransform,
        Option<&CalculatedClip>,
    )>,
    areas: Query<(
        &DropArea,
        &ComputedNode,
        &UiGlobalTransform,
        Option<&CalculatedClip>,
    )>,
    mut ghost: Single<(&mut Node, &mut Text), With<DragGhost>>,
) {
    let open = session.menu
        && matches!(
            session.camp_page,
            Some(camp::Page::Workshop | camp::Page::Skills | camp::Page::Forge)
        );
    if !open {
        state.drag = None;
        ghost.0.display = Display::None;
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        if !mouse.pressed(MouseButton::Left) {
            state.drag = None;
            ghost.0.display = Display::None;
        }
        return;
    };
    let point = cursor * window.scale_factor();
    let hovered = tiles
        .iter()
        .find(|(_, n, t, c)| hit(point, n, t, *c))
        .map(|(tile, _, _, _)| *tile);
    if mouse.just_pressed(MouseButton::Right) {
        if let Some(tile) = hovered {
            match tile {
                Tile::Active(i) => apply(Action::Skill(false, i, None), &mut session),
                Tile::Passive(i) => apply(Action::Skill(true, i, None), &mut session),
                _ => {}
            }
        }
    }
    if mouse.just_pressed(MouseButton::Left) {
        state.start = cursor;
        if let Some(tile) = hovered {
            if let Some(location) = location(tile) {
                if object(&session, location).is_some() {
                    state.focus = Some(location);
                    state.drag = Some(Payload::Item(location));
                }
            } else if let Some(s) = skill_at(&session, tile) {
                let (t, f) = selected(&session);
                let f = &session.campaign.teams[t].fighters[f];
                if s.unlocked(f.class, f.mastery) {
                    state.drag = Some(Payload::Skill(s));
                } else {
                    session.save_notice =
                        "Reach 100 practice in this discipline to retain its abilities".into();
                }
            }
            match tile {
                Tile::Recipe(g) => apply(Action::Craft(g), &mut session),
                Tile::ForgeButton => {
                    if let Some(l) = state.focus {
                        apply(Action::Forge(l), &mut session)
                    }
                }
                Tile::Turn(sign) => state.angle += sign as f32 * 0.5,
                _ => {}
            }
        }
    }
    if mouse.just_released(MouseButton::Left) {
        if let Some(payload) = state.drag.take() {
            if cursor.distance(state.start) > 5. {
                match payload {
                    Payload::Item(source) => {
                        if hovered == Some(Tile::Workbench) {
                            state.focus = Some(source);
                        } else {
                            let destination = match hovered {
                                Some(Tile::Equipped(s)) => Some(Destination::Equipped(s)),
                                _ => areas
                                    .iter()
                                    .find(|(_, n, t, c)| hit(point, n, t, *c))
                                    .map(|(area, _, _, _)| area.0),
                            };
                            if let Some(destination) = destination {
                                apply(Action::Transfer(source, destination), &mut session);
                                state.focus = None;
                            }
                        }
                    }
                    Payload::Skill(skill) => {
                        if let Some(tile) = hovered {
                            match tile {
                                Tile::Active(i) => {
                                    apply(Action::Skill(false, i, Some(skill)), &mut session)
                                }
                                Tile::Passive(i) => {
                                    apply(Action::Skill(true, i, Some(skill)), &mut session)
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
    ghost.0.display = if state.drag.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    ghost.0.left = px(cursor.x + 14.);
    ghost.0.top = px(cursor.y + 12.);
    ghost.1.0 = match state.drag {
        Some(Payload::Item(l)) => object(&session, l).map(Item::label).unwrap_or_default(),
        Some(Payload::Skill(s)) => s.name().into(),
        None => String::new(),
    };
}
fn forge_animation(
    time: Res<Time>,
    mut state: ResMut<SheetState>,
    mut glow: Query<&mut BackgroundColor, With<ForgeGlow>>,
    mut hammer: Query<&mut Node, (With<Hammer>, Without<Spark>)>,
    mut sparks: Query<(&Spark, &mut Node), Without<Hammer>>,
) {
    state.forge_clock = (state.forge_clock - time.delta_secs()).max(0.);
    let active = state.forge_clock > 0.;
    let phase = time.elapsed_secs() * 15.;
    for mut c in &mut glow {
        c.0 = Color::srgb(
            if active {
                0.38 + phase.sin() * 0.08
            } else {
                0.18
            },
            if active { 0.10 } else { 0.052 },
            0.015,
        );
    }
    for mut n in &mut hammer {
        n.top = px(if active {
            10. + phase.sin().abs() * 35.
        } else {
            10.
        });
    }
    for (spark, mut n) in &mut sparks {
        n.display = if active { Display::Flex } else { Display::None };
        let t = (time.elapsed_secs() * 2.5 + spark.0 as f32 * 0.173).fract();
        let angle = spark.0 as f32 * 2.4;
        n.left = percent(64. + angle.cos() * t * 30.);
        n.top = px(80. - t * 100. + t * t * 90.);
    }
}
fn scroll(
    session: Res<Session>,
    window: Single<&Window>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut scrollers: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            Option<&CalculatedClip>,
            &mut ScrollPosition,
        ),
        With<Scroller>,
    >,
) {
    for e in wheel.read() {
        if session.menu
            && matches!(
                session.camp_page,
                Some(camp::Page::Workshop | camp::Page::Skills | camp::Page::Forge)
            )
        {
            if let Some(p) = window.cursor_position() {
                for (n, t, c, mut pos) in &mut scrollers {
                    if hit(p * window.scale_factor(), n, t, c) {
                        pos.0.y = (pos.0.y - e.y * 32.).max(0.);
                    }
                }
            }
        }
    }
}

/// Drives the same pointer path as a player. Only installed for isolated visual QA.
fn witness_input(
    time: Res<Time>,
    mut window: Single<&mut Window>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    session: Res<Session>,
    mut state: ResMut<SheetState>,
    tiles: Query<(&Tile, &ComputedNode, &UiGlobalTransform)>,
    areas: Query<(&DropArea, &ComputedNode, &UiGlobalTransform)>,
    mut stage: Local<usize>,
    mut frame: Local<u32>,
    mut done: Local<bool>,
    mut targets: Local<Option<(Vec2, Vec2)>>,
    mut commands: Commands,
) {
    if time.elapsed_secs() < 2.0 || *done {
        return;
    }
    let center = |tile: Tile| {
        tiles
            .iter()
            .find(|(kind, n, _)| **kind == tile && n.size.x > 0.)
            .map(|(_, _, t)| t.translation / window.scale_factor())
    };
    let area = |dest: Destination| {
        areas
            .iter()
            .find(|(kind, n, _)| kind.0 == dest && n.size.x > 0.)
            .map(|(_, n, t)| {
                (t.translation + Vec2::new(0., -n.size.y * 0.44)) / window.scale_factor()
            })
    };
    let forge = std::env::args().any(|a| a == "--forge-witness");
    let skills = std::env::args().any(|a| a == "--skills-witness");
    let positions = if forge {
        if *stage == 0 {
            Some((center(Tile::Personal(3)), center(Tile::Workbench)))
        } else {
            Some((center(Tile::ForgeButton), center(Tile::ForgeButton)))
        }
    } else if skills {
        Some((center(Tile::Catalog(Skill::Heal)), center(Tile::Active(9))))
    } else {
        match *stage {
            0 => Some((center(Tile::Personal(3)), center(Tile::Equipped(0)))),
            1 => Some((center(Tile::Equipped(0)), area(Destination::Shared))),
            2 => Some((center(Tile::Shared(2)), area(Destination::Personal))),
            _ => None,
        }
    };
    if *frame == 0 {
        if let Some((Some(from), Some(to))) = positions {
            *targets = Some((from, to));
        }
    }
    if let Some((from, to)) = *targets {
        match *frame {
            0 => {
                window.set_cursor_position(Some(from));
                mouse.press(MouseButton::Left);
            }
            2 => window.set_cursor_position(Some(to)),
            4 => mouse.release(MouseButton::Left),
            6 => {
                if forge {
                    let i = object(
                        &session,
                        state.focus.expect("Pointer selected a forge item"),
                    )
                    .unwrap();
                    if *stage == 0 {
                        assert_eq!(i.level, 5);
                        *stage = 1;
                        *frame = 0;
                        *targets = None;
                        return;
                    }
                    assert_eq!(i.level, 1);
                    assert_eq!(object(&session, state.focus.unwrap()).unwrap().alloy, 1);
                    state.forge_clock = 1.7;
                    commands
                        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                        .observe(bevy::render::view::screenshot::save_to_disk(
                            "outputs/rpg-forge-witness.png",
                        ));
                    println!("RPG_POINTER_WITNESS reforged=steel level=1 old_item_consumed=true");
                    *done = true;
                } else if skills {
                    assert_eq!(skill_slots(&session).active[9], Some(Skill::Heal));
                    assert_eq!(
                        skill_slots(&session)
                            .active
                            .iter()
                            .filter(|s| **s == Some(Skill::Heal))
                            .count(),
                        1
                    );
                    println!("RPG_POINTER_WITNESS skill_moved=true capacity=10+3");
                    commands
                        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                        .observe(bevy::render::view::screenshot::save_to_disk(
                            "outputs/rpg-skills-witness.png",
                        ));
                    *done = true;
                } else {
                    match *stage {
                        0 => assert_eq!(
                            object(&session, Location::Equipped(0)).unwrap().gear,
                            Gear::TemperedWeapon
                        ),
                        1 => assert!(object(&session, Location::Equipped(0)).is_none()),
                        2 => {
                            assert!(session.campaign.teams[0].inventory.len() == 2);
                            assert_eq!(object(&session, Location::Personal(3)).unwrap().level, 5);
                            println!(
                                "RPG_POINTER_WITNESS equip=true unequip_to_chest=true chest_to_bag=true level_preserved=true"
                            );
                            session
                                .campaign
                                .save(std::path::Path::new("outputs/rpg-pointer-save.json"))
                                .unwrap();
                            let restored = Campaign::load(std::path::Path::new(
                                "outputs/rpg-pointer-save.json",
                            ))
                            .unwrap();
                            assert_eq!(
                                restored.teams[0].fighters[1].equipment,
                                session.campaign.teams[0].fighters[1].equipment
                            );
                            commands
                                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                                .observe(bevy::render::view::screenshot::save_to_disk(
                                    "outputs/rpg-inventory-witness.png",
                                ));
                            *done = true;
                        }
                        _ => {}
                    }
                    *stage += 1;
                    *targets = None;
                    *frame = 0;
                    return;
                }
            }
            _ => {}
        }
        *frame += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rpg_edits_preserve_completed_combat_and_refresh_every_saved_member() {
        let mut session = Session::new(Battle::new(42, false));
        session.autosave = false;
        session.campaign = Campaign::new();
        session.campaign.teams[0].gold = 1000;
        session.campaign.teams[0].salvage = 1000;
        let (id, battle) = session
            .campaign
            .begin_battle(42, Arena::OpenField, Controller::Utility)
            .unwrap();
        session.battle = battle;
        session.active_battle_id = Some(id);
        while !session.battle.finished {
            session.battle.step();
        }
        session.settle_finished();
        let combat = serde_json::to_value(&session.battle).unwrap();
        let report = serde_json::to_value(&session.campaign.last_report).unwrap();
        for id in 0..3 {
            session.selected = id;
            apply(Action::Craft(Gear::BalancedWeapon), &mut session);
            apply(Action::Forge(Location::Personal(0)), &mut session);
            apply(
                Action::Transfer(Location::Personal(0), Destination::Equipped(0)),
                &mut session,
            );
            assert_eq!(
                session
                    .preview
                    .as_ref()
                    .unwrap()
                    .equipment
                    .worn(0)
                    .unwrap()
                    .level,
                2
            );
            assert_eq!(combat, serde_json::to_value(&session.battle).unwrap());
            assert_eq!(
                report,
                serde_json::to_value(&session.campaign.last_report).unwrap()
            );
        }
        session.battle.finished = false;
        session.battle.tick = 1;
        let before = serde_json::to_value(&session.campaign).unwrap();
        apply(Action::Forge(Location::Equipped(0)), &mut session);
        assert_eq!(before, serde_json::to_value(&session.campaign).unwrap());
    }
}
