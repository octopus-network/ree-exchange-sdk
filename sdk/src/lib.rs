//! The REE Exchange SDK provides a set of types and interfaces for building REE exchanges.
//!
//! # Example
//! ```rust
//! use self::exchange::*;
//! use candid::CandidType;
//! use ic_cdk::{query, update};
//! use ree_exchange_sdk::{prelude::*, types::*, error::*};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
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
//!
//!     fn set_nonce(&mut nonce) {
//!         self.nonce = nonce;
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
//!         type PoolState = DummyPoolState;
//!
//!         type BlockState = u32;
//!
//!         const POOL_STATE_MEMORY: u8 = 1;
//!
//!         const BLOCK_STATE_MEMORY: u8 = 2;
//!
//!         fn network() -> Network {
//!             Network::Testnet4
//!         }
//!     }
//!
//!     // This is optional
//!     #[hook]
//!     impl Hook for DummyPools {}
//!
//!     // `swap` is the action function that will be called by the REE Orchestrator
//!     // All actions should return an `ActionResult<S>` where `S` is the pool state of `Pools`.
//!     // The SDK will automatically commit this state to the IC stable memory.
//!     #[action(name = "swap")]
//!     pub async fn execute_swap(
//!         psbt: &bitcoin::Psbt,
//!         args: ActionArgs,
//!     ) -> ActionResult<DummyPoolState> {
//!         let pool = DummyPools::get(&args.intention.pool_address)
//!             .ok_or(Error::PoolNotFound)?;
//!         let mut state = pool.last_state().cloned().unwrap_or_default();
//!         // do some checks...
//!         state.nonce = state.nonce + 1;
//!         state.txid = args.txid.clone();
//!         Ok(state)
//!     }
//! }
//!
//! #[update]
//! pub async fn new_pool(name: String) {
//!     let metadata = Metadata::new::<DummyPools>(name)
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
pub mod schnorr;
#[doc(hidden)]
pub mod states;
pub mod store;
pub mod prelude {
    pub use crate::*;
    pub use ree_exchange_sdk_macro::*;
}

use crate::types::{
    CoinBalance, Intention, IntentionSet, Pubkey, TxRecord, Txid, Utxo, exchange_interfaces::*,
};
use candid::{CandidType, Principal};
use ic_stable_structures::{
    BTreeMap, DefaultMemoryImpl, Storable, memory_manager::VirtualMemory, storable::Bound,
};
use serde::{Deserialize, Serialize};

/// essential types of REE
pub use ree_types as types;

pub mod error {
    pub const POOL_NOT_FOUND: u16 = 101;
    pub const NONCE_EXPIRED: u16 = 102;
    pub const UNKNOWN_ACTION: u16 = 103;
    pub const ILLEGAL_PSBT: u16 = 104;
    pub const POOL_BEING_EXECUTED: u16 = 105;
    pub const TXID_NOT_FOUND: u16 = 106;
    pub const NONCE_NOT_FOUND: u16 = 107;
    pub const MISSING_CALLER: u16 = 108;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Error {
        PoolNotFound,
        NonceExpired,
        UnknownAction,
        IllegalPsbt,
        PoolBeingExecuted,
        TxidNotFound,
        NonceNotFound,
        MissingCallerPrincipal,
        Custom(u16, String),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                Error::PoolNotFound => write!(f, "{}:Pool not found", POOL_NOT_FOUND),
                Error::NonceExpired => write!(f, "{}:Nonce expired", NONCE_EXPIRED),
                Error::UnknownAction => write!(f, "{}:Unknown action", UNKNOWN_ACTION),
                Error::IllegalPsbt => write!(f, "{}:Illegal PSBT", ILLEGAL_PSBT),
                Error::TxidNotFound => write!(f, "{}:Txid not found", TXID_NOT_FOUND),
                Error::NonceNotFound => write!(f, "{}:Nonce not found", NONCE_NOT_FOUND),
                Error::PoolBeingExecuted => {
                    write!(f, "{}:Pool is being executed", POOL_BEING_EXECUTED)
                }
                Error::MissingCallerPrincipal => {
                    write!(f, "{}:Missing caller principal", MISSING_CALLER)
                }
                Error::Custom(code, msg) => write!(f, "{}:{}", code % 100 + 200, msg),
            }
        }
    }
}

