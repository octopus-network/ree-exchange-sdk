//! The REE Exchange SDK provides a set of types and interfaces for building REE exchanges.
//!
//! # Example
//! ```rust
//! use self::exchange::*;
//! use candid::CandidType;
//! use ic_cdk::{query, update};
//! use ree_exchange_sdk::{prelude::*, types::*};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
//! pub struct DummyPoolState {
//!     pub txid: Txid,
//!     pub nonce: u64,
//!     pub coin_reserved: Vec<CoinBalance>,
//!     pub btc_reserved: u64,
//!     pub utxos: Vec<Utxo>,
//!     pub attributes: String,
//! }
//!
//! impl StateView for DummyPoolState {
//!     fn inspect_state(&self) -> StateInfo {
//!         StateInfo {
//!             txid: self.txid,
//!             nonce: self.nonce,
//!             coin_reserved: self.coin_reserved.clone(),
//!             btc_reserved: self.btc_reserved,
//!             utxos: self.utxos.clone(),
//!             attributes: "{}".to_string(),
//!         }
//!     }
//! }
//!
//! #[exchange]
//! pub mod exchange {
//!     use super::*;
//!
//!     #[pools]
//!     pub struct DummyPools;
//!
//!     impl Pools for DummyPools {
//!         type State = DummyPoolState;
//!
//!         const POOL_MEMORY: u8 = 102;
//!
//!         const BLOCK_MEMORY: u8 = 100;
//!
//!         const TRANSACTION_MEMORY: u8 = 101;
//!
//!         fn network() -> Network {
//!             Network::Testnet4
//!         }
//!
//!         // This is optional
//!         fn finalize_threshold() -> u32 {
//!             60
//!         }
//!     }
//!
//!     // This is optional
//!     #[hook]
//!     impl Hook for DummyPools {
//!         // This function is called when a new block is received, before any processing.
//!         fn pre_new_block(_args: NewBlockInfo) {}
//!
//!         // This function is called when a transaction is dropped from the mempool.
//!         fn on_tx_rollbacked(_address: String, _txid: Txid, _reason: String) {}
//!
//!         // This function is called when a transaction is confirmed in a block.
//!         fn on_tx_confirmed(_address: String, _txid: Txid, _block: Block) {}
//!
//!         // This function is called when a transaction reaches the finalize threshold.
//!         fn on_tx_finalized(_address: String, _txid: Txid, _block: Block) {}
//!
//!         // This function is called after a new block is processed.
//!         fn post_new_block(_args: NewBlockInfo) {}
//!     }
//!
//!     // `swap` is the action function that will be called by the REE Orchestrator
//!     // All actions should return an `ActionResult<S>` where `S` is the pool state of `Pools`.
//!     // The SDK will automatically commit this state to the IC stable memory.
//!     #[action(name = "swap")]
//!     pub async fn execute_swap(
//!         psbt: &mut bitcoin::Psbt,
//!         args: ActionArgs,
//!     ) -> ActionResult<DummyPoolState> {
//!         let pool = DummyPools::get(&args.intention.pool_address)
//!             .ok_or_else(|| format!("Pool not found: {}", args.intention.pool_address))?;
//!         let mut state = pool.last_state().cloned().unwrap_or_default();
//!         // do some checks...
//!         state.nonce = state.nonce + 1;
//!         state.txid = args.txid.clone();
//!         // if all check passes, invoke the chain-key API to sign the PSBT
//!         ree_exchange_sdk::schnorr::sign_p2tr_in_psbt(
//!             psbt,
//!             &state.utxos,
//!             DummyPools::network(),
//!             pool.metadata().key_derivation_path.clone(),
//!         )
//!         .await
//!         .map_err(|e| format!("Failed to sign PSBT: {}", e))?;
//!         Ok(state)
//!     }
//! }
//!
//! #[update]
//! pub async fn new_pool(name: String) {
//!     let metadata = Metadata::generate_new::<DummyPools>(name.clone(), name)
//!         .await
//!         .expect("Failed to call chain-key API");
//!     let pool = Pool::new(metadata);
//!     DummyPools::insert(pool);
//! }
//!
//! #[query]
//! pub fn pre_swap(addr: String) -> Option<StateInfo> {
//!     DummyPools::get(&addr).and_then(|pool| pool.last_state().map(|s| s.inspect_state()))
//! }
//!
//! ic_cdk::export_candid!();
//!```

