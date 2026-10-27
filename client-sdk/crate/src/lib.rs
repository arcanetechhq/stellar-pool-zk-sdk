pub mod coin;
pub mod convert;
pub mod derived_escrow;
pub mod groth16;
pub mod merkle;
pub mod params;
pub mod poseidon;
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

/// Owner-bound nullifier: Poseidon255(3)(DOM_NULLIFIER_V3, nullifier, Poseidon255(2)(ownerPub)).
/// `priv_key_scalar_decimal` is the BabyJub spend scalar; ownerPub is scalar·G.
/// Returns hex string (0x...)
#[wasm_bindgen(js_name = "calculateNullifierHash")]
pub fn calculate_nullifier_hash(
    nullifier_decimal: &str,
    priv_key_scalar_decimal: &str,
) -> Result<String, JsValue> {
    coin::calculate_nullifier_hash(nullifier_decimal, priv_key_scalar_decimal)
        .map_err(|e| JsValue::from_str(&e))
}

/// Five-link Poseidon255(3) commitment. Decimal field strings in, `0x` hex out.
#[wasm_bindgen(js_name = "commitmentChainHex")]
pub fn commitment_chain_hex_js(
    value: &str,
    secret: &str,
    nullifier: &str,
    owner_x: &str,
    owner_y: &str,
    owner_mode: &str,
    asset_hi: &str,
    asset_lo: &str,
    application_id: &str,
) -> Result<String, JsValue> {
    coin::commitment_chain_hex(
        value,
        secret,
        nullifier,
        owner_x,
        owner_y,
        owner_mode,
        asset_hi,
        asset_lo,
        application_id,
    )
    .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen(js_name = "commitmentChainHexWithAccumulator")]
pub fn commitment_chain_hex_with_accumulator_js(
    value: &str,
    secret: &str,
    nullifier: &str,
    owner_x: &str,
    owner_y: &str,
    owner_mode: &str,
    asset_hi: &str,
    asset_lo: &str,
    application_id: &str,
    accumulator: &str,
) -> Result<String, JsValue> {
    coin::commitment_chain_hex_with_accumulator(
        value,
        secret,
        nullifier,
        owner_x,
        owner_y,
        owner_mode,
        asset_hi,
        asset_lo,
        application_id,
        accumulator,
    )
    .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen(js_name = "poseidon1Decimal")]
pub fn poseidon1_decimal_js(value: &str) -> Result<String, JsValue> {
    let field = decimal_to_fr_result(value).map_err(|e| JsValue::from_str(&e))?;
    Ok(utils::fr_to_decimal(&poseidon::poseidon_hash_1(field)))
}

