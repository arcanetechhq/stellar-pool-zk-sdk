//! Split a Circom Groth16 prove into a witness map plus chunked MSMs.
//!
//! `finish` follows `ark_groth16::Groth16::create_proof_with_assignment` for
//! ark-groth16 0.6, including the `query[0]` term that is not part of the MSM.

use super::circom_reduction::CircomReduction;
use super::prover::{
    bytes_to_fr, g1_bytes_uncompressed, g2_bytes_uncompressed, proof_to_uncompressed_bytes,
    FIELD_SIZE,
};
use anyhow::{anyhow, bail, Result};
use ark_bn254_zk::{Bn254, Fq, Fq2, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec_zk::{CurveGroup, VariableBaseMSM};
use ark_ff_zk::{BigInteger, PrimeField, Zero};
use ark_groth16::r1cs_to_qap::R1CSToQAP;
use ark_groth16::{Groth16, Proof, ProvingKey};
use ark_poly::GeneralEvaluationDomain;
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, OptimizationGoal, SynthesisMode,
};
use ark_std::vec::Vec;
use core::ops::{AddAssign, Neg};

const G1_BYTES: usize = 64;
const G2_BYTES: usize = 128;
/// `a0`, `b_g1_0`, `b_g2_0`, `alpha_g1`, `beta_g1`, `beta_g2`, `delta_g1`, `delta_g2`.
pub const CONSTANTS_BYTES: usize = G1_BYTES * 5 + G2_BYTES * 3;

pub struct PreparedScalars {
    pub assignment: Vec<Fr>,
    pub h: Vec<Fr>,
    pub aux: Vec<Fr>,
    pub r: Fr,
    pub s: Fr,
}

pub struct ProofConstants {
    pub a0: G1Affine,
    pub b_g1_0: G1Affine,
    pub b_g2_0: G2Affine,
    pub alpha_g1: G1Affine,
    pub beta_g1: G1Affine,
    pub beta_g2: G2Affine,
    pub delta_g1: G1Affine,
    pub delta_g2: G2Affine,
}

pub struct MsmBases {
    pub a: Vec<G1Affine>,
    pub b_g1: Vec<G1Affine>,
    pub b_g2: Vec<G2Affine>,
    pub h: Vec<G1Affine>,
    pub l: Vec<G1Affine>,
    pub constants: ProofConstants,
}

pub fn prepare_circuit<C: ConstraintSynthesizer<Fr>>(
    circuit: C,
    r: Fr,
    s: Fr,
) -> Result<PreparedScalars> {
    let cs = ConstraintSystem::new_ref();
    cs.set_optimization_goal(OptimizationGoal::Constraints);
    cs.set_mode(SynthesisMode::Prove {
        construct_matrices: true,
        generate_lc_assignments: false,
    });
    circuit
        .generate_constraints(cs.clone())
        .map_err(|e| anyhow!("constraint synthesis: {e:?}"))?;
    cs.finalize();
    let h = CircomReduction::witness_map::<Fr, GeneralEvaluationDomain<Fr>>(cs.clone())
        .map_err(|e| anyhow!("witness map: {e:?}"))?;
    let held = cs
        .borrow()
        .ok_or_else(|| anyhow!("constraint system is already borrowed"))?;
    let instance = held
        .instance_assignment()
        .map_err(|e| anyhow!("instance assignment: {e:?}"))?;
    let aux = held
        .witness_assignment()
        .map_err(|e| anyhow!("witness assignment: {e:?}"))?;
    if instance.is_empty() {
        bail!("instance assignment is empty");
    }
    let mut assignment = Vec::with_capacity(instance.len() - 1 + aux.len());
    assignment.extend_from_slice(&instance[1..]);
    assignment.extend_from_slice(aux);
    Ok(PreparedScalars {
        assignment,
        h,
        aux: aux.to_vec(),
        r,
        s,
    })
}

pub fn bases_from_proving_key(pk: &ProvingKey<Bn254>) -> Result<MsmBases> {
    if pk.a_query.is_empty() || pk.b_g1_query.is_empty() || pk.b_g2_query.is_empty() {
        bail!("proving key queries are empty");
    }
    Ok(MsmBases {
        a: pk.a_query[1..].to_vec(),
        b_g1: pk.b_g1_query[1..].to_vec(),
        b_g2: pk.b_g2_query[1..].to_vec(),
        h: pk.h_query.clone(),
        l: pk.l_query.clone(),
        constants: ProofConstants {
            a0: pk.a_query[0],
            b_g1_0: pk.b_g1_query[0],
            b_g2_0: pk.b_g2_query[0],
            alpha_g1: pk.vk.alpha_g1,
            beta_g1: pk.beta_g1,
            beta_g2: pk.vk.beta_g2,
            delta_g1: pk.delta_g1,
            delta_g2: pk.vk.delta_g2,
        },
    })
}