#[doc(hidden)]
pub mod reorg;
pub mod schnorr;
pub mod prelude {
    pub use crate::*;
    pub use ree_exchange_sdk_macro::*;
}

use crate::types::{
    CoinBalance, Intention, IntentionSet, Pubkey, TxRecord, Txid, Utxo, exchange_interfaces::*,
};
use candid::CandidType;
use ic_stable_structures::{
    BTreeMap, DefaultMemoryImpl, Storable,
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::Bound,
};
use serde::{Deserialize, Serialize};

/// essential types of REE
pub use ree_types as types;

#[doc(hidden)]
pub type Memory = VirtualMemory<DefaultMemoryImpl>;
#[doc(hidden)]
pub type BlockStorage = BTreeMap<u32, NewBlockInfo, Memory>;
#[doc(hidden)]
pub type TransactionStorage = BTreeMap<(Txid, bool), TxRecord, Memory>;
#[doc(hidden)]
pub type PoolStorage<S> = BTreeMap<String, Pool<S>, Memory>;

/// The network enum defines the networks supported by the exchange.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Copy)]
pub enum Network {
    Bitcoin,
    Testnet4,
}

impl Into<crate::types::bitcoin::Network> for Network {
    fn into(self) -> crate::types::bitcoin::Network {
        match self {
            Network::Bitcoin => crate::types::bitcoin::Network::Bitcoin,
            Network::Testnet4 => crate::types::bitcoin::Network::Testnet4,
        }
    }
}

#[doc(hidden)]
pub fn ensure_access<P: Pools>() -> Result<(), String> {
    match P::network() {
        Network::Bitcoin => crate::types::orchestrator_interfaces::ensure_orchestrator(),
        Network::Testnet4 => crate::types::orchestrator_interfaces::ensure_testnet4_orchestrator(),
    }
}

/// The parameters for the hook `on_state_confirmed` and `on_state_finalized`
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Block {
    pub height: u32,
    pub hash: String,
    pub timestamp: u64,
}

/// The metadata for the pool, which includes the key, name, and address.
/// Typically, the key and address should be generated by the IC chain-key.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Metadata {
    pub key: Pubkey,
    pub key_derivation_path: Vec<Vec<u8>>,
    pub name: String,
    pub address: String,
}

impl Metadata {
    /// Creates a new metadata instance with the given name and key_derivation_path.
    /// The key and address are generated based on the network.
    #[deprecated(
        since = "0.8.1",
        note = "Use `generate_new` instead to create a Metadata instance."
    )]
    pub async fn generate<P: Pools>(
        name: String,
        key_derivation_path: Vec<Vec<u8>>,
    ) -> Result<Self, String> {
        let (key, _, address) =
            crate::schnorr::request_p2tr_address(key_derivation_path.clone(), P::network())
                .await
                .map_err(|e| format!("Failed to generate pool address: {}", e))?;
        Ok(Self {
            key,
            key_derivation_path,
            name,
            address: address.to_string(),
        })
    }

    /// Creates a new metadata instance with the given name and key_derivation_path using IC chain-key API.
    /// NOTE: the `key_derivation_path` doesn't follow BIP-32, it is a simple string path.
    pub async fn generate_new<P: Pools>(
        name: String,
        key_derivation_path: String,
    ) -> Result<Self, String> {
        let key_derivation_path: Vec<Vec<u8>> = vec![key_derivation_path.into_bytes()];
        let (key, _, address) =
            crate::schnorr::request_p2tr_address(key_derivation_path.clone(), P::network())
                .await
                .map_err(|e| format!("Failed to generate pool address: {}", e))?;
        Ok(Self {
            key,
            key_derivation_path,
            name,
            address: address.to_string(),
        })
    }
}

/// The essential information about the pool state.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct StateInfo {
    pub nonce: u64,
    pub txid: Txid,
    pub coin_reserved: Vec<CoinBalance>,
    pub btc_reserved: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}

/// The parameter for the action function, which is used to execute a transaction in the exchange.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ActionArgs {
    pub txid: Txid,
    pub initiator_address: String,
    pub intention: Intention,
    pub other_intentions: Vec<Intention>,
    pub unconfirmed_tx_count: usize,
}

