use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SignPsbtArgs {
    pub psbt_hex: String,
    pub txid: Txid,
    pub intention_set: IntentionSet,
    pub intention_index: u32,
    pub zero_confirmed_tx_count_in_queue: u32,
}

pub type SignPsbtResponse = Result<String, String>;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FinalizeTxArgs {
    pub pool_key: Pubkey,
    pub txid: Txid,
}

pub type FinalizeTxResponse = ();

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RollbackTxArgs {
    pub pool_key: Pubkey,
    pub txid: Txid,
}

pub type RollbackTxResponse = ();

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PoolOverview {
    pub id: Pubkey,
    pub name: String,
    pub address: String,
    pub nonce: u64,
    pub btc_reserved: u64,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetPoolListArgs {
    pub from: Option<Pubkey>,
    pub limit: u32,
}

pub type GetPoolListResponse = Vec<PoolOverview>;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PoolInfo {
    pub id: Pubkey,
    pub name: String,
    pub address: String,
    pub nonce: u64,
    pub coin_reserved: Vec<CoinBalance>,
    pub btc_reserved: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetPoolInfoArgs {
    pub pool_address: String,
}

pub type GetPoolInfoResponse = Option<PoolInfo>;
