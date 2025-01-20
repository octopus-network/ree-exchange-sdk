pub use bitcoin;

use candid::{
    types::{Serializer, Type, TypeInner},
    CandidType, Deserialize,
};
use ic_stable_structures::{storable::Bound, Storable};
use serde::Serialize;
use std::str::FromStr;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct Pubkey(Vec<u8>);

impl Pubkey {
    pub fn from_raw(key: Vec<u8>) -> Result<Pubkey, String> {
        if key.len() != 33 {
            return Err("invalid pubkey".to_string());
        }
        Ok(Self(key))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn to_x_only_public_key(&self) -> bitcoin::XOnlyPublicKey {
        bitcoin::XOnlyPublicKey::from_slice(&self.0[1..]).expect("The inner is 33 bytes")
    }

    pub fn to_public_key(&self) -> bitcoin::PublicKey {
        bitcoin::PublicKey::from_slice(&self.0).expect("The inner is 33 bytes")
    }
}

impl CandidType for Pubkey {
    fn _ty() -> Type {
        TypeInner::Text.into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_text(&self.to_string())
    }
}

impl Storable for Pubkey {
    const BOUND: Bound = Bound::Bounded {
        max_size: 33,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Borrowed(&self.0)
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Self::from_raw(bytes.to_vec()).expect("Couldn't deserialize pubkey from stable memory")
    }
}

impl std::fmt::Display for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

impl FromStr for Pubkey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim_start_matches("0x");
        hex::decode(s)
            .map_err(|_| "invalid pubkey".to_string())
            .and_then(|key| Self::from_raw(key))
    }
}

struct PubkeyVisitor;

impl<'de> serde::de::Visitor<'de> for PubkeyVisitor {
    type Value = Pubkey;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a 33 or 32-bytes pubkey")
    }

    fn visit_str<E>(self, value: &str) -> Result<Pubkey, E>
    where
        E: serde::de::Error,
    {
        Pubkey::from_str(value)
            .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(value), &self))
    }
}

impl<'de> serde::Deserialize<'de> for Pubkey {
    fn deserialize<D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_any(PubkeyVisitor)
    }
}

impl serde::Serialize for Pubkey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct Txid([u8; 32]);

impl CandidType for Txid {
    fn _ty() -> Type {
        TypeInner::Text.into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: Serializer,
    {
        let rev = self.0.iter().rev().copied().collect::<Vec<_>>();
        serializer.serialize_text(&hex::encode(&rev))
    }
}

impl FromStr for Txid {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = bitcoin::Txid::from_str(s).map_err(|_| "Invalid txid".to_string())?;
        Ok(Self(*AsRef::<[u8; 32]>::as_ref(&bytes)))
    }
}

impl Into<bitcoin::Txid> for Txid {
    fn into(self) -> bitcoin::Txid {
        use bitcoin::hashes::Hash;
        bitcoin::Txid::from_byte_array(self.0)
    }
}

impl From<bitcoin::Txid> for Txid {
    fn from(txid: bitcoin::Txid) -> Self {
        Self(*AsRef::<[u8; 32]>::as_ref(&txid))
    }
}

impl AsRef<[u8; 32]> for Txid {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl AsRef<[u8]> for Txid {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Display for Txid {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let rev = self.0.iter().rev().copied().collect::<Vec<_>>();
        write!(f, "{}", hex::encode(&rev))
    }
}

struct TxidVisitor;

impl<'de> serde::de::Visitor<'de> for TxidVisitor {
    type Value = Txid;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a Bitcoin Txid")
    }

    fn visit_str<E>(self, value: &str) -> Result<Txid, E>
    where
        E: serde::de::Error,
    {
        Txid::from_str(value)
            .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(value), &self))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Txid, E>
    where
        E: serde::de::Error,
    {
        Ok(Txid(v.try_into().map_err(|_| {
            E::invalid_value(serde::de::Unexpected::Bytes(v), &"a Bitcoin Txid")
        })?))
    }
}

impl<'de> serde::Deserialize<'de> for Txid {
    fn deserialize<D>(deserializer: D) -> Result<Txid, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_any(TxidVisitor)
    }
}

impl serde::Serialize for Txid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub struct CoinId {
    pub block: u64,
    pub tx: u32,
}

impl Ord for CoinId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.block.cmp(&other.block).then(self.tx.cmp(&other.tx))
    }
}

impl PartialOrd for CoinId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Storable for CoinId {
    const BOUND: Bound = Bound::Bounded {
        max_size: 12,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut bytes = vec![];
        bytes.extend_from_slice(self.block.to_be_bytes().as_ref());
        bytes.extend_from_slice(self.tx.to_be_bytes().as_ref());
        std::borrow::Cow::Owned(bytes)
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let block: [u8; 8] = bytes.as_ref()[0..8]
            .try_into()
            .expect("failed to decode CoinId");
        let tx: [u8; 4] = bytes.as_ref()[8..12]
            .try_into()
            .expect("failed to decode CoinId");
        Self {
            block: u64::from_be_bytes(block),
            tx: u32::from_be_bytes(tx),
        }
    }
}

impl CoinId {
    pub fn rune(block: u64, tx: u32) -> Self {
        Self { block, tx }
    }

    #[inline]
    pub const fn btc() -> Self {
        Self { block: 0, tx: 0 }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(self.block.to_be_bytes().as_ref());
        bytes.extend_from_slice(self.tx.to_be_bytes().as_ref());
        bytes
    }
}

impl FromStr for CoinId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');
        let block = parts
            .next()
            .map(|s| s.parse().ok())
            .flatten()
            .ok_or("Invalid CoinId".to_string())?;
        let tx = parts
            .next()
            .map(|s| s.parse().ok())
            .flatten()
            .ok_or("Invalid CoinId".to_string())?;
        Ok(Self { block, tx })
    }
}

impl serde::Serialize for CoinId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl CandidType for CoinId {
    fn _ty() -> Type {
        TypeInner::Text.into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_text(&self.to_string())
    }
}

impl std::fmt::Display for CoinId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.block, self.tx)
    }
}

struct CoinIdVisitor;

impl<'de> serde::de::Visitor<'de> for CoinIdVisitor {
    type Value = CoinId;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "Id of a coin in btc")
    }

    fn visit_str<E>(self, value: &str) -> Result<CoinId, E>
    where
        E: serde::de::Error,
    {
        CoinId::from_str(value)
            .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(value), &self))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<CoinId, E>
    where
        E: serde::de::Error,
    {
        let block: [u8; 8] = v[0..8].try_into().expect("failed to decode CoinId");
        let tx: [u8; 4] = v[8..12].try_into().expect("failed to decode CoinId");
        Ok(CoinId {
            block: u64::from_be_bytes(block),
            tx: u32::from_be_bytes(tx),
        })
    }
}

impl<'de> serde::Deserialize<'de> for CoinId {
    fn deserialize<D>(deserializer: D) -> Result<CoinId, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_any(CoinIdVisitor)
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, CandidType, Deserialize, Serialize)]
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
    pub pool_key: Option<Pubkey>,
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
