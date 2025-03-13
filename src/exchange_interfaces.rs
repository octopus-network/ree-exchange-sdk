use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::{CoinBalance, IntentionSet, Pubkey, Txid, Utxo};

/// The parameters for the `get_pool_list` function.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetPoolListArgs {
    pub from: Option<Pubkey>,
    pub limit: u32,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PoolOverview {
    pub key: Pubkey,
    pub name: String,
    pub address: String,
    pub nonce: u64,
    pub btc_reserved: u64,
    pub coin_reserved: Vec<CoinBalance>,
}

/// The response for the `get_pool_list` function.
pub type GetPoolListResponse = Vec<PoolOverview>;

/// The parameters for the `get_pool_info` function.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetPoolInfoArgs {
    pub pool_address: String,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PoolInfo {
    pub key: Pubkey,
    pub name: String,
    pub address: String,
    pub nonce: u64,
    pub coin_reserved: Vec<CoinBalance>,
    pub btc_reserved: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}

/// The response for the `get_pool_info` function.
pub type GetPoolInfoResponse = Option<PoolInfo>;

/// The parameters for the `get_minimal_tx_value` function.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetMinimalTxValueArgs {
    pub pool_address: String,
    pub zero_confirmed_tx_queue_length: u32,
}

/// The response for the `get_minimal_tx_value` function.
pub type GetMinimalTxValueResponse = Result<u64, String>;

/// The parameters for the `execute_tx` function.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExecuteTxArgs {
    pub psbt_hex: String,
    pub txid: Txid,
    pub intention_set: IntentionSet,
    pub intention_index: u32,
    pub zero_confirmed_tx_queue_length: u32,
}

/// The response for the `execute_tx` function.
pub type ExecuteTxResponse = Result<String, String>;

/// The parameters for the `finalize_tx` function.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FinalizeTxArgs {
    pub pool_key: Pubkey,
    pub txid: Txid,
}

/// The response for the `finalize_tx` function.
pub type FinalizeTxResponse = Result<(), String>;

/// The parameters for the `rollback_tx` function.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RollbackTxArgs {
    pub pool_key: Pubkey,
    pub txid: Txid,
}

/// The response for the `rollback_tx` function.
pub type RollbackTxResponse = Result<(), String>;
