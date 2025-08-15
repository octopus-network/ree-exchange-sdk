use candid::CandidType;
use ic_stable_structures::{Storable, storable::Bound};
use ree_exchange_sdk::prelude::*;
use ree_exchange_sdk::types::Pubkey;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::str::FromStr;

// A minimal pool state used by the exchange.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default, CandidType)]
pub struct DummyPoolState {
    pub txid: Txid,
    pub nonce: u64,
    pub coin_reserved: Vec<CoinBalance>,
    pub btc_reserved: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}

// Implement Storable using bincode for examples (no need for extra deps).
impl Storable for DummyPoolState {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(bincode::serialize(self).expect("serialize state"))
    }

    fn into_bytes(self) -> Vec<u8> {
        bincode::serialize(&self).expect("serialize state")
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(bytes.as_ref()).expect("deserialize state")
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
            attributes: self.attributes.clone(),
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

        // Optional: customize finalize threshold if needed
        fn finalize_threshold() -> u32 {
            60
        }
    }

    // Optional hooks example (no-ops for the example binary)
    #[hook]
    impl Hook for DummyPools {}

    // `swap` action to update the pool
    #[action(name = "swap")]
    pub async fn execute_swap(
        psbt: &mut bitcoin::Psbt,
        args: ActionArgs,
    ) -> ActionResult<DummyPoolState> {
        let pool = DummyPools::get(&args.intention.pool_address)
            .ok_or_else(|| format!("Pool not found: {}", args.intention.pool_address))?;
        let mut state = pool.last_state().cloned().unwrap_or_default();
        // do some checks...
        state.nonce = state.nonce + 1;
        state.txid = args.txid.clone();
        // if all check passes, invoke the chain-key API to sign the PSBT
        ree_exchange_sdk::schnorr::sign_p2tr_in_psbt(
            psbt,
            &state.utxos,
            DummyPools::network(),
            pool.metadata().key_derivation_path.clone(),
        )
        .await
        .map_err(|e| format!("Failed to sign PSBT: {}", e))?;
        Ok(state)
    }
}

#[update]
pub fn new_pool(name: String) {
    let metadata = Metadata::<DummyPools>::generate_new(name.clone(), name)
        .await
        .expect("Failed to call chain-key API");
    let pool = Pool::new(metadata);
    DummyPools::insert(pool);
}

#[query]
pub fn pre_swap(addr: String) -> Option<StateInfo> {
    DummyPools::get(&addr).and_then(|pool| pool.last_state().map(|s| s.inspect_state()))
}

ic_cdk::export_candid!();
