//! Reproducible, staged research campaigns; never reads or writes the player save.
use phalanx::{
    FighterClass as C, learned::Controller, progression::Campaign, study::Recorder,
    styles::BattleStyle, terrain::Arena,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args().nth(1).ok_or("output directory required")?;
    let root = std::path::Path::new(&root);
    if root.exists() {
        return Err("use a fresh corpus directory".into());
    }
    std::fs::create_dir_all(root.join("studies"))?;
    let mut rows = 0;
    let mut companion_events = 0;
    let mut social_events = 0;
    for seed in 1..=24u64 {
        let mut c = Campaign::new();
        c.teams[0].gold = 1000;
        c.unlock_fourth()?;
        for (team, t) in c.teams.iter_mut().enumerate() {
            for (slot, f) in t.fighters.iter_mut().enumerate() {
                f.fighter_id = format!("research-campaign-{seed}-{team}-{slot}");
                // Keep persistent pack/support anchors, rotating the other roles
                // through every class on each split, including ranged combatants.
                f.class = match slot {
                    0 => C::Houndwarden,
                    1 => C::Healer,
                    _ => C::ALL[((seed as usize - 1) * 2 + slot - 2 + team * 7) % C::ALL.len()],
                };
                f.mastery.weapons = [100; 4]; // Staged recipe eligibility, not earned player practice.
                f.mind = BattleStyle::ALL[(slot + team + (seed as usize % 3)) % 3].mind();
                f.theus = Some(phalanx::theus::Sheet {
                    attributes: [11, 13, 12, 12],
                    practice: [100, 0, 100, 0, 0, 0],
                    ..Default::default()
                });
            }
        }
        for team in 0..2 {
            c.teams[team].gold = 1000;
            c.teams[team].salvage = 1000;
            for slot in 0..4 {
                use phalanx::equipment::Gear as G;
                let weapon = match c.teams[team].fighters[slot].class {
                    C::Ranger => G::Bow,
                    C::RockThrower | C::Alchemist => G::ThrowingSpear,
                    C::Shinobi => G::Kusarigama,
                    C::Reaper => G::RazorWheel,
                    _ => [
                        G::BellMace,
                        G::BalancedWeapon,
                        G::BoneGlaive,
                        G::TemperedWeapon,
                    ][slot],
                };
                c.workshop(team, slot, weapon)?;
                c.workshop(
                    team,
                    slot,
                    [
                        G::PorcelainHarness,
                        G::SilkLamellar,
                        G::TrailArmor,
                        G::IronArmor,
                    ][slot],
                )?;
            }
        }
        for episode in 0..2 {
            let arena = if seed % 3 == 0 {
                c.battlefield(seed)
            } else if seed % 3 == 1 {
                Arena::OpenField
            } else {
                phalanx::pressure::generate(&c.teams, seed)
            };
            let (id, mut b) = c.begin_battle(seed, arena, Controller::Utility)?;
            let mut recorder = Recorder::start(&root.join("studies"), &b)?;
            while !b.finished {
                recorder.step(&mut b)?;
            }
            rows += b
                .units
                .iter()
                .map(|u| u.decision_counts.iter().sum::<u32>())
                .sum::<u32>();
            companion_events += b.kennel.events.len();
            social_events += b.social_events.len();
            c.settle(id, &b)?;
            println!(
                "seed={seed} episode={episode} ticks={} companions={} social={}",
                b.tick,
                b.kennel.events.len(),
                b.social_events.len()
            );
        }
    }
    let manifest = serde_json::json!({"schema_version":1,"source_sha256":phalanx::challenges::BUILD,"battles":48,"seeds":24,"decision_ticks":rows,"companion_events":companion_events,"social_events":social_events,"fixture":"Independent four-fighter campaigns; staged classes and initial training; two completed episodes per seed; not player data"});
    std::fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(())
}
