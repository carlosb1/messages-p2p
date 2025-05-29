use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")] // para serializar como { "type": "RequestVote", ... }
pub enum RaftElectionMessage {
    RequestVote {
        term: u64,
        candidate_id: String,
        last_log_index: u64,
        last_log_term: u64,
    },
    RequestVoteResponse {
        term: u64,
        vote_granted: bool,
        voter_id: String,
    },
}

