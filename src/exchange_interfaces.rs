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
pub struct InputCoin {
    // The address of the owner of the coins
    pub from: String,
    pub coin: CoinBalance,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OutputCoins {
    // The address of the receiver of the coins
    pub to: String,
    pub coins: Vec<CoinBalance>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReeInstruction {
    pub exchange_id: String,
    pub method: String,
    pub pool_address: String,
    pub nonce: u64,
    pub pool_utxo_spend: Vec<String>,
    pub pool_utxo_receive: Vec<String>,
    pub input_coins: Vec<InputCoin>,
    pub output_coins: Option<OutputCoins>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReeInstructionSet {
    pub initiator_address: String,
    pub instructions: Vec<ReeInstruction>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SignPsbtArgs {
    pub psbt_hex: String,
    pub txid: Txid,
    pub all_instructions: Vec<ReeInstruction>,
    pub instruction_index: u32,
    pub zero_confirmed_tx_count_in_queue: u32,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FinalizeTxArgs {
    pub pool_key: Pubkey,
    pub txid: Txid,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RollbackTxArgs {
    pub pool_key: Pubkey,
    pub txid: Txid,
}

impl ReeInstruction {
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
            .as_ref()
            .map(|output_coins| {
                output_coins
                    .coins
                    .iter()
                    .map(|coin| coin.id.clone())
                    .collect()
            })
            .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_ree_instruction_json() {
        let instruction_set_1 = ReeInstructionSet {
            initiator_address: "bc1q8anrrgczju8zn02ww06slsfh9grm07de7r9e3k".to_string(),
            instructions: vec![ReeInstruction {
                exchange_id: "RICH_SWAP".to_string(),
                method: "add_liquidity".to_string(),
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
                output_coins: None,
            }],
        };
        println!(
            "Add liquidity sample instruction: {}",
            serde_json::to_string(&instruction_set_1).unwrap()
        );
        //
        //
        //
        let instruction_set_2 = ReeInstructionSet {
            initiator_address: "bc1qvwvcttn5dtxleu73uuyh8w759gukjr22l7z503".to_string(),
            instructions: vec![ReeInstruction {
                exchange_id: "RICH_SWAP".to_string(),
                method: "withdraw_liquidity".to_string(),
                pool_address: "bc1pu3pv54uxfps00a8ydle67fd3rktz090l07lyg7wadurq4h0lpjhqnet990"
                    .to_string(),
                nonce: 11,
                pool_utxo_spend: vec![
                    "71c9aa9a015e0fcd5cbd6354fbd61c290f9c0a77cecb920df1f0917e7ddc75b7:0"
                        .to_string(),
                ],
                pool_utxo_receive: vec![],
                input_coins: vec![],
                output_coins: Some(OutputCoins {
                    to: "bc1qvwvcttn5dtxleu73uuyh8w759gukjr22l7z503".to_string(),
                    coins: vec![
                        CoinBalance {
                            id: CoinId::btc(),
                            value: 10_124,
                        },
                        CoinBalance {
                            id: CoinId::from_str("840106:129").unwrap(),
                            value: 7_072_563,
                        },
                    ],
                }),
            }],
        };
        println!(
            "Withdraw liquidity sample instruction: {}",
            serde_json::to_string(&instruction_set_2).unwrap()
        );
        //
        //
        //
        let instruction_set_3 = ReeInstructionSet {
            initiator_address: "bc1plvgrpk6mxwyppvqa5j275ujatn8qgs2dcm8m3r2w7sfkn395x6us9l5qdj"
                .to_string(),

            instructions: vec![ReeInstruction {
                exchange_id: "RICH_SWAP".to_string(),
                method: "swap".to_string(),
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
                output_coins: Some(OutputCoins {
                    to: "bc1plvgrpk6mxwyppvqa5j275ujatn8qgs2dcm8m3r2w7sfkn395x6us9l5qdj"
                        .to_string(),
                    coins: vec![CoinBalance {
                        id: CoinId::btc(),
                        value: 25_523,
                    }],
                }),
            }],
        };
        println!(
            "Swap sample instruction: {}",
            serde_json::to_string(&instruction_set_3).unwrap()
        );
    }
}
