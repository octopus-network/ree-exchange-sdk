# REE Exchange Rust SDK

[![docs.rs](https://img.shields.io/docsrs/ree-exchange-sdk)](https://docs.rs/ree-exchange-sdk/latest/ree_exchange_sdk/)

## Quick start

If you are familier with IC canister development, you could easily create an empty rust crate and paste the code into the `lib.rs`.

``` rust
use candid::{CandidType, Deserialize};
use ic_cdk::{query, update};
use ic_stable_structures::{Storable, storable::Bound};
use ree_exchange_sdk::{
    prelude::*,
    {CoinBalance, Txid, Utxo},
};
use serde::Serialize;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct DummyPoolState {
    pub txid: Txid,
    pub nonce: u64,
    pub coin_reserved: Vec<CoinBalance>,
    pub btc_reserved: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}

impl Storable for DummyPoolState {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut bytes = vec![];
        let _ = ciborium::ser::into_writer(self, &mut bytes);
        std::borrow::Cow::Owned(bytes)
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut bytes = vec![];
        let _ = ciborium::ser::into_writer(&self, &mut bytes);
        bytes
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let dire = ciborium::de::from_reader(bytes.as_ref()).expect("failed to decode Pool");
        dire
    }
}

impl StateView for DummyPoolState {
    fn inspect_state(&self) -> StateInfo {
        StateInfo {
            txid: self.txid,
            nonce: self.nonce,
            coin_reserved: self.coin_reserved.clone(),
            btc_reserved: self.btc_reserved,
            utxos: self.utxos.clone(),
            attributes: "{}".to_string(),
        }
    }
}

#[exchange]
pub mod exchange {
    use super::*;

    #[pools]
    pub struct DummyPools;

    impl Pools for DummyPools {
        type State = DummyPoolState;

        const POOL_MEMORY: u8 = 102;

        const BLOCK_MEMORY: u8 = 100;

        const TRANSACTION_MEMORY: u8 = 101;

        fn network() -> Network {
            Network::Testnet4
        }

        // This is optional
        fn finalize_threshold() -> u32 {
            60
        }
    }

    // This is optional
    #[hook]
    impl Hook for DummyPools {
        // This function is called when a new block is received, before any processing.
        fn on_block_received(_args: NewBlockInfo) {}

        // This function is called when a transaction is dropped from the mempool.
        fn on_state_reverted(_address: String, _txid: Txid) {}

        // This function is called when a transaction is confirmed in a block.
        fn on_state_confirmed(_address: String, _txid: Txid, _block: Block) {}

        // This function is called when a transaction reaches the finalize threshold.
        fn on_state_finalized(_address: String, _txid: Txid, _block: Block) {}

        // This function is called after a new block is processed.
        fn on_block_processed(_args: NewBlockInfo) {}
    }

    #[update]
    pub fn new_pool(args: Metadata) {
        let pool = Pool::new(args.clone());
        DummyPools::insert(pool.clone());
    }

    #[query]
    pub fn pre_swap(addr: String) -> Option<StateInfo> {
        DummyPools::get(&addr).and_then(|pool| pool.last_state().map(|s| s.inspect_state()))
    }
    
    #[action(name = "swap")]
    pub fn execute_swap(psbt: &mut Psbt, args: ActionArgs) -> ActionResult<Pools::State> {
        let mut state = DummyPools::get(&addr)
            .and_then(|pool| pool.last_state().cloned())
            .unwrap_or_default();
        // do some checks..
        state.nonce = state.nonce + 1;
        state.txid = args.txid.clone();
        // sign the psbt
        Ok(state)
    }
}
```

