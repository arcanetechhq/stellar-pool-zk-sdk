//! Groth16 prove from a Circom witness + R1CS, with snarkjs-compatible QAP.

use crate::groth16::circom_reduction::CircomReduction;
use crate::groth16::r1cs::R1CS;
use anyhow::{anyhow, Result};
use ark_bn254_zk::{Bn254, Fr, G1Affine, G2Affine};
use ark_ff_zk::{AdditiveGroup, BigInteger, Field, PrimeField, UniformRand};
use ark_groth16::{PreparedVerifyingKey, Proof, ProvingKey};
use ark_relations::{
    gr1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable},
    lc,
};
use ark_serialize_zk::CanonicalDeserialize;
use ark_snark::SNARK;
use ark_std::rand::rngs::OsRng;
use core::ops::AddAssign;
use std::cell::RefCell;

pub const FIELD_SIZE: usize = 32;

pub(crate) fn bytes_to_fr(bytes: &[u8]) -> Result<Fr> {
    if bytes.len() != FIELD_SIZE {
        return Err(anyhow!(
            "Expected {} bytes, got {}",
            FIELD_SIZE,
            bytes.len()
        ));
    }
    Ok(Fr::from_le_bytes_mod_order(bytes))
}

fn bigint_to_be_32<B: BigInteger>(value: B) -> [u8; 32] {
    let bytes = value.to_bytes_be();
    let mut out = [0u8; 32];
    let start = 32usize.saturating_sub(bytes.len());
    out[start..].copy_from_slice(&bytes[..bytes.len().min(32)]);
    out
}

pub(crate) fn g1_bytes_uncompressed(p: &G1Affine) -> [u8; 64] {
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&bigint_to_be_32(p.x.into_bigint()));
    out[32..].copy_from_slice(&bigint_to_be_32(p.y.into_bigint()));
    out
}

pub(crate) fn g2_bytes_uncompressed(p: &G2Affine) -> [u8; 128] {
    let mut out = [0u8; 128];
    let x0 = bigint_to_be_32(p.x.c0.into_bigint());
    let x1 = bigint_to_be_32(p.x.c1.into_bigint());
    let y0 = bigint_to_be_32(p.y.c0.into_bigint());
    let y1 = bigint_to_be_32(p.y.c1.into_bigint());
    out[..32].copy_from_slice(&x1);
    out[32..64].copy_from_slice(&x0);
    out[64..96].copy_from_slice(&y1);
    out[96..].copy_from_slice(&y0);
    out
}

pub(crate) fn proof_to_uncompressed_bytes(proof: &Proof<Bn254>) -> Vec<u8> {
    let mut out = Vec::with_capacity(256);
    out.extend_from_slice(&g1_bytes_uncompressed(&proof.a));
    out.extend_from_slice(&g2_bytes_uncompressed(&proof.b));
    out.extend_from_slice(&g1_bytes_uncompressed(&proof.c));
    out
}

struct R1CSCircuit {
    r1cs: R1CS,
    witness: Vec<Fr>,
}

impl ConstraintSynthesizer<Fr> for R1CSCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        if self.witness.first() != Some(&Fr::ONE) {
            return Err(SynthesisError::Unsatisfiable);
        }
        if self
            .r1cs
            .num_public
            .checked_add(1)
            .expect("R1CS num of public inputs addition failed")
            > self.r1cs.num_wires
        {
            return Err(SynthesisError::Unsatisfiable);
        }
        let num_public = self.r1cs.num_public as usize;
        let num_wires = self.r1cs.num_wires as usize;
        let mut variables: Vec<Variable> = Vec::with_capacity(num_wires);
        variables.push(Variable::One);
        if self.witness.len() < num_wires {
            return Err(SynthesisError::Unsatisfiable);
        }
        for i in 1..=num_public {
            let value = self
                .witness
                .get(i)
                .copied()
                .ok_or(SynthesisError::Unsatisfiable)?;
            let var = cs.new_input_variable(|| Ok(value))?;
            variables.push(var);
        }
        for i in num_public
            .checked_add(1)
            .ok_or(SynthesisError::Unsatisfiable)?..num_wires
        {
            let value = self.witness.get(i).copied().unwrap_or(Fr::ZERO);
            let var = cs.new_witness_variable(|| Ok(value))?;
            variables.push(var);
        }
        let max_wire = self.r1cs.num_wires as usize;
        for constraint in &self.r1cs.constraints {
            for t in constraint
                .a
                .terms
                .iter()
                .chain(&constraint.b.terms)
                .chain(&constraint.c.terms)
            {
                if (t.wire_id as usize) >= max_wire {
                    return Err(SynthesisError::Unsatisfiable);
                }
            }
        }
        for constraint in &self.r1cs.constraints {
            cs.enforce_r1cs_constraint(
                || {
                    let mut lc_a = lc!();
                    for t in &constraint.a.terms {
                        lc_a.add_assign((t.coefficient, variables[t.wire_id as usize]));
                    }
                    lc_a
                },
                || {
                    let mut lc_b = lc!();
                    for t in &constraint.b.terms {
                        lc_b.add_assign((t.coefficient, variables[t.wire_id as usize]));
                    }
                    lc_b
                },
                || {
                    let mut lc_c = lc!();
                    for t in &constraint.c.terms {
                        lc_c.add_assign((t.coefficient, variables[t.wire_id as usize]));
                    }
                    lc_c
                },
            )?;
        }
        Ok(())
    }
}

