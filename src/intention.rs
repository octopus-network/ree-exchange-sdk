use alloc::collections::BTreeSet;
use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::{CoinBalance, CoinId};

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct InputCoin {
    // The address of the owner of the coins
    pub from: String,
    pub coin: CoinBalance,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputCoin {
    // The address of the receiver of the coins
    pub to: String,
    pub coin: CoinBalance,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Intention {
    pub exchange_id: String,
    pub action: String,
    pub action_params: String,
    pub pool_address: String,
    pub nonce: u64,
    pub pool_utxo_spend: Vec<String>,
    pub pool_utxo_receive: Vec<String>,
    pub input_coins: Vec<InputCoin>,
    pub output_coins: Vec<OutputCoin>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct IntentionSet {
    pub initiator_address: String,
    pub tx_fee_in_sats: u64,
    pub intentions: Vec<Intention>,
}

impl Intention {
    //
    pub fn input_coin_ids(&self) -> Vec<CoinId> {
        self.input_coins
            .iter()
            .map(|input_coin| input_coin.coin.id.clone())
            .collect()
    }
    //
    pub fn output_coin_ids(&self) -> Vec<CoinId> {
        self.output_coins
            .iter()
            .map(|output_coin| output_coin.coin.id.clone())
            .collect()
    }
    //
    pub fn all_coin_ids(&self) -> Vec<CoinId> {
        let mut coin_ids: BTreeSet<CoinId> = BTreeSet::new();
        for coin_id in self.input_coin_ids().into_iter() {
            coin_ids.insert(coin_id);
        }
        for coin_id in self.output_coin_ids().into_iter() {
            coin_ids.insert(coin_id);
        }
        coin_ids.into_iter().collect()
    }
}

impl IntentionSet {
    //
    pub fn all_input_coins(&self) -> Vec<InputCoin> {
        let mut input_coins = BTreeSet::new();
        for intention in self.intentions.iter() {
            for input_coin in intention.input_coins.iter() {
                input_coins.insert(input_coin.clone());
            }
        }
        input_coins.into_iter().collect()
    }
    //
    pub fn all_output_coins(&self) -> Vec<OutputCoin> {
        let mut output_coins = BTreeSet::new();
        for intention in self.intentions.iter() {
            for output_coin in intention.output_coins.iter() {
                output_coins.insert(output_coin.clone());
            }
        }
        output_coins.into_iter().collect()
    }
    //
    pub fn all_coin_ids(&self) -> Vec<CoinId> {
        let mut coin_ids: BTreeSet<CoinId> = BTreeSet::new();
        for intention in self.intentions.iter() {
            for coin_id in intention.all_coin_ids().into_iter() {
                coin_ids.insert(coin_id);
            }
        }
        coin_ids.into_iter().collect()
    }
}
