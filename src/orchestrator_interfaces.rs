use crate::IntentionSet;
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct InvokeArgs {
    pub psbt_hex: String,
    pub intention_set: IntentionSet,
    pub initiator_utxo_proof: Vec<u8>,
}

/// If successful, returns the txid of the transaction broadcasted,
/// otherwise returns the formatted status message
pub type InvokeResponse = Result<String, String>;

pub const TESTNET4_ORCHESTRATOR_CANISTER: &'static str = "hvyp5-5yaaa-aaaao-qjxha-cai";
// mainnet orchestrator
pub const ORCHESTRATOR_CANISTER: &'static str = "kqs64-paaaa-aaaar-qamza-cai";

pub fn ensure_testnet4_orchestrator() -> Result<(), String> {
    let o = Principal::from_str(TESTNET4_ORCHESTRATOR_CANISTER).expect("is valid principal; qed");
    (o == ic_cdk::api::msg_caller())
        .then(|| ())
        .ok_or("Access denied".to_string())
}

pub fn ensure_orchestrator() -> Result<(), String> {
    let o = Principal::from_str(ORCHESTRATOR_CANISTER).expect("is valid principal; qed");
    (o == ic_cdk::api::msg_caller())
        .then(|| ())
        .ok_or("Access denied".to_string())
}
