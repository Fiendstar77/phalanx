//! Earned XP becomes a visible harvest. Awards remain exclusively in Campaign::settle.
use super::*;
const ORBS:usize=64;
#[derive(Resource,Default)]pub(crate) struct Harvest {
    battle:Option<u64>,clock:f32,pub(crate) active:bool,
    pub(crate) xp:[u32;4],before:[u32;4],received:[u32;4],levels:[u32;4],chime:f32,
}
#[derive(Component)] pub(super) struct Panel;
#[derive(Component)] pub(super) struct Row(usize);
#[derive(Component)] pub(super) struct Caption(usize);
#[derive(Component)] pub(super) struct Orb(usize, usize);
#[derive(Component)] pub(super) struct Fill(usize);
#[derive(Component)] pub(super) struct Heading;
pub struct Rewards;
impl Plugin for Rewards {
    fn build(&self,app:&mut App) {app.init_resource::<Harvest>().add_systems(Startup,setup_rewards).add_systems(Update,tick.after(settle_progression));}
}
fn level(experience:u32)->u32 {1+(experience/100).min(9)}
fn setup_rewards(mut commands:Commands) {
    commands.spawn((Panel,GlobalZIndex(80),Node {position_type:PositionType::Absolute,left:px(20),right:px(20),bottom:px(18),padding:UiRect::all(px(16)),flex_direction:FlexDirection::Column,row_gap:px(12),display:Display::None,..default()},BackgroundColor(Color::srgba(0.025,0.025,0.07,0.95))))
    .with_children(|p| {
        p.spawn((Heading,Text::new("EXPERIENCE HARVEST"),font(27.),TextColor(Color::srgb(0.7,1.,0.8))));
        p.spawn(Node {column_gap:px(12),..default()}).with_children(|row| {for i in 0..4 {
            row.spawn((Row(i),Node {flex_grow:1.,flex_basis:percent(25.),padding:UiRect::all(px(10)),flex_direction:FlexDirection::Column,row_gap:px(6),..default()},BackgroundColor(Color::srgb(0.08,0.07,0.14))))
            .with_children(|p| {p.spawn((Caption(i),Text::new(""),font(21.)));p.spawn((Fill(i),Node {height:px(7),width:percent(0.),..default()},BackgroundColor(Color::srgb(0.7,0.45,1.))));});
        }});
        button(p,"Continue / inspect battle report",Control::Camp(camp::Page::Report));
    });
    for i in 0..4 {for j in 0..ORBS {commands.spawn((Orb(i,j),GlobalZIndex(79),Node {position_type:PositionType::Absolute,width:px(7),height:px(7),border_radius:BorderRadius::all(percent(50.)),display:Display::None,..default()},BackgroundColor(Color::srgb(0.65,1.,0.8))));}}
}
#[allow(clippy::too_many_arguments)]
pub(super) fn tick(time:Res<Time>,mut session:ResMut<Session>,mut harvest:ResMut<Harvest>,window:Single<&Window>,camera:Single<(&Camera,&GlobalTransform),MainCameraFilter>,
    mut nodes:ParamSet<(Query<&mut Node,With<Panel>>,Query<(&Row,&mut Node)>,Query<(&Orb,&mut Node)>,Query<(&Fill,&mut Node)>)>,
    mut captions:Query<(&Caption,&mut Text),Without<Heading>>,mut headings:Query<&mut Text,With<Heading>>,mut cues:MessageWriter<audio::Cue>) {
    let enabled=!std::env::args().any(|a|a=="--visual-witness") || std::env::args().any(|a|a=="--player-demo");
    let report=session.campaign.last_report.as_ref();
    let fresh=enabled && session.exchange.live.is_none() && session.battle.finished && session.rewarded
        && report.is_some_and(|r|Some(r.battle_id)==session.active_battle_id && harvest.battle!=Some(r.battle_id));
    if fresh {
        let r=report.unwrap();harvest.battle=Some(r.battle_id);harvest.clock=0.;harvest.active=true;harvest.chime=0.;
        for i in 0..r.fighters[0].len() {harvest.xp[i]=r.fighters[0][i].xp;harvest.before[i]=session.campaign.teams[0].fighters[i].experience.saturating_sub(harvest.xp[i]);harvest.levels[i]=level(harvest.before[i]);}
        session.menu=false;session.camp_page=None;session.player.research=false;
    }
    if harvest.active && session.menu && harvest.clock>0.25 {harvest.active=false;}
    if !session.battle.finished {harvest.active=false;}
    if harvest.active {harvest.clock+=time.delta_secs().min(0.1);}
    if harvest.active && harvest.clock>7. {harvest.active=false;session.menu=true;session.camp_page=Some(camp::Page::Report);}
    session.harvesting=harvest.active;
    for mut node in &mut nodes.p0() {node.display=if harvest.active {Display::Flex}else{Display::None};}
    let count=session.battle.team_size().min(4);harvest.received=[0;4];
    for (row,mut node) in &mut nodes.p1() {node.display=if row.0<count {Display::Flex}else{Display::None};}
    for (orb,mut node) in &mut nodes.p2() {
        node.display=Display::None;let i=orb.0;let j=orb.1;
        if !harvest.active || i>=count || j>=harvest.xp[i].min(ORBS as u32) as usize {continue;}
        let n=harvest.xp[i].min(ORBS as u32).max(1) as usize;
        let delay=j as f32/n as f32*1.9+i as f32*0.18;
        let t=((harvest.clock-0.3-delay)/1.6).clamp(0.,1.);
        let value=((j+1) as u32*harvest.xp[i]/n as u32)-(j as u32*harvest.xp[i]/n as u32);
        if t>=1. {harvest.received[i]+=value;continue;}
        if harvest.clock<delay+0.3 {continue;}
        let unit=&session.battle.units[i];let p=session.presented(i);
        let world=Vec3::new(p[0],session.battle.arena.elevation(p)+0.7,p[1]);
        let source=camera.0.world_to_viewport(camera.1,world).unwrap_or(Vec2::new(window.width()*0.5,window.height()*0.4));
        let angle=j as f32*2.3999;let source=source+Vec2::new(angle.cos(),angle.sin())*(22.+j as f32*0.7);
        let target=Vec2::new((i as f32+0.5)*window.width()/count as f32,window.height()-95.);
        let p=source.lerp(target,t*t*(3.-2.*t))+Vec2::new((angle.sin()*40.)*(std::f32::consts::PI*t).sin(),-70.*(std::f32::consts::PI*t).sin());
        node.left=px(p.x);node.top=px(p.y);node.display=Display::Flex;
        let _=unit;
    }
    for (caption,mut text) in &mut captions {let i=caption.0;if i>=count {continue;}let exp=harvest.before[i]+harvest.received[i];let leveled=level(exp)>level(harvest.before[i]);
        text.0=format!("{}\n+{} / {} XP\nLEVEL {}{}",session.campaign.teams[0].fighters[i].name,harvest.received[i],harvest.xp[i],level(exp),if leveled {"  LEVEL UP!"}else{""});
        if level(exp)>harvest.levels[i] {harvest.levels[i]=level(exp);if harvest.active {cues.write(audio::Cue::Level);}}
    }
    for (fill,mut node) in &mut nodes.p3() {let i=fill.0;node.width=percent(((harvest.before[i]+harvest.received[i])%100) as f32);}
    for mut text in &mut headings {text.0=format!("{} | +{} XP",if session.battle.winner()==Some(0) {"YOUR PHALANX KICKED ASS!"}else{"LIVE. LEARN. COME BACK ANGRIER."},harvest.received.iter().sum::<u32>());}
    if harvest.active && harvest.clock>0.5 && harvest.clock<4.7 {harvest.chime+=time.delta_secs();if harvest.chime>=0.14 {harvest.chime=0.;cues.write(audio::Cue::Xp);}}
}
