use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::IntentionSet;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct InvokeArgs {
    pub psbt_hex: String,
    pub intention_set: IntentionSet,
}

/// Invoke status code to be used in the response of invoke function,
/// will be formatted as a string before returning to the caller
///
/// 3xx - Latency caused errors, may be retried
/// 4xx - InvokeArgs errors
/// 5xx - Orchestrator errors
/// 7xx - Exchange errors
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum InvokeStatus {
    /// Success
    _200,
    /// Transaction fee too low
    _301(String),
    /// Another invoke is in progress
    _399,
    /// Invalid psbt_hex
    _401(String),
    /// Invalid psbt data
    _402(String),
    /// Input UTXO already spent
    _404(String),
    /// Invalid rune balance in psbt
    _406(String),
    /// Missing intentions
    _407,
    /// Too many intentions
    _408,
    /// Invalid intention
    _409 {
        intention_index: usize,
        error: String,
    },
    /// Intention mismatched with the psbt
    _410 {
        intention_index: usize,
        error: String,
    },
    /// Invoke is paused
    _501,
    /// Orchestrator internal error
    _502(String),
    /// Rune indexer not reachable
    _503(String),
    /// Rune indexer returned error
    _504(String),
    /// Invalid final tx
    _505(String),
    /// Invoke failed due to exchange error
    _599 { txid: String, inner_error: String },
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
        returned_psbt_hex: String,
        error: String,
    },
}

/// If successful, returns the txid of the transaction broadcasted,
/// otherwise returns the formatted status message
pub type InvokeResponse = Result<String, String>;

impl core::fmt::Display for InvokeStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InvokeStatus::_200 => write!(f, "200 Success."),
            InvokeStatus::_301(msg) => {
                write!(f, "301 Transaction fee too low: {}. Maybe try again.", msg)
            }
            InvokeStatus::_399 => write!(
                f,
                "399 Another invoke is in progress. Please try again later."
            ),
            InvokeStatus::_401(msg) => write!(f, "401 Invalid psbt hex: {}", msg),
            InvokeStatus::_402(msg) => write!(f, "402 Invalid psbt data: {}", msg),
            InvokeStatus::_404(msg) => write!(f, "404 Input UTXO already spent: {}", msg),
            InvokeStatus::_406(msg) => write!(f, "406 Invalid rune balance in psbt: {}", msg),
            InvokeStatus::_407 => write!(f, "407 Missing intentions."),
            InvokeStatus::_408 => write!(f, "408 Too many intentions."),
            InvokeStatus::_409 {
                intention_index,
                error,
            } => {
                write!(
                    f,
                    "409 Invalid intention: Intention index: {}, error: {}",
                    intention_index, error
                )
            }
            InvokeStatus::_410 {
                intention_index,
                error,
            } => {
                write!(
                    f,
                    "410 Intention mismatched with the psbt: Intention index: {}, error: {}",
                    intention_index, error
                )
            }
            InvokeStatus::_501 => write!(f, "501 Invoke is paused. Please contact support."),
            InvokeStatus::_502(msg) => write!(f, "502 Orchestrator internal error: {}", msg),
            InvokeStatus::_503(msg) => write!(f, "503 Rune indexer not reachable: {}", msg),
            InvokeStatus::_504(msg) => write!(f, "504 Rune indexer returned error: {}", msg),
            InvokeStatus::_505(msg) => write!(f, "505 Invalid final tx: {}", msg),
            InvokeStatus::_599 { txid, inner_error } => write!(
                f,
                "599 Invoke failed due to exchange error. Txid: {}, Inner error: {}",
                txid, inner_error
            ),
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
                returned_psbt_hex,
                error,
            } => {
                write!(
                    f,
                    "703 Exchange returned invalid psbt: Intention index: {}, returned psbt hex: {}, error: {}",
                    intention_index, returned_psbt_hex, error
                )
            }
        }
    }
}
