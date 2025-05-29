use serde::{Deserialize, Serialize};
use sled;
use serde_json;
use crate::messages::RaftElectionMessage;

pub fn store_vote(db: &sled::Db, peer_id: &str, msg: &RaftElectionMessage) -> Result<(), Box<dyn std::error::Error>> {
    let key = format!("election/votes/{}", peer_id);
    let value = serde_json::to_vec(msg)?;
    db.insert(key, value)?;
    Ok(())
}

pub fn get_vote(db: &sled::Db, peer_id: &str) -> Result<Option<RaftElectionMessage>, Box<dyn std::error::Error>> {
    let key = format!("election/votes/{}", peer_id);
    if let Some(value) = db.get(key)? {
        let msg: RaftElectionMessage = serde_json::from_slice(&value)?;
        Ok(Some(msg))
    } else {
        Ok(None)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RaftNodeState {
    current_term: u64,
    voted_for: Option<String>,
    role: String,
}

fn store_node_state(db: &sled::Db, state: &RaftNodeState) -> Result<(), Box<dyn std::error::Error>> {
    db.insert("election/state", serde_json::to_vec(state)?)?;
    Ok(())
}

