use crate::Pubkey;
use bitcoin::{key::TapTweak, secp256k1::Secp256k1};
use candid::{CandidType, Principal};
use ic_cdk::api::management_canister::schnorr::{
    self, SchnorrAlgorithm, SchnorrKeyId, SchnorrPublicKeyArgument,
};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

type CanisterId = Principal;

#[derive(CandidType, Serialize, Debug)]
struct ManagementCanisterSignatureRequest {
    pub message: Vec<u8>,
    pub aux: Option<SignWithSchnorrAux>,
    pub derivation_path: Vec<Vec<u8>>,
    pub key_id: SchnorrKeyId,
}

#[derive(Eq, PartialEq, Debug, CandidType, Serialize)]
pub enum SignWithSchnorrAux {
    #[serde(rename = "bip341")]
    Bip341(SignWithBip341Aux),
}

#[derive(Eq, PartialEq, Debug, CandidType, Serialize)]
pub struct SignWithBip341Aux {
    pub merkle_root_hash: ByteBuf,
}

#[derive(CandidType, Deserialize, Debug)]
struct ManagementCanisterSignatureReply {
    pub signature: Vec<u8>,
}

const MGMT_CANISTER_ID: &str = "aaaaa-aa";

fn mgmt_canister_id() -> CanisterId {
    CanisterId::from_text(MGMT_CANISTER_ID).unwrap()
}

/// Validates that the schnorr key name is either "test_key_1" or "key_1"
pub fn validate_schnorr_key_name(key_name: &str) -> Result<(), String> {
    match key_name {
        "test_key_1" | "key_1" => Ok(()),
        _ => Err(format!(
            "Invalid schnorr key name '{}'. Must be either 'test_key_1' or 'key_1'",
            key_name
        )),
    }
}

pub async fn schnorr_sign(
    message: Vec<u8>,
    schnorr_key_name: String,
    derivation_path: Vec<Vec<u8>>,
    merkle_root: Option<Vec<u8>>,
) -> Result<Vec<u8>, String> {
    validate_schnorr_key_name(&schnorr_key_name)?;

    let merkle_root_hash = merkle_root
        .map(|bytes| {
            if bytes.len() == 32 || bytes.is_empty() {
                Ok(ByteBuf::from(bytes))
            } else {
                Err(format!(
                    "merkle tree root bytes must be 0 or 32 bytes long but got {}",
                    bytes.len()
                ))
            }
        })
        .transpose()?
        .unwrap_or_default();
    let aux = Some(SignWithSchnorrAux::Bip341(SignWithBip341Aux {
        merkle_root_hash,
    }));
    let request = ManagementCanisterSignatureRequest {
        message,
        derivation_path,
        key_id: SchnorrKeyId {
            algorithm: SchnorrAlgorithm::Bip340secp256k1,
            name: schnorr_key_name,
        },
        aux,
    };
    let (reply,): (ManagementCanisterSignatureReply,) = ic_cdk::api::call::call_with_payment(
        mgmt_canister_id(),
        "sign_with_schnorr",
        (request,),
        26_153_846_153,
    )
    .await
    .map_err(|e| format!("sign_with_schnorr failed {e:?}"))?;
    Ok(reply.signature)
}

pub async fn sign_prehash_with_schnorr(
    digest: impl AsRef<[u8; 32]>,
    schnorr_key_name: String,
    derivation_path: Vec<Vec<u8>>,
) -> Result<Vec<u8>, String> {
    validate_schnorr_key_name(&schnorr_key_name)?;

    let signature = crate::schnorr::schnorr_sign(
        digest.as_ref().to_vec(),
        schnorr_key_name,
        derivation_path,
        None,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(signature)
}

pub fn tweak_pubkey_with_empty(untweaked: Pubkey) -> Pubkey {
    let secp = Secp256k1::new();
    let (tweaked, _) = untweaked.to_x_only_public_key().tap_tweak(&secp, None);
    let raw = tweaked.serialize().to_vec();
    Pubkey::from_raw([&[0x00], &raw[..]].concat()).expect("tweaked 33bytes; qed")
}

// https://internetcomputer.org/docs/references/t-sigs-how-it-works#key-derivation
pub async fn request_ree_pool_address(
    schnorr_key_name: &str,
    derivation_path: Vec<Vec<u8>>,
    network: bitcoin::Network,
) -> Result<(Pubkey, Pubkey, bitcoin::Address), String> {
    validate_schnorr_key_name(&schnorr_key_name)?;

    let arg = SchnorrPublicKeyArgument {
        canister_id: None,
        derivation_path,
        key_id: SchnorrKeyId {
            algorithm: SchnorrAlgorithm::Bip340secp256k1,
            name: schnorr_key_name.to_string(),
        },
    };
    let res = schnorr::schnorr_public_key(arg)
        .await
        .map_err(|(code, err)| format!("schnorr_public_key failed {code:?} {err:?}"))?;
    let mut raw = res.0.public_key.to_vec();
    raw[0] = 0x00;
    let untweaked_pubkey = Pubkey::from_raw(raw).expect("management api error: invalid pubkey");

    let tweaked_pubkey = tweak_pubkey_with_empty(untweaked_pubkey.clone());
    let key = bitcoin::key::TweakedPublicKey::dangerous_assume_tweaked(
        tweaked_pubkey.to_x_only_public_key(),
    );
    let addr = bitcoin::Address::p2tr_tweaked(key, network);
    Ok((untweaked_pubkey, tweaked_pubkey, addr))
}