#[doc(hidden)]
pub type BlockStateStorage<S> = BTreeMap<u32, GlobalStateWrapper<S>, Memory>;
#[doc(hidden)]
pub type Memory = VirtualMemory<DefaultMemoryImpl>;
#[doc(hidden)]
pub type BlockStorage = BTreeMap<u32, Block, Memory>;
#[doc(hidden)]
pub type UnconfirmedTxStorage = BTreeMap<Txid, TxRecord, Memory>;
#[doc(hidden)]
pub type PoolStorage<S> = BTreeMap<String, Pool<S>, Memory>;

/// The network enum defines the networks supported by the exchange.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Copy)]
pub enum Network {
    Bitcoin,
    Testnet4,
    Devnet,
}

impl Into<crate::types::bitcoin::Network> for Network {
    fn into(self) -> crate::types::bitcoin::Network {
        match self {
            Network::Bitcoin => crate::types::bitcoin::Network::Bitcoin,
            Network::Testnet4 => crate::types::bitcoin::Network::Testnet4,
            Network::Devnet => crate::types::bitcoin::Network::Testnet4,
        }
    }
}

#[doc(hidden)]
pub fn ensure_access<P: Pools>() -> Result<(), String> {
    match P::network() {
        Network::Bitcoin => crate::types::orchestrator_interfaces::ensure_orchestrator(),
        Network::Testnet4 => crate::types::orchestrator_interfaces::ensure_testnet4_orchestrator(),
        Network::Devnet => Ok(()),
    }
}

/// The parameters for the hook `on_block_confirmed` and `on_block_finalized`
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Block {
    /// The height of the block just received
    pub block_height: u32,
    /// The block hash
    pub block_hash: String,
    /// The block timestamp in seconds since the Unix epoch.
    pub block_timestamp: u64,
    /// transactions confirmed in this block
    pub txs: Vec<TxRecord>,
}

