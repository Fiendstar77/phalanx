use phalanx::{
    FighterClass,
    challenges::{Challenge, ChallengeResult, TeamSnapshot},
    equipment::{Gear, Item},
    learned::Controller,
    progression::Campaign,
    terrain::Arena,
};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "phalanx-challenge-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn encode<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap()
}
fn snapshot(campaign: &Campaign, side: usize) -> TeamSnapshot {
    TeamSnapshot::from_campaign(campaign, side).unwrap()
}

#[test]
fn independent_installations_exchange_veterans_and_reproduce_the_complete_result() {
    let scratch = Scratch::new();
    let mut owner = Campaign::new();
    owner.teams[0].fighters[0].class = FighterClass::Ranger;
    owner.teams[0].fighters[1]
        .equipment
        .insert(Item::new(Gear::VitalityCharm))
        .unwrap();
    let (id, mut battle) = owner
        .begin_battle(41, Arena::OpenField, Controller::Utility)
        .unwrap();
    while !battle.finished {
        battle.step();
    }
    owner.settle(id, &battle).unwrap();
    let challenger = Campaign::new();
    let original_owner = encode(&owner);
    let original_challenger = encode(&challenger);
    let published = snapshot(&owner, 0);
    published.save(&scratch.file("owner.json")).unwrap();
    let imported = TeamSnapshot::load(&scratch.file("owner.json")).unwrap();
    assert_eq!(encode(&published), encode(&imported));
    assert!(!imported.team.fighters[0].history.is_empty());
    let challenge = Challenge::new(snapshot(&challenger, 0), imported, 123).unwrap();
    let outcome = challenge.run().unwrap();
    outcome.save(&scratch.file("result.json")).unwrap();
    let returned = ChallengeResult::load(&scratch.file("result.json")).unwrap();
    returned.verify().unwrap();
    assert_eq!(returned.battle, challenge.run().unwrap().battle);
    assert_eq!(encode(&owner), original_owner);
    assert_eq!(encode(&challenger), original_challenger);
}

#[test]
fn identical_source_ids_can_fight_without_losing_the_published_identities() {
    let owner = Campaign::new();
    let published = snapshot(&owner, 0);
    let original = encode(&published);
    let challenge = Challenge::new(published.clone(), published, 42).unwrap();
    let (campaign, battle) = challenge.start().unwrap();
    campaign.validate().unwrap();
    assert_ne!(battle.units[0].fighter_id, battle.units[3].fighter_id);
    assert_eq!(encode(&challenge.teams[0]), original);
    assert_eq!(encode(&challenge.teams[1]), original);
    for (slot, fighter) in challenge.teams[0].team.fighters.iter().enumerate() {
        assert_eq!(
            campaign.teams[1].fighters[slot].appearance,
            fighter.appearance
        );
        assert_eq!(fighter.appearance.female, Some(slot == 1));
        assert_eq!(
            fighter.appearance.hair,
            owner.teams[0].fighters[slot]
                .appearance
                .resolved(0, slot)
                .hair
        );
    }
}

#[test]
fn invalid_versions_rosters_loadouts_and_foreign_leagues_are_rejected() {
    let owner = Campaign::new();
    let valid = snapshot(&owner, 0);
    let mut invalid = valid.clone();
    invalid.schema_version += 1;
    assert!(invalid.validate().is_err());
    invalid = valid.clone();
    invalid.simulation_build.push_str("different");
    assert!(invalid.validate().is_err());
    invalid = valid.clone();
    invalid.team.fighters[0].mind.caution = f32::NAN;
    assert!(invalid.validate().is_err());
    invalid = valid.clone();
    invalid.team.fighters[1].fighter_id = invalid.team.fighters[0].fighter_id.clone();
    assert!(invalid.validate().is_err());
    invalid = valid.clone();
    invalid.team.standard_bearer = 999;
    assert!(invalid.validate().is_err());
    invalid = valid.clone();
    invalid.team.fighters[0].equipment.equipped[0] = Some(23);
    assert!(invalid.validate().is_err());
    let mut larger = owner.clone();
    larger.teams[0].gold = 1_000;
    larger.unlock_fourth().unwrap();
    assert!(
        Challenge::new(valid, snapshot(&larger, 0), 42)
            .unwrap_err()
            .contains("League mismatch")
    );
    Challenge::new(snapshot(&larger, 0), snapshot(&larger, 1), 42)
        .unwrap()
        .run()
        .unwrap()
        .verify()
        .unwrap();
}