pub struct Prover {
    pk: ProvingKey<Bn254>,
    r1cs: R1CS,
}

struct ProverCache {
    pk_fingerprint: (u64, usize),
    r1cs_fingerprint: (u64, usize),
    prover: Prover,
}

thread_local! {
    static PROVER_CACHE: RefCell<Option<ProverCache>> = RefCell::new(None);
}

fn fingerprint(bytes: &[u8]) -> (u64, usize) {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash, bytes.len())
}

impl Prover {
    /// Reuse the decompressed proving key when the same bytes are loaded again.
    pub fn with_cached<T>(
        pk_bytes: &[u8],
        r1cs_bytes: &[u8],
        use_prover: impl FnOnce(&Prover) -> Result<T>,
    ) -> Result<T> {
        let pk_fingerprint = fingerprint(pk_bytes);
        let r1cs_fingerprint = fingerprint(r1cs_bytes);
        PROVER_CACHE.with(|slot| {
            {
                let cached = slot.borrow();
                if let Some(hit) = cached.as_ref() {
                    if hit.pk_fingerprint == pk_fingerprint
                        && hit.r1cs_fingerprint == r1cs_fingerprint
                    {
                        return use_prover(&hit.prover);
                    }
                }
            }
            let prover = Prover::new(pk_bytes, r1cs_bytes)?;
            let result = use_prover(&prover);
            *slot.borrow_mut() = Some(ProverCache {
                pk_fingerprint,
                r1cs_fingerprint,
                prover,
            });
            result
        })
    }

    pub fn new(pk_bytes: &[u8], r1cs_bytes: &[u8]) -> Result<Prover> {
        let pk = ProvingKey::<Bn254>::deserialize_compressed_unchecked(pk_bytes)
            .map_err(|e| anyhow!("Failed to load proving key: {}", e))?;
        Self::from_pk(pk, r1cs_bytes)
    }

    fn from_pk(pk: ProvingKey<Bn254>, r1cs_bytes: &[u8]) -> Result<Prover> {
        let r1cs = R1CS::parse(r1cs_bytes)?;
        if pk.vk.gamma_abc_g1.len().saturating_sub(1) != r1cs.num_public as usize {
            return Err(anyhow!("VK public input count doesn't match R1CS"));
        }
        let _pvk: PreparedVerifyingKey<Bn254> =
            <ark_groth16::Groth16<Bn254, CircomReduction> as SNARK<Fr>>::process_vk(&pk.vk)
                .map_err(|e| anyhow!("Failed to process VK: {}", e))?;
        Ok(Prover { pk, r1cs })
    }

    pub fn prove_bytes_uncompressed(&self, witness_bytes: &[u8]) -> Result<Vec<u8>> {
        if !witness_bytes.len().is_multiple_of(FIELD_SIZE) {
            return Err(anyhow!(
                "Invalid witness size: {} bytes (not multiple of {})",
                witness_bytes.len(),
                FIELD_SIZE
            ));
        }
        let num_witness_elements = witness_bytes.len() / FIELD_SIZE;
        if num_witness_elements < self.r1cs.num_wires as usize {
            return Err(anyhow!(
                "Witness too short: {} elements, circuit needs {} wires",
                num_witness_elements,
                self.r1cs.num_wires
            ));
        }
        let mut witness: Vec<Fr> = Vec::with_capacity(num_witness_elements);
        for chunk in witness_bytes.chunks_exact(FIELD_SIZE) {
            witness.push(bytes_to_fr(chunk)?);
        }
        let circuit = R1CSCircuit {
            r1cs: self.r1cs.clone(),
            witness,
        };
        let mut rng = OsRng;
        let proof = <ark_groth16::Groth16<Bn254, CircomReduction> as SNARK<Fr>>::prove(
            &self.pk, circuit, &mut rng,
        )
        .map_err(|e| anyhow!("Proof generation failed: {}", e))?;
        Ok(proof_to_uncompressed_bytes(&proof))
    }

