use crate::{CoinBalance, IntentionSet, Pubkey, TxRecord, Txid, Utxo};
use alloc::borrow::Cow;
use candid::CandidType;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::Bound,
    BTreeMap, DefaultMemoryImpl, Storable,
};
use serde::{Deserialize, Serialize};

pub type Memory = VirtualMemory<DefaultMemoryImpl>;
pub type BlockStorage = BTreeMap<u32, NewBlockInfo, Memory>;
pub type TransactionStorage = BTreeMap<(Txid, bool), TxRecord, Memory>;
pub type PoolStorage<S> = BTreeMap<String, Pool<S>, Memory>;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PoolBasic {
    pub name: String,
    pub address: String,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PoolInfo {
    pub key: Pubkey,
    pub key_derivation_path: Vec<Vec<u8>>,
    pub name: String,
    pub address: String,
    pub nonce: u64,
    pub coin_reserved: Vec<CoinBalance>,
    pub btc_reserved: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}

/// The response for the `get_pool_list` function.
pub type GetPoolListResponse = Vec<PoolBasic>;

/// The parameters for the `get_pool_info` function.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetPoolInfoArgs {
    pub pool_address: String,
}

/// The response for the `get_pool_info` function.
pub type GetPoolInfoResponse = Option<PoolInfo>;

/// The parameters for the `execute_tx` function.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExecuteTxArgs {
    pub psbt_hex: String,
    pub txid: Txid,
    pub intention_set: IntentionSet,
    pub intention_index: u32,
    pub zero_confirmed_tx_queue_length: u32,
}

impl ExecuteTxArgs {
    pub fn psbt(&self) -> Result<bitcoin::Psbt, String> {
        let raw = hex::decode(&self.psbt_hex).map_err(|_| "invalid psbt".to_string())?;
        bitcoin::Psbt::deserialize(raw.as_slice()).map_err(|_| "invalid psbt".to_string())
    }
}

/// The response for the `execute_tx` function.
pub type ExecuteTxResponse = Result<String, String>;

/// The parameters for the `rollback_tx` function.
///
/// This function will be called by REE Orchestrator when
/// an unconfirmed transaction is rejected by the Mempool.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RollbackTxArgs {
    pub txid: Txid,
}

/// The response for the `rollback_tx` function.
pub type RollbackTxResponse = Result<(), String>;

/// The `confirmed_txids` field contains the txids of all transactions confirmed in the new block,
/// which are associated with the exchange to be called.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NewBlockInfo {
    pub block_height: u32,
    pub block_hash: String,
    /// The block timestamp in seconds since the Unix epoch.
    pub block_timestamp: u64,
    pub confirmed_txids: Vec<Txid>,
}

/// Parameters for the `new_block` function.
///
/// This function is called by the REE Orchestrator when
/// a new block is detected by the Rune Indexer.
pub type NewBlockArgs = NewBlockInfo;

/// The response for the `new_block` function.
pub type NewBlockResponse = Result<(), String>;

impl Storable for NewBlockInfo {
    fn to_bytes(&self) -> Cow<[u8]> {
        let bytes = bincode::serialize(self).unwrap();
        Cow::Owned(bytes)
    }