pub fn msm_g1(bases: &[G1Affine], scalars: &[Fr]) -> Result<G1Projective> {
    msm_projective(bases, scalars)
}

pub fn msm_g2(bases: &[G2Affine], scalars: &[Fr]) -> Result<G2Projective> {
    msm_projective(bases, scalars)
}

fn msm_projective<P, G>(bases: &[G], scalars: &[Fr]) -> Result<P>
where
    P: VariableBaseMSM<MulBase = G, ScalarField = Fr> + Zero,
{
    if bases.len() != scalars.len() {
        bail!(
            "MSM length mismatch: {} bases and {} scalars",
            bases.len(),
            scalars.len()
        );
    }
    if bases.is_empty() {
        return Ok(P::zero());
    }
    let bigints = scalars
        .iter()
        .map(|scalar| scalar.into_bigint())
        .collect::<Vec<_>>();
    Ok(P::msm_bigint(bases, &bigints))
}

#[allow(clippy::arithmetic_side_effects)]
pub fn finish(
    constants: &ProofConstants,
    a_acc: G1Projective,
    b_g1_acc: G1Projective,
    b_g2_acc: G2Projective,
    h_acc: G1Projective,
    l_acc: G1Projective,
    r: Fr,
    s: Fr,
) -> Proof<Bn254> {
    let r_s_delta_g1 = constants.delta_g1 * (r * s);
    let mut g_a = constants.delta_g1 * r;
    g_a.add_assign(&constants.a0);
    g_a += a_acc;
    g_a.add_assign(&constants.alpha_g1);
    let s_g_a = g_a * s;

    let g1_b = if r.is_zero() {
        G1Projective::zero()
    } else {
        let mut point = constants.delta_g1 * s;
        point.add_assign(&constants.b_g1_0);
        point += b_g1_acc;
        point.add_assign(&constants.beta_g1);
        point
    };

    let mut g2_b = constants.delta_g2 * s;
    g2_b.add_assign(&constants.b_g2_0);
    g2_b += b_g2_acc;
    g2_b.add_assign(&constants.beta_g2);

    let mut g_c = s_g_a;
    g_c += g1_b * r;
    g_c += r_s_delta_g1.neg();
    g_c += l_acc;
    g_c += h_acc;

    Proof {
        a: g_a.into_affine(),
        b: g2_b.into_affine(),
        c: g_c.into_affine(),
    }
}

pub fn encode_scalars(prepared: &PreparedScalars) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        12 + (prepared.assignment.len() + prepared.h.len() + 2) * FIELD_SIZE,
    );
    push_u32(&mut out, prepared.assignment.len() as u32);
    push_u32(&mut out, prepared.h.len() as u32);
    push_u32(&mut out, prepared.aux.len() as u32);
    for scalar in prepared
        .assignment
        .iter()
        .chain(&prepared.h)
        .chain([&prepared.r, &prepared.s])
    {
        out.extend_from_slice(&fr_to_le(*scalar));
    }
    out
}

pub fn encode_bases(bases: &MsmBases) -> Vec<u8> {
    let mut out = Vec::new();
    push_u32(&mut out, bases.a.len() as u32);
    push_u32(&mut out, bases.h.len() as u32);
    push_u32(&mut out, bases.l.len() as u32);
    push_g1s(&mut out, &bases.a);
    push_g1s(&mut out, &bases.b_g1);
    push_g2s(&mut out, &bases.b_g2);
    push_g1s(&mut out, &bases.h);
    push_g1s(&mut out, &bases.l);
    let constants = &bases.constants;
    out.extend_from_slice(&g1_bytes_uncompressed(&constants.a0));
    out.extend_from_slice(&g1_bytes_uncompressed(&constants.b_g1_0));
    out.extend_from_slice(&g2_bytes_uncompressed(&constants.b_g2_0));
    out.extend_from_slice(&g1_bytes_uncompressed(&constants.alpha_g1));
    out.extend_from_slice(&g1_bytes_uncompressed(&constants.beta_g1));
    out.extend_from_slice(&g2_bytes_uncompressed(&constants.beta_g2));
    out.extend_from_slice(&g1_bytes_uncompressed(&constants.delta_g1));
    out.extend_from_slice(&g2_bytes_uncompressed(&constants.delta_g2));
    out
}

