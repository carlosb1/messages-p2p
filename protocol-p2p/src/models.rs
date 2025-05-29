use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RaftRole {
    Follower,
    Candidate,
    Leader,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RaftNodeState {
    pub current_term: u64,
    pub voted_for: Option<String>,
    pub role: RaftRole,
}