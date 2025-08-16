use crate::Network;
use crate::types::{
    Pubkey, Utxo,
    bitcoin::{
        self, OutPoint, TapSighashType, Witness,
        psbt::Psbt,
        sighash::{Prevouts, SighashCache},
        {key::TapTweak, secp256k1::Secp256k1},
    },
};
use candid::{CandidType, Principal};
use ic_cdk::management_canister::{self, SchnorrAlgorithm, SchnorrKeyId, SchnorrPublicKeyArgs};
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
enum SignWithSchnorrAux {
    #[serde(rename = "bip341")]
    Bip341(SignWithBip341Aux),
}

#[derive(Eq, PartialEq, Debug, CandidType, Serialize)]
struct SignWithBip341Aux {
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

/// sign the provided message using the IC chain-key API.
async fn sign_with_schnorr(
    message: Vec<u8>,
    network: Network,
    derivation_path: Vec<Vec<u8>>,
    merkle_root: Option<Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let key_name = match network {
        Network::Bitcoin => "key_1",
        Network::Testnet4 => "test_key_1",
    };
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
            name: key_name.to_string(),
        },
        aux,
    };
    #[allow(deprecated)]
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

/// sign the provided pre-hashed digest using the IC chain-key API, i.e. the P2TR key path spend
/// reference: <https://learnmeabitcoin.com/technical/upgrades/taproot/#key-path-spend>
pub async fn sign_p2tr_key_spend(
    digest: impl AsRef<[u8; 32]>,
    network: Network,
    derivation_path: Vec<Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let signature =
        self::sign_with_schnorr(digest.as_ref().to_vec(), network, derivation_path, None)
            .await
            .map_err(|e| e.to_string())?;
    Ok(signature)
}

#[deprecated(since = "0.8.2", note = "Use `sign_p2tr_key_spend` instead")]
pub async fn sign_p2tr_prehashed(
    digest: impl AsRef<[u8; 32]>,
    network: Network,
    derivation_path: Vec<Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let signature =
        self::sign_with_schnorr(digest.as_ref().to_vec(), network, derivation_path, None)
            .await
            .map_err(|e| e.to_string())?;
    Ok(signature)
}

/// Tweak the schnoor public key with an empty TapTweak.
pub fn tweak_pubkey_with_empty(untweaked: Pubkey) -> Pubkey {
    let secp = Secp256k1::new();
    let (tweaked, _) = untweaked.to_x_only_public_key().tap_tweak(&secp, None);
    let raw = tweaked.serialize().to_vec();
    Pubkey::from_raw([&[0x00], &raw[..]].concat()).expect("tweaked 33bytes; qed")
}

/// request the IC chain-key API to generate a P2TR address
/// reference: <https://internetcomputer.org/docs/references/t-sigs-how-it-works#key-derivation>
pub async fn request_p2tr_address(
    derivation_path: Vec<Vec<u8>>,
    network: Network,
) -> Result<(Pubkey, Pubkey, bitcoin::Address), String> {
    // validate_schnorr_key_name(&schnorr_key_name)?;
    let key_name = match network {
        Network::Bitcoin => "key_1",
        Network::Testnet4 => "test_key_1",
    };
    let arg = SchnorrPublicKeyArgs {
        canister_id: None,
        derivation_path,
        key_id: SchnorrKeyId {
            algorithm: SchnorrAlgorithm::Bip340secp256k1,
            name: key_name.to_string(),
        },
    };
    let res = management_canister::schnorr_public_key(&arg)
        .await
        .map_err(|err| format!("schnorr_public_key failed {:?}", err))?;
    let mut raw = res.public_key.to_vec();
    raw[0] = 0x00;
    let untweaked_pubkey = Pubkey::from_raw(raw).expect("management api error: invalid pubkey");
    let tweaked_pubkey = tweak_pubkey_with_empty(untweaked_pubkey.clone());
    let key = bitcoin::key::TweakedPublicKey::dangerous_assume_tweaked(
        tweaked_pubkey.to_x_only_public_key(),
    );
    let network: bitcoin::Network = network.into();
    let addr = bitcoin::Address::p2tr_tweaked(key, network);
    Ok((untweaked_pubkey, tweaked_pubkey, addr))
}

fn cmp<'a>(mine: &'a Utxo, outpoint: &OutPoint) -> bool {
    Into::<bitcoin::Txid>::into(mine.txid) == outpoint.txid && mine.vout == outpoint.vout
}

/// Signs the PSBT inputs using IC chain-key that match the provided pool inputs with a Taproot key spend signature.
pub async fn sign_p2tr_in_psbt(
    psbt: &mut Psbt,
    pool_inputs: &[Utxo],
    network: Network,
    derivation_path: Vec<Vec<u8>>,
) -> Result<(), String> {
    let mut cache = SighashCache::new(&psbt.unsigned_tx);
    let mut prevouts = vec![];
    for input in psbt.inputs.iter() {
        let pout = input
            .witness_utxo
            .as_ref()
            .cloned()
            .ok_or("witness_utxo required".to_string())?;
        prevouts.push(pout);
    }
    for (i, input) in psbt.unsigned_tx.input.iter().enumerate() {
        let outpoint = &input.previous_output;
        if let Some(_) = pool_inputs.iter().find(|input| cmp(input, outpoint)) {
            (i < psbt.inputs.len()).then(|| ()).ok_or(format!(
                "Input index {i} exceeds available inputs ({})",
                psbt.inputs.len()
            ))?;
            let input = &mut psbt.inputs[i];
            let sighash = cache
                .taproot_key_spend_signature_hash(
                    i,
                    &Prevouts::All(&prevouts),
                    TapSighashType::Default,
                )
                .expect("couldn't construct taproot sighash");
            let raw_sig = self::sign_p2tr_key_spend(&sighash, network, derivation_path.clone())
                .await
                .map_err(|e| e.to_string())?;
            let inner_sig = bitcoin::secp256k1::schnorr::Signature::from_slice(&raw_sig)
                .expect("assert: chain-key schnorr signature is 64-bytes format");
            let signature = bitcoin::taproot::Signature {
                signature: inner_sig,
                sighash_type: TapSighashType::Default,
            };
            input.final_script_witness = Some(Witness::p2tr_key_spend(&signature));
        }
    }
    Ok(())
}
