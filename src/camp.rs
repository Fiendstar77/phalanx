//! Campaign-owned workshop and immutable last-battle report.
use super::*;
use phalanx::equipment::Gear;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Report,
    Workshop,
    Skills,
    Forge,
    Training,
    Rivals,
    Challenges,
}
pub struct Camp;
#[derive(Component)]
struct Panel(Page);
#[derive(Component)]
struct CampScroll(Page);
#[derive(Component)]
enum Label {
    Report,
    Fighter,
    Heraldry,
    Rivals,
    Rival(usize),
    Exchange,
    ExchangeFile(usize),
    Recipe(Gear),
    Roster(usize),
    Notice,
}
impl Plugin for Camp {
    fn build(&self, app: &mut App) {
        if std::env::args().any(|a| a == "--camp-scroll-witness") {
            app.add_systems(Update, scroll_witness.before(scroll));
        }
        app.add_systems(Startup, setup_camp).add_systems(
            Update,
            (
                refresh.after(settle_progression),
                polish.after(controls),
                scroll,
            ),
        );
    }
}
fn label(p: &mut ChildSpawnerCommands, kind: Label, size: f32) {
    p.spawn((
        kind,
        Text::new(""),
        font(size),
        TextColor(Color::srgb(0.79, 0.87, 0.89)),
    ));
}
fn setup_camp(mut commands: Commands) {
    for page in [Page::Report, Page::Training, Page::Rivals, Page::Challenges] {
        commands.spawn((Panel(page), GlobalZIndex(105), Interaction::default(), Node {
            position_type: PositionType::Absolute, width: percent(100), height: percent(100),
            align_items: AlignItems::Center, justify_content: JustifyContent::Center, display: Display::None, ..default()
        }, BackgroundColor(Color::srgba(0.018, 0.026, 0.035, 0.98))))
        .with_children(|root| {
            root.spawn((Node { width: px(1180), max_width: vw(94), height: vh(92), padding: UiRect::all(px(24)), flex_direction: FlexDirection::Column, row_gap: px(12), ..default() }, BackgroundColor(Color::srgb(0.035, 0.055, 0.075))))
            .with_children(|p| {
                p.spawn((Text::new(if page == Page::Report { "BRING THEM HOME | BATTLE REPORT" } else if page == Page::Rivals { "THE RIVAL PHALANXES" } else if page == Page::Challenges { "PHALANX EXCHANGE | SNAPSHOT CHALLENGES" } else { "SQUAD WORKSHOP" }), font(28.), TextColor(Color::srgb(1.,0.72,0.08))));
                p.spawn(Node { flex_wrap: FlexWrap::Wrap, ..default() }).with_children(|row| {
                    button(row, "Battle report", Control::Camp(Page::Report));
                    button(row, "Character", Control::Camp(Page::Workshop));
                    button(row, "Skills", Control::Camp(Page::Skills));
                    button(row, "Forge", Control::Camp(Page::Forge));
                    button(row, "Training & titles", Control::Camp(Page::Training));
                    button(row, "Rival phalanxes", Control::Camp(Page::Rivals));
                    button(row, "Snapshot challenges", Control::Camp(Page::Challenges));
                    button(row, "Return to arena", Control::Continue);
                    button(row, "Main menu", Control::Menu);
                });
                p.spawn((CampScroll(page), ScrollPosition::default(), Node {
                    flex_grow: 1., min_height: px(0), overflow: Overflow::scroll_y(), flex_direction: FlexDirection::Column, row_gap: px(12), ..default()
                })).with_children(|body| {
                    if page == Page::Report {
                        label(body, Label::Report, 17.);
                    } else if page == Page::Challenges {
                        label(body, Label::Exchange, 16.);
                        body.spawn(Node { flex_wrap: FlexWrap::Wrap, ..default() }).with_children(|row| {
                            button(row, "Export my Phalanx", Control::ExportSnapshot);
                            button(row, "Refresh inbox", Control::RefreshSnapshots);
                            button(row, "Next seed", Control::ChallengeSeed);
                            button(row, "Fight imported Phalanx", Control::StartChallenge);
                            button(row, "Export battle result", Control::ExportChallengeResult);
                            button(row, "Return to my campaign", Control::LeaveChallenge);
                        });
                        body.spawn((Text::new("IMPORT | Drop a team or result JSON file onto this page, or put files in outputs/challenges/inbox and refresh.\nExports appear in outputs/challenges/outbox. Send a team file to another player running the same game build.\nSnapshot matches are exhibitions: your campaign, rivals, gold, XP and injuries stay as they were."),font(14.)));
                        for index in 0..12 {
                            body.spawn((Button, Control::ImportSnapshot(index), PressLatch::default(), BackgroundColor(Color::srgb(0.12,0.18,0.25)), Node { padding:UiRect::all(px(10)), flex_shrink:0., ..default() })).with_children(|p|label(p, Label::ExchangeFile(index),14.));
                        }
                    } else if page == Page::Rivals {
                        label(body, Label::Rivals, 17.);
                        for index in 0..5 {
                            body.spawn((Button, Control::ChooseRival(index), PressLatch::default(), BackgroundColor(Color::srgb(0.12,0.18,0.25)), Node { padding:UiRect::all(px(12)), flex_shrink:0., ..default() })).with_children(|p|label(p, Label::Rival(index),15.));
                        }
                    } else {
                        body.spawn(Node { column_gap: px(24), align_items: AlignItems::Start, ..default() }).with_children(|columns| {
                            columns.spawn(Node { width: px(330), flex_shrink: 0., flex_direction: FlexDirection::Column, row_gap: px(8), ..default() }).with_children(|roster| {
                                roster.spawn((Text::new("COMPARE FIGHTERS\nFull-health stats for the next battle\nNo temporary blessing or rage bonuses\nIncoming: lower means less damage"), font(14.)));
                                for id in 0..8 {
                                    roster.spawn((Control::SelectFighter(id), BorderColor::all(Color::srgb(0.18,0.24,0.30)), Button, PressLatch::default(), BackgroundColor(Color::srgb(0.12,0.18,0.25)), Node { border: UiRect::all(px(2)), padding: UiRect::all(px(9)), flex_shrink: 0., ..default() }))
                                        .with_children(|p| label(p, Label::Roster(id), 13.));
                                }
                            });
                            columns.spawn(Node { flex_grow: 1., min_width: px(0), flex_direction: FlexDirection::Column, row_gap: px(10), ..default() }).with_children(|details| {
                                label(details, Label::Fighter, 16.);
                                label(details, Label::Heraldry, 14.);
                                details.spawn(Node { flex_wrap: FlexWrap::Wrap, ..default() }).with_children(|row| {
                                    button(row, "Next emblem", Control::FlagEmblem);
                                    button(row, "Primary color", Control::FlagPrimary);
                                    button(row, "Accent color", Control::FlagAccent);
                                    button(row, "Make selected standard-bearer", Control::StandardBearer);
                                });
                                details.spawn((Text::new("TITLES | free; occupied offices swap holders"), font(13.)));
                                details.spawn(Node { flex_wrap: FlexWrap::Wrap, ..default() }).with_children(|row| {
                                    for title in CommandTitle::ALL { button(row, &format!("{} | {}", title.name(), title.description()), Control::Title(title)); }
                                });
                                details.spawn((Text::new("RETRAIN | 10 gold (all titles may train any discipline)"), font(13.)));
                                details.spawn(Node { flex_wrap: FlexWrap::Wrap, ..default() }).with_children(|row| {
                                    for class in FighterClass::ALL { button(row, class.name(), Control::Class(class)); }
                                });
                                button(details, "Fit Iron I limb | 15 gold + 5 salvage", Control::Prosthetic);
                                for gear in [Gear::ThrowingSpear,Gear::Atlatl,Gear::Bow].into_iter().chain(Gear::ALL.into_iter().filter(|g|!matches!(g,Gear::ThrowingSpear|Gear::Atlatl|Gear::Bow))) {
                                    details.spawn((Node { padding: UiRect::all(px(10)), flex_direction: FlexDirection::Column, flex_shrink: 0., row_gap: px(4), ..default() }, BackgroundColor(Color::srgb(0.055,0.08,0.105))))
                                    .with_children(|recipe| {
                                        button(recipe, gear.name(), Control::Workshop(gear));
                                        label(recipe, Label::Recipe(gear), 13.);
                                    });
                                }
                            });
                        });
                    }
                });
                label(p, Label::Notice, 14.);
                p.spawn(Node { flex_wrap: FlexWrap::Wrap, flex_shrink: 0., ..default() }).with_children(|row| {
                    if page != Page::Challenges {
                        button(row, "Next battle", Control::NextBattle);
                        button(row, "Save squads", Control::Save);
                    }
                });
            });
        });
    }
}
/// Derive a fresh-battle preview from saved identity, class, XP, wounds and gear.
/// Never mutate the live combat snapshot or write a preview to disk.
pub(crate) fn prepared(session: &Session, id: usize) -> phalanx::Unit {
    let mut unit = session.battle.units[id].clone();
    let fighter = &session.campaign.teams[unit.team].fighters[id % session.battle.team_size()];
    unit.command_title = Some(fighter.title);
    unit.mastery = fighter.mastery;
    unit.skills = fighter.skills;
    unit.carries_standard =
        id % session.battle.team_size() == session.campaign.teams[unit.team].standard_bearer;
    unit.class = Some(fighter.class);
    unit.experience = fighter.experience;
    unit.lost_limbs = fighter.lost_limbs;
    unit.prosthetics = fighter.prosthetics;
    unit.equipment = fighter.equipment;
    unit.equipment.migrate();
    unit.role = unit.trained_role();
    unit.ammunition = unit
        .projectile_weapon()
        .map(|w| w.ammunition())
        .unwrap_or(0);
    unit.reset_weapon();
    unit.cooldown = 0.;
    unit.action = 6;
    unit.bow_melee = false;
    unit.bow_release = 0.;
    unit.last_shot_target = None;
    unit.blessed = false;
    unit.health = unit.max_health();
    unit
}
fn stats(u: &phalanx::Unit) -> String {
    format!(
        "HP {:.0} | Damage x{:.2} | Move x{:.2} | Incoming x{:.2}\n{}",
        u.max_health(),
        u.damage_multiplier(),
        u.movement_multiplier(),
        u.incoming_multiplier(),
        u.weapon_status()
    )
}
fn report(session: &Session) -> String {
    if session.exchange.live.is_some() {
        return exchange::summary(session);
    }
    let Some(r) = &session.campaign.last_report else {
        return "No completed battle yet.\n\nFight a Founders battle to earn gold, salvage and XP.\nYour results will appear here after rewards are settled.".into();
    };
    let outcome = r
        .winner
        .map(|t| {
            format!(
                "{} wins",
                r.team_names
                    .as_ref()
                    .map(|n| n[t].as_str())
                    .unwrap_or("historical opponent")
            )
        })
        .unwrap_or_else(|| "Draw".into());
    let mut text = format!(
        "{} | Battle {} | {} | Seed {}\n\n",
        outcome,
        r.battle_id + 1,
        r.arena.name(),
        r.seed
    );
    for team in 0..2 {
        text.push_str(&format!(
            "{} | +{} gold | +{} salvage\n",
            r.team_names
                .as_ref()
                .map(|n| n[team].as_str())
                .unwrap_or("historical squad"),
            r.gold[team],
            r.salvage[team]
        ));
        for f in &r.fighters[team] {
            text.push_str(&format!("{} [{}] / {} | {} ({:.0}/{:.0} HP)\n  Damage {:.0} | Healing {:.0} | +{} XP | New wounds: {}\n", f.name, f.title.map(CommandTitle::name).unwrap_or("title not recorded"), f.class.name(), if f.health > 0. { "Survived" } else { "Downed" }, f.health, f.max_health, f.damage, f.healing, f.xp, limb_names(f.new_wounds)));
            for weapon in phalanx::weapons::ProjectileWeapon::ALL {
                let gained = f.weapon_training[weapon.index()];
                if gained > 0 {
                    text.push_str(&format!("  {} practice +{gained}\n", weapon.name()));
                }
            }
            for gear in &f.discoveries {
                text.push_str(&format!(
                    "  DISCOVERED {}: craft it in the squad workshop\n",
                    gear.name()
                ));
            }
        }
        text.push('\n');
    }
    text.push_str("RECOVERY\nAll fighters return at full HP next battle. Missing limbs and psychological scars persist.\nFit prosthetics or change loadouts in the squad workshop before continuing.\n\nRewards are granted once. This report stays available after saving and restarting.");
    text
}
fn refresh(
    session: Res<Session>,
    mut panels: Query<(&Panel, &mut Node)>,
    mut texts: Query<(&Label, &mut Text)>,
    mut roster: Query<(&Control, &mut Node), Without<Panel>>,
) {
    for (panel, mut node) in &mut panels {
        node.display = if session.menu && session.camp_page == Some(panel.0) {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !session.menu || session.camp_page.is_none() || !session.battle.founders {
        return;
    }
    let unit = prepared(&session, session.selected);
    let team = &session.campaign.teams[unit.team];
    let fighter = &team.fighters[unit.id % session.battle.team_size()];
    let locked = session.battle.tick > 0 && !session.battle.finished;
    for (control, mut node) in &mut roster {
        if let Control::SelectFighter(id) = *control {
            node.display = if id < session.battle.units.len() {
                Display::Flex
            } else {
                Display::None
            };
        } else if let Control::ImportSnapshot(index) = *control {
            node.display =
                if session.exchange.live.is_none() && index < session.exchange.files.len() {
                    Display::Flex
                } else {
                    Display::None
                };
        } else if matches!(
            control,
            Control::ExportSnapshot
                | Control::RefreshSnapshots
                | Control::ChallengeSeed
                | Control::StartChallenge
        ) {
            node.display = if session.exchange.live.is_none() {
                Display::Flex
            } else {
                Display::None
            };
        } else if matches!(
            control,
            Control::ExportChallengeResult | Control::LeaveChallenge
        ) {
            node.display = if session.exchange.live.is_some() {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
    for (kind, mut text) in &mut texts {
        let value = match kind {
            Label::Exchange => exchange::summary(&session),
            Label::ExchangeFile(index) => session
                .exchange
                .files
                .get(*index)
                .map(|path| {
                    format!(
                        "Import / verify {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    )
                })
                .unwrap_or_default(),
            Label::Report => report(&session),
            Label::Rivals => format!(
                "Current opponent: {}\n{}\nRivals earn XP, retain mastery, carry wounds and buy gear with earned gold/salvage.\nThey progress when fought. Choose a challenger before or after battle.\nYour previous battle report stays unchanged.",
                session.campaign.teams[1].name,
                session.campaign.teams[1]
                    .rival_style
                    .map(|s| s.doctrine())
                    .unwrap_or("Original local opponent; preserved with its existing identity")
            ),
            Label::Rival(index) => session
                .campaign
                .rivals
                .get(*index)
                .map(|r| {
                    format!(
                        "CHALLENGE {}\n{}\n{} gold / {} salvage | {} | {} battles / {} wins",
                        r.name,
                        r.rival_style
                            .map(|s| s.doctrine())
                            .unwrap_or("Legacy opponent"),
                        r.gold,
                        r.salvage,
                        r.fighters
                            .iter()
                            .map(|f| format!(
                                "{} L{}",
                                f.class.name(),
                                1 + (f.experience / 100).min(9)
                            ))
                            .collect::<Vec<_>>()
                            .join(" / "),
                        r.fighters[0].battles,
                        r.fighters[0].wins
                    )
                })
                .unwrap_or_default(),
            Label::Notice => format!(
                "{}\n{}",
                if session.exchange.live.is_some() {
                    "Frozen exhibition match. Return to your campaign for preparation and progression."
                } else if locked {
                    "Battle in progress: equipment and retraining locked. Return to arena to finish."
                } else {
                    "Ready for preparation. Owned gear switches free; purchases save with your squad."
                },
                session.save_notice
            ),
            Label::Heraldry => format!(
                "PHALANX FLAG | {} emblem | bearer: {} [{}]\nPrimary: shared with squad colors | Accent: {:.0}/{:.0}/{:.0}\nThe standard is carried independently of class and title.",
                team.heraldry.emblem.name(),
                team.fighters[team.standard_bearer].name,
                team.fighters[team.standard_bearer].title.name(),
                team.heraldry.accent[0] * 255.0,
                team.heraldry.accent[1] * 255.0,
                team.heraldry.accent[2] * 255.0
            ),
            Label::Fighter => format!(
                "{} | {} [title: {}] / {} | {} gold / {} salvage\n{}\n{}\nEquipped: {}\nMissing: {} | Iron I: {}",
                team.name,
                fighter.name,
                fighter.title.name(),
                fighter.class.name(),
                team.gold,
                team.salvage,
                stats(&unit),
                format!(
                    "{}\n{}",
                    fighter.mastery.summary(fighter.class),
                    fighter.mastery.weapon_summary()
                ),
                fighter.equipment.summary(),
                limb_names(fighter.lost_limbs),
                limb_names(fighter.prosthetics)
            ),
            Label::Roster(id) => {
                if *id >= session.battle.units.len() {
                    String::new()
                } else {
                    let u = prepared(&session, *id);
                    let t = &session.campaign.teams[u.team];
                    let f = &t.fighters[*id % session.battle.team_size()];
                    format!(
                        "{}{} / {} [{}]\n{} | Level {} | {} XP\n{}",
                        if *id == session.selected { "> " } else { "" },
                        t.name,
                        f.name,
                        f.title.name(),
                        f.class.name(),
                        u.level(),
                        f.experience,
                        stats(&u)
                    )
                }
            }
            Label::Recipe(gear) => {
                let mut preview = unit.clone();
                preview.equipment.slots[gear.slot()] = if unit.equipment.has(*gear) {
                    None
                } else {
                    Some(*gear)
                };
                preview.reset_weapon();
                preview.health = preview.max_health();
                let (gold, salvage) = gear.cost();
                format!(
                    "{}\n{}\nAfter {}: {}\nDelta: HP {:+.0} / damage {:+.2} / movement {:+.2} / incoming {:+.2}",
                    gear.effect(),
                    if unit.equipment.owns(*gear) {
                        "OWNED | free to equip or remove".into()
                    } else if let Some(reason) = gear.locked_reason(fighter.mastery) {
                        format!("LOCKED | {reason}")
                    } else if team.gold >= gold && team.salvage >= salvage {
                        format!("AVAILABLE | {gold} gold + {salvage} salvage")
                    } else {
                        format!(
                            "NEED {gold} gold + {salvage} salvage | have {} / {}",
                            team.gold, team.salvage
                        )
                    },
                    if unit.equipment.has(*gear) {
                        "removing"
                    } else {
                        "equipping"
                    },
                    stats(&preview),
                    preview.max_health() - unit.max_health(),
                    preview.damage_multiplier() - unit.damage_multiplier(),
                    preview.movement_multiplier() - unit.movement_multiplier(),
                    preview.incoming_multiplier() - unit.incoming_multiplier()
                )
            }
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}
fn scroll(
    session: Res<Session>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut panels: Query<(&CampScroll, &mut ScrollPosition)>,
) {
    for event in wheel.read() {
        if session.menu
            && let Some(page) = session.camp_page
        {
            let delta = match event.unit {
                bevy::input::mouse::MouseScrollUnit::Line => event.y * 32.0,
                bevy::input::mouse::MouseScrollUnit::Pixel => event.y,
            };
            for (panel, mut position) in &mut panels {
                if panel.0 == page {
                    position.0.y = (position.0.y - delta).max(0.);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_workshop_and_next_battle_keep_authoritative_state_separate() {
        let mut session = Session::new(Battle::new(42, false));
        session.autosave = false;
        session.campaign = Campaign::new();
        session
            .campaign
            .set_class(0, 1, FighterClass::Healer)
            .unwrap();
        session.restart_founders(42, Arena::OpenField, Controller::Utility);
        session.selected = 1;
        while !session.battle.finished {
            session.battle.step();
        }
        session.settle_finished();
        assert_eq!(session.camp_page, Some(Page::Report));
        assert!(session.menu);
        let report_before = report(&session);
        let campaign_before = serde_json::to_value(&session.campaign).unwrap();
        let combat_before = serde_json::to_value(&session.battle).unwrap();
        for id in 0..session.battle.units.len() {
            let u = prepared(&session, id);
            assert_eq!(u.health, u.max_health());
            assert!(!u.blessed);
            for gear in Gear::ALL {
                let mut preview = u.clone();
                preview.equipment.slots[gear.slot()] = Some(gear);
                preview.reset_weapon();
                preview.health = preview.max_health();
                assert!(preview.damage_multiplier().is_finite());
            }
        }
        assert_eq!(
            campaign_before,
            serde_json::to_value(&session.campaign).unwrap()
        );
        assert_eq!(
            combat_before,
            serde_json::to_value(&session.battle).unwrap()
        );
        apply(Control::Camp(Page::Workshop), &mut session);
        apply(Control::Workshop(Gear::BalancedWeapon), &mut session);
        assert_eq!(report_before, report(&session));
        assert!(!session.battle.units[1].equipment.has(Gear::BalancedWeapon));
        assert!(
            session.campaign.teams[0].fighters[1]
                .equipment
                .has(Gear::BalancedWeapon)
        );
        apply(Control::StandardBearer, &mut session);
        apply(Control::FlagEmblem, &mut session);
        apply(Control::FlagPrimary, &mut session);
        apply(Control::FlagAccent, &mut session);
        assert!(!session.battle.units[1].carries_standard);
        assert!(prepared(&session, 1).carries_standard);
        apply(Control::Title(CommandTitle::Captain), &mut session);
        assert_eq!(
            session.battle.units[1].command_title,
            Some(CommandTitle::Queen)
        );
        assert_eq!(
            prepared(&session, 1).command_title,
            Some(CommandTitle::Captain)
        );
        assert_eq!(report_before, report(&session));
        apply(Control::NextBattle, &mut session);
        assert_eq!(
            session.battle.units[1].command_title,
            Some(CommandTitle::Captain)
        );
        assert!(session.battle.units[1].carries_standard);
        assert_eq!(session.camp_page, None);
        assert!(!session.menu);
        assert!(session.battle.units[1].equipment.has(Gear::BalancedWeapon));
        assert_eq!(report_before, report(&session));
        assert_eq!(session.battle.units[1].class, Some(FighterClass::Healer));
    }
}

fn scroll_witness(
    time: Res<Time>,
    window: Single<Entity, With<Window>>,
    mut wheel: MessageWriter<bevy::input::mouse::MouseWheel>,
    mut sent: Local<bool>,
) {
    if time.elapsed_secs() > 0.8 && !*sent {
        *sent = true;
        wheel.write(bevy::input::mouse::MouseWheel {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.,
            y: -16.,
            window: *window,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
    }
}

/// Visual availability only: clicking still explains the rejection through the existing action path.
fn unavailable(session: &Session, control: Control) -> bool {
    if !matches!(
        control,
        Control::Workshop(_)
            | Control::Prosthetic
            | Control::Class(_)
            | Control::Title(_)
            | Control::FlagPrimary
            | Control::FlagAccent
            | Control::FlagEmblem
            | Control::StandardBearer
    ) {
        return false;
    }
    if !session.battle.founders || (session.battle.tick > 0 && !session.battle.finished) {
        return true;
    }
    let unit = &session.battle.units[session.selected];
    let team = &session.campaign.teams[unit.team];
    let fighter = &team.fighters[unit.id % session.battle.team_size()];
    match control {
        Control::Workshop(gear) => {
            let (gold, salvage) = gear.cost();
            !fighter.equipment.owns(gear)
                && (gear.locked_reason(fighter.mastery).is_some()
                    || team.gold < gold
                    || team.salvage < salvage)
        }
        Control::Class(class) => team.gold < 10 || fighter.class == class,
        Control::Title(title) => fighter.title == title,
        Control::StandardBearer => team.standard_bearer == unit.id % session.battle.team_size(),
        Control::Prosthetic => {
            team.gold < 15 || team.salvage < 5 || fighter.lost_limbs & !fighter.prosthetics == 0
        }
        _ => false,
    }
}
fn polish(
    session: Res<Session>,
    mut buttons: Query<(
        &Control,
        &Interaction,
        &Children,
        &mut BackgroundColor,
        Option<&mut BorderColor>,
    )>,
    mut texts: Query<&mut TextColor>,
) {
    for (control, interaction, children, mut background, border) in &mut buttons {
        let selected = matches!(*control, Control::SelectFighter(id) if id == session.selected);
        let dimmed = unavailable(&session, *control);
        if matches!(control, Control::SelectFighter(_)) {
            if let Some(mut border) = border {
                *border = BorderColor::all(if selected {
                    Color::srgb(1.0, 0.72, 0.08)
                } else {
                    Color::srgb(0.18, 0.24, 0.30)
                });
            }
            if selected {
                background.0 = if *interaction == Interaction::None {
                    Color::srgb(0.24, 0.21, 0.12)
                } else {
                    Color::srgb(0.34, 0.29, 0.14)
                };
            }
        } else if dimmed {
            background.0 = if *interaction == Interaction::None {
                Color::srgb(0.055, 0.07, 0.085)
            } else {
                Color::srgb(0.09, 0.11, 0.13)
            };
        } else if !matches!(
            control,
            Control::Workshop(_)
                | Control::Prosthetic
                | Control::Class(_)
                | Control::Title(_)
                | Control::FlagPrimary
                | Control::FlagAccent
                | Control::FlagEmblem
                | Control::StandardBearer
        ) {
            continue;
        }
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                text.0 = if selected {
                    Color::srgb(1.0, 0.82, 0.36)
                } else if dimmed {
                    Color::srgb(0.49, 0.54, 0.59)
                } else {
                    Color::srgb(0.85, 0.91, 0.96)
                };
            }
        }
    }
}
