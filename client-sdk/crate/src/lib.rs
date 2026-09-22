pub mod coin;
pub mod convert;
pub mod derived_escrow;
pub mod groth16;
pub mod merkle;
pub mod params;
pub mod poseidon;
pub mod poseidon_opt;
pub mod poseidon_opt_params;
pub mod stealth;
pub mod utils;

pub use cryptography::{
    ecdh_ephemeral_public_key, ecdh_shared_key, CoordBytes, PointError, ScalarBytes,
};

use wasm_bindgen::prelude::*;

use crate::utils::decimal_to_fr_result;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::{LazyLock, Mutex};

static TREES: LazyLock<Mutex<HashMap<u32, merkle::LeanIMT>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static NEXT_TREE_ID: AtomicU32 = AtomicU32::new(1);

/// Generate a new coin with random nullifier, secret, and owner public-key field elements.
/// `amount_decimal` is an arbitrary-precision decimal field element (never JS `number`).
/// `asset_hi_decimal` / `asset_lo_decimal` are decimal Fr strings for the Stellar asset contract id (two limbs).
/// Returns JSON: { coin: { value, nullifier, secret, commitment, asset_hi, asset_lo }, commitment_hex, precommitement_hex }
#[wasm_bindgen(js_name = "generateCoin")]
pub fn generate_coin(
    amount_decimal: &str,
    asset_hi_decimal: &str,
    asset_lo_decimal: &str,
    application_id_decimal: &str,
) -> Result<JsValue, JsValue> {
    let hi = decimal_to_fr_result(asset_hi_decimal).map_err(|e| JsValue::from_str(&e))?;
    let lo = decimal_to_fr_result(asset_lo_decimal).map_err(|e| JsValue::from_str(&e))?;
    let generated = coin::generate_coin(amount_decimal, hi, lo, application_id_decimal)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&generated).map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Same as `generateCoin`, but commitment uses the given owner public key (64-char hex coords).
#[wasm_bindgen(js_name = "generateCoinWithOwnerPubHex")]
pub fn generate_coin_with_owner_pub_hex(
    owner_x_hex: &str,
    owner_y_hex: &str,
    amount_decimal: &str,
    asset_hi_decimal: &str,
    asset_lo_decimal: &str,
    application_id_decimal: &str,
) -> Result<JsValue, JsValue> {
    let owner_pub = coin::OwnerPub::from_coord_hex(owner_x_hex, owner_y_hex)
        .map_err(|e| JsValue::from_str(&e))?;
    let hi = decimal_to_fr_result(asset_hi_decimal).map_err(|e| JsValue::from_str(&e))?;
    let lo = decimal_to_fr_result(asset_lo_decimal).map_err(|e| JsValue::from_str(&e))?;
    let generated = coin::generate_coin_with_owner_pub(
        owner_pub,
        amount_decimal,
        hi,
        lo,
        application_id_decimal,
    )
    .map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&generated).map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// `secret` in coin = `Poseidon255(1)(scalar)` per `deposit.circom`; scalar is 32-byte hex (64 chars, optional `0x`).
#[wasm_bindgen(js_name = "generateCoinFromDepositEphemeralScalarHex")]
pub fn generate_coin_from_deposit_ephemeral_scalar_hex(
    scalar_hex: &str,
    amount_decimal: &str,
    asset_hi_decimal: &str,
    asset_lo_decimal: &str,
    application_id_decimal: &str,
) -> Result<JsValue, JsValue> {
    let hi = decimal_to_fr_result(asset_hi_decimal).map_err(|e| JsValue::from_str(&e))?;
    let lo = decimal_to_fr_result(asset_lo_decimal).map_err(|e| JsValue::from_str(&e))?;
    let generated = coin::generate_coin_with_deposit_ephemeral_scalar_hex(
        scalar_hex,
        amount_decimal,
        hi,
        lo,
        application_id_decimal,
    )
    .map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&generated).map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// `Poseidon₁(scalar)` secret + recipient owner public key (hex), matching an aligned deposit witness.
#[wasm_bindgen(js_name = "generateCoinForDepositWithOwnerPubHex")]
pub fn generate_coin_for_deposit_with_owner_pub_hex(
    scalar_hex: &str,
    owner_x_hex: &str,
    owner_y_hex: &str,
    amount_decimal: &str,
    asset_hi_decimal: &str,
    asset_lo_decimal: &str,
    application_id_decimal: &str,
) -> Result<JsValue, JsValue> {
    let hi = decimal_to_fr_result(asset_hi_decimal).map_err(|e| JsValue::from_str(&e))?;
    let lo = decimal_to_fr_result(asset_lo_decimal).map_err(|e| JsValue::from_str(&e))?;
    let generated = coin::generate_coin_for_deposit_with_owner_pub_hex(
        scalar_hex,
        owner_x_hex,
        owner_y_hex,
        amount_decimal,
        hi,
        lo,
        application_id_decimal,
    )
    .map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&generated).map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Merkle root, path, and coin field strings for the first withdraw leg (JSON → JSON).
#[wasm_bindgen(js_name = "buildWithdrawMerkleWitness")]
pub fn build_withdraw_merkle_witness_js(
    coin_json: &str,
    state_json: &str,
) -> Result<String, JsValue> {
    let coin: coin::CoinData = serde_json::from_str(coin_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid coin JSON: {}", e)))?;
    let state: coin::StateFile = serde_json::from_str(state_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid state JSON: {}", e)))?;

    let witness =
        coin::build_withdraw_merkle_witness(&coin, &state).map_err(|e| JsValue::from_str(&e))?;

    serde_json::to_string(&witness).map_err(|e| {
        JsValue::from_str(&format!(
            "Failed to serialize withdraw merkle witness: {}",
            e
        ))
    })
}

fn lock_trees() -> Result<
    std::sync::MutexGuard<'static, std::collections::HashMap<u32, merkle::LeanIMT>>,
    JsValue,
> {
    TREES
        .lock()
        .map_err(|_| JsValue::from_str("lean imt registry poisoned"))
}

fn insert_tree(tree: merkle::LeanIMT) -> Result<u32, JsValue> {
    let handle = NEXT_TREE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    lock_trees()?.insert(handle, tree);
    Ok(handle)
}

/// Import a LeanIMT snapshot and return a session handle.
#[wasm_bindgen(js_name = "importLeanImt")]
pub fn import_lean_imt_js(snapshot_json: &str) -> Result<u32, JsValue> {
    let snapshot: merkle::LeanImtSnapshot = serde_json::from_str(snapshot_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid LeanIMT snapshot JSON: {}", e)))?;
    let tree =
        merkle::LeanIMT::from_snapshot(&snapshot).map_err(|e| JsValue::from_str(&e))?;
    insert_tree(tree)
}

/// Import commitments (and optional nodes) as a LeanIMT session handle.
#[wasm_bindgen(js_name = "importLeanImtFromState")]
pub fn import_lean_imt_from_state_js(state_json: &str) -> Result<u32, JsValue> {
    let state: coin::StateFile = serde_json::from_str(state_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid state JSON: {}", e)))?;
    let tree = coin::lean_imt_from_state(&state).map_err(|e| JsValue::from_str(&e))?;
    insert_tree(tree)
}

/// Export the session LeanIMT as a snapshot JSON object.
#[wasm_bindgen(js_name = "exportLeanImt")]
pub fn export_lean_imt_js(handle: u32) -> Result<String, JsValue> {
    let trees = lock_trees()?;
    let tree = trees
        .get(&handle)
        .ok_or_else(|| JsValue::from_str("unknown LeanIMT handle"))?;
    serde_json::to_string(&tree.export_snapshot()).map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Append two decimal-Fr leaves to a session LeanIMT. Returns the new root as a decimal string.
#[wasm_bindgen(js_name = "insertTwoLeanImt")]
pub fn insert_two_lean_imt_js(
    handle: u32,
    leaf_a_decimal: &str,
    leaf_b_decimal: &str,
) -> Result<String, JsValue> {
    let a = decimal_to_fr_result(leaf_a_decimal).map_err(|e| JsValue::from_str(&e))?;
    let b = decimal_to_fr_result(leaf_b_decimal).map_err(|e| JsValue::from_str(&e))?;
    let mut trees = lock_trees()?;
    let tree = trees
        .get_mut(&handle)
        .ok_or_else(|| JsValue::from_str("unknown LeanIMT handle"))?;
    tree.insert_two(a, b)
        .map_err(|e| JsValue::from_str(e))?;
    Ok(crate::utils::fr_to_decimal(&tree.get_root()))
}

/// Generate siblings for `leaf_index` on a session LeanIMT.
#[wasm_bindgen(js_name = "generateLeanImtProof")]
pub fn generate_lean_imt_proof_js(handle: u32, leaf_index: u32) -> Result<String, JsValue> {
    let trees = lock_trees()?;
    let tree = trees
        .get(&handle)
        .ok_or_else(|| JsValue::from_str("unknown LeanIMT handle"))?;
    let (siblings, depth) = tree
        .generate_proof(leaf_index)
        .ok_or_else(|| JsValue::from_str("Failed to generate merkle proof"))?;
    #[derive(serde::Serialize)]
    struct Out {
        root: String,
        depth: u32,
        siblings: Vec<String>,
    }
    serde_json::to_string(&Out {
        root: crate::utils::fr_to_decimal(&tree.get_root()),
        depth,
        siblings: siblings
            .iter()
            .map(crate::utils::fr_to_decimal)
            .collect(),
    })
    .map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Build a withdraw merkle witness against a session LeanIMT.
#[wasm_bindgen(js_name = "buildWithdrawMerkleWitnessFromHandle")]
pub fn build_withdraw_merkle_witness_from_handle_js(
    coin_json: &str,
    handle: u32,
) -> Result<String, JsValue> {
    let coin: coin::CoinData = serde_json::from_str(coin_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid coin JSON: {}", e)))?;
    let trees = lock_trees()?;
    let tree = trees
        .get(&handle)
        .ok_or_else(|| JsValue::from_str("unknown LeanIMT handle"))?;
    let witness = coin::withdraw_merkle_witness_from_tree(&coin, tree)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&witness).map_err(|e| {
        JsValue::from_str(&format!(
            "Failed to serialize withdraw merkle witness: {}",
            e
        ))
    })
}

/// Drop a session LeanIMT handle.
#[wasm_bindgen(js_name = "dropLeanImt")]
pub fn drop_lean_imt_js(handle: u32) -> Result<(), JsValue> {
    lock_trees()?.remove(&handle);
    Ok(())
}

/// Convert snarkjs proof JSON to hex bytes for Soroban contract.
#[wasm_bindgen(js_name = "proofToHex")]
pub fn proof_to_hex(proof_json: &str) -> String {
    convert::proof_to_hex(proof_json)
}

/// Convert snarkjs public signals JSON to hex bytes for Soroban contract.
#[wasm_bindgen(js_name = "publicToHex")]
pub fn public_to_hex(public_json: &str) -> String {
    convert::public_to_hex(public_json)
}

/// Calculate owner-bound nullifier hash from nullifier and spend-scalar decimal strings.
/// Returns hex string (0x...)
#[wasm_bindgen(js_name = "calculateNullifierHash")]
pub fn calculate_nullifier_hash(
    nullifier_decimal: &str,
    priv_key_scalar_decimal: &str,
) -> Result<String, JsValue> {
    coin::calculate_nullifier_hash(nullifier_decimal, priv_key_scalar_decimal)
        .map_err(|e| JsValue::from_str(&e))
}

/// UTF-8 seed → `SHA256` → scalar → BabyJubJub `BASE8 * r` (circom `ECDHEphemeralKey`).
/// `x` and `y` are lowercase hex (no `0x`). Bech32 / stealth string: TypeScript `encodeStealthAddress`.
#[wasm_bindgen(js_name = "ecdhEphemeralPublicKey")]
pub fn ecdh_ephemeral_public_key_js(seed: &str) -> Result<JsValue, JsValue> {
    let (x, y) = stealth::stealth_coords_from_string(seed);
    #[derive(serde::Serialize)]
    struct Out {
        x: String,
        y: String,
    }
    let out = Out {
        x: hex::encode(x),
        y: hex::encode(y),
    };
    serde_wasm_bindgen::to_value(&out).map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// 32-byte scalar as 64 hex chars (optional `0x`) → `ecdh_ephemeral_public_key` (no UTF-8 seed hash).
#[wasm_bindgen(js_name = "ecdhEphemeralPublicKeyFromScalarHex")]
pub fn ecdh_ephemeral_public_key_from_scalar_hex(scalar_hex: &str) -> Result<JsValue, JsValue> {
    let (x, y) =
        stealth::stealth_coords_from_scalar_hex(scalar_hex).map_err(|e| JsValue::from_str(&e))?;
    #[derive(serde::Serialize)]
    struct Out {
        x: String,
        y: String,
    }
    let out = Out {
        x: hex::encode(x),
        y: hex::encode(y),
    };
    serde_wasm_bindgen::to_value(&out).map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// `R = Poseidon255(2)(hi, lo)`; `scalar` is the raw Poseidon hash when it is
/// already in `(0, BabyJub subgroup order)`. Out-of-range hashes fail so the
/// caller can rejection-sample a new nonce.
#[wasm_bindgen(js_name = "derivedEscrowKey")]
pub fn derived_escrow_key_js(
    nonce_decimal: &str,
    recipient_hi_decimal: &str,
    recipient_lo_decimal: &str,
) -> Result<JsValue, JsValue> {
    let nonce = decimal_to_fr_result(nonce_decimal).map_err(|e| JsValue::from_str(&e))?;
    let hi = decimal_to_fr_result(recipient_hi_decimal).map_err(|e| JsValue::from_str(&e))?;
    let lo = decimal_to_fr_result(recipient_lo_decimal).map_err(|e| JsValue::from_str(&e))?;
    let key = derived_escrow::try_derived_escrow_key(nonce, hi, lo).ok_or_else(|| {
        JsValue::from_str("derived escrow hash is outside BabyJub subgroup; retry nonce")
    })?;
    #[derive(serde::Serialize)]
    struct Out {
        #[serde(rename = "scalarHex")]
        scalar_hex: String,
        #[serde(rename = "pointXHex")]
        point_x_hex: String,
        #[serde(rename = "pointYHex")]
        point_y_hex: String,
    }
    serde_wasm_bindgen::to_value(&Out {
        scalar_hex: key.scalar_hex,
        point_x_hex: key.point_x_hex,
        point_y_hex: key.point_y_hex,
    })
    .map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Graph witness + arkworks Groth16 prove. Returns `{ proof_hex, public_hex }`.
#[wasm_bindgen(js_name = "proveGroth16")]
pub fn prove_groth16(
    graph: &[u8],
    proving_key: &[u8],
    r1cs: &[u8],
    inputs_json: &str,
) -> Result<JsValue, JsValue> {
    let calc = groth16::witness::WitnessCalculator::from_graph(graph)
        .map_err(|e| JsValue::from_str(&format!("witness graph: {e:#}")))?;
    let witness = calc
        .compute_witness(inputs_json)
        .map_err(|e| JsValue::from_str(&format!("witness: {e:#}")))?;
    let prover = groth16::prover::Prover::new(proving_key, r1cs)
        .map_err(|e| JsValue::from_str(&format!("prover init: {e:#}")))?;
    let proof_bytes = prover
        .prove_bytes_uncompressed(&witness)
        .map_err(|e| JsValue::from_str(&format!("prove: {e:#}")))?;
    let public_le = prover
        .extract_public_inputs(&witness)
        .map_err(|e| JsValue::from_str(&format!("public inputs: {e:#}")))?;
    let public_hex = groth16::prover::public_inputs_le_to_soroban_hex(&public_le)
        .map_err(|e| JsValue::from_str(&format!("public hex: {e:#}")))?;
    #[derive(serde::Serialize)]
    struct Out {
        proof_hex: String,
        public_hex: String,
    }
    serde_wasm_bindgen::to_value(&Out {
        proof_hex: hex::encode(proof_bytes),
        public_hex,
    })
    .map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Circuit `ECDH`: `priv * (pub_x, pub_y)` → shared key (`key[0], key[1]` hex).
#[wasm_bindgen(js_name = "ecdhSharedKey")]
pub fn ecdh_shared_key_js(
    priv_hex: &str,
    pub_x_hex: &str,
    pub_y_hex: &str,
) -> Result<JsValue, JsValue> {
    let priv_b = stealth::parse_scalar_hex(priv_hex).map_err(|e| JsValue::from_str(&e))?;
    let pub_x = stealth::parse_scalar_hex(pub_x_hex).map_err(|e| JsValue::from_str(&e))?;
    let pub_y = stealth::parse_scalar_hex(pub_y_hex).map_err(|e| JsValue::from_str(&e))?;
    let (x, y) = ecdh_shared_key(&priv_b, &pub_x, &pub_y)
        .map_err(|e| JsValue::from_str(&format!("ecdh_shared_key: {:?}", e)))?;
    #[derive(serde::Serialize)]
    struct Out {
        x: String,
        y: String,
    }
    let out = Out {
        x: hex::encode(x),
        y: hex::encode(y),
    };
    serde_wasm_bindgen::to_value(&out).map_err(|e| JsValue::from_str(&format!("{e}")))
}