pub fn decode_constants(bytes: &[u8]) -> Result<ProofConstants> {
    if bytes.len() != CONSTANTS_BYTES {
        bail!("proof constants must be {CONSTANTS_BYTES} bytes");
    }
    let mut offset = 0;
    let a0 = g1_from_bytes(&bytes[offset..offset + G1_BYTES])?;
    offset += G1_BYTES;
    let b_g1_0 = g1_from_bytes(&bytes[offset..offset + G1_BYTES])?;
    offset += G1_BYTES;
    let b_g2_0 = g2_from_bytes(&bytes[offset..offset + G2_BYTES])?;
    offset += G2_BYTES;
    let alpha_g1 = g1_from_bytes(&bytes[offset..offset + G1_BYTES])?;
    offset += G1_BYTES;
    let beta_g1 = g1_from_bytes(&bytes[offset..offset + G1_BYTES])?;
    offset += G1_BYTES;
    let beta_g2 = g2_from_bytes(&bytes[offset..offset + G2_BYTES])?;
    offset += G2_BYTES;
    let delta_g1 = g1_from_bytes(&bytes[offset..offset + G1_BYTES])?;
    offset += G1_BYTES;
    let delta_g2 = g2_from_bytes(&bytes[offset..offset + G2_BYTES])?;
    Ok(ProofConstants {
        a0,
        b_g1_0,
        b_g2_0,
        alpha_g1,
        beta_g1,
        beta_g2,
        delta_g1,
        delta_g2,
    })
}

pub fn decode_g1_points(bytes: &[u8]) -> Result<Vec<G1Affine>> {
    decode_points(bytes, G1_BYTES, g1_from_bytes)
}

pub fn decode_g2_points(bytes: &[u8]) -> Result<Vec<G2Affine>> {
    decode_points(bytes, G2_BYTES, g2_from_bytes)
}

pub fn decode_scalars(bytes: &[u8]) -> Result<Vec<Fr>> {
    if !bytes.len().is_multiple_of(FIELD_SIZE) {
        bail!("scalar bytes are not a multiple of {FIELD_SIZE}");
    }
    bytes
        .chunks_exact(FIELD_SIZE)
        .map(bytes_to_fr)
        .collect()
}

pub fn encode_g1_projective(point: &G1Projective) -> [u8; G1_BYTES] {
    g1_bytes_uncompressed(&point.into_affine())
}

pub fn encode_g2_projective(point: &G2Projective) -> [u8; G2_BYTES] {
    g2_bytes_uncompressed(&point.into_affine())
}

pub fn decode_g1_projective(bytes: &[u8]) -> Result<G1Projective> {
    Ok(g1_from_bytes(bytes)?.into())
}

pub fn decode_g2_projective(bytes: &[u8]) -> Result<G2Projective> {
    Ok(g2_from_bytes(bytes)?.into())
}

pub fn sum_g1(points: &[G1Projective]) -> G1Projective {
    points.iter().fold(G1Projective::zero(), |acc, point| acc + point)
}

pub fn sum_g2(points: &[G2Projective]) -> G2Projective {
    points.iter().fold(G2Projective::zero(), |acc, point| acc + point)
}

/// Prove by splitting every MSM into `parts` contiguous chunks.
/// The byte codecs used by workers sit on this path.
pub fn prove_in_chunks<C: ConstraintSynthesizer<Fr>>(
    circuit: C,
    pk: &ProvingKey<Bn254>,
    r: Fr,
    s: Fr,
    parts: usize,
) -> Result<Proof<Bn254>> {
    if parts == 0 {
        bail!("MSM needs at least one chunk");
    }
    let prepared = prepare_circuit(circuit, r, s)?;
    let bases = bases_from_proving_key(pk)?;
    if prepared.assignment.len() != bases.a.len()
        || prepared.assignment.len() != bases.b_g1.len()
        || prepared.assignment.len() != bases.b_g2.len()
        || prepared.h.len() != bases.h.len()
        || prepared.aux.len() != bases.l.len()
    {
        bail!("prepared scalar lengths do not match proving-key queries");
    }
    let encoded = encode_bases(&bases);
    let decoded = decode_bases_for_test(&encoded)?;
    let a = chunk_msm_g1(&decoded.a, &prepared.assignment, parts)?;
    let b_g1 = chunk_msm_g1(&decoded.b_g1, &prepared.assignment, parts)?;
    let b_g2 = chunk_msm_g2(&decoded.b_g2, &prepared.assignment, parts)?;
    let h = chunk_msm_g1(&decoded.h, &prepared.h, parts)?;
    let l = chunk_msm_g1(&decoded.l, &prepared.aux, parts)?;
    Ok(finish(
        &decoded.constants,
        a,
        b_g1,
        b_g2,
        h,
        l,
        prepared.r,
        prepared.s,
    ))
}

