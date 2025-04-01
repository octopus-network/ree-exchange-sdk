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
struct ManagementCanisterSchnorrPublicKeyRequest {
    pub canister_id: Option<CanisterId>,
    pub derivation_path: Vec<Vec<u8>>,
    pub key_id: SchnorrKeyId,
}

#[derive(CandidType, Deserialize, Debug)]
struct ManagementCanisterSchnorrPublicKeyReply {
    pub public_key: Vec<u8>,
    pub chain_code: Vec<u8>,
}

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

pub async fn schnorr_pubkey(
    derive_path: Vec<u8>,
    key_id: impl ToString,
) -> Result<Vec<u8>, String> {
    let request = ManagementCanisterSchnorrPublicKeyRequest {
        canister_id: None,
        derivation_path: vec![derive_path],
        key_id: SchnorrKeyId {
            algorithm: SchnorrAlgorithm::Bip340secp256k1,
            name: key_id.to_string(),
        },
    };
    let (res,): (ManagementCanisterSchnorrPublicKeyReply,) =
        ic_cdk::call(mgmt_canister_id(), "schnorr_public_key", (request,))
            .await
            .map_err(|e| format!("schnorr_public_key failed {}", e.1))?;
    Ok(res.public_key)
}

pub async fn schnorr_sign(
    message: Vec<u8>,
    derive_path: Vec<u8>,
    key_id: impl ToString,
    merkle_root: Option<Vec<u8>>,
) -> Result<Vec<u8>, String> {
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
        derivation_path: vec![derive_path],
        key_id: SchnorrKeyId {
            algorithm: SchnorrAlgorithm::Bip340secp256k1,
            name: key_id.to_string(),
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
    key_name: impl ToString,
    path: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let signature = crate::schnorr::schnorr_sign(digest.as_ref().to_vec(), path, key_name, None)
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

pub async fn request_schnorr_key(key_name: impl ToString, path: Vec<u8>) -> Result<Pubkey, String> {
    let arg = SchnorrPublicKeyArgument {
        canister_id: None,
        derivation_path: vec![path],
        key_id: SchnorrKeyId {
            algorithm: SchnorrAlgorithm::Bip340secp256k1,
            name: key_name.to_string(),
        },
    };
    let res = schnorr::schnorr_public_key(arg)
        .await
        .map_err(|(code, err)| format!("schnorr_public_key failed {code:?} {err:?}"))?;
    let mut raw = res.0.public_key.to_vec();
    raw[0] = 0x00;
    let pubkey = Pubkey::from_raw(raw).expect("management api error: invalid pubkey");
    Ok(pubkey)
}
