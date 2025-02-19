use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct InvokeArgs {
    pub psbt_hex: String,
    pub intention_set: IntentionSet,
}

/// Invoke status code to be used in the response of invoke function,
/// will be formatted as a string before returning to the caller
///
/// 4xx - InvokeArgs Errors
/// 5xx - Orchestrator Errors
/// 7xx - Exchange Errors
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum InvokeStatus {
    /// Invalid psbt_hex
    _401(String),
    /// Invalid psbt data
    _402(String),
    /// Transaction fee too low
    _403(String),
    /// Input UTXO already spent
    _404(String),
    /// Invalid OP_RETURN data
    _405(String),
    /// Invalid rune balance in psbt
    _406(String),
    /// Invalid intention
    _407 {
        intention_index: usize,
        error: String,
    },
    /// Intention mismatched with the psbt
    _408 {
        intention_index: usize,
        error: String,
    },
    /// Rune indexer not reachable
    _501(String),
    /// Exchange not reachable
    _701 {
        intention_index: usize,
        error: String,
    },
    /// Exchange returned error
    _702 {
        intention_index: usize,
        error: String,
    },
    /// Exchange returned invalid psbt
    _703 {
        intention_index: usize,
        error: String,
    },
}

/// If successful, returns the txid of the transaction broadcasted,
/// otherwise returns the formatted status message
pub type InvokeResponse = Result<String, String>;

impl core::fmt::Display for InvokeStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InvokeStatus::_401(msg) => write!(f, "401 Invalid psbt hex: {}", msg),
            InvokeStatus::_402(msg) => write!(f, "402 Invalid psbt data: {}", msg),
            InvokeStatus::_403(msg) => write!(f, "403 Transaction fee too low: {}", msg),
            InvokeStatus::_404(msg) => write!(f, "404 Input UTXO already spent: {}", msg),
            InvokeStatus::_405(msg) => write!(f, "405 Invalid OP_RETURN data: {}", msg),
            InvokeStatus::_406(msg) => write!(f, "406 Invalid rune balance in psbt: {}", msg),
            InvokeStatus::_407 {
                intention_index,
                error,
            } => {
                write!(
                    f,
                    "407 Invalid intention: Intention index: {}, error: {}",
                    intention_index, error
                )
            }
            InvokeStatus::_408 {
                intention_index,
                error,
            } => {
                write!(
                    f,
                    "408 Intention mismatched with the psbt: Intention index: {}, error: {}",
                    intention_index, error
                )
            }
            InvokeStatus::_501(msg) => write!(f, "501 Rune indexer not reachable: {}", msg),
            InvokeStatus::_701 {
                intention_index,
                error,
            } => {
                write!(
                    f,
                    "701 Exchange not reachable: Intention index: {}, error: {}",
                    intention_index, error
                )
            }
            InvokeStatus::_702 {
                intention_index,
                error,
            } => {
                write!(
                    f,
                    "702 Exchange returned error: Intention index: {}, error: {}",
                    intention_index, error
                )
            }
            InvokeStatus::_703 {
                intention_index,
                error,
            } => {
                write!(
                    f,
                    "703 Exchange returned invalid psbt: Intention index: {}, error: {}",
                    intention_index, error
                )
            }
        }
    }
}