impl From<ExecuteTxArgs> for ActionArgs {
    fn from(args: ExecuteTxArgs) -> Self {
        let ExecuteTxArgs {
            psbt_hex: _,
            txid,
            intention_set,
            intention_index,
            zero_confirmed_tx_queue_length,
        } = args;
        let IntentionSet {
            mut intentions,
            initiator_address,
            tx_fee_in_sats: _,
        } = intention_set;
        let intention = intentions.swap_remove(intention_index as usize);
        Self {
            txid,
            initiator_address,
            intention,
            other_intentions: intentions,
            unconfirmed_tx_count: zero_confirmed_tx_queue_length as usize,
        }
    }
}

/// The result type for actions in the exchange, which can either be successful with a state or an error message.
pub type ActionResult<S> = Result<S, String>;

/// User must implement the `StateView` trait for customized state to provide this information.
pub trait StateView {
    fn inspect_state(&self) -> StateInfo;
}

/// The concrete type stored in the IC stable memory.
/// The SDK will automatically manage the pool state `S`.
#[derive(Debug, Deserialize, Serialize)]
pub struct Pool<S> {
    metadata: Metadata,
    states: Vec<S>,
}

impl<S> Storable for Pool<S>
where
    S: Serialize + for<'de> Deserialize<'de>,
{
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let bytes = bincode::serialize(self).unwrap();
        std::borrow::Cow::Owned(bytes)
    }

    fn into_bytes(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        bincode::deserialize(bytes.as_ref()).unwrap()
    }
}

impl<S> Pool<S> {
    /// Creates a new pool with the given metadata.
    pub fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            states: Vec::new(),
        }
    }

    /// Returns the metadata of the pool.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Returns a reference of the last state of the pool.
    pub fn last_state(&self) -> Option<&S> {
        self.states.last()
    }

    /// Returns the states of the pool.
    pub fn states(&self) -> &Vec<S> {
        &self.states
    }

    /// Returns a mutable reference to the states of the pool.
    pub fn states_mut(&mut self) -> &mut Vec<S> {
        &mut self.states
    }
}

#[doc(hidden)]
pub trait ReePool<S> {
    fn get_pool_info(&self) -> PoolInfo;

    fn get_pool_basic(&self) -> PoolBasic;

    fn rollback(&mut self, txid: Txid) -> Result<(), String>;

    fn finalize(&mut self, txid: Txid) -> Result<(), String>;
}

#[doc(hidden)]
impl<S> ReePool<S> for Pool<S>
where
    S: StateView,
{
    fn get_pool_basic(&self) -> PoolBasic {
        PoolBasic {
            name: self.metadata.name.clone(),
            address: self.metadata.address.clone(),
        }
    }

    fn get_pool_info(&self) -> PoolInfo {
        let metadata: Metadata = self.metadata.clone();
        let Metadata {
            key,
            key_derivation_path,
            name,
            address,
        } = metadata;
        let state = self
            .states
            .last()
            .map(|s| s.inspect_state())
            .unwrap_or_default();
        let StateInfo {
            txid: _,
            nonce,
            coin_reserved,
            btc_reserved,
            utxos,
            attributes,
        } = state;
        PoolInfo {
            key,
            key_derivation_path,
            name,
            address,
            nonce,
            coin_reserved,
            btc_reserved,
            utxos,
            attributes,
        }
    }

    fn rollback(&mut self, txid: Txid) -> Result<(), String> {
        let idx = self
            .states
            .iter()
            .position(|state| state.inspect_state().txid == txid)
            .ok_or("txid not found".to_string())?;
        if idx == 0 {
            self.states.clear();
            return Ok(());
        }
        self.states.truncate(idx);
        Ok(())
    }

    fn finalize(&mut self, txid: Txid) -> Result<(), String> {
        let idx = self
            .states
            .iter()
            .position(|state| state.inspect_state().txid == txid)
            .ok_or("txid not found".to_string())?;
        if idx == 0 {
            return Ok(());
        }
        self.states.rotate_left(idx);
        self.states.truncate(self.states.len() - idx);
        Ok(())
    }
}

/// The Pools trait defines the interface for the exchange pools, must be marked as `#[ree_exchange_sdk::pools]`.
pub trait Pools {
    /// The concrete type of the pool state.
    type State: StateView + Serialize + for<'de> Deserialize<'de>;

    /// The memory ID for the pool storage.
    const POOL_MEMORY: u8;

    /// The memory ID for the block storage.
    const BLOCK_MEMORY: u8;

    /// The memory ID for the transaction storage.
    const TRANSACTION_MEMORY: u8;

    /// useful for ensuring that the exchange is running on the correct network.
    fn network() -> Network;