#[wasm_bindgen(js_name = "poseidon2Decimal")]
pub fn poseidon2_decimal_js(left: &str, right: &str) -> Result<String, JsValue> {
    let a = decimal_to_fr_result(left).map_err(|e| JsValue::from_str(&e))?;
    let b = decimal_to_fr_result(right).map_err(|e| JsValue::from_str(&e))?;
    Ok(utils::fr_to_decimal(&poseidon::poseidon_hash_2(a, b)))
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

/// Witness map plus Groth16 blinding scalars. MSM bases stay in `exportMsmBases`.
#[wasm_bindgen]
pub struct PreparedGroth16 {
    scalars: Vec<u8>,
    public_hex: String,
}

#[wasm_bindgen]
impl PreparedGroth16 {
    #[wasm_bindgen(getter)]
    pub fn scalars(&self) -> Vec<u8> {
        self.scalars.clone()
    }

    #[wasm_bindgen(getter, js_name = publicHex)]
    pub fn public_hex(&self) -> String {
        self.public_hex.clone()
    }
}

/// Graph witness, public signals, and the scalar vectors for a chunked MSM.
#[wasm_bindgen(js_name = "prepareGroth16")]
pub fn prepare_groth16(
    graph: &[u8],
    proving_key: &[u8],
    r1cs: &[u8],
    inputs_json: &str,
) -> Result<PreparedGroth16, JsValue> {
    let calc = groth16::witness::WitnessCalculator::from_graph(graph)
        .map_err(|e| JsValue::from_str(&format!("witness graph: {e:#}")))?;
    let witness = calc
        .compute_witness(inputs_json)
        .map_err(|e| JsValue::from_str(&format!("witness: {e:#}")))?;
    let (scalars, public_le) =
        groth16::prover::Prover::with_cached(proving_key, r1cs, |prover| {
            let scalars = prover.prepare_scalar_bytes(&witness)?;
            let public_le = prover.extract_public_inputs(&witness)?;
            Ok((scalars, public_le))
        })
        .map_err(|e| JsValue::from_str(&format!("prepare: {e:#}")))?;
    let public_hex = groth16::prover::public_inputs_le_to_soroban_hex(&public_le)
        .map_err(|e| JsValue::from_str(&format!("public hex: {e:#}")))?;
    Ok(PreparedGroth16 {
        scalars,
        public_hex,
    })
}

/// Uncompressed MSM bases and the Groth16 constants used by `finishGroth16`.
#[wasm_bindgen(js_name = "exportMsmBases")]
pub fn export_msm_bases(proving_key: &[u8], r1cs: &[u8]) -> Result<Vec<u8>, JsValue> {
    groth16::prover::Prover::with_cached(proving_key, r1cs, |prover| prover.export_base_bytes())
        .map_err(|e| JsValue::from_str(&format!("export bases: {e:#}")))
}

#[wasm_bindgen(js_name = "msmG1")]
pub fn msm_g1(bases: &[u8], scalars: &[u8]) -> Result<Vec<u8>, JsValue> {
    let bases = groth16::shard::decode_g1_points(bases)
        .map_err(|e| JsValue::from_str(&format!("g1 bases: {e:#}")))?;
    let scalars = groth16::shard::decode_scalars(scalars)
        .map_err(|e| JsValue::from_str(&format!("scalars: {e:#}")))?;
    let point = groth16::shard::msm_g1(&bases, &scalars)
        .map_err(|e| JsValue::from_str(&format!("msm g1: {e:#}")))?;
    Ok(groth16::shard::encode_g1_projective(&point).to_vec())
}

#[wasm_bindgen(js_name = "msmG2")]
pub fn msm_g2(bases: &[u8], scalars: &[u8]) -> Result<Vec<u8>, JsValue> {
    let bases = groth16::shard::decode_g2_points(bases)
        .map_err(|e| JsValue::from_str(&format!("g2 bases: {e:#}")))?;
    let scalars = groth16::shard::decode_scalars(scalars)
        .map_err(|e| JsValue::from_str(&format!("scalars: {e:#}")))?;
    let point = groth16::shard::msm_g2(&bases, &scalars)
        .map_err(|e| JsValue::from_str(&format!("msm g2: {e:#}")))?;
    Ok(groth16::shard::encode_g2_projective(&point).to_vec())
}

#[wasm_bindgen(js_name = "sumG1")]
pub fn sum_g1(points: &[u8]) -> Result<Vec<u8>, JsValue> {
    let decoded = groth16::shard::decode_g1_points(points)
        .map_err(|e| JsValue::from_str(&format!("g1 points: {e:#}")))?;
    let projective = decoded
        .iter()
        .map(|point| (*point).into())
        .collect::<Vec<ark_bn254_zk::G1Projective>>();
    Ok(groth16::shard::encode_g1_projective(&groth16::shard::sum_g1(&projective)).to_vec())
}

#[wasm_bindgen(js_name = "sumG2")]
pub fn sum_g2(points: &[u8]) -> Result<Vec<u8>, JsValue> {
    let decoded = groth16::shard::decode_g2_points(points)
        .map_err(|e| JsValue::from_str(&format!("g2 points: {e:#}")))?;
    let projective = decoded
        .iter()
        .map(|point| (*point).into())
        .collect::<Vec<ark_bn254_zk::G2Projective>>();
    Ok(groth16::shard::encode_g2_projective(&groth16::shard::sum_g2(&projective)).to_vec())
}

/// Add Groth16 blinding to the summed MSM points. Returns the 256-byte proof.
#[wasm_bindgen(js_name = "finishGroth16")]
pub fn finish_groth16(
    constants: &[u8],
    a_acc: &[u8],
    b_g1_acc: &[u8],
    b_g2_acc: &[u8],
    h_acc: &[u8],
    l_acc: &[u8],
    r: &[u8],
    s: &[u8],
) -> Result<Vec<u8>, JsValue> {
    let constants = groth16::shard::decode_constants(constants)
        .map_err(|e| JsValue::from_str(&format!("constants: {e:#}")))?;
    let a = groth16::shard::decode_g1_projective(a_acc)
        .map_err(|e| JsValue::from_str(&format!("A: {e:#}")))?;
    let b_g1 = groth16::shard::decode_g1_projective(b_g1_acc)
        .map_err(|e| JsValue::from_str(&format!("B G1: {e:#}")))?;
    let b_g2 = groth16::shard::decode_g2_projective(b_g2_acc)
        .map_err(|e| JsValue::from_str(&format!("B G2: {e:#}")))?;
    let h = groth16::shard::decode_g1_projective(h_acc)
        .map_err(|e| JsValue::from_str(&format!("H: {e:#}")))?;
    let l = groth16::shard::decode_g1_projective(l_acc)
        .map_err(|e| JsValue::from_str(&format!("L: {e:#}")))?;
    let r = groth16::shard::decode_scalars(r)
        .map_err(|e| JsValue::from_str(&format!("r: {e:#}")))?;
    let s = groth16::shard::decode_scalars(s)
        .map_err(|e| JsValue::from_str(&format!("s: {e:#}")))?;
    let (Some(r), Some(s)) = (r.first().copied(), s.first().copied()) else {
        return Err(JsValue::from_str("r and s must each be one field element"));
    };
    let proof = groth16::shard::finish(&constants, a, b_g1, b_g2, h, l, r, s);
    Ok(groth16::prover::proof_to_uncompressed_bytes(&proof))
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
    let (proof_bytes, public_le) =
        groth16::prover::Prover::with_cached(proving_key, r1cs, |prover| {
            let proof_bytes = prover.prove_bytes_uncompressed(&witness)?;
            let public_le = prover.extract_public_inputs(&witness)?;
            Ok((proof_bytes, public_le))
        })
        .map_err(|e| JsValue::from_str(&format!("prove: {e:#}")))?;
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