    pub fn prepare_scalar_bytes(&self, witness_bytes: &[u8]) -> Result<Vec<u8>> {
        let witness = witness_field_elements(witness_bytes, self.r1cs.num_wires as usize)?;
        let mut rng = OsRng;
        let prepared = crate::groth16::shard::prepare_circuit(
            R1CSCircuit {
                r1cs: self.r1cs.clone(),
                witness,
            },
            Fr::rand(&mut rng),
            Fr::rand(&mut rng),
        )?;
        let query_len = self.pk.a_query.len();
        if query_len == 0
            || self.pk.b_g1_query.len() != query_len
            || self.pk.b_g2_query.len() != query_len
        {
            return Err(anyhow!(
                "proving key query lengths differ: a {}, b_g1 {}, b_g2 {}",
                query_len,
                self.pk.b_g1_query.len(),
                self.pk.b_g2_query.len()
            ));
        }
        let assignment_len = query_len - 1;
        if prepared.assignment.len() != assignment_len
            || prepared.h.len() != self.pk.h_query.len()
            || prepared.aux.len() != self.pk.l_query.len()
        {
            return Err(anyhow!(
                "prepared scalars do not match proving key queries: assignment {}/{}, h {}/{}, aux {}/{}",
                prepared.assignment.len(),
                assignment_len,
                prepared.h.len(),
                self.pk.h_query.len(),
                prepared.aux.len(),
                self.pk.l_query.len()
            ));
        }
        Ok(crate::groth16::shard::encode_scalars(&prepared))
    }

    pub fn export_base_bytes(&self) -> Result<Vec<u8>> {
        let bases = crate::groth16::shard::bases_from_proving_key(&self.pk)?;
        Ok(crate::groth16::shard::encode_bases(&bases))
    }

    pub fn extract_public_inputs(&self, witness_bytes: &[u8]) -> Result<Vec<u8>> {
        if !witness_bytes.len().is_multiple_of(FIELD_SIZE) {
            return Err(anyhow!("Invalid witness size"));
        }
        let num_public = self.r1cs.num_public as usize;
        let start = FIELD_SIZE;
        let public_size = num_public
            .checked_mul(FIELD_SIZE)
            .ok_or_else(|| anyhow!("Overflow calculating public inputs size"))?;
        let end = start
            .checked_add(public_size)
            .ok_or_else(|| anyhow!("Overflow calculating end offset"))?;
        if end > witness_bytes.len() {
            return Err(anyhow!(
                "Witness too short: expected at least {} bytes for {} public inputs",
                end,
                num_public
            ));
        }
        Ok(witness_bytes[start..end].to_vec())
    }
}

fn witness_field_elements(witness_bytes: &[u8], num_wires: usize) -> Result<Vec<Fr>> {
    if !witness_bytes.len().is_multiple_of(FIELD_SIZE) {
        return Err(anyhow!(
            "Invalid witness size: {} bytes (not multiple of {})",
            witness_bytes.len(),
            FIELD_SIZE
        ));
    }
    let num_witness_elements = witness_bytes.len() / FIELD_SIZE;
    if num_witness_elements < num_wires {
        return Err(anyhow!(
            "Witness too short: {} elements, circuit needs {} wires",
            num_witness_elements,
            num_wires
        ));
    }
    let mut witness = Vec::with_capacity(num_witness_elements);
    for chunk in witness_bytes.chunks_exact(FIELD_SIZE) {
        witness.push(bytes_to_fr(chunk)?);
    }
    Ok(witness)
}

/// Encode Circom public inputs (LE Frs) as Soroban `public_hex`: u32 BE count + BE-32 fields.
pub fn public_inputs_le_to_soroban_hex(le_bytes: &[u8]) -> Result<String> {
    if !le_bytes.len().is_multiple_of(FIELD_SIZE) {
        return Err(anyhow!("Invalid public input size"));
    }
    let n = le_bytes.len() / FIELD_SIZE;
    let mut out = Vec::with_capacity(4 + le_bytes.len());
    out.extend_from_slice(&(n as u32).to_be_bytes());
    for chunk in le_bytes.chunks_exact(FIELD_SIZE) {
        let mut be = [0u8; FIELD_SIZE];
        for (i, b) in chunk.iter().rev().enumerate() {
            be[i] = *b;
        }
        out.extend_from_slice(&be);
    }
    Ok(hex::encode(out))
}
