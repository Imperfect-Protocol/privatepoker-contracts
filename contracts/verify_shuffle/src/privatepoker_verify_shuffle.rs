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
        if len != masked_after.len() || traces.len() > len {
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

            v_masked_before.push(g1_masked_before);
            v_masked_after.push(g1_masked_after);
        }

        for trace in traces {
            const LEN: usize = size_of::<u32>();
            if trace.len() != LEN * 2 {
                return Err(b"INVALID_TRACE_LENGTH")?;
            }
            let mut b_after_index = [0u8; LEN];
            let mut b_claimed_before_index = [0u8; LEN];

            b_after_index.copy_from_slice(&trace[0..LEN]);
            b_claimed_before_index.copy_from_slice(&trace[LEN..]);

            let after_index = u32::from_le_bytes(b_after_index);
            let claimed_before_index = u32::from_le_bytes(b_claimed_before_index);

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

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use bls12_381::{G1Affine, G1Projective, G2Affine, G2Projective, Scalar};
    use pairing::group::Curve;
    use stylus_sdk::{abi::Bytes, testing::TestVM};

    use super::{verify_shuffle_traced, PrivatePokerVerifyShuffle, ShuffleTrace};

    fn card(seed: u64) -> G1Affine {
        (G1Projective::generator() * Scalar::from(seed)).to_affine()
    }

    fn public_key(sk: Scalar) -> G2Affine {
        (G2Projective::generator() * sk).to_affine()
    }

    fn mask(point: G1Affine, sk: Scalar) -> G1Affine {
        (G1Projective::from(point) * sk).to_affine()
    }

    fn known_shuffle_state() -> (Vec<G1Affine>, Vec<G1Affine>, G2Affine, Vec<ShuffleTrace>) {
        let sk = Scalar::from(17u64);
        let masked_before = vec![card(11), card(12), card(13), card(14)];
        let order = [2usize, 0, 3, 1];
        let masked_after = order
            .iter()
            .map(|before_index| mask(masked_before[*before_index], sk))
            .collect::<Vec<_>>();
        let traces = order
            .iter()
            .enumerate()
            .map(|(after_index, before_index)| ShuffleTrace {
                after_index: after_index as u32,
                claimed_before_index: *before_index as u32,
            })
            .collect::<Vec<_>>();

        (masked_before, masked_after, public_key(sk), traces)
    }

    fn g1_bytes(point: G1Affine) -> Bytes {
        Bytes(point.to_compressed().to_vec())
    }

    fn g2_bytes(point: G2Affine) -> Bytes {
        Bytes(point.to_compressed().to_vec())
    }

    fn trace_bytes(trace: ShuffleTrace) -> Bytes {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend_from_slice(&trace.after_index.to_le_bytes());
        bytes.extend_from_slice(&trace.claimed_before_index.to_le_bytes());
        Bytes(bytes)
    }

    fn shuffle_contract_args() -> (Vec<Bytes>, Vec<Bytes>, Bytes, Vec<Bytes>) {
        let (masked_before, masked_after, pk, traces) = known_shuffle_state();
        (
            masked_before.into_iter().map(g1_bytes).collect(),
            masked_after.into_iter().map(g1_bytes).collect(),
            g2_bytes(pk),
            traces.into_iter().map(trace_bytes).collect(),
        )
    }

    #[test]
    fn verifies_known_shuffle_state() {
        let (masked_before, masked_after, pk, traces) = known_shuffle_state();

        assert!(verify_shuffle_traced(&masked_before, &masked_after, &pk, &traces).is_ok());
    }

    #[test]
    fn rejects_shuffle_with_wrong_public_key() {
        let (masked_before, masked_after, _pk, traces) = known_shuffle_state();
        let wrong_pk = public_key(Scalar::from(19u64));

        assert_eq!(
            verify_shuffle_traced(&masked_before, &masked_after, &wrong_pk, &traces),
            Err("BATCHED_TRACE_VERIFICATION_FAILED")
        );
    }

    #[test]
    fn rejects_shuffle_with_duplicate_input_index() {
        let (masked_before, masked_after, pk, mut traces) = known_shuffle_state();
        traces[1].claimed_before_index = traces[0].claimed_before_index;

        assert_eq!(
            verify_shuffle_traced(&masked_before, &masked_after, &pk, &traces),
            Err("DUPLICATE_INPUT_INDEX")
        );
    }

    #[test]
    fn testvm_contract_accepts_known_shuffle_state() {
        let vm = TestVM::default();
        let mut contract = PrivatePokerVerifyShuffle::from(&vm);
        let (masked_before, masked_after, pk, traces) = shuffle_contract_args();

        assert!(contract
            .verify_shuffle(masked_before, masked_after, pk, traces)
            .is_ok());
    }

    #[test]
    fn testvm_contract_rejects_wrong_shuffle_public_key() {
        let vm = TestVM::default();
        let mut contract = PrivatePokerVerifyShuffle::from(&vm);
        let (masked_before, masked_after, _pk, traces) = shuffle_contract_args();
        let wrong_pk = g2_bytes(public_key(Scalar::from(19u64)));

        assert_eq!(
            contract.verify_shuffle(masked_before, masked_after, wrong_pk, traces),
            Err(b"BATCHED_TRACE_VERIFICATION_FAILED".to_vec())
        );
    }

    #[test]
    fn testvm_contract_rejects_malformed_trace_bytes() {
        let vm = TestVM::default();
        let mut contract = PrivatePokerVerifyShuffle::from(&vm);
        let (masked_before, masked_after, pk, mut traces) = shuffle_contract_args();
        traces[0] = Bytes(vec![0, 1, 2]);

        assert_eq!(
            contract.verify_shuffle(masked_before, masked_after, pk, traces),
            Err(b"INVALID_TRACE_LENGTH".to_vec())
        );
    }
}
