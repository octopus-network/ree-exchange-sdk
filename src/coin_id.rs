use alloc::str::FromStr;
use candid::{
    types::{Serializer, Type, TypeInner},
    CandidType,
};
use ic_stable_structures::{storable::Bound, Storable};

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub struct CoinId {
    pub block: u64,
    pub tx: u32,
}

impl Ord for CoinId {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.block.cmp(&other.block).then(self.tx.cmp(&other.tx))
    }
}

impl PartialOrd for CoinId {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Storable for CoinId {
    const BOUND: Bound = Bound::Bounded {
        max_size: 12,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> alloc::borrow::Cow<[u8]> {
        let mut bytes = vec![];
        bytes.extend_from_slice(self.block.to_be_bytes().as_ref());
        bytes.extend_from_slice(self.tx.to_be_bytes().as_ref());
        alloc::borrow::Cow::Owned(bytes)
    }

    fn from_bytes(bytes: alloc::borrow::Cow<[u8]>) -> Self {
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

impl core::fmt::Display for CoinId {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}:{}", self.block, self.tx)
    }
}

struct CoinIdVisitor;

impl<'de> serde::de::Visitor<'de> for CoinIdVisitor {
    type Value = CoinId;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
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
