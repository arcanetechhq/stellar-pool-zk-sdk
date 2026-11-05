//! Seller swap authorization. No proving key and no SDK session.

use ark_bn254::Fr;
use ark_ff::PrimeField;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use serde::Serialize;

use crate::circom_poseidon::poseidon5;
use crate::coin::{commitment_chain_with_accumulator, CoinData, OwnerPub};
use crate::poseidon::{poseidon_hash_1, poseidon_hash_2};
use crate::utils::{fr_to_bytes_be, fr_to_decimal};

/// ASCII "PSWP".
const DOM_SWAP: u64 = 1_347_639_120;
const BABYJUB_SUBGROUP_ORDER: &str =
    "2736030358979909402780800718157159386076813972158567259200215660948447373041";
/// BN254 scalar field modulus.
const FIELD_MODULUS: &str =
    "21888242871839275222246405745257275088548364400416034343698204186575808495617";
/// EdDSA nonce. Fixed so the same authorization signs the same way.
const SIGNATURE_NONCE: u64 = 9;

#[derive(Serialize)]
pub(crate) struct Signature {
    #[serde(rename = "R8")]
    r8: [String; 2],
    #[serde(rename = "S")]
    s: String,
}

#[derive(Serialize)]
pub(crate) struct NoteOpening {
    value: String,
    nullifier: String,
    secret: String,
    commitment: String,
    asset_hi: String,
    asset_lo: String,
    application_id: String,
    accumulator: String,
    owner_x: String,
    owner_y: String,
    ephemeral_key_scalar: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SwapAuthorization {
    signature: Signature,
    signed_fields: Vec<String>,
    minimum_sender_balance: String,
    #[serde(rename = "bPaid")]
    b_paid: String,
    note_openings: Vec<NoteOpening>,
}

struct OpenedNote {
    value: BigUint,
    asset_hi: Fr,
    asset_lo: Fr,
    application_id: Fr,
    commitment: Fr,
}

pub(crate) fn create_swap_authorization(
    spend_scalar_decimal: &str,
    notes_json: &str,
    sender_owner_x: &str,
    sender_owner_y: &str,
    sender_asset_hi: &str,
    sender_asset_lo: &str,
    quote_numerator: &str,
    quote_denominator: &str,
    swap_amount: &str,
    expiration_ledger: &str,
) -> Result<SwapAuthorization, String> {
    let raw_notes: Vec<serde_json::Value> = serde_json::from_str(notes_json)
        .map_err(|err| format!("seller notes are not a note list: {err}"))?;
    if raw_notes.is_empty() || raw_notes.len() > 5 {
        return Err(format!(
            "a swap authorization takes one to five notes, got {}",
            raw_notes.len()
        ));
    }
    let notes = raw_notes
        .iter()
        .map(|note| {
            serde_json::from_value::<CoinData>(note.clone())
                .map_err(|err| format!("seller note is not a withdrawal note: {err}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let scalar = parse_scalar(spend_scalar_decimal)?;
    let owner = owner_point(&scalar)?;
    let opened = notes
        .iter()
        .map(|note| open_note(note, owner))
        .collect::<Result<Vec<_>, _>>()?;
    let asset_hi = opened[0].asset_hi;
    let asset_lo = opened[0].asset_lo;
    let application_id = opened[0].application_id;
    if opened
        .iter()
        .any(|note| note.asset_hi != asset_hi || note.asset_lo != asset_lo)
    {
        return Err("seller notes must share one asset".to_string());
    }
    if opened
        .iter()
        .any(|note| note.application_id != application_id)
    {
        return Err("seller notes must share one application id".to_string());
    }

    let quote_num = parse_bounded(quote_numerator, "quote numerator")?;
    let quote_den = parse_bounded(quote_denominator, "quote denominator")?;
    let swap_amount_int = parse_bounded(swap_amount, "swap amount")?;
    if quote_num.is_zero() || quote_den.is_zero() || swap_amount_int.is_zero() {
        return Err("quote and swap amount must be non-zero".to_string());
    }
    let product = &swap_amount_int * &quote_num;
    if product >= (BigUint::one() << 253u32) {
        return Err("swap amount times quote numerator must be below 2^253".to_string());
    }
    let b_paid = &product / &quote_den;
    if b_paid.is_zero() {
        return Err("bPaid is zero".to_string());
    }
    if b_paid >= (BigUint::one() << 127u32) {
        return Err("bPaid must be below 2^127".to_string());
    }
    let input_sum = opened
        .iter()
        .fold(BigUint::zero(), |sum, note| sum + &note.value);
    if input_sum < b_paid {
        return Err("seller notes do not cover bPaid".to_string());
    }
    let change = input_sum - &b_paid;

    let sender_x = parse_field(sender_owner_x, "sender owner key")?;
    let sender_y = parse_field(sender_owner_y, "sender owner key")?;
    let sender_hi = parse_field(sender_asset_hi, "sender asset")?;
    let sender_lo = parse_field(sender_asset_lo, "sender asset")?;
    let expiration = parse_field(expiration_ledger, "expiration ledger")?;

    let mut input_commitments: Vec<Fr> = opened.iter().map(|note| note.commitment).collect();
    input_commitments.sort_by(|left, right| integer(left).cmp(&integer(right)));
    if input_commitments.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("seller input commitments are not strictly ascending".to_string());
    }

    let mut openings = Vec::with_capacity(2);
    openings.push(output_opening(
        &swap_amount_int,
        [sender_hi, sender_lo],
        application_id,
        owner,
    )?);
    if !change.is_zero() {
        openings.push(output_opening(
            &change,
            [asset_hi, asset_lo],
            application_id,
            owner,
        )?);
    }
    let mut output_commitments: Vec<Fr> = openings
        .iter()
        .map(|opening| parse_field(&opening.commitment, "output commitment"))
        .collect::<Result<Vec<_>, _>>()?;
    output_commitments.sort_by(|left, right| integer(left).cmp(&integer(right)));
    if output_commitments.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("seller output commitments are not strictly ascending".to_string());
    }

    let mut fields = vec![
        Fr::from(DOM_SWAP),
        sender_x,
        sender_y,
        sender_hi,
        sender_lo,
        asset_hi,
        asset_lo,
        field_from_uint(&quote_num)?,
        field_from_uint(&quote_den)?,
        field_from_uint(&swap_amount_int)?,
        expiration,
        Fr::from(input_commitments.len() as u64),
    ];
    fields.extend(input_commitments);
    fields.push(Fr::from(output_commitments.len() as u64));
    fields.extend(output_commitments);
    let signed_fields: Vec<String> = fields.iter().map(fr_to_decimal).collect();
    let message = poseidon_chain(&fields);
    let signature = sign_poseidon(message, &scalar)?;

    Ok(SwapAuthorization {
        signature,
        signed_fields,
        minimum_sender_balance: swap_amount_int.to_str_radix(10),
        b_paid: b_paid.to_str_radix(10),
        note_openings: openings,
    })
}

fn open_note(note: &CoinData, owner: OwnerPub) -> Result<OpenedNote, String> {
    let value = parse_field(&note.value, "note value")?;
    let secret = parse_field(&note.secret, "note secret")?;
    let nullifier = parse_field(&note.nullifier, "note nullifier")?;
    let asset_hi = parse_field(&note.asset_hi, "note asset")?;
    let asset_lo = parse_field(&note.asset_lo, "note asset")?;
    let application_id = parse_field(&note.application_id, "note application id")?;
    let expected = parse_field(&note.commitment, "note commitment")?;
    let commitment = commitment_chain_with_accumulator(
        value,
        secret,
        nullifier,
        owner,
        Fr::from(0u64),
        [asset_hi, asset_lo],
        application_id,
        Fr::from(0u64),
    );
    if commitment != expected {
        return Err("spend scalar does not open the notes".to_string());
    }
    Ok(OpenedNote {
        value: integer(&value),
        asset_hi,
        asset_lo,
        application_id,
        commitment,
    })
}

fn output_opening(
    value: &BigUint,
    asset: [Fr; 2],
    application_id: Fr,
    owner: OwnerPub,
) -> Result<NoteOpening, String> {
    let value_field = field_from_uint(value)?;
    let ephemeral = fresh_scalar()?;
    let secret = poseidon_hash_1(ephemeral);
    let mut nullifier = fresh_scalar()?;
    let mut commitment = commitment_chain_with_accumulator(
        value_field,
        secret,
        nullifier,
        owner,
        Fr::from(0u64),
        asset,
        application_id,
        Fr::from(0u64),
    );
    // A second sample only matters if the nullifier collided with itself.
    if commitment == Fr::from(0u64) {
        nullifier = fresh_scalar()?;
        commitment = commitment_chain_with_accumulator(
            value_field,
            secret,
            nullifier,
            owner,
            Fr::from(0u64),
            asset,
            application_id,
            Fr::from(0u64),
        );
    }
    Ok(NoteOpening {
        value: value.to_str_radix(10),
        nullifier: fr_to_decimal(&nullifier),
        secret: fr_to_decimal(&secret),
        commitment: fr_to_decimal(&commitment),
        asset_hi: fr_to_decimal(&asset[0]),
        asset_lo: fr_to_decimal(&asset[1]),
        application_id: fr_to_decimal(&application_id),
        accumulator: "0".to_string(),
        owner_x: fr_to_decimal(&owner.x),
        owner_y: fr_to_decimal(&owner.y),
        ephemeral_key_scalar: fr_to_decimal(&ephemeral),
    })
}

fn sign_poseidon(message: Fr, scalar: &BigUint) -> Result<Signature, String> {
    let nonce = BigUint::from(SIGNATURE_NONCE);
    let nonce_point = owner_point(&nonce)?;
    let owner = owner_point(scalar)?;
    let challenge = poseidon5([nonce_point.x, nonce_point.y, owner.x, owner.y, message]);
    let order = BigUint::parse_bytes(BABYJUB_SUBGROUP_ORDER.as_bytes(), 10)
        .expect("subgroup order");
    let s = (&nonce + integer(&challenge) * scalar * 8u32) % order;
    Ok(Signature {
        r8: [fr_to_decimal(&nonce_point.x), fr_to_decimal(&nonce_point.y)],
        s: s.to_str_radix(10),
    })
}

fn poseidon_chain(fields: &[Fr]) -> Fr {
    let mut acc = fields[0];
    for field in &fields[1..] {
        acc = poseidon_hash_2(acc, *field);
    }
    acc
}

fn owner_point(scalar: &BigUint) -> Result<OwnerPub, String> {
    let bytes = scalar_bytes(scalar)?;
    let (x, y) = cryptography::ecdh_ephemeral_public_key(&bytes);
    Ok(OwnerPub {
        x: Fr::from_be_bytes_mod_order(&x),
        y: Fr::from_be_bytes_mod_order(&y),
    })
}

fn parse_scalar(decimal: &str) -> Result<BigUint, String> {
    let scalar = parse_uint(decimal, "spend scalar")?;
    let order = BigUint::parse_bytes(BABYJUB_SUBGROUP_ORDER.as_bytes(), 10)
        .expect("subgroup order");
    if scalar.is_zero() || scalar >= order {
        return Err("spend scalar is outside the BabyJub subgroup".to_string());
    }
    Ok(scalar)
}

fn parse_bounded(decimal: &str, name: &str) -> Result<BigUint, String> {
    let value = parse_uint(decimal, name)?;
    if value >= (BigUint::one() << 127u32) {
        return Err(format!("{name} must be below 2^127"));
    }
    Ok(value)
}

fn parse_field(decimal: &str, name: &str) -> Result<Fr, String> {
    let value = parse_uint(decimal, name)?;
    let modulus = BigUint::parse_bytes(FIELD_MODULUS.as_bytes(), 10).expect("field modulus");
    if value >= modulus {
        return Err(format!("{name} is not a field element"));
    }
    field_from_uint(&value)
}

fn parse_uint(decimal: &str, name: &str) -> Result<BigUint, String> {
    let decimal = decimal.trim();
    if decimal.is_empty() || !decimal.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{name} is not an integer"));
    }
    BigUint::parse_bytes(decimal.as_bytes(), 10).ok_or_else(|| format!("{name} is not an integer"))
}

fn field_from_uint(value: &BigUint) -> Result<Fr, String> {
    let bytes = value.to_bytes_be();
    if bytes.len() > 32 {
        return Err("integer does not fit in a field element".to_string());
    }
    let mut padded = [0u8; 32];
    padded[32 - bytes.len()..].copy_from_slice(&bytes);
    Ok(Fr::from_be_bytes_mod_order(&padded))
}

fn integer(field: &Fr) -> BigUint {
    BigUint::from_bytes_be(&fr_to_bytes_be(field))
}

fn scalar_bytes(scalar: &BigUint) -> Result<[u8; 32], String> {
    let bytes = scalar.to_bytes_be();
    if bytes.len() > 32 {
        return Err("spend scalar does not fit in 32 bytes".to_string());
    }
    let mut padded = [0u8; 32];
    padded[32 - bytes.len()..].copy_from_slice(&bytes);
    if padded[0] & 0xE0 != 0 {
        return Err("spend scalar must be below 2^253".to_string());
    }
    Ok(padded)
}

fn fresh_scalar() -> Result<Fr, String> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|err| err.to_string())?;
    bytes[0] &= 0x1F;
    if bytes.iter().all(|byte| *byte == 0) {
        bytes[31] = 1;
    }
    Ok(Fr::from_be_bytes_mod_order(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_the_circomlib_eddsa_vector() {
        let signature = sign_poseidon(Fr::from(12345u64), &BigUint::from(424242u32))
            .expect("signature");
        assert_eq!(
            signature.r8,
            [
                "4705897243203718691035604313913899717760209962238015362153877735592901317263",
                "11533909001000295577818857040682494493436124051895563619976413559559984357704",
            ]
        );
        assert_eq!(
            signature.s,
            "358829666924093346893412930508875309967692793886401469120113378645357300984"
        );
    }
}
