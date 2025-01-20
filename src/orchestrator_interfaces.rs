use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NewBlockDetectedArgs {
    pub block_height: u32,
    pub block_hash: String,
    pub tx_ids: Vec<String>,
}