pub fn direct_proof<C: ConstraintSynthesizer<Fr>>(
    circuit: C,
    pk: &ProvingKey<Bn254>,
    r: Fr,
    s: Fr,
) -> Result<Vec<u8>> {
    let proof = Groth16::<Bn254, CircomReduction>::create_proof_with_reduction(circuit, pk, r, s)
        .map_err(|e| anyhow!("direct prove: {e:?}"))?;
    Ok(proof_to_uncompressed_bytes(&proof))
}

fn chunk_msm_g1(bases: &[G1Affine], scalars: &[Fr], parts: usize) -> Result<G1Projective> {
    let mut acc = G1Projective::zero();
    for (base_chunk, scalar_chunk) in chunks(bases, scalars, parts) {
        acc += msm_g1(base_chunk, scalar_chunk)?;
    }
    Ok(acc)
}

fn chunk_msm_g2(bases: &[G2Affine], scalars: &[Fr], parts: usize) -> Result<G2Projective> {
    let mut acc = G2Projective::zero();
    for (base_chunk, scalar_chunk) in chunks(bases, scalars, parts) {
        acc += msm_g2(base_chunk, scalar_chunk)?;
    }
    Ok(acc)
}

fn chunks<'a, T>(
    bases: &'a [T],
    scalars: &'a [Fr],
    parts: usize,
) -> Vec<(&'a [T], &'a [Fr])> {
    let count = bases.len();
    let width = count.div_ceil(parts).max(1);
    let mut out = Vec::new();
    let mut start = 0;
    while start < count {
        let end = (start + width).min(count);
        out.push((&bases[start..end], &scalars[start..end]));
        start = end;
    }
    out
}

struct DecodedBases {
    a: Vec<G1Affine>,
    b_g1: Vec<G1Affine>,
    b_g2: Vec<G2Affine>,
    h: Vec<G1Affine>,
    l: Vec<G1Affine>,
    constants: ProofConstants,
}

fn decode_bases_for_test(bytes: &[u8]) -> Result<DecodedBases> {
    if bytes.len() < 12 + CONSTANTS_BYTES {
        bail!("base export is too short");
    }
    let assignment_len = read_u32(bytes, 0)? as usize;
    let h_len = read_u32(bytes, 4)? as usize;
    let aux_len = read_u32(bytes, 8)? as usize;
    let mut offset = 12;
    let a = take_g1(bytes, &mut offset, assignment_len)?;
    let b_g1 = take_g1(bytes, &mut offset, assignment_len)?;
    let b_g2 = take_g2(bytes, &mut offset, assignment_len)?;
    let h = take_g1(bytes, &mut offset, h_len)?;
    let l = take_g1(bytes, &mut offset, aux_len)?;
    let constants = decode_constants(&bytes[offset..offset + CONSTANTS_BYTES])?;
    offset += CONSTANTS_BYTES;
    if offset != bytes.len() {
        bail!("base export has trailing bytes");
    }
    Ok(DecodedBases {
        a,
        b_g1,
        b_g2,
        h,
        l,
        constants,
    })
}

fn take_g1(bytes: &[u8], offset: &mut usize, count: usize) -> Result<Vec<G1Affine>> {
    let size = count * G1_BYTES;
    let slice = bytes
        .get(*offset..*offset + size)
        .ok_or_else(|| anyhow!("truncated G1 bases"))?;
    *offset += size;
    decode_g1_points(slice)
}

fn take_g2(bytes: &[u8], offset: &mut usize, count: usize) -> Result<Vec<G2Affine>> {
    let size = count * G2_BYTES;
    let slice = bytes
        .get(*offset..*offset + size)
        .ok_or_else(|| anyhow!("truncated G2 bases"))?;
    *offset += size;
    decode_g2_points(slice)
}

fn decode_points<T>(
    bytes: &[u8],
    width: usize,
    decode: fn(&[u8]) -> Result<T>,
) -> Result<Vec<T>> {
    if !bytes.len().is_multiple_of(width) {
        bail!("point bytes are not a multiple of {width}");
    }
    bytes.chunks_exact(width).map(decode).collect()
}