    /// Returns the state finalize threshold, useful for determining when a transaction is considered finalized.
    fn finalize_threshold() -> u32 {
        60
    }
}

/// A set of hooks that can be implemented to respond to various events in the exchange lifecycle.
/// It must be implemented over the `Pools` type and marked as `#[ree_exchange_sdk::hook]`.
/// NOTE: Any modification to the pool state within `Hook` would cause panic.
pub trait Hook {
    /// This function is called when a new block is received, before any processing.
    fn pre_new_block(_args: NewBlockInfo) {}

    /// This function is called when a transaction is dropped from the mempool.
    fn on_tx_rollbacked(_address: String, _txid: Txid, _reason: String) {}

    /// This function is called when a transaction is confirmed in a block.
    fn on_tx_confirmed(_address: String, _txid: Txid, _block: Block) {}

    /// This function is called when a transaction reaches the finalize threshold.
    fn on_tx_finalized(_address: String, _txid: Txid, _block: Block) {}

    /// This function is called after a new block is processed.
    fn post_new_block(_args: NewBlockInfo) {}
}

/// A trait for accessing the pool storage.
/// The user-defined `Pools` type will automatically implement this trait.
pub trait PoolStorageAccess<P: Pools> {
    fn get(address: &String) -> Option<Pool<P::State>>;

    fn insert(pool: Pool<P::State>);

    fn remove(address: &String) -> Option<Pool<P::State>>;

    fn iter() -> iter::PoolIterator<P>;
}

#[doc(hidden)]
pub fn iterator<P>() -> iter::PoolIterator<P>
where
    P: Pools,
{
    let mm = MemoryManager::init(DefaultMemoryImpl::default());
    let vm = mm.get(MemoryId::new(P::POOL_MEMORY));
    iter::PoolIterator {
        inner: PoolStorage::<P::State>::init(vm),
        cursor: None,
    }
}

#[doc(hidden)]
pub mod iter {
    pub struct PoolIterator<P: super::Pools> {
        pub(crate) inner: super::PoolStorage<P::State>,
        pub(crate) cursor: Option<String>,
    }

    impl<P> std::iter::Iterator for PoolIterator<P>
    where
        P: super::Pools,
    {
        type Item = (String, super::Pool<P::State>);

        fn next(&mut self) -> Option<Self::Item> {
            match self.cursor {
                Some(ref cursor) => match self
                    .inner
                    .iter_from_prev_key(cursor)
                    .next()
                    .map(|e| e.into_pair())
                {
                    Some((k, v)) => {
                        self.cursor = Some(k.clone());
                        Some((k, v))
                    }
                    None => None,
                },
                None => match self.inner.iter().next().map(|e| e.into_pair()) {
                    Some((k, v)) => {
                        self.cursor = Some(k.clone());
                        Some((k, v))
                    }
                    None => None,
                },
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct DummyPoolState {
        nonce: u64,
        txid: Txid,
        coin_reserved: Vec<CoinBalance>,
        btc_reserved: u64,
        utxos: Vec<Utxo>,
        attributes: String,
    }

    #[test]
    pub fn test_candid_and_bincode_serialize() {
        let state = DummyPoolState {
            nonce: 1,
            txid: Txid::default(),
            coin_reserved: vec![],
            btc_reserved: 0,
            utxos: vec![],
            attributes: "{}".to_string(),
        };
        let pool = Pool::<DummyPoolState> {
            metadata: Metadata {
                key: Pubkey::from_raw(vec![2u8; 33]).unwrap(),
                key_derivation_path: vec![vec![0; 32]],
                name: "Test Pool".to_string(),
                address: "test-address".to_string(),
            },
            states: vec![state.clone()],
        };
        let bincode_serialized = pool.to_bytes();
        Pool::<DummyPoolState>::from_bytes(bincode_serialized);
        assert_eq!(pool.metadata.name, "Test Pool");

        let mut candid_ser = candid::ser::IDLBuilder::new();
        candid_ser.arg(&state).unwrap();
        let candid_serialized = candid_ser.serialize_to_vec();
        assert!(candid_serialized.is_ok());
        let candid_serialized = candid_serialized.unwrap();
        let mut candid_de = candid::de::IDLDeserialize::new(&candid_serialized).unwrap();
        let candid_deserialized = candid_de.get_value::<DummyPoolState>();
        assert!(candid_deserialized.is_ok());
    }
}
