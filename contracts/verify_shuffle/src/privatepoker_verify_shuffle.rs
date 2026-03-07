use alloc::vec::Vec;

use alloy_primitives::map::HashSet;
use bls12_381::{Bls12, G1Affine, G2Affine, G2Prepared};
use pairing::{group::Group, MultiMillerLoop};
use stylus_sdk::{abi::Bytes, prelude::*};

#[storage]
#[entrypoint]
pub struct PrivatePokerVerifyShuffle;

#[public]
impl PrivatePokerVerifyShuffle {
    pub fn verify_shuffle(
        &mut self,
        masked_before: Vec<Bytes>,
        masked_after: Vec<Bytes>,
        pk: Bytes,
        traces: Vec<Bytes>,
    ) -> Result<(), Vec<u8>> {
        let len = masked_before.len();
        if len != masked_after.len() || len != traces.len() {
            return Err(b"UNMATCHED_LENGTH")?;
        }

        let mut v_masked_before = Vec::with_capacity(len);
        let mut v_masked_after = Vec::with_capacity(len);
        let mut v_traces = Vec::with_capacity(len);

        for i in 0..len {
            let Ok(g1_masked_before) = make_g1_from_compressed_slice(&masked_before[i].0) else {
                return Err(b"INVALID_G1_MASKED_BEFORE")?;
            };
            let Ok(g1_masked_after) = make_g1_from_compressed_slice(&masked_after[i].0) else {
                return Err(b"INVALID_G1_MASKED_AFTER")?;
            };

            const LEN: usize = size_of::<u32>();
            let mut b_after_index = [0u8; LEN];
            let mut b_claimed_before_index = [0u8; LEN];

            b_after_index.copy_from_slice(&traces[i][0..LEN]);
            b_claimed_before_index.copy_from_slice(&traces[i][LEN..]);

            let after_index = u32::from_le_bytes(b_after_index);
            let claimed_before_index = u32::from_le_bytes(b_claimed_before_index);

            v_masked_before.push(g1_masked_before);
            v_masked_after.push(g1_masked_after);
            v_traces.push(ShuffleTrace {
                after_index,
                claimed_before_index,
            });
        }

        let Ok(v_pk) = make_g2_from_compressed_slice(&pk.0) else {
            return Err(b"INVALID_G2_PK")?;
        };

        verify_shuffle_traced(&v_masked_before, &v_masked_after, &v_pk, &v_traces)?;

        Ok(())
    }
}

pub struct ShuffleTrace {
    pub after_index: u32,
    pub claimed_before_index: u32,
}

/// Verifies that "masked_before" data has been shuffled into "masked_after"
/// data with signing key corresponding to public key.
///
/// This is efficient O(M) algorithm using only single Final Exponentiation call.
///
pub fn verify_shuffle_traced(
    masked_before: &[G1Affine],
    masked_after: &[G1Affine],
    pk: &G2Affine,
    traces: &[ShuffleTrace], // Only M traces submitted
) -> Result<(), &'static str> {
    let pk_prepared = G2Prepared::from(*pk);
    let neg_g2_gen = -G2Affine::generator();
    let neg_g2_prepared = G2Prepared::from(neg_g2_gen);

    // 1. THE BIJECTION CHECK
    let mut used_before_indices = HashSet::new();

    // Create a vector to hold all pairing terms for the batched Miller Loop.
    // Each trace adds 2 terms: one for the card after, one for the card before.
    let mut miller_loop_terms = Vec::with_capacity(traces.len() * 2);

    for trace in traces {
        // Prevent out-of-bounds panics
        if trace.after_index as usize >= masked_after.len()
            || trace.claimed_before_index as usize >= masked_before.len()
        {
            return Err("TRACE_INDEX_OUT_OF_BOUNDS");
        }

        // Ensure no two outputs point to the same input card
        if !used_before_indices.insert(trace.claimed_before_index) {
            // Cheater attempted to clone a card.
            return Err("DUPLICATE_INPUT_INDEX");
        }

        let point_after = &masked_after[trace.after_index as usize];
        let point_before = &masked_before[trace.claimed_before_index as usize];

        // Push the tuples for this specific trace into the batch array
        miller_loop_terms.push((point_after, &neg_g2_prepared));
        miller_loop_terms.push((point_before, &pk_prepared));
    }

    // 2. THE O(M) BATCHED MILLER LOOP
    // We run the Miller loop over all pairs at once, then do a SINGLE final exponentiation.
    let is_valid: bool = Bls12::multi_miller_loop(&miller_loop_terms)
        .final_exponentiation()
        .is_identity()
        .into();

    if !is_valid {
        // Cryptographic forgery: The batched trace verification failed.
        return Err("BATCHED_TRACE_VERIFICATION_FAILED");
    }

    Ok(())
}

pub const G1AFFINE_COMPRESSED_LEN: usize = 48;
pub const G2AFFINE_COMPRESSED_LEN: usize = 96;

pub fn make_g2_from_compressed_slice(data: &[u8]) -> Result<G2Affine, &'static str> {
    if data.len() != G2AFFINE_COMPRESSED_LEN {
        return Err("INVALID_G2_COMPRESSED_LENGTH");
    }
    let mut bytes = [0u8; G2AFFINE_COMPRESSED_LEN];
    bytes.copy_from_slice(data);
    G2Affine::from_compressed(&bytes)
        .into_option()
        .ok_or("G2_DECODE_ERROR")
}

pub fn make_g1_from_compressed_slice(data: &[u8]) -> Result<G1Affine, &'static str> {
    if data.len() != G1AFFINE_COMPRESSED_LEN {
        return Err("INVALID_G1_COMPRESSED_LENGTH");
    }
    let mut bytes = [0u8; G1AFFINE_COMPRESSED_LEN];
    bytes.copy_from_slice(data);
    G1Affine::from_compressed(&bytes)
        .into_option()
        .ok_or("G1_DECODE_ERROR")
}
