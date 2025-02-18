use alloc::{collections::BTreeSet, str::FromStr};
use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::{txid::Txid, CoinId, Pubkey};

#[derive(CandidType, Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CoinBalance {
    pub id: CoinId,
    pub value: u128,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct InputCoin {
    // The address of the owner of the coins
    pub from: String,
    pub coin: CoinBalance,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OutputCoin {
    // The address of the receiver of the coins
    pub to: String,
    pub coin: CoinBalance,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Intention {
    pub exchange_id: String,
    pub action: String,
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
    pub intentions: Vec<Intention>,
}

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

pub type FinalizeTxResponse = Result<(), String>;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RollbackTxArgs {
    pub pool_key: Pubkey,
    pub txid: Txid,
}

pub type RollbackTxResponse = Result<(), String>;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PoolOverview {
    pub id: Pubkey,
    pub name: String,
    pub address: String,
    pub coin_ids: Vec<CoinId>,
    pub nonce: u64,
    pub btc_supply: u64,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetPoolListArgs {
    pub from: Option<Pubkey>,
    pub limit: u32,
}

pub type GetPoolListResponse = Vec<PoolOverview>;

#[derive(CandidType, Eq, PartialEq, Clone, Debug, Deserialize, Serialize)]
pub struct Utxo {
    pub txid: Txid,
    pub vout: u32,
    pub maybe_rune: Option<CoinBalance>,
    pub sats: u64,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PoolInfo {
    pub id: Pubkey,
    pub name: String,
    pub address: String,
    pub coin_ids: Vec<CoinId>,
    pub nonce: u64,
    pub btc_supply: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetPoolInfoArgs {
    pub pool_key: Pubkey,
}

pub type GetPoolInfoResponse = Option<PoolInfo>;

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

impl Utxo {
    pub fn try_from(
        outpoint: impl AsRef<str>,
        maybe_rune: Option<CoinBalance>,
        sats: u64,
    ) -> Result<Self, String> {
        let parts = outpoint.as_ref().split(':').collect::<Vec<_>>();
        let txid = parts
            .get(0)
            .map(|s| Txid::from_str(s).map_err(|_| "Invalid txid in outpoint."))
            .transpose()?
            .ok_or("Invalid txid in outpoint.")?;
        let vout = parts
            .get(1)
            .map(|s| s.parse::<u32>().map_err(|_| "Invalid vout in outpoint."))
            .transpose()?
            .ok_or("Invalid vout in outpoint")?;
        Ok(Utxo {
            txid,
            vout,
            maybe_rune,
            sats,
        })
    }

    pub fn outpoint(&self) -> String {
        format!("{}:{}", self.txid, self.vout)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_ree_instruction_json() {
        let instruction_set_1 = IntentionSet {
            initiator_address: "bc1q8anrrgczju8zn02ww06slsfh9grm07de7r9e3k".to_string(),
            intentions: vec![Intention {
                exchange_id: "RICH_SWAP".to_string(),
                action: "add_liquidity".to_string(),
                pool_address: "bc1pxtmh763568jd8pz9m8wekt2yrqyntqv2wk465mgpzlr9f2aq2vqs52l0hq"
                    .to_string(),
                nonce: 1,
                pool_utxo_spend: vec![],
                pool_utxo_receive: vec![
                    "4b004c33c5b7bce9a8f5a3a0dab48dd47e33486a8cea5f63ef558849f5604b88:1"
                        .to_string(),
                ],
                input_coins: vec![
                    InputCoin {
                        from: "bc1q8anrrgczju8zn02ww06slsfh9grm07de7r9e3k".to_string(),
                        coin: CoinBalance {
                            id: CoinId::btc(),
                            value: 23_000,
                        },
                    },
                    InputCoin {
                        from: "bc1q8anrrgczju8zn02ww06slsfh9grm07de7r9e3k".to_string(),
                        coin: CoinBalance {
                            id: CoinId::from_str("868703:142").unwrap(),
                            value: 959_000_000,
                        },
                    },
                ],
                output_coins: vec![],
            }],
        };
        println!(
            "Add liquidity sample instruction: {}\n",
            serde_json::to_string(&instruction_set_1).unwrap()
        );
        //
        //
        //
        let instruction_set_2 = IntentionSet {
            initiator_address: "bc1qvwvcttn5dtxleu73uuyh8w759gukjr22l7z503".to_string(),
            intentions: vec![Intention {
                exchange_id: "RICH_SWAP".to_string(),
                action: "withdraw_liquidity".to_string(),
                pool_address: "bc1pu3pv54uxfps00a8ydle67fd3rktz090l07lyg7wadurq4h0lpjhqnet990"
                    .to_string(),
                nonce: 11,
                pool_utxo_spend: vec![
                    "71c9aa9a015e0fcd5cbd6354fbd61c290f9c0a77cecb920df1f0917e7ddc75b7:0"
                        .to_string(),
                ],
                pool_utxo_receive: vec![],
                input_coins: vec![],
                output_coins: vec![
                    OutputCoin {
                        to: "bc1qvwvcttn5dtxleu73uuyh8w759gukjr22l7z503".to_string(),
                        coin: CoinBalance {
                            id: CoinId::btc(),
                            value: 10_124,
                        },
                    },
                    OutputCoin {
                        to: "bc1qvwvcttn5dtxleu73uuyh8w759gukjr22l7z503".to_string(),
                        coin: CoinBalance {
                            id: CoinId::from_str("840106:129").unwrap(),
                            value: 7_072_563,
                        },
                    },
                ],
            }],
        };
        println!(
            "Withdraw liquidity sample instruction: {}\n",
            serde_json::to_string(&instruction_set_2).unwrap()
        );
        //
        //
        //
        let instruction_set_3 = IntentionSet {
            initiator_address: "bc1plvgrpk6mxwyppvqa5j275ujatn8qgs2dcm8m3r2w7sfkn395x6us9l5qdj"
                .to_string(),

            intentions: vec![Intention {
                exchange_id: "RICH_SWAP".to_string(),
                action: "swap".to_string(),
                pool_address: "bc1ptnxf8aal3apeg8r4zysr6k2mhadg833se2dm4nssl7drjlqdh2jqa4tk3p"
                    .to_string(),
                nonce: 5,
                pool_utxo_spend: vec![
                    "17616a9d2258c41bea2175e64ecc2e5fc45ae18be5c9003e058cb0bb85301fd8:0"
                        .to_string(),
                ],
                pool_utxo_receive: vec![
                    "0cec5e1ac7688744dc7af59e8e3cd7be794b0f6dfec9357181759dc4c9c9e541:0"
                        .to_string(),
                ],
                input_coins: vec![InputCoin {
                    from: "bc1plvgrpk6mxwyppvqa5j275ujatn8qgs2dcm8m3r2w7sfkn395x6us9l5qdj"
                        .to_string(),
                    coin: CoinBalance {
                        id: CoinId::from_str("840000:846").unwrap(),
                        value: 10_000_000,
                    },
                }],
                output_coins: vec![OutputCoin {
                    to: "bc1plvgrpk6mxwyppvqa5j275ujatn8qgs2dcm8m3r2w7sfkn395x6us9l5qdj"
                        .to_string(),
                    coin: CoinBalance {
                        id: CoinId::btc(),
                        value: 25_523,
                    },
                }],
            }],
        };
        println!(
            "Runes swap btc sample instruction: {}\n",
            serde_json::to_string(&instruction_set_3).unwrap()
        );
        //
        //
        //
        let instruction_set_4 = IntentionSet {
            initiator_address: "bc1plvgrpk6mxwyppvqa5j275ujatn8qgs2dcm8m3r2w7sfkn395x6us9l5qdj"
                .to_string(),
            intentions: vec![
                Intention {
                    exchange_id: "RICH_SWAP".to_string(),
                    action: "swap".to_string(),
                    pool_address: "bc1ptnxf8aal3apeg8r4zysr6k2mhadg833se2dm4nssl7drjlqdh2jqa4tk3p"
                        .to_string(),
                    nonce: 5,
                    pool_utxo_spend: vec![
                        "17616a9d2258c41bea2175e64ecc2e5fc45ae18be5c9003e058cb0bb85301fd8:0"
                            .to_string(),
                    ],
                    pool_utxo_receive: vec![
                        "0cec5e1ac7688744dc7af59e8e3cd7be794b0f6dfec9357181759dc4c9c9e541:0"
                            .to_string(),
                    ],
                    input_coins: vec![InputCoin {
                        from: "bc1plvgrpk6mxwyppvqa5j275ujatn8qgs2dcm8m3r2w7sfkn395x6us9l5qdj"
                            .to_string(),
                        coin: CoinBalance {
                            id: CoinId::from_str("840000:846").unwrap(),
                            value: 10_000_000,
                        },
                    }],
                    output_coins: vec![OutputCoin {
                        to: "bc1pu3pv54uxfps00a8ydle67fd3rktz090l07lyg7wadurq4h0lpjhqnet990"
                            .to_string(),
                        coin: CoinBalance {
                            id: CoinId::btc(),
                            value: 25_523,
                        },
                    }],
                },
                Intention {
                    exchange_id: "RICH_SWAP".to_string(),
                    action: "swap".to_string(),
                    pool_address: "bc1pu3pv54uxfps00a8ydle67fd3rktz090l07lyg7wadurq4h0lpjhqnet990"
                        .to_string(),
                    nonce: 9,
                    pool_utxo_spend: vec![
                        "9c3590a30d7b5d27f264a295aec6ed15c83618c152c89b28b81a460fcbb66514:1"
                            .to_string(),
                    ],
                    pool_utxo_receive: vec![
                        "0cec5e1ac7688744dc7af59e8e3cd7be794b0f6dfec9357181759dc4c9c9e541:2"
                            .to_string(),
                    ],
                    input_coins: vec![InputCoin {
                        from: "bc1pu3pv54uxfps00a8ydle67fd3rktz090l07lyg7wadurq4h0lpjhqnet990"
                            .to_string(),
                        coin: CoinBalance {
                            id: CoinId::btc(),
                            value: 25_523,
                        },
                    }],
                    output_coins: vec![OutputCoin {
                        to: "bc1plvgrpk6mxwyppvqa5j275ujatn8qgs2dcm8m3r2w7sfkn395x6us9l5qdj"
                            .to_string(),
                        coin: CoinBalance {
                            id: CoinId::from_str("840106:129").unwrap(),
                            value: 672_563,
                        },
                    }],
                },
            ],
        };
        println!(
            "Runes swap runes sample instruction: {}\n",
            serde_json::to_string(&instruction_set_4).unwrap()
        );
    }
}