    fn into_bytes(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(bytes.as_ref()).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Network {
    Bitcoin,
    Testnet4,
}

pub fn ensure_access<P: Pools>() -> Result<(), String> {
    match P::network() {
        Network::Bitcoin => crate::orchestrator_interfaces::ensure_orchestrator(),
        Network::Testnet4 => crate::orchestrator_interfaces::ensure_testnet4_orchestrator(),
    }
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Block {
    pub height: u32,
    pub hash: String,
    pub timestamp: u64,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Metadata {
    pub key: Pubkey,
    pub key_derivation_path: Vec<Vec<u8>>,
    pub name: String,
    pub address: String,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StateInfo {
    pub nonce: u64,
    pub txid: Txid,
    pub coin_reserved: Vec<CoinBalance>,
    pub btc_reserved: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Pool<S> {
    metadata: Metadata,
    states: Vec<S>,
}

impl Storable for Metadata {
    fn to_bytes(&self) -> Cow<[u8]> {
        let bytes = bincode::serialize(self).unwrap();
        Cow::Owned(bytes)
    }

    fn into_bytes(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(bytes.as_ref()).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl<S> Storable for Pool<S>
where
    S: Storable,
{
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let metadata_bytes = self.metadata.to_bytes();
        let metadata_bytes_len = metadata_bytes.as_ref().len();
        let mut bytes = vec![];
        bytes.extend_from_slice(&(metadata_bytes_len as u32).to_le_bytes());
        bytes.extend_from_slice(metadata_bytes.as_ref());
        bytes.extend_from_slice(&(self.states.len() as u32).to_le_bytes());
        for state in self.states.iter() {
            let state_bytes = state.to_bytes();
            let state_bytes_len = state_bytes.as_ref().len();
            bytes.extend_from_slice(&(state_bytes_len as u32).to_le_bytes());
            bytes.extend_from_slice(state_bytes.as_ref());
        }
        std::borrow::Cow::Owned(bytes)
    }

    fn into_bytes(self) -> Vec<u8> {
        let metadata_bytes = self.metadata.to_bytes();
        let metadata_bytes_len = metadata_bytes.as_ref().len();
        let mut bytes = vec![];
        bytes.extend_from_slice(&(metadata_bytes_len as u32).to_le_bytes());
        bytes.extend_from_slice(metadata_bytes.as_ref());
        bytes.extend_from_slice(&(self.states.len() as u32).to_le_bytes());
        for state in self.states.into_iter() {
            let state_bytes = state.to_bytes();
            let state_bytes_len = state_bytes.as_ref().len();
            bytes.extend_from_slice(&(state_bytes_len as u32).to_le_bytes());
            bytes.extend_from_slice(state_bytes.as_ref());
        }
        bytes
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let bytes = bytes.into_owned();
        let metadata_len = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
        let metadata_bytes = &bytes[4..4 + metadata_len];
        let metadata = Metadata::from_bytes(metadata_bytes.into());
        let mut states = Vec::new();
        let states_len = u32::from_le_bytes(
            bytes[4 + metadata_len..8 + metadata_len]
                .try_into()
                .unwrap(),
        ) as usize;
        let mut offset = 8 + metadata_len;
        for _ in 0..states_len {
            let state_len =
                u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            let state_bytes = &bytes[offset..offset + state_len];
            offset += state_len;
            let state = S::from_bytes(state_bytes.into());
            states.push(state);
        }
        Self { metadata, states }
    }
}

pub trait StateView {
    fn inspect_state(&self) -> StateInfo;
}

impl<S> Pool<S>
where
    S: Storable + StateView,
{
    pub fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            states: Vec::new(),
        }
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn get_pool_info(&self) -> PoolInfo {
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
            .unwrap_or(StateInfo {
                txid: Txid::zero(),
                nonce: 0,
                coin_reserved: vec![],
                btc_reserved: 0,
                utxos: vec![],
                attributes: "{}".to_string(),
            });
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

    pub fn get_pool_basic(&self) -> PoolBasic {
        PoolBasic {
            name: self.metadata.name.clone(),
            address: self.metadata.address.clone(),
        }
    }

    pub fn push(&mut self, state: S) {
        self.states.push(state);
    }

    pub fn states(&self) -> &Vec<S> {
        &self.states
    }

    pub fn states_mut(&mut self) -> &mut Vec<S> {
        &mut self.states
    }

    pub fn rollback(&mut self, txid: Txid) -> Result<(), String> {
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

    pub fn finalize(&mut self, txid: Txid) -> Result<(), String> {
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

/// this trait is for developer
pub trait Pools {
    type State: Storable + StateView;

    const POOL_MEMORY: u8;

    const BLOCK_MEMORY: u8;

    const TRANSACTION_MEMORY: u8;

    fn network() -> Network;

    fn finalize_threshold() -> u32 {
        32
    }
}

pub trait Hook {
    /// This function is called when a new block is received, before any processing.
    fn on_block_received(_args: NewBlockInfo) {}

    /// This function is called when a transaction is dropped from the mempool.
    fn on_state_reverted(_address: String, _txid: Txid) {}

    /// This function is called when a transaction is confirmed in a block.
    fn on_state_confirmed(_address: String, _txid: Txid, _block: Block) {}

    /// This function is called when a transaction reaches the finalize threshold.
    fn on_state_finalized(_address: String, _txid: Txid, _block: Block) {}

    /// This function is called after a new block is processed.
    fn on_block_processed(_args: NewBlockInfo) {}
}

pub trait PoolStorageAccess<P: Pools> {
    fn get(address: &String) -> Option<Pool<P::State>>;

    fn insert(pool: Pool<P::State>);

    fn remove(address: &String) -> Option<Pool<P::State>>;

    fn iter() -> iter::PoolIterator<P>;
}

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