#[test]
fn replay_rejects_altered_result_seed_arena_and_unfinished_matches() {
    let owner = Campaign::new();
    let challenge = Challenge::new(snapshot(&owner, 0), snapshot(&owner, 1), 42).unwrap();
    let (_, battle) = challenge.start().unwrap();
    assert!(ChallengeResult::capture(challenge.clone(), &battle).is_err());
    let mut wrong_arena = challenge.clone();
    wrong_arena.arena = Arena::OpenField;
    assert!(wrong_arena.start().is_err());
    let valid = challenge.run().unwrap();
    valid.verify().unwrap();
    let mut forged = valid.clone();
    forged.battle["units"][0]["health"] = serde_json::json!(999);
    assert!(forged.verify().unwrap_err().contains("replay differs"));
    forged = valid.clone();
    forged.battle["tick"] = serde_json::json!(1);
    assert!(forged.verify().is_err());
    forged = valid;
    forged.challenge.seed += 1;
    assert!(forged.verify().is_err());
}

#[test]
fn exports_never_overwrite_saves_and_input_size_is_bounded() {
    let scratch = Scratch::new();
    let owner = Campaign::new();
    let save = scratch.file("campaign.json");
    owner.save(&save).unwrap();
    let before = std::fs::read(&save).unwrap();
    assert!(snapshot(&owner, 0).save(&save).is_err());
    assert_eq!(std::fs::read(&save).unwrap(), before);
    let oversized = scratch.file("large.json");
    let file = std::fs::File::create(&oversized).unwrap();
    file.set_len(8 * 1024 * 1024 + 1).unwrap();
    assert!(
        TeamSnapshot::load(&oversized)
            .unwrap_err()
            .contains("8 MiB")
    );
    let malformed = scratch.file("broken.json");
    std::fs::write(&malformed, b"{not a snapshot}").unwrap();
    assert!(TeamSnapshot::load(&malformed).is_err());
}

fn cli(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_phalanx-challenge"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn command_line_exchanges_files_between_separate_installation_directories() {
    let a = Scratch::new();
    let b = Scratch::new();
    Campaign::new().save(&a.file("save.json")).unwrap();
    Campaign::new().save(&b.file("save.json")).unwrap();
    let before_a = std::fs::read(a.file("save.json")).unwrap();
    let before_b = std::fs::read(b.file("save.json")).unwrap();
    for install in [&a, &b] {
        let output = cli(
            &install.0,
            &["export", "--save", "save.json", "--output", "team.json"],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    std::fs::copy(a.file("team.json"), b.file("opponent.json")).unwrap();
    let output = cli(
        &b.0,
        &[
            "fight",
            "--home",
            "team.json",
            "--away",
            "opponent.json",
            "--seed",
            "123",
            "--output",
            "result.json",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::copy(b.file("result.json"), a.file("returned.json")).unwrap();
    let output = cli(&a.0, &["verify", "--result", "returned.json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read(a.file("save.json")).unwrap(), before_a);
    assert_eq!(std::fs::read(b.file("save.json")).unwrap(), before_b);
    assert!(
        !cli(
            &b.0,
            &["export", "--save", "save.json", "--output", "save.json"]
        )
        .status
        .success()
    );
    assert!(!cli(&b.0, &["fight", "--seed"]).status.success());
}