impl Storable for Block {
    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        let bytes = bincode::serialize(self).unwrap();
        std::borrow::Cow::Owned(bytes)
    }

    fn into_bytes(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    fn from_bytes(bytes: std::borrow::Cow<'_, [u8]>) -> Self {
        bincode::deserialize(bytes.as_ref()).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
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
    /// Creates a new metadata instance with the given name. It will automatically generate the key and address.
    pub async fn new<P: Pools>(name: String) -> Result<Self, String> {
        let key_derivation_path: Vec<Vec<u8>> = vec![name.clone().into_bytes()];
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
    pub is_reapply: bool,
    pub invoke_caller_principal: Principal,
}

impl TryFrom<ExecuteTxArgs> for ActionArgs {
    type Error = error::Error;

    fn try_from(args: ExecuteTxArgs) -> Result<Self, Self::Error> {
        let ExecuteTxArgs {
            psbt_hex: _,
            txid,
            intention_set,
            intention_index,
            zero_confirmed_tx_queue_length,
            is_reapply,
            invoke_caller_principal,
        } = args;
        let IntentionSet {
            mut intentions,
            initiator_address,
            tx_fee_in_sats: _,
        } = intention_set;
        let intention = intentions.swap_remove(intention_index as usize);
        Ok(Self {
            txid,
            initiator_address,
            intention,
            other_intentions: intentions,
            unconfirmed_tx_count: zero_confirmed_tx_queue_length as usize,
            is_reapply: is_reapply.unwrap_or(false),
            invoke_caller_principal: invoke_caller_principal
                .ok_or(error::Error::MissingCallerPrincipal)?,
        })
    }
}

/// The result type for actions in the exchange, which can either be successful with a state or an error message.
pub type ActionResult<S> = Result<S, error::Error>;

/// User must implement the `StateView` trait for customized state to provide this information.
pub trait StateView {
    fn inspect_state(&self) -> StateInfo;

    fn set_nonce(&mut self, nonce: u64);
}

/// The concrete type stored in the IC stable memory.
/// The SDK will automatically manage the pool state `S`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pool<S> {
    metadata: Metadata,
    states: Vec<S>,
}

impl<S> Storable for Pool<S>
where
    S: Serialize + for<'de> Deserialize<'de>,
{
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        let bytes = bincode::serialize(self).unwrap();
        std::borrow::Cow::Owned(bytes)
    }

    fn into_bytes(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    fn from_bytes(bytes: std::borrow::Cow<'_, [u8]>) -> Self {
        bincode::deserialize(bytes.as_ref()).unwrap()
    }
}

impl<S> Pool<S>
where
    S: StateView,
{
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

    /// Return the state matches the given txid.
    pub fn get(&self, txid: Txid) -> Option<&S> {
        self.states
            .iter()
            .find(|state| state.inspect_state().txid == txid)
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

    fn truncate(&mut self, nonce: u64) -> Result<(), String>;

    fn rollback(&mut self, txid: Txid) -> Result<Vec<S>, String>;

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

    fn truncate(&mut self, nonce: u64) -> Result<(), String> {
        while let Some(state) = self.states.last() {
            if state.inspect_state().nonce >= nonce {
                self.states.pop();
            } else {
                break;
            }
        }
        Ok(())
    }

    fn rollback(&mut self, txid: Txid) -> Result<Vec<S>, String> {
        let idx = self
            .states
            .iter()
            .position(|state| state.inspect_state().txid == txid)
            .ok_or("txid not found".to_string())?;

        let mut rollbacked_states = vec![];
        while self.states.len() > idx {
            rollbacked_states.push(self.states.pop().unwrap());
        }

        Ok(rollbacked_states)
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
    type PoolState: StateView + Serialize + for<'de> Deserialize<'de>;

    /// The concret type of the block state.
    type BlockState: Serialize + for<'de> Deserialize<'de>;

    /// The memory ID for the block state storage.
    const BLOCK_STATE_MEMORY: u8;

    /// The memory ID for the pool state storage.
    const POOL_STATE_MEMORY: u8;

    /// useful for ensuring that the exchange is running on the correct network.
    fn network() -> Network;

    /// Returns the state finalize threshold, useful for determining when a transaction is considered finalized.
    /// For `Testnet4`, it should be great than 60 while in `Bitcoin` it should be ~ 3-6.
    fn finalize_threshold() -> u32 {
        60
    }
}

/// A hook that can be implemented to respond to block event in the exchange lifecycle.
/// It must be implemented over the `BlockState` type and marked as `#[ree_exchange_sdk::hook]`.
pub trait Hook: Pools {
    /// This function is called when a transaction is rejected and never confirmed.
    fn on_tx_rollbacked(
        _address: String,
        _txid: Txid,
        _reason: String,
        _rollbacked_states: Vec<Self::PoolState>,
    ) {
    }

    /// This function is called when a transaction is placed in a new block, before the `on_block_confirmed`.
    fn on_tx_confirmed(_address: String, _txid: Txid, _block: Block) {}

    /// This function is called when a block is received.
    fn on_block_confirmed(_block: Block) {}

    /// This function is called when a block is received but before any other hooks.
    fn pre_block_confirmed(_height: u32) {}
}

/// A trait for accessing the pool storage.
/// The user-defined `Pools` type will automatically implement this trait.
pub trait PoolStorageAccess<P: Pools> {
    fn block_state() -> Option<P::BlockState>;

    fn commit(height: u32, block_state: P::BlockState) -> Result<(), String>;

    fn get(address: &String) -> Option<Pool<P::PoolState>>;

    fn insert(pool: Pool<P::PoolState>);

    fn remove(address: &String) -> Option<Pool<P::PoolState>>;

    fn iter() -> iter::PoolIterator<P>;
}

#[doc(hidden)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GlobalStateWrapper<S> {
    pub inner: S,
}

#[doc(hidden)]
impl<S> GlobalStateWrapper<S> {
    pub fn new(s: S) -> Self {
        Self { inner: s }
    }
}

#[doc(hidden)]
impl<S> Storable for GlobalStateWrapper<S>
where
    S: Serialize + for<'de> Deserialize<'de>,
{
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        let bytes = bincode::serialize(self).unwrap();
        std::borrow::Cow::Owned(bytes)
    }

    fn into_bytes(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    fn from_bytes(bytes: std::borrow::Cow<'_, [u8]>) -> Self {
        bincode::deserialize(bytes.as_ref()).unwrap()
    }
}

/// The Upgrade trait is used to handle state migrations when the state type of a Pools implementation changes.
/// Assume `MyPools` originally has a pool state type `MyPoolState` and block state type `MyBlockState`.
///
/// ```rust
/// #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
/// pub struct MyPoolState {
///     pub txid: Txid,
///     pub nonce: u64,
///     pub coin_reserved: Vec<CoinBalance>,
///     pub btc_reserved: u64,
///     pub utxos: Vec<Utxo>,
///     pub attributes: String,
/// }
///
/// #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
/// pub struct MyBlockState {
///     pub block_number: u32,
/// }
///
/// impl Pools for MyPools {
///     type PoolState = MyPoolState;
///
///     type BlockState = MyBlockState;
///
///     const POOL_STATE_MEMORY: u8 = 1;
///
///     const BLOCK_STATE_MEMORY: u8 = 2;
/// }
/// ```
/// Now we would like to update the `MyPoolState` type.
///
/// The best practice is to rename the `MyPoolState` to `OldPoolState` and define a new state type `MyPoolState`
///
/// ```rust
/// #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
/// pub struct OldPoolState {
///     pub txid: Txid,
///     pub nonce: u64,
///     pub coin_reserved: Vec<CoinBalance>,
///     pub btc_reserved: u64,
///     pub utxos: Vec<Utxo>,
///     pub attributes: String,
/// }
///
/// #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
/// pub struct MyPoolState {
///     pub txid: Txid,
///     pub nonce: u64,
///     pub coin_reserved: Vec<CoinBalance>,
///     pub btc_reserved: u64,
///     pub utxos: Vec<Utxo>,
///     pub attributes: String,
///     pub new_field: u32,
/// }
///
/// impl Into<MyState> for OldState {
///     fn into(self) -> MyState {
///         // ...
///     }
/// }
///
/// #[upgrade]
/// impl Upgrade<MyPools> for MyPools {
///     type PoolState = OldState;
///
///     type BlockState = u32;
///
///     // there is where we store the pool data before upgrade
///     const POOL_STATE_MEMORY: u8 = 1;
///
///     const BLOCK_STATE_MEMORY: u8 = 2;
/// }
///
/// impl Pools for MyPools {
///     type PoolState = MyPoolState;
///
///     type BlockState = u32;
///
///     // this is where we store the pool data after upgrade
///     const POOL_STATE_MEMORY: u8 = 3;
///
///     const BLOCK_STATE_MEMORY: u8 = 4;
/// }
///
/// ```
/// Now you can call `MyPools::upgrade()` in the `post_upgrade` hook.
pub trait Upgrade<P: Pools> {
    /// The previous pool state type before the upgrade.
    type PoolState: Into<P::PoolState> + for<'de> Deserialize<'de> + Clone;

    /// The previous block state type before the upgrade.
    type BlockState: Into<P::BlockState> + for<'de> Deserialize<'de> + Clone;

    /// The memory ID for the pool state storage in the previous version.
    const POOL_STATE_MEMORY: u8;

    /// The memory ID for the block state storage in the previous version.
    const BLOCK_STATE_MEMORY: u8;
}

#[doc(hidden)]
pub fn iterator<P>(memory: Memory) -> iter::PoolIterator<P>
where
    P: Pools,
{
    let inner = PoolStorage::<P::PoolState>::init(memory);
    let keys = inner.keys().collect::<Vec<_>>();
    iter::PoolIterator {
        inner,
        cursor: 0,
        keys,
    }
}

#[doc(hidden)]
pub mod iter {
    pub struct PoolIterator<P: super::Pools> {
        pub(crate) inner: super::PoolStorage<P::PoolState>,
        pub(crate) cursor: usize,
        pub(crate) keys: Vec<String>,
    }

    impl<P> std::iter::Iterator for PoolIterator<P>
    where
        P: super::Pools,
    {
        type Item = (String, super::Pool<P::PoolState>);

        fn next(&mut self) -> Option<Self::Item> {
            if self.cursor < self.keys.len() {
                let key = self.keys[self.cursor].clone();
                self.cursor += 1;
                self.inner.get(&key).map(|v| (key.clone(), v))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::str::FromStr;

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

    impl StateView for DummyPoolState {
        fn inspect_state(&self) -> StateInfo {
            StateInfo {
                txid: self.txid.clone(),
                nonce: self.nonce,
                coin_reserved: self.coin_reserved.clone(),
                btc_reserved: self.btc_reserved,
                utxos: self.utxos.clone(),
                attributes: self.attributes.clone(),
            }
        }

        fn set_nonce(&mut self, nonce: u64) {
            self.nonce = nonce;
        }
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

    #[test]
    fn test_pool_rollback() {
        let mut pool = Pool::<DummyPoolState> {
            metadata: Metadata {
                key: Pubkey::from_raw(vec![2u8; 33]).unwrap(),
                key_derivation_path: vec![vec![0; 32]],
                name: "Test Pool".to_string(),
                address: "test-address".to_string(),
            },
            states: vec![],
        };
        let push_random_state_by_txid = |txid: &str, pool: &mut Pool<DummyPoolState>| {
            let txid = Txid::from_str(txid).unwrap();
            let nonce = pool.states.len() as u64;
            let state = DummyPoolState {
                nonce,
                txid,
                coin_reserved: vec![],
                btc_reserved: 0,
                utxos: vec![],
                attributes: "{}".to_string(),
            };
            pool.states.push(state);
        };

        let txs = [
            "51230fe70deae44a92f8f44a600585e3e57b8c8720a0b67c4c422f579d9ace2a",
            "51230fe70deae44a92f8f44a600585e3e57b8c8720a0b67c4c422f579d9ace2b",
            "51230fe70deae44a92f8f44a600585e3e57b8c8720a0b67c4c422f579d9ace2c",
        ];

        let init_pool_state = |pool: &mut Pool<DummyPoolState>| {
            pool.states.clear();
            for txid in txs.iter() {
                push_random_state_by_txid(txid, pool);
            }
        };

        // test rollback first tx
        init_pool_state(&mut pool);
        assert_eq!(pool.states.len(), 3);
        let before_rollback_states = pool.states.clone();
        let rollbacked_states = pool.rollback(Txid::from_str(txs[0]).unwrap()).unwrap();
        assert_eq!(rollbacked_states.len(), 3);
        assert_eq!(pool.states.len(), 0);
        assert_eq!(rollbacked_states[0], before_rollback_states[2]);
        assert_eq!(rollbacked_states[1], before_rollback_states[1]);
        assert_eq!(rollbacked_states[2], before_rollback_states[0]);

        // test rollback mid tx
        init_pool_state(&mut pool);
        assert_eq!(pool.states.len(), 3);
        let before_rollback_states = pool.states.clone();
        let rollbacked_states = pool.rollback(Txid::from_str(txs[1]).unwrap()).unwrap();
        assert_eq!(rollbacked_states.len(), 2);
        assert_eq!(pool.states.len(), 1);
        assert_eq!(pool.states[0], before_rollback_states[0]);
        assert_eq!(rollbacked_states[0], before_rollback_states[2]);
        assert_eq!(rollbacked_states[1], before_rollback_states[1]);

        // test rollback last tx
        init_pool_state(&mut pool);
        assert_eq!(pool.states.len(), 3);
        let before_rollback_states = pool.states.clone();
        let rollbacked_states = pool.rollback(Txid::from_str(txs[2]).unwrap()).unwrap();
        assert_eq!(rollbacked_states.len(), 1);
        assert_eq!(pool.states.len(), 2);
        assert_eq!(pool.states[0], before_rollback_states[0]);
        assert_eq!(pool.states[1], before_rollback_states[1]);
        assert_eq!(rollbacked_states[0], before_rollback_states[2]);
    }
}
