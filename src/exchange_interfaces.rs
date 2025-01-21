use alloc::collections::BTreeSet;
use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::{txid::Txid, CoinId, Pubkey};

#[derive(CandidType, Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CoinBalance {
    pub id: CoinId,
    pub value: u128,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct InputRune {
    pub tx_id: Txid,
    pub vout: u32,
    pub btc_amount: u64,
    pub coin_balance: Option<CoinBalance>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OutputRune {
    pub btc_amount: u64,
    pub coin_balance: Option<CoinBalance>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AssetWithOwner {
    pub coin_balance: CoinBalance,
    pub owner_address: String,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReeInstruction {
    pub exchange_id: String,
    pub method: String,
    pub pool_id: Option<String>,
    pub nonce: Option<u64>,
    pub input_coins: Vec<AssetWithOwner>,
    pub output_coins: Vec<AssetWithOwner>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SignPsbtArgs {
    pub psbt_hex: String,
    pub tx_id: Txid,
    pub instruction: ReeInstruction,
    pub input_runes: Vec<InputRune>,
    pub output_runes: Vec<OutputRune>,
    pub zero_confirmed_tx_count_in_queue: u32,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FinalizeTxArgs {
    pub pool_key: Pubkey,
    pub tx_id: Txid,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RollbackTxArgs {
    pub pool_key: Pubkey,
    pub tx_id: Txid,
}

impl InputRune {
    //
    pub fn outpoint(&self) -> String {
        format!("{}:{}", self.tx_id, self.vout)
    }
}

impl ReeInstruction {
    //
    pub fn input_coin_ids(&self) -> Vec<CoinId> {
        self.input_coins
            .iter()
            .map(|coin| coin.coin_balance.id.clone())
            .collect()
    }
    //
    pub fn output_coin_ids(&self) -> Vec<CoinId> {
        self.output_coins
            .iter()
            .map(|coin| coin.coin_balance.id.clone())
            .collect()
    }
    //
    pub fn all_coin_ids(&self) -> Vec<CoinId> {
        let mut coin_ids: BTreeSet<CoinId> = BTreeSet::new();
        for coin in self.input_coins.iter() {
            coin_ids.insert(coin.coin_balance.id.clone());
        }
        for coin in self.output_coins.iter() {
            coin_ids.insert(coin.coin_balance.id.clone());
        }
        coin_ids.into_iter().collect()
    }
}
