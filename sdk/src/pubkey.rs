use alloc::str::FromStr;
use candid::{
    types::{Serializer, Type, TypeInner},
    CandidType,
};
use ic_stable_structures::{storable::Bound, Storable};

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

    pub fn to_public_key(&self) -> Result<bitcoin::PublicKey, String> {
        match self.0[0] {
            0x02 | 0x03 => {
                Ok(bitcoin::PublicKey::from_slice(&self.0).expect("The inner is 33 bytes"))
            }
            _ => Err("the pubkey is deserialized from a XOnlyPublicKey".to_string()),
        }
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

    fn to_bytes(&self) -> alloc::borrow::Cow<[u8]> {
        alloc::borrow::Cow::Borrowed(&self.0)
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    fn from_bytes(bytes: alloc::borrow::Cow<[u8]>) -> Self {
        Self::from_raw(bytes.to_vec()).expect("Couldn't deserialize pubkey from stable memory")
    }
}

impl core::fmt::Display for Pubkey {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self.0[0] {
            0x02 | 0x03 => {
                let key = bitcoin::PublicKey::from_slice(&self.0).expect("The inner is 33 bytes");
                write!(f, "{}", key)
            }
            _ => {
                let key = bitcoin::XOnlyPublicKey::from_slice(&self.0[1..])
                    .expect("The inner is 33 bytes");
                write!(f, "{}", key)
            }
        }
    }
}

impl FromStr for Pubkey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim_start_matches("0x");
        let raw = hex::decode(s).map_err(|_| "invalid pubkey".to_string())?;
        if raw.len() == 32 {
            let v = [&[0x00], &raw[..]].concat();
            Self::from_raw(v)
        } else {
            Self::from_raw(raw)
        }
    }
}

struct PubkeyVisitor;

impl<'de> serde::de::Visitor<'de> for PubkeyVisitor {
    type Value = Pubkey;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(formatter, "a 33-bytes pubkey")
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
        deserializer.deserialize_str(PubkeyVisitor)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bincode_serde() {
        let pubkey_hex = "007c06bc45a24f098e327b1e27ed5a9b4477b58c3bbfed5a3bb36c6f59bda290b2";
        let pubkey_bytes = hex::decode(pubkey_hex).unwrap();
        let pubkey = Pubkey(pubkey_bytes);
        let encoded_hex = hex::encode(bincode::serialize(&pubkey).unwrap());
        assert_eq!(
            pubkey.0,
            bincode::deserialize::<Pubkey>(&hex::decode(encoded_hex).unwrap())
                .unwrap()
                .0
        );
    }
}
