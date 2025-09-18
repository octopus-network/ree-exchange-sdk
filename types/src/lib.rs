extern crate alloc;

use alloc::str::FromStr;
use candid::CandidType;
use serde::{Deserialize, Serialize};

mod coin_id;
mod intention;
mod pubkey;
mod txid;

pub mod exchange_interfaces;
pub mod orchestrator_interfaces;

pub use bitcoin;
pub use coin_id::CoinId;
pub use exchange_interfaces::NewBlockInfo;
pub use ic_cdk;
pub use intention::*;
pub use pubkey::Pubkey;
pub use txid::{TxRecord, Txid};

/// The CoinBalance struct represents a balance of a specific coin type.
#[derive(
    CandidType, Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct CoinBalance {
    pub id: CoinId,
    pub value: u128,
}

/// The CoinBalances struct is a collection of CoinBalance objects.
#[derive(CandidType, Eq, PartialEq, Clone, Debug, Deserialize, Serialize, Default)]
pub struct CoinBalances(Vec<CoinBalance>);

/// The Bitcoin UTXO with Runes coin balances.
#[derive(CandidType, Eq, PartialEq, Clone, Debug, Deserialize, Serialize)]
pub struct Utxo {
    pub txid: Txid,
    pub vout: u32,
    pub coins: CoinBalances,
    pub sats: u64,
}

impl Utxo {
    pub fn try_from(
        outpoint: impl AsRef<str>,
        coins: CoinBalances,
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
            coins,
            sats,
        })
    }

    pub fn outpoint(&self) -> String {
        format!("{}:{}", self.txid, self.vout)
    }
}

impl CoinBalances {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn single(coin: CoinBalance) -> Self {
        Self(vec![coin])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    //
    pub fn iter(&self) -> impl Iterator<Item = &CoinBalance> {
        self.0.iter()
    }
    //
    pub fn add_coin(&mut self, coin: &CoinBalance) {
        let mut found = false;
        for existing_coin in &mut self.0 {
            if existing_coin.id == coin.id {
                existing_coin.value += coin.value;
                found = true;
                break;
            }
        }
        if !found {
            self.0.push(coin.clone());
        }
    }
    //
    pub fn subtract_coin(&mut self, coin: &CoinBalance) -> bool {
        for i in 0..self.0.len() {
            if self.0[i].id == coin.id {
                if self.0[i].value >= coin.value {
                    self.0[i].value -= coin.value;
                    if self.0[i].value == 0 {
                        self.0.remove(i);
                    }
                    return true;
                } else {
                    return false; // Not enough value to remove
                }
            }
        }
        false // Coin not found
    }
    //
    pub fn value_of(&self, coin_id: &CoinId) -> u128 {
        for coin in &self.0 {
            if coin.id == *coin_id {
                return coin.value;
            }
        }
        0
    }
    //
    pub fn add_coins(&mut self, coins: &CoinBalances) {
        for coin in &coins.0 {
            self.add_coin(coin);
        }
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use super::*;

    #[test]
    fn test_ree_instruction_json() {
        let instruction_set_1 = IntentionSet {
            initiator_address: "bc1q8anrrgczju8zn02ww06slsfh9grm07de7r9e3k".to_string(),
            tx_fee_in_sats: 360,
            intentions: vec![Intention {
                exchange_id: "RICH_SWAP".to_string(),
                action: "add_liquidity".to_string(),
                action_params: String::new(),
                pool_address: "bc1pxtmh763568jd8pz9m8wekt2yrqyntqv2wk465mgpzlr9f2aq2vqs52l0hq"
                    .to_string(),
                nonce: 1,
                pool_utxo_spent: vec![],
                pool_utxo_received: vec![],
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
            tx_fee_in_sats: 330,
            intentions: vec![Intention {
                exchange_id: "RICH_SWAP".to_string(),
                action: "withdraw_liquidity".to_string(),
                action_params: String::new(),
                pool_address: "bc1pu3pv54uxfps00a8ydle67fd3rktz090l07lyg7wadurq4h0lpjhqnet990"
                    .to_string(),
                nonce: 11,
                pool_utxo_spent: vec![
                    "71c9aa9a015e0fcd5cbd6354fbd61c290f9c0a77cecb920df1f0917e7ddc75b7:0"
                        .to_string(),
                ],
                pool_utxo_received: vec![],
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
            tx_fee_in_sats: 340,
            intentions: vec![Intention {
                exchange_id: "RICH_SWAP".to_string(),
                action: "swap".to_string(),
                action_params: String::new(),
                pool_address: "bc1ptnxf8aal3apeg8r4zysr6k2mhadg833se2dm4nssl7drjlqdh2jqa4tk3p"
                    .to_string(),
                nonce: 5,
                pool_utxo_spent: vec![
                    "17616a9d2258c41bea2175e64ecc2e5fc45ae18be5c9003e058cb0bb85301fd8:0"
                        .to_string(),
                ],
                pool_utxo_received: vec![],
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
            tx_fee_in_sats: 410,
            intentions: vec![
                Intention {
                    exchange_id: "RICH_SWAP".to_string(),
                    action: "swap".to_string(),
                    action_params: String::new(),
                    pool_address: "bc1ptnxf8aal3apeg8r4zysr6k2mhadg833se2dm4nssl7drjlqdh2jqa4tk3p"
                        .to_string(),
                    nonce: 5,
                    pool_utxo_spent: vec![
                        "17616a9d2258c41bea2175e64ecc2e5fc45ae18be5c9003e058cb0bb85301fd8:0"
                            .to_string(),
                    ],
                    pool_utxo_received: vec![],
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
                    action_params: String::new(),
                    pool_address: "bc1pu3pv54uxfps00a8ydle67fd3rktz090l07lyg7wadurq4h0lpjhqnet990"
                        .to_string(),
                    nonce: 9,
                    pool_utxo_spent: vec![
                        "9c3590a30d7b5d27f264a295aec6ed15c83618c152c89b28b81a460fcbb66514:1"
                            .to_string(),
                    ],
                    pool_utxo_received: vec![],
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

    #[test]
    /// Test the CoinBalances struct
    fn test_coin_balances() {
        let mut balances = CoinBalances::new();
        let coin1 = CoinBalance {
            id: CoinId::btc(),
            value: 1000,
        };
        let coin2 = CoinBalance {
            id: CoinId::from_str("840106:129").unwrap(),
            value: 500,
        };

        balances.add_coin(&coin1);
        balances.add_coin(&coin2);

        assert_eq!(balances.value_of(&CoinId::btc()), 1000);
        assert_eq!(
            balances.value_of(&CoinId::from_str("840106:129").unwrap()),
            500
        );

        let coin3 = CoinBalance {
            id: CoinId::btc(),
            value: 200,
        };
        balances.add_coin(&coin3);
        assert_eq!(balances.value_of(&CoinId::btc()), 1200);

        let coin4 = CoinBalance {
            id: CoinId::from_str("840106:129").unwrap(),
            value: 600,
        };
        assert!(!balances.subtract_coin(&coin4));

        let coin4 = CoinBalance {
            id: CoinId::from_str("840106:129").unwrap(),
            value: 500,
        };
        assert!(balances.subtract_coin(&coin4));
        assert_eq!(
            balances.value_of(&CoinId::from_str("840106:129").unwrap()),
            0
        );

        println!("Coin Balances: {:?}", balances);
    }
}
