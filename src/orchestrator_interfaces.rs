use crate::IntentionSet;
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RegisterExchangeArgs {
    pub exchange_id: String,
    pub exchange_canister: Principal,
    pub client_canisters: Vec<Principal>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct InvokeArgs {
    pub psbt_hex: String,
    pub intention_set: IntentionSet,
    pub initiator_utxo_proof: Vec<u8>,
}

pub type InvokeStatusSubCode = u8;

/// Invoke status code to be used in the response of invoke function,
/// will be formatted as a string before returning to the caller
///
/// 3xx - Latency caused errors, may be retried
/// 4xx - InvokeArgs errors
/// 5xx - Orchestrator errors
/// 7xx - Exchange errors
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum InvokeStatus {
    _200,
    _301,
    _401(InvokeStatusSubCode),
    _402(InvokeStatusSubCode),
    _403,
    _404,
    _405,
    _406,
    _407,
    _408,
    _409(InvokeStatusSubCode),
    _410(InvokeStatusSubCode),
    _411,
    _412,
    _413(InvokeStatusSubCode),
    _414(InvokeStatusSubCode),
    _415(InvokeStatusSubCode),
    _501,
    _502(InvokeStatusSubCode),
    _503(String),
    _504(String),
    _505(InvokeStatusSubCode),
    _599 { txid: String, inner_error: String },
    _701 { exchange_id: String, error: String },
    _702 { exchange_id: String, error: String },
    _703(InvokeStatusSubCode),
    _704(InvokeStatusSubCode),
}

/// If successful, returns the txid of the transaction broadcasted,
/// otherwise returns the formatted status message
pub type InvokeResponse = Result<String, String>;

impl core::fmt::Display for InvokeStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InvokeStatus::_200 => write!(f, "200"),
            InvokeStatus::_301 => write!(f, "301"),
            InvokeStatus::_401(sub_code) => write!(f, "401:{:03}", sub_code),
            InvokeStatus::_402(sub_code) => write!(f, "402:{:03}", sub_code),
            InvokeStatus::_403 => write!(f, "403"),
            InvokeStatus::_404 => write!(f, "404"),
            InvokeStatus::_405 => write!(f, "405"),
            InvokeStatus::_406 => write!(f, "406"),
            InvokeStatus::_407 => write!(f, "407"),
            InvokeStatus::_408 => write!(f, "408"),
            InvokeStatus::_409(sub_code) => write!(f, "409:{:03}", sub_code),
            InvokeStatus::_410(sub_code) => write!(f, "410:{:03}", sub_code),
            InvokeStatus::_411 => write!(f, "411"),
            InvokeStatus::_412 => write!(f, "412"),
            InvokeStatus::_413(sub_code) => write!(f, "413:{:03}", sub_code),
            InvokeStatus::_414(sub_code) => write!(f, "414:{:03}", sub_code),
            InvokeStatus::_415(sub_code) => write!(f, "415:{:03}", sub_code),
            InvokeStatus::_501 => write!(f, "501"),
            InvokeStatus::_502(sub_code) => write!(f, "502:{:03}", sub_code),
            InvokeStatus::_503(msg) => write!(f, "503: {}", msg),
            InvokeStatus::_504(msg) => write!(f, "504: {}", msg),
            InvokeStatus::_505(sub_code) => write!(f, "505:{:03}", sub_code),
            InvokeStatus::_599 { txid, inner_error } => {
                write!(f, "599: Txid: {}, Inner error: {}", txid, inner_error)
            }
            InvokeStatus::_701 { exchange_id, error } => {
                write!(f, "701: Exchange id: {}, error: {}", exchange_id, error)
            }
            InvokeStatus::_702 { exchange_id, error } => {
                write!(f, "702: Exchange id: {}, error: {}", exchange_id, error)
            }
            InvokeStatus::_703(sub_code) => write!(f, "703:{:03}", sub_code),
            InvokeStatus::_704(sub_code) => write!(f, "704:{:03}", sub_code),
        }
    }
}

pub const TESTNET4_ORCHESTRATOR_CANISTER: &'static str = "hvyp5-5yaaa-aaaao-qjxha-cai";
// mainnet orchestrator
pub const ORCHESTRATOR_CANISTER: &'static str = "kqs64-paaaa-aaaar-qamza-cai";

pub fn ensure_testnet4_orchestrator() -> Result<(), String> {
    let o = Principal::from_str(TESTNET4_ORCHESTRATOR_CANISTER).expect("is valid principal; qed");
    (o == ic_cdk::caller())
        .then(|| ())
        .ok_or("Access denied".to_string())
}

pub fn ensure_orchestrator() -> Result<(), String> {
    let o = Principal::from_str(ORCHESTRATOR_CANISTER).expect("is valid principal; qed");
    (o == ic_cdk::caller())
        .then(|| ())
        .ok_or("Access denied".to_string())
}
