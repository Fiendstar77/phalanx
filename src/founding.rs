//! Word-seeded founder reveals and editable names. Initial choices do not spend resources.
use super::*;
use bevy::input::keyboard::{Key,KeyboardInput};
use phalanx::genesis::{self,Founder,Origin};
pub struct Draft {
    pub open:bool,pub name:String,pub word:String,pub classes:[FighterClass;3],
    pub cards:[Founder;3],pub revealed:[bool;3],pub rolls:[u32;3],pub stamps:[u32;3],pub focus:usize,
}
impl Draft {
    pub fn new(c:&Campaign,open:bool)->Self {
        let classes=std::array::from_fn(|i|c.teams[0].fighters[i].class);
        Self {open,name:String::new(),word:"BONES".into(),cards:std::array::from_fn(|i|genesis::reveal("BONES",i,0,classes[i]).unwrap()),
            classes,revealed:[false;3],rolls:[0;3],stamps:[0;3],focus:0}
    }
    pub fn reveal(&mut self,i:usize)->Result<(),String> {
        if i>=3 {return Err("Unknown founder".into());}
        if self.revealed[i] {self.rolls[i]=(self.rolls[i]+1)%1_000_000;}
        self.cards[i]=genesis::reveal(&self.word,i,self.rolls[i],self.classes[i])?;
        self.revealed[i]=true;self.stamps[i]=self.stamps[i].wrapping_add(1);Ok(())
    }
    fn text(&mut self)->&mut String {
        match self.focus {0=>&mut self.name,1=>&mut self.word,i=>&mut self.cards[(i-2).min(2)].name}
    }
    fn changed(&mut self) {if self.focus==1 {self.revealed=[false;3];self.rolls=[0;3];}}
    pub fn origin(&self)->Origin {Origin {version:1,word:self.word.trim().into(),rolls:self.rolls}}
}
pub struct Founding;
#[derive(Component)]struct Panel;
#[derive(Component)]struct Scroll;
#[derive(Component)]struct Card(usize);
#[derive(Component)]struct Portrait(usize);
#[derive(Component)]struct RenamePanel;
#[derive(Component)]struct RenameText;
#[derive(Component)]enum Label {Name,Seed,Fighter(usize),Slot(usize),Notice}
impl Plugin for Founding {
    fn build(&self,app:&mut App) {
        app.add_systems(Startup,setup_founding.after(setup))
            .add_systems(Update,(type_name,refresh,scroll,portraits).chain().after(controls));
    }
}
fn label(p:&mut ChildSpawnerCommands,kind:Label,size:f32) {p.spawn((kind,Text::new(""),font(size)));}
fn field(p:&mut ChildSpawnerCommands,kind:Label,focus:usize) {
    p.spawn((Button,Control::FocusFounderField(focus),PressLatch::default(),BackgroundColor(Color::srgb(0.10,0.14,0.23)),
        Node {padding:UiRect::all(px(9)),..default()})).with_children(|p|label(p,kind,18.));
}
fn setup_founding(mut commands:Commands,mut images:ResMut<Assets<Image>>) {
    let portraits:[Handle<Image>;3]=std::array::from_fn(|i| {
        let image=images.add(Image::new_target_texture(256,192,bevy::render::render_resource::TextureFormat::Rgba8Unorm,Some(bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb)));
        commands.spawn((Portrait(i),Camera3d::default(),Camera {order:-5+i as isize,is_active:false,..default()},bevy::camera::RenderTarget::Image(image.clone().into()),Transform::default()));image
    });
    commands.spawn((Panel,GlobalZIndex(150),Interaction::default(),Node {position_type:PositionType::Absolute,width:percent(100),height:percent(100),align_items:AlignItems::Center,justify_content:JustifyContent::Center,display:Display::None,..default()},BackgroundColor(Color::srgb(0.018,0.022,0.045))))
    .with_children(|root| {root.spawn((Scroll,ScrollPosition::default(),Node {width:px(1220),max_width:vw(96),max_height:vh(97),overflow:Overflow::scroll_y(),padding:UiRect::all(px(18)),flex_direction:FlexDirection::Column,row_gap:px(8),..default()})).with_children(|p| {
        p.spawn((Text::new("A WORD. THREE LIVES. YOUR PHALANX."),font(29.),TextColor(Color::srgb(0.85,0.7,1.))));
        field(p,Label::Name,0);field(p,Label::Seed,1);
        p.spawn((Text::new("Click a name or seed to type. Reveal each founder, then choose a class. The same word reproduces the same cards."),font(13.)));
        p.spawn(Node {column_gap:px(14),..default()}).with_children(|row| {for (i,photo) in portraits.into_iter().enumerate() {
            row.spawn((Card(i),BackgroundColor(Color::srgb(0.055,0.06,0.10)),Node {flex_basis:percent(33.),min_width:px(0),padding:UiRect::all(px(10)),flex_direction:FlexDirection::Column,row_gap:px(5),..default()})).with_children(|card| {
                card.spawn((ImageNode::new(photo),Node {height:px(125),width:percent(100),..default()}));
                field(card,Label::Fighter(i),i+2);label(card,Label::Slot(i),13.);
                button(card,"Reveal / reroll this founder",Control::RevealFounder(i));
                for pair in FighterClass::ALL.chunks(2) {card.spawn(Node {flex_wrap:FlexWrap::Wrap,column_gap:px(3),..default()}).with_children(|r| {
                    for class in pair {button(r,class.name(),Control::StarterClass(i,*class));}
                });}
            });
        }});
        p.spawn((Text::new("Initial choices are free. Later retraining: 10 gold. Fourth fighter slot: 1,000 gold. Names stay editable."),font(13.)));
        label(p,Label::Notice,13.);button(p,"Accept these lives / enter the first trial",Control::ConfirmFounders);button(p,"Quit",Control::Quit);
    });});
    commands.spawn((RenamePanel,GlobalZIndex(170),Node {position_type:PositionType::Absolute,left:vw(25),top:vh(30),width:vw(50),padding:UiRect::all(px(24)),flex_direction:FlexDirection::Column,row_gap:px(12),display:Display::None,..default()},BackgroundColor(Color::srgb(0.04,0.05,0.10))))
        .with_children(|p| {p.spawn((Text::new("NAME YOUR FIGHTER"),font(25.)));p.spawn((RenameText,Text::new(""),font(22.)));p.spawn((Text::new("Type to edit. Backspace deletes. Enter saves. Escape cancels."),font(14.)));button(p,"Save name",Control::FinishRename);button(p,"Cancel",Control::CancelRename);});
}
fn portraits(session:Res<Session>,pawns:Query<(&Pawn,&Transform),Without<Portrait>>,mut cameras:Query<(&Portrait,&mut Camera,&mut Transform)>) {
    for (portrait,mut camera,mut transform) in &mut cameras {
        camera.is_active=session.founding.open && session.founding.revealed[portrait.0];
        if let Some((_,pawn))=pawns.iter().find(|(p,_)|p.0==portrait.0) {
            let target=pawn.translation+Vec3::Y*0.5;
            *transform=Transform::from_translation(target+pawn.rotation*Vec3::new(0.08,0.12,1.15)).looking_at(target,Vec3::Y);
        }
    }
}
fn scroll(session:Res<Session>,mut wheel:MessageReader<bevy::input::mouse::MouseWheel>,mut panels:Query<&mut ScrollPosition,With<Scroll>>) {
    for event in wheel.read() {if session.founding.open {let delta=match event.unit {bevy::input::mouse::MouseScrollUnit::Line=>event.y*30.,bevy::input::mouse::MouseScrollUnit::Pixel=>event.y};for mut p in &mut panels {p.0.y=(p.0.y-delta).max(0.);}}}
}
fn type_name(mut input:MessageReader<KeyboardInput>,mut session:ResMut<Session>) {
    for event in input.read() {
        if !event.state.is_pressed() {continue;}
        if session.renaming.is_some() {
            match &event.logical_key {Key::Enter=>apply(Control::FinishRename,&mut session),Key::Escape=>apply(Control::CancelRename,&mut session),_=> {
                let (_,text)=session.renaming.as_mut().unwrap();if event.logical_key==Key::Backspace {text.pop();}else if let Some(value)=&event.text {for c in value.chars().filter(|c|!c.is_control()) {if text.chars().count()<40 {text.push(c);}}}
            }};continue;
        }
        if !session.founding.open {continue;}
        match &event.logical_key {Key::Enter=>apply(Control::ConfirmFounders,&mut session),_=> {
            let text=session.founding.text();if event.logical_key==Key::Backspace {text.pop();}else if let Some(value)=&event.text {for c in value.chars().filter(|c|!c.is_control()) {if text.chars().count()<40 {text.push(c);}}}
            session.founding.changed();
        }}
    }
}
fn refresh(session:Res<Session>,time:Res<Time>,mut panels:Query<&mut Node,With<Panel>>,mut rename:Query<&mut Node,(With<RenamePanel>,Without<Panel>)>,mut rename_text:Query<&mut Text,With<RenameText>>,mut labels:Query<(&Label,&mut Text),Without<RenameText>>,mut buttons:Query<(&Control,&mut BackgroundColor),Without<Card>>,mut cards:Query<(&Card,&mut BackgroundColor)>,mut animation:Local<([u32;3],[f32;3])>) {
    for mut p in &mut panels {p.display=if session.founding.open {Display::Flex}else{Display::None};}
    for mut p in &mut rename {p.display=if session.renaming.is_some() {Display::Flex}else{Display::None};}
    if let Some((_,name))=&session.renaming {for mut t in &mut rename_text {t.0=format!("{name}_");}}
    if !session.founding.open {return;}
    for (kind,mut text) in &mut labels {
        let content=match kind {Label::Name=>format!("PHALANX: {}{}",session.founding.name,if session.founding.focus==0 {"_"}else{""}),
            Label::Seed=>format!("SEED WORD: {}{}",session.founding.word,if session.founding.focus==1 {"_"}else{""}),
            Label::Fighter(i)=>format!("{} / {}{}",session.campaign.teams[0].fighters[*i].title.name(),if session.founding.revealed[*i] {&session.founding.cards[*i].name}else{"UNREVEALED"},if session.founding.focus==i+2 {"_"}else{""}),
            Label::Slot(i)=> {let f=&session.founding.cards[*i];format!("{}\n{}\nCourage {:.0} / Teamwork {:.0} / Caution {:.0}",session.founding.classes[*i].name(),session.founding.classes[*i].description(),f.mind.courage*100.,f.mind.teamwork*100.,f.mind.caution*100.)},Label::Notice=>session.save_notice.clone()};
        if text.0!=content {text.0=content;}
    }
    for (c,mut color) in &mut buttons {if let Control::StarterClass(i,class)=c && session.founding.classes[*i]==*class {color.0=Color::srgb(0.3,0.23,0.1);}}
    for (card,mut color) in &mut cards {let i=card.0;if animation.0[i]!=session.founding.stamps[i] {animation.0[i]=session.founding.stamps[i];animation.1[i]=0.;}animation.1[i]+=time.delta_secs();let glow=if session.founding.revealed[i] {(-animation.1[i]*2.).exp()}else{0.};color.0=Color::srgb(0.055+glow*0.25,0.06+glow*0.10,0.10+glow*0.35);}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typing_selection_and_confirmation_start_the_chosen_team_once() {
        let mut s = Session::new(Battle::new(42, false));
        s.autosave = false;
        s.campaign = Campaign::new();
        s.restart_founders(41, Arena::OpenField, Controller::Utility);
        s.founding = Draft::new(&s.campaign, true);
        for i in 0..3 {s.founding.reveal(i).unwrap();}
        apply(Control::Run, &mut s);
        assert!(!s.running);
        apply(Control::ConfirmFounders, &mut s);
        assert!(s.founding.open);
        let mut app = App::new();
        app.add_message::<KeyboardInput>()
            .insert_resource(s)
            .add_systems(Update, type_name);
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::KeyP,
                logical_key: Key::Character("P".into()),
                state: bevy::input::ButtonState::Pressed,
                text: Some("Penelope's Phalanx".into()),
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        let mut s = app.world_mut().resource_mut::<Session>();
        apply(Control::StarterClass(0, FighterClass::Guardian), &mut s);
        apply(Control::StarterClass(1, FighterClass::Healer), &mut s);
        apply(Control::ConfirmFounders, &mut s);
        assert!(!s.founding.open);
        assert!(s.campaign.founders_prepared);
        assert_eq!(s.campaign.teams[0].name, "Penelope's Phalanx");
        assert_eq!(s.battle.units[0].class, Some(FighterClass::Guardian));
        assert_eq!(s.battle.units[1].class, Some(FighterClass::Healer));
        assert_eq!(s.campaign.teams[0].gold, 100);
        assert_eq!(s.campaign.teams[0].fighters.len(), 3);
        assert_eq!(
            s.campaign.teams[1].rival_style,
            Some(phalanx::rivals::RivalStyle::Bloodhound)
        );
        let before = serde_json::to_string(&s.campaign).unwrap();
        apply(Control::ConfirmFounders, &mut s);
        assert_eq!(serde_json::to_string(&s.campaign).unwrap(), before);
    }
}
