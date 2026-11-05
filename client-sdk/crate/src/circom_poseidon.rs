//! Circomlib `Poseidon(5)`, the hash inside `EdDSAPoseidonVerifier`.
//!
//! The swap authorization message itself is a Poseidon255 chain. The EdDSA
//! challenge `H(R8, A, message)` is circomlib Poseidon, not Poseidon255.

use ark_bn254::Fr;
use ark_ff::{Field, PrimeField};
use std::sync::OnceLock;

struct Params {
    mds: Vec<Vec<Fr>>,
    rc: Vec<Vec<Fr>>,
    rounds_f: usize,
    rounds_p: usize,
    t: usize,
}

fn parse_constant(hex: &str) -> Fr {
    let digits = hex.trim().trim_start_matches("0x");
    let padded = format!("{:0>64}", digits);
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(&padded, &mut bytes).expect("circomlib poseidon constant");
    Fr::from_be_bytes_mod_order(&bytes)
}

fn load_t6() -> Params {
    let t = 6;
    let rounds_f = 8;
    let rounds_p = 60;
    let constants = t6::C_T6;
    assert_eq!(constants.len(), t * (rounds_f + rounds_p));
    let mut rc = vec![vec![Fr::from(0u64); t]; rounds_f + rounds_p];
    for round in 0..(rounds_f + rounds_p) {
        for limb in 0..t {
            rc[round][limb] = parse_constant(constants[round * t + limb]);
        }
    }
    let mut mds = vec![vec![Fr::from(0u64); t]; t];
    for (row_index, row) in t6::M_T6.iter().enumerate() {
        assert_eq!(row.len(), t);
        for (column, value) in row.iter().enumerate() {
            mds[row_index][column] = parse_constant(value);
        }
    }
    Params {
        mds,
        rc,
        rounds_f,
        rounds_p,
        t,
    }
}

fn permute(state: &mut [Fr], params: &Params) {
    let t = params.t;
    let half_f = params.rounds_f / 2;
    for round in 0..(params.rounds_f + params.rounds_p) {
        for limb in 0..t {
            state[limb] += params.rc[round][limb];
        }
        if round < half_f || round >= half_f + params.rounds_p {
            for limb in 0..t {
                state[limb] = state[limb].pow([5]);
            }
        } else {
            state[0] = state[0].pow([5]);
        }
        let mut next = vec![Fr::from(0u64); t];
        for row in 0..t {
            for column in 0..t {
                next[row] += params.mds[row][column] * state[column];
            }
        }
        state.copy_from_slice(&next);
    }
}

static PARAMS_T6: OnceLock<Params> = OnceLock::new();

/// Circomlib `Poseidon(5)` of five field elements. Output is the capacity element.
pub fn poseidon5(inputs: [Fr; 5]) -> Fr {
    let params = PARAMS_T6.get_or_init(load_t6);
    let mut state = vec![
        Fr::from(0u64),
        inputs[0],
        inputs[1],
        inputs[2],
        inputs[3],
        inputs[4],
    ];
    permute(&mut state, params);
    state[0]
}

mod t6 {
    include!("circom_poseidon_t6_constants.rs");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::fr_to_decimal;

    #[test]
    fn poseidon5_matches_circomlibjs() {
        let hash = poseidon5([
            Fr::from(1u64),
            Fr::from(2u64),
            Fr::from(3u64),
            Fr::from(4u64),
            Fr::from(5u64),
        ]);
        assert_eq!(
            fr_to_decimal(&hash),
            "6183221330272524995739186171720101788151706631170188140075976616310159254464"
        );
    }
}
