//! Portable frozen teams and reproducible local exhibition challenges.
//! Files never authorize campaign mutation or establish online result authenticity.
use crate::{
    Battle,
    learned::Controller,
    progression::{Campaign, Phalanx},
    terrain::Arena,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    io::{Read, Write},
    path::Path,
};

pub const BUILD: &str = concat!(env!("CARGO_PKG_VERSION"), ":", env!("PHALANX_BUILD_ID"));
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamSnapshot {
    pub schema_version: u32,
    pub simulation_build: String,
    pub next_battle_id: u64,
    pub team: Phalanx,
}
impl TeamSnapshot {
    pub fn from_campaign(campaign: &Campaign, side: usize) -> Result<Self, String> {
        campaign.validate()?;
        let mut snapshot = Self {
            schema_version: 1,
            simulation_build: BUILD.into(),
            next_battle_id: campaign.next_battle_id,
            team: campaign.teams.get(side).ok_or("invalid squad")?.clone(),
        };
        // Default looks depend on the local side. Publish the resolved appearance
        // so becoming the away team cannot change a fighter's face or body base.
        for (slot, fighter) in snapshot.team.fighters.iter_mut().enumerate() {
            fighter.appearance = fighter.appearance.resolved(side, slot);
            fighter.appearance.female = Some(
                fighter
                    .appearance
                    .female
                    .unwrap_or(fighter.title == crate::CommandTitle::Queen),
            );
        }
        snapshot.validate()?;
        Ok(snapshot)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported team snapshot version".into());
        }
        if self.simulation_build != BUILD {
            return Err(
                "Snapshot uses a different game build; exchange files from the same build".into(),
            );
        }
        if self
            .team
            .fighters
            .iter()
            .any(|f| f.appearance.female.is_none())
        {
            return Err("Snapshot must include each fighter's resolved body base".into());
        }
        self.team.validate(self.next_battle_id)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let snapshot: Self = read_json(path)?;
        snapshot.validate()?;
        Ok(snapshot)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        write_json(path, self)
    }
    pub fn summary(&self) -> String {
        format!(
            "{} | League {}\n{}",
            self.team.name,
            self.team.fighters.len(),
            self.team
                .fighters
                .iter()
                .map(|f| format!(
                    "{} / {} / {} | level {} | {} battles, {} wins",
                    f.name,
                    f.title.name(),
                    f.class.name(),
                    1 + (f.experience / 100).min(9),
                    f.battles,
                    f.wins
                ))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Challenge {
    pub schema_version: u32,
    pub teams: [TeamSnapshot; 2],
    pub seed: u64,
    pub arena: Arena,
}
impl Challenge {
    pub fn new(home: TeamSnapshot, away: TeamSnapshot, seed: u64) -> Result<Self, String> {
        home.validate()?;
        away.validate()?;
        if home.team.fighters.len() != away.team.fighters.len() {
            return Err(
                "League mismatch: both teams must field the same number of fighters".into(),
            );
        }
        let arena = crate::pressure::generate(&[home.team.clone(), away.team.clone()], seed);
        Ok(Self {
            schema_version: 1,
            teams: [home, away],
            seed,
            arena,
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported challenge version".into());
        }
        let expected = Self::new(self.teams[0].clone(), self.teams[1].clone(), self.seed)?;
        if self.arena != expected.arena {
            return Err("Challenge arena does not match its frozen teams and seed".into());
        }
        Ok(())
    }
    /// Build an isolated match. Source identities remain in the packet; combat IDs
    /// are namespaced by side so two copies of the same squad can fight safely.
    pub fn start(&self) -> Result<(Campaign, Battle), String> {
        self.validate()?;
        let mut campaign = Campaign {
            schema_version: 1,
            teams: std::array::from_fn(|i| self.teams[i].team.clone()),
            rivals: Vec::new(),
            last_report: None,
            next_battle_id: self.teams.iter().map(|s| s.next_battle_id).max().unwrap(),
            last_settled_id: None,
        };
        for (side, team) in campaign.teams.iter_mut().enumerate() {
            for (slot, fighter) in team.fighters.iter_mut().enumerate() {
                fighter.fighter_id = format!("challenge-{side}-{slot}");
            }
        }
        let (_, battle) = campaign.begin_battle(self.seed, self.arena, Controller::Utility)?;
        Ok((campaign, battle))
    }
    pub fn run(&self) -> Result<ChallengeResult, String> {
        let (_, mut battle) = self.start()?;
        while !battle.finished {
            battle.step();
        }
        ChallengeResult::capture(self.clone(), &battle)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeResult {
    pub schema_version: u32,
    pub challenge: Challenge,
    pub battle: serde_json::Value,
}
impl ChallengeResult {
    pub fn capture(challenge: Challenge, battle: &Battle) -> Result<Self, String> {
        challenge.validate()?;
        if !battle.finished
            || battle.seed != challenge.seed
            || battle.initial_arena != challenge.arena
        {
            return Err("Finish the frozen challenge before exporting a result".into());
        }
        Ok(Self {
            schema_version: 1,
            challenge,
            battle: serde_json::to_value(battle).map_err(|e| e.to_string())?,
        })
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let result: Self = read_json(path)?;
        result.validate()?;
        Ok(result)
    }
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.battle.get("finished") != Some(&serde_json::Value::Bool(true))
        {
            return Err("Unsupported or unfinished challenge result".into());
        }
        self.challenge.validate()
    }
    pub fn verify(&self) -> Result<(), String> {
        self.validate()?;
        let replay = self.challenge.run()?;
        if replay.battle != self.battle {
            return Err("Result rejected: replay differs from the submitted battle".into());
        }
        Ok(())
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        write_json(path, self)
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("Challenge file exceeds the 8 MiB limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("Invalid challenge file: {e}"))
}
/// Exchange exports never replace an existing file, including a campaign save.
fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("Challenge file exceeds the 8 MiB limit".into());
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("Cannot create {}: {e}", path.display()))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}
