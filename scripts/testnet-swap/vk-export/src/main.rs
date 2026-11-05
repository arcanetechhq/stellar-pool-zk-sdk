//! Write a snarkjs verification-key JSON from a compressed arkworks proving key.
//! The G2 coordinate order matches `cli/circom2soroban` (`Fq2::new(first, second)`).

use anyhow::{Context, Result};
use ark_bn254::{Bn254, Fq, G1Affine, G2Affine};
use ark_ff::PrimeField;
use ark_groth16::ProvingKey;
use ark_serialize::CanonicalDeserialize;
use serde_json::{json, Value};
use std::{env, fs, path::Path};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let proving_key = args
        .next()
        .context("usage: swap-vk-export <proving-key.bin> <verification_key.json>")?;
    let out = args
        .next()
        .context("usage: swap-vk-export <proving-key.bin> <verification_key.json>")?;
    let bytes = fs::read(&proving_key).with_context(|| format!("read {proving_key}"))?;
    let key = ProvingKey::<Bn254>::deserialize_compressed_unchecked(&bytes[..])
        .map_err(|err| anyhow::anyhow!("proving key: {err}"))?;
    let vk = key.vk;
    let ic: Vec<Value> = vk.gamma_abc_g1.iter().map(g1_json).collect();
    let n_public = ic.len().checked_sub(1).context("verification key IC is empty")?;
    let json = json!({
        "protocol": "groth16",
        "curve": "bn128",
        "nPublic": n_public,
        "vk_alpha_1": g1_json(&vk.alpha_g1),
        "vk_beta_2": g2_json(&vk.beta_g2),
        "vk_gamma_2": g2_json(&vk.gamma_g2),
        "vk_delta_2": g2_json(&vk.delta_g2),
        "IC": ic,
    });
    if let Some(parent) = Path::new(&out).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, serde_json::to_vec_pretty(&json)?)?;
    println!("Wrote {out} nPublic={n_public}");
    Ok(())
}

fn fq_dec(value: Fq) -> String {
    value.into_bigint().to_string()
}

fn g1_json(point: &G1Affine) -> Value {
    json!([fq_dec(point.x), fq_dec(point.y), "1"])
}

fn g2_json(point: &G2Affine) -> Value {
    json!([
        [fq_dec(point.x.c0), fq_dec(point.x.c1)],
        [fq_dec(point.y.c0), fq_dec(point.y.c1)],
        ["1", "0"],
    ])
}
