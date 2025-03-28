use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::{CoinBalance, IntentionSet, Pubkey, Txid, Utxo};

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PoolInfo {
    pub key: Pubkey,
    pub key_derivation_path: Vec<Vec<u8>>,
    pub name: String,
    pub address: String,
    pub nonce: u64,
    pub coin_reserved: Vec<CoinBalance>,
    pub btc_reserved: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}

/// The response for the `get_pool_list` function.
pub type GetPoolListResponse = Vec<PoolInfo>;

/// The parameters for the `get_pool_info` function.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetPoolInfoArgs {
    pub pool_address: String,
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
pub type GetMinimalTxValueResponse = u64;

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

/// The parameters for the `unconfirm_tx` function.
///
/// This function will be called by REE Orchestrator when
/// a previously confirmed transaction is unconfirmed because of a reorg or other reason.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UnconfirmTxArgs {
    pub txid: Txid,
}

/// The response for the `unconfirm_tx` function.
pub type UnconfirmTxResponse = Result<(), String>;

/// The parameters for the `rollback_tx` function.
///
/// This function will be called by REE Orchestrator when
/// an unconfirmed transaction is rejected by the Mempool.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RollbackTxArgs {
    pub txid: Txid,
}

/// The response for the `rollback_tx` function.
pub type RollbackTxResponse = Result<(), String>;

/// Parameters for the `new_block` function.
///
/// This function is called by the REE Orchestrator when
/// a new block is detected by the Rune Indexer.
///
/// The `confirmed_txids` field contains the txids of all transactions confirmed in the new block,
/// which are associated with the exchange to be called.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NewBlockArgs {
    pub block_height: u64,
    pub block_hash: String,
    pub block_time: u64,
    pub confirmed_txids: Vec<Txid>,
}

/// The response for the `new_block` function.
pub type NewBlockResponse = Result<(), String>;
