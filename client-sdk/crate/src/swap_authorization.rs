//! Seller swap authorization. No proving key and no SDK session.

use ark_bn254::Fr;
use ark_ff::{Field, PrimeField};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone)]
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
            serde_json::from_value::<InputNote>(note.clone())
                .map_err(|err| format!("seller note is not a withdrawal note: {err}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for note in &notes {
        refuse_note_shape(&note.owner_mode, &note.accumulator, &note.escrow_nonce)?;
    }

    let scalar = parse_scalar(spend_scalar_decimal)?;
    let owner = owner_point(&scalar)?;
    let opened = notes
        .iter()
        .map(|note| open_note(&note.coin(), owner))
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
    let sender_hi = parse_field(sender_asset_hi, "sender asset")?;
    let sender_lo = parse_field(sender_asset_lo, "sender asset")?;
    if sender_hi == asset_hi && sender_lo == asset_lo {
        return Err("same asset on both sides".to_string());
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
    let nonce = fresh_signature_nonce()?;
    let nonce_point = owner_point(&nonce)?;
    let owner = owner_point(scalar)?;
    let challenge = poseidon5([nonce_point.x, nonce_point.y, owner.x, owner.y, message]);
    let order =
        BigUint::parse_bytes(BABYJUB_SUBGROUP_ORDER.as_bytes(), 10).expect("subgroup order");
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
    let order =
        BigUint::parse_bytes(BABYJUB_SUBGROUP_ORDER.as_bytes(), 10).expect("subgroup order");
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

#[derive(Deserialize)]
struct InputNote {
    value: String,
    nullifier: String,
    secret: String,
    commitment: String,
    asset_hi: String,
    asset_lo: String,
    #[serde(default)]
    application_id: String,
    #[serde(default)]
    owner_mode: String,
    #[serde(default)]
    accumulator: String,
    #[serde(default)]
    escrow_nonce: String,
}

impl InputNote {
    fn coin(&self) -> CoinData {
        CoinData {
            value: self.value.clone(),
            nullifier: self.nullifier.clone(),
            secret: self.secret.clone(),
            commitment: self.commitment.clone(),
            asset_hi: self.asset_hi.clone(),
            asset_lo: self.asset_lo.clone(),
            application_id: zero_if_empty(&self.application_id).to_string(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmittedAuthorization {
    signature: Signature,
    signed_fields: Vec<String>,
    minimum_sender_balance: String,
    #[serde(rename = "bPaid")]
    b_paid: String,
    note_openings: Vec<SubmittedOpening>,
}

#[derive(Deserialize)]
struct SubmittedOpening {
    commitment: String,
    #[serde(default)]
    application_id: String,
    #[serde(default)]
    accumulator: String,
    owner_x: String,
    owner_y: String,
    #[serde(default)]
    owner_mode: String,
    #[serde(default)]
    escrow_nonce: String,
}

struct SignedOffer {
    sender_x: Fr,
    sender_y: Fr,
    sender_asset_hi: Fr,
    sender_asset_lo: Fr,
    seller_asset_hi: Fr,
    seller_asset_lo: Fr,
    quote_num: String,
    quote_den: String,
    swap_amount: String,
    expiration: String,
    inputs: Vec<String>,
    outputs: Vec<String>,
}

/// Reject a swap the circuit would reject, before Groth16 starts.
pub(crate) fn refuse_swap_proof(
    authorization_json: &str,
    seller_notes_json: &str,
    sender_notes_json: &str,
    sender_spend_scalar: &str,
    current_ledger: &str,
) -> Result<(), String> {
    let authorization: SubmittedAuthorization = serde_json::from_str(authorization_json)
        .map_err(|err| format!("swap authorization is not an authorization: {err}"))?;
    let seller_notes = parse_notes(seller_notes_json, "seller notes")?;
    let sender_notes = parse_notes(sender_notes_json, "sender notes")?;
    if seller_notes.is_empty() {
        return Err("subset of the seller's notes".to_string());
    }
    if sender_notes.is_empty() {
        return Err("sender notes do not cover the minimum sender balance".to_string());
    }
    if seller_notes.len() > 5 || sender_notes.len() > 5 {
        return Err("a swap accepts one to five notes on each side".to_string());
    }
    for note in seller_notes.iter().chain(sender_notes.iter()) {
        refuse_note_shape(&note.owner_mode, &note.accumulator, &note.escrow_nonce)?;
    }
    if authorization.note_openings.is_empty() {
        return Err("substituted seller output".to_string());
    }
    for opening in &authorization.note_openings {
        refuse_note_shape(
            &opening.owner_mode,
            &opening.accumulator,
            &opening.escrow_nonce,
        )?;
    }
    let mut application_ids = seller_notes
        .iter()
        .chain(sender_notes.iter())
        .map(|note| zero_if_empty(&note.application_id).to_string())
        .collect::<Vec<_>>();
    application_ids.extend(
        authorization
            .note_openings
            .iter()
            .map(|opening| zero_if_empty(&opening.application_id).to_string()),
    );
    if application_ids
        .iter()
        .any(|application_id| application_id != &application_ids[0])
    {
        return Err("notes do not share one application id".to_string());
    }
    let seller_asset = (
        parse_field(&seller_notes[0].asset_hi, "seller asset")?,
        parse_field(&seller_notes[0].asset_lo, "seller asset")?,
    );
    let sender_asset = (
        parse_field(&sender_notes[0].asset_hi, "sender asset")?,
        parse_field(&sender_notes[0].asset_lo, "sender asset")?,
    );
    if sender_asset == seller_asset {
        return Err("same asset on both sides".to_string());
    }
    let signed = parse_signed_offer(&authorization.signed_fields)?;
    let declared_b_paid = parse_uint(&authorization.b_paid, "bPaid")?;
    let floor = b_paid_floor(&signed.quote_num, &signed.quote_den, &signed.swap_amount)?;
    if declared_b_paid.is_zero() || floor.is_zero() {
        return Err("bPaid is zero".to_string());
    }
    if declared_b_paid != floor {
        return Err("bPaid does not match the quote".to_string());
    }
    let seller = (
        parse_field(&authorization.note_openings[0].owner_x, "seller owner key")?,
        parse_field(&authorization.note_openings[0].owner_y, "seller owner key")?,
    );
    verify_poseidon(
        poseidon_chain(
            &authorization
                .signed_fields
                .iter()
                .map(|field| parse_field(field, "signed field"))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        &authorization.signature,
        seller,
    )?;
    if !current_ledger.is_empty() {
        let current = parse_uint(current_ledger, "current ledger")?;
        let expiration = parse_uint(&signed.expiration, "expiration ledger")?;
        if current >= expiration {
            return Err("current ledger has reached the expiration ledger".to_string());
        }
    }
    let sender = owner_point(&parse_scalar(sender_spend_scalar)?)?;
    if sender.x != signed.sender_x || sender.y != signed.sender_y {
        return Err("sender key does not match the swap authorization".to_string());
    }
    if signed.sender_asset_hi != sender_asset.0
        || signed.sender_asset_lo != sender_asset.1
        || signed.seller_asset_hi != seller_asset.0
        || signed.seller_asset_lo != seller_asset.1
    {
        return Err("notes do not match the swap authorization".to_string());
    }
    let provided_inputs = sorted_decimals(
        seller_notes
            .iter()
            .map(|note| note.commitment.clone())
            .collect(),
    );
    if provided_inputs.len() < signed.inputs.len()
        && provided_inputs
            .iter()
            .all(|commitment| signed.inputs.contains(commitment))
    {
        return Err("subset of the seller's notes".to_string());
    }
    if provided_inputs != signed.inputs {
        return Err("seller notes do not match the swap authorization".to_string());
    }
    let provided_outputs = sorted_decimals(
        authorization
            .note_openings
            .iter()
            .map(|opening| opening.commitment.clone())
            .collect(),
    );
    if provided_outputs != signed.outputs {
        return Err("substituted seller output".to_string());
    }
    let sender_sum = sum_values(&sender_notes)?;
    let minimum = parse_uint(
        &authorization.minimum_sender_balance,
        "minimum sender balance",
    )?;
    if sender_sum < minimum || sender_sum < parse_uint(&signed.swap_amount, "swap amount")? {
        return Err("sender notes do not cover the minimum sender balance".to_string());
    }
    let seller_sum = sum_values(&seller_notes)?;
    if seller_sum < declared_b_paid {
        return Err("seller notes do not cover bPaid".to_string());
    }
    Ok(())
}

fn parse_notes(notes_json: &str, name: &str) -> Result<Vec<InputNote>, String> {
    serde_json::from_str(notes_json).map_err(|err| format!("{name} are not a note list: {err}"))
}

fn refuse_note_shape(
    owner_mode: &str,
    accumulator: &str,
    escrow_nonce: &str,
) -> Result<(), String> {
    if field_nonzero(owner_mode, "owner mode")? || field_nonzero(escrow_nonce, "escrow nonce")? {
        return Err("escrow note".to_string());
    }
    if field_nonzero(accumulator, "accumulator")? {
        return Err("non-zero accumulator".to_string());
    }
    Ok(())
}

fn field_nonzero(value: &str, name: &str) -> Result<bool, String> {
    Ok(!parse_uint(zero_if_empty(value), name)?.is_zero())
}

fn zero_if_empty(value: &str) -> &str {
    if value.is_empty() {
        "0"
    } else {
        value
    }
}

fn parse_signed_offer(fields: &[String]) -> Result<SignedOffer, String> {
    if fields.len() < 12 {
        return Err("swap authorization is missing signed fields".to_string());
    }
    let input_count = parse_count(&fields[11])?;
    let input_at = 12;
    let output_count_at = input_at + input_count;
    if fields.len() <= output_count_at {
        return Err("swap authorization signed fields do not match the note counts".to_string());
    }
    let output_count = parse_count(&fields[output_count_at])?;
    let output_at = output_count_at + 1;
    let output_end = output_at + output_count;
    if fields.len() != output_end {
        return Err("swap authorization signed fields do not match the note counts".to_string());
    }
    Ok(SignedOffer {
        sender_x: parse_field(&fields[1], "sender owner key")?,
        sender_y: parse_field(&fields[2], "sender owner key")?,
        sender_asset_hi: parse_field(&fields[3], "sender asset")?,
        sender_asset_lo: parse_field(&fields[4], "sender asset")?,
        seller_asset_hi: parse_field(&fields[5], "seller asset")?,
        seller_asset_lo: parse_field(&fields[6], "seller asset")?,
        quote_num: fields[7].clone(),
        quote_den: fields[8].clone(),
        swap_amount: fields[9].clone(),
        expiration: fields[10].clone(),
        inputs: fields[input_at..output_count_at].to_vec(),
        outputs: fields[output_at..output_end].to_vec(),
    })
}

fn parse_count(decimal: &str) -> Result<usize, String> {
    let count = parse_uint(decimal, "note count")?;
    if count > BigUint::from(5u8) {
        return Err("note count does not fit".to_string());
    }
    Ok(count.to_string().parse().unwrap_or(0))
}

fn b_paid_floor(quote_num: &str, quote_den: &str, swap_amount: &str) -> Result<BigUint, String> {
    let quote_num = parse_bounded(quote_num, "quote numerator")?;
    let quote_den = parse_bounded(quote_den, "quote denominator")?;
    let swap_amount = parse_bounded(swap_amount, "swap amount")?;
    if quote_num.is_zero() || quote_den.is_zero() || swap_amount.is_zero() {
        return Err("bPaid is zero".to_string());
    }
    Ok((&swap_amount * &quote_num) / &quote_den)
}

fn sum_values(notes: &[InputNote]) -> Result<BigUint, String> {
    notes.iter().try_fold(BigUint::zero(), |sum, note| {
        Ok(sum + parse_uint(&note.value, "note value")?)
    })
}

fn sorted_decimals(mut values: Vec<String>) -> Vec<String> {
    values.sort_by(|left, right| {
        let left_int = BigUint::parse_bytes(left.as_bytes(), 10).unwrap_or_else(BigUint::zero);
        let right_int = BigUint::parse_bytes(right.as_bytes(), 10).unwrap_or_else(BigUint::zero);
        left_int.cmp(&right_int)
    });
    values
}

fn verify_poseidon(message: Fr, signature: &Signature, owner: (Fr, Fr)) -> Result<(), String> {
    let r8 = (
        parse_field(&signature.r8[0], "signature")?,
        parse_field(&signature.r8[1], "signature")?,
    );
    let s = parse_uint(&signature.s, "signature")?;
    let challenge = poseidon5([r8.0, r8.1, owner.0, owner.1, message]);
    let base8 = (
        parse_field(
            "5299619240641551281634865583518297030282874472190772894086521144482721001553",
            "base",
        )?,
        parse_field(
            "16950150798460657717958625567821834550301663161624707787222815936182638968203",
            "base",
        )?,
    );
    let doubled = baby_add(owner, owner).and_then(|point| baby_add(point, point));
    let times_eight = doubled.and_then(|point| baby_add(point, point));
    let right = times_eight.and_then(|point| {
        baby_mul(point, &integer(&challenge), 254).and_then(|point| baby_add(r8, point))
    });
    let left = baby_mul(base8, &s, 253);
    match (left, right) {
        (Some(left), Some(right)) if left == right => Ok(()),
        _ => Err("swap authorization signature does not verify".to_string()),
    }
}

fn baby_add(left: (Fr, Fr), right: (Fr, Fr)) -> Option<(Fr, Fr)> {
    let curve_a = Fr::from(168700u64);
    let curve_d = Fr::from(168696u64);
    let (x1, y1) = left;
    let (x2, y2) = right;
    let beta = x1 * y2;
    let gamma = y1 * x2;
    let delta = (-curve_a * x1 + y1) * (x2 + y2);
    let tau = beta * gamma;
    let x_den = Fr::from(1u64) + curve_d * tau;
    let y_den = Fr::from(1u64) - curve_d * tau;
    Some((
        (beta + gamma) * x_den.inverse()?,
        (delta + curve_a * beta - gamma) * y_den.inverse()?,
    ))
}

fn baby_mul(point: (Fr, Fr), scalar: &BigUint, bits: usize) -> Option<(Fr, Fr)> {
    let mut acc = (Fr::from(0u64), Fr::from(1u64));
    let mut current = point;
    for bit in 0..bits {
        if scalar.bit(bit as u64) {
            acc = baby_add(acc, current)?;
        }
        current = baby_add(current, current)?;
    }
    Some(acc)
}

fn fresh_signature_nonce() -> Result<BigUint, String> {
    let order =
        BigUint::parse_bytes(BABYJUB_SUBGROUP_ORDER.as_bytes(), 10).expect("subgroup order");
    for _ in 0..64 {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).map_err(|err| err.to_string())?;
        bytes[0] &= 0x0f;
        let nonce = BigUint::from_bytes_be(&bytes);
        if !nonce.is_zero() && nonce < order {
            return Ok(nonce);
        }
    }
    Err("could not sample a signature nonce".to_string())
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
    fn two_signatures_from_one_scalar_do_not_share_a_nonce() {
        let scalar = BigUint::from(424242u32);
        let message = Fr::from(12345u64);
        let first = sign_poseidon(message, &scalar).expect("signature");
        let second = sign_poseidon(message, &scalar).expect("signature");
        assert_ne!(first.r8, second.r8);
        let owner = owner_point(&scalar).expect("owner");
        verify_poseidon(message, &first, (owner.x, owner.y)).expect("first signature");
        verify_poseidon(message, &second, (owner.x, owner.y)).expect("second signature");
    }

    #[test]
    fn signs_the_circomlib_eddsa_vector() {
        let signature =
            sign_poseidon(Fr::from(12345u64), &BigUint::from(424242u32)).expect("signature");
        let owner = owner_point(&BigUint::from(424242u32)).expect("owner");
        verify_poseidon(Fr::from(12345u64), &signature, (owner.x, owner.y)).expect("signature");
        let mut tampered = signature;
        tampered.s = "3".to_string();
        assert!(verify_poseidon(Fr::from(12345u64), &tampered, (owner.x, owner.y)).is_err());
    }
}
