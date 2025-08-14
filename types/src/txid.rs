use alloc::{borrow::Cow, str::FromStr};
use candid::{
    types::{Serializer, Type, TypeInner},
    CandidType,
};
use ic_stable_structures::storable::{Bound, Storable};

/// The Bitcoin Txid compatible with the IC storage.
#[derive(Eq, Ord, PartialOrd, PartialEq, Clone, Copy, Debug)]
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

impl Storable for Txid {
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

    const BOUND: Bound = Bound::Bounded {
        max_size: 72,
        is_fixed_size: true,
    };
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

impl core::fmt::Display for Txid {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let rev = self.0.iter().rev().copied().collect::<Vec<_>>();
        write!(f, "{}", hex::encode(&rev))
    }
}

struct TxidVisitor;

impl<'de> serde::de::Visitor<'de> for TxidVisitor {
    type Value = Txid;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(formatter, "a Bitcoin Txid")
    }

    fn visit_str<E>(self, value: &str) -> Result<Txid, E>
    where
        E: serde::de::Error,
    {
        Txid::from_str(value)
            .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(value), &self))
    }
}

impl<'de> serde::Deserialize<'de> for Txid {
    fn deserialize<D>(deserializer: D) -> Result<Txid, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_str(TxidVisitor)
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

impl Txid {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        Ok(Txid(bytes.try_into().map_err(|e| {
            format!("Invalid bytes for a Bitcoin Txid: {:?}", e)
        })?))
    }

    pub const fn zero() -> Self {
        Txid([0; 32])
    }
}

impl Default for Txid {
    fn default() -> Self {
        Txid::zero()
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Default, CandidType, serde::Serialize, serde::Deserialize)]
pub struct TxRecord {
    pub pools: Vec<String>,
}

impl Storable for TxRecord {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bincode_serde() {
        let txid_hex = "51230fe70deae44a92f8f44a600585e3e57b8c8720a0b67c4c422f579d9ace2a";
        let txid_bytes = hex::decode(txid_hex).unwrap();
        let txid = Txid(txid_bytes.try_into().unwrap());
        let encoded_hex = hex::encode(bincode::serialize(&txid).unwrap());
        assert_eq!(
            txid.0,
            bincode::deserialize::<Txid>(&hex::decode(encoded_hex).unwrap())
                .unwrap()
                .0
        );
    }

    #[test]
    fn test_bytes_serde() {
        let txid_hex = "51230fe70deae44a92f8f44a600585e3e57b8c8720a0b67c4c422f579d9ace2a";
        let txid_bytes = hex::decode(txid_hex).unwrap();
        let txid = Txid::from_bytes(&txid_bytes).unwrap();
        assert_eq!(txid.0, Txid(txid_bytes.try_into().unwrap()).0);
    }
}