fn g1_from_bytes(bytes: &[u8]) -> Result<G1Affine> {
    if bytes.len() != G1_BYTES {
        bail!("G1 point must be {G1_BYTES} bytes");
    }
    if bytes.iter().all(|byte| *byte == 0) {
        return Ok(G1Affine::identity());
    }
    let x = Fq::from_be_bytes_mod_order(&bytes[..32]);
    let y = Fq::from_be_bytes_mod_order(&bytes[32..]);
    Ok(G1Affine::new_unchecked(x, y))
}

fn g2_from_bytes(bytes: &[u8]) -> Result<G2Affine> {
    if bytes.len() != G2_BYTES {
        bail!("G2 point must be {G2_BYTES} bytes");
    }
    if bytes.iter().all(|byte| *byte == 0) {
        return Ok(G2Affine::identity());
    }
    let x = Fq2::new(
        Fq::from_be_bytes_mod_order(&bytes[32..64]),
        Fq::from_be_bytes_mod_order(&bytes[..32]),
    );
    let y = Fq2::new(
        Fq::from_be_bytes_mod_order(&bytes[96..128]),
        Fq::from_be_bytes_mod_order(&bytes[64..96]),
    );
    Ok(G2Affine::new_unchecked(x, y))
}

fn fr_to_le(scalar: Fr) -> [u8; FIELD_SIZE] {
    let bytes = scalar.into_bigint().to_bytes_le();
    let mut out = [0u8; FIELD_SIZE];
    let width = bytes.len().min(FIELD_SIZE);
    out[..width].copy_from_slice(&bytes[..width]);
    out
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow!("truncated header"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn push_g1s(out: &mut Vec<u8>, points: &[G1Affine]) {
    for point in points {
        out.extend_from_slice(&g1_bytes_uncompressed(point));
    }
}

fn push_g2s(out: &mut Vec<u8>, points: &[G2Affine]) {
    for point in points {
        out.extend_from_slice(&g2_bytes_uncompressed(point));
    }
}

#[cfg(test)]
mod tests {
    use super::{direct_proof, prove_in_chunks};
    use crate::groth16::circom_reduction::CircomReduction;
    use crate::groth16::prover::proof_to_uncompressed_bytes;
    use ark_bn254_zk::{Bn254, Fr};
    use ark_groth16::Groth16;
    use ark_relations::{
        gr1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError},
        lc,
    };
    use ark_snark::SNARK;
    use ark_std::rand::{rngs::StdRng, SeedableRng};
    use core::ops::AddAssign as _;

    type CircomGroth16 = Groth16<Bn254, CircomReduction>;

    #[derive(Clone)]
    struct MulCircuit {
        a: Fr,
        b: Fr,
        c: Fr,
    }

    impl ConstraintSynthesizer<Fr> for MulCircuit {
        fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
            let a = cs.new_witness_variable(|| Ok(self.a))?;
            let b = cs.new_witness_variable(|| Ok(self.b))?;
            let c = cs.new_input_variable(|| Ok(self.c))?;
            cs.enforce_r1cs_constraint(
                || {
                    let mut linear = lc!();
                    linear.add_assign((Fr::from(1u64), a));
                    linear
                },
                || {
                    let mut linear = lc!();
                    linear.add_assign((Fr::from(1u64), b));
                    linear
                },
                || {
                    let mut linear = lc!();
                    linear.add_assign((Fr::from(1u64), c));
                    linear
                },
            )
        }
    }

    #[test]
    fn sharded_chunks_match_direct_proof() {
        let mut rng = StdRng::seed_from_u64(7);
        let a = Fr::from(3u64);
        let b = Fr::from(11u64);
        let circuit = MulCircuit { a, b, c: a * b };
        let (pk, vk) = CircomGroth16::circuit_specific_setup(circuit.clone(), &mut rng)
            .expect("setup");
        let r = Fr::from(9u64);
        let s = Fr::from(5u64);
        let direct = direct_proof(circuit.clone(), &pk, r, s).expect("direct");
        let sharded = prove_in_chunks(circuit, &pk, r, s, 3).expect("sharded");
        assert_eq!(direct, proof_to_uncompressed_bytes(&sharded));
        let pvk = CircomGroth16::process_vk(&vk).expect("process vk");
        assert!(
            CircomGroth16::verify_with_processed_vk(&pvk, &[a * b], &sharded).expect("verify")
        );
    }
}
