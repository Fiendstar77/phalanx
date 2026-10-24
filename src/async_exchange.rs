//! Transport-independent asynchronous exchange contract and a serializable local emulator.
//! Owner labels are local fixtures, not authenticated accounts. No campaign reward authority.
use crate::challenges::{Challenge, ChallengeResult, TeamSnapshot};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Publication {
    pub id: u64,
    pub owner: String,
    pub snapshot: TeamSnapshot,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ticket {
    pub id: u64,
    pub publications: [u64; 2],
    pub challenge: Challenge,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub ticket: u64,
    pub owners: [String; 2],
    pub result: ChallengeResult,
}
/// A network adapter can implement this boundary without moving authoritative combat into UI.
pub trait ChallengeService {
    fn publish(&mut self, owner: &str, snapshot: TeamSnapshot) -> Result<Publication, String>;
    fn discover(&self, league: usize) -> Vec<Publication>;
    fn issue(&mut self, home: u64, away: u64, seed: u64) -> Result<Ticket, String>;
    fn submit(&mut self, ticket: u64, result: ChallengeResult) -> Result<Receipt, String>;
    fn inbox(&self, owner: &str) -> Vec<Receipt>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalExchange {
    schema_version: u32,
    next_id: u64,
    publications: BTreeMap<u64, Publication>,
    tickets: BTreeMap<u64, Ticket>,
    delivered: BTreeMap<u64, Receipt>,
}
impl Default for LocalExchange {
    fn default() -> Self {
        Self {
            schema_version: 1,
            next_id: 1,
            publications: BTreeMap::new(),
            tickets: BTreeMap::new(),
            delivered: BTreeMap::new(),
        }
    }
}
impl LocalExchange {
    fn id(&mut self) -> Result<u64, String> {
        if self.schema_version != 1 || self.next_id == u64::MAX {
            return Err("Unsupported exchange state".into());
        }
        let id = self.next_id;
        self.next_id += 1;
        Ok(id)
    }
}
impl ChallengeService for LocalExchange {
    fn publish(&mut self, owner: &str, snapshot: TeamSnapshot) -> Result<Publication, String> {
        if owner.is_empty()
            || owner.len() > 48
            || !owner
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err(
                "Use a short local owner label with letters, digits, hyphens or underscores".into(),
            );
        }
        snapshot.validate()?;
        let publication = Publication {
            id: self.id()?,
            owner: owner.into(),
            snapshot,
        };
        self.publications
            .insert(publication.id, publication.clone());
        Ok(publication)
    }
    fn discover(&self, league: usize) -> Vec<Publication> {
        let mut latest = BTreeMap::new();
        for p in self.publications.values() {
            if p.snapshot.team.fighters.len() == league {
                latest.insert(p.owner.clone(), p.clone());
            }
        }
        latest.into_values().collect()
    }
    fn issue(&mut self, home: u64, away: u64, seed: u64) -> Result<Ticket, String> {
        let a = self
            .publications
            .get(&home)
            .ok_or("Unknown challenger publication")?;
        let b = self
            .publications
            .get(&away)
            .ok_or("Unknown opponent publication")?;
        if a.owner == b.owner {
            return Err("Choose another owner for this exhibition".into());
        }
        let challenge = Challenge::new(a.snapshot.clone(), b.snapshot.clone(), seed)?;
        let ticket = Ticket {
            id: self.id()?,
            publications: [home, away],
            challenge,
        };
        self.tickets.insert(ticket.id, ticket.clone());
        Ok(ticket)
    }
    fn submit(&mut self, id: u64, result: ChallengeResult) -> Result<Receipt, String> {
        let ticket = self.tickets.get(&id).ok_or("Unknown challenge ticket")?;
        if serde_json::to_value(&ticket.challenge).map_err(|e| e.to_string())?
            != serde_json::to_value(&result.challenge).map_err(|e| e.to_string())?
        {
            return Err(
                "Submitted teams, build, seed or arena differ from the issued challenge".into(),
            );
        }
        if let Some(receipt) = self.delivered.get(&id) {
            if serde_json::to_value(&receipt.result).map_err(|e| e.to_string())?
                != serde_json::to_value(&result).map_err(|e| e.to_string())?
            {
                return Err("A different result was already delivered".into());
            }
            return Ok(receipt.clone());
        }
        result.verify()?;
        let receipt = Receipt {
            ticket: id,
            owners: ticket
                .publications
                .map(|p| self.publications[&p].owner.clone()),
            result,
        };
        self.delivered.insert(id, receipt.clone());
        Ok(receipt)
    }
    fn inbox(&self, owner: &str) -> Vec<Receipt> {
        self.delivered
            .values()
            .filter(|r| r.owners.iter().any(|o| o == owner))
            .cloned()
            .collect()
    }
}
