use crate::{CoinBalance, IntentionSet, Pubkey, Txid, Utxo};
use alloc::borrow::Cow;
use candid::CandidType;
use ic_stable_structures::{Storable, storable::Bound};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PoolBasic {
    pub name: String,
    pub address: String,
}

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

pub type GetPoolListResponse = Vec<PoolBasic>;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetPoolInfoArgs {
    pub pool_address: String,
}

pub type GetPoolInfoResponse = Option<PoolInfo>;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExecuteTxArgs {
    pub psbt_hex: String,
    pub txid: Txid,
    pub intention_set: IntentionSet,
    pub intention_index: u32,
    pub zero_confirmed_tx_queue_length: u32,
    pub is_reapply: Option<bool>,
}

impl ExecuteTxArgs {
    pub fn psbt(&self) -> Result<bitcoin::Psbt, String> {
        let raw = hex::decode(&self.psbt_hex).map_err(|_| "invalid psbt".to_string())?;
        bitcoin::Psbt::deserialize(raw.as_slice()).map_err(|_| "invalid psbt".to_string())
    }
}

pub type ExecuteTxResponse = Result<String, String>;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RollbackTxArgs {
    pub txid: Txid,
    pub reason_code: String,
}

pub type RollbackTxResponse = Result<(), String>;

/// The parameters for the hook `on_block_received` and `on_block_processed`
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NewBlockInfo {
    pub block_height: u32,
    pub block_hash: String,
    /// The block timestamp in seconds since the Unix epoch.
    pub block_timestamp: u64,
    pub confirmed_txids: Vec<Txid>,
}

pub type NewBlockArgs = NewBlockInfo;

pub type NewBlockResponse = Result<(), String>;

impl Storable for NewBlockInfo {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let bytes = bincode::serialize(self).unwrap();
        Cow::Owned(bytes)
    }

    fn into_bytes(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        bincode::deserialize(bytes.as_ref()).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}
