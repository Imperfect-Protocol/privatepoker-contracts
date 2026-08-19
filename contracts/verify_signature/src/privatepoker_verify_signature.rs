use alloc::{vec, vec::Vec};

use bls12_381::{Bls12, G1Affine, G2Affine};
use pairing::{group::Group, MultiMillerLoop};
use stylus_sdk::{abi::Bytes, prelude::*};

#[storage]
#[entrypoint]
pub struct PrivatePokerVerifySignature;

#[public]
impl PrivatePokerVerifySignature {
    pub fn verify_signature(
        &mut self,
        hashed_message: Bytes,
        aggregate_public_key: Bytes,
        aggregate_signature: Bytes,
    ) -> Result<bool, Vec<u8>> {
        if hashed_message.len() != G1AFFINE_COMPRESSED_LEN {
            return Err(vec![1]);
        }
        if aggregate_public_key.len() != G2AFFINE_COMPRESSED_LEN {
            return Err(vec![2]);
        }
        if aggregate_signature.len() != G1AFFINE_COMPRESSED_LEN {
            return Err(vec![3]);
        }

        Ok(verify_inner(
            &hashed_message.0,
            &aggregate_public_key.0,
            &aggregate_signature.0,
        ))
    }
}

pub fn verify_inner(
    hashed_message: &[u8],
    aggregate_public_key: &[u8],
    aggregate_signature: &[u8],
) -> bool {
    let Some(h) = make_g1_from_compressed_slice(hashed_message) else {
        return false;
    };
    let Some(pk) = make_g2_from_compressed_slice(aggregate_public_key) else {
        return false;
    };
    let Some(sig) = make_g1_from_compressed_slice(aggregate_signature) else {
        return false;
    };

    Bls12::multi_miller_loop(&[
        (&G1Affine::from(sig), &G2Affine::generator().into()),
        (&G1Affine::from(h), &(-G2Affine::from(pk)).into()),
    ])
    .final_exponentiation()
    .is_identity()
    .into()
}

pub const G1AFFINE_COMPRESSED_LEN: usize = 48;
pub const G2AFFINE_COMPRESSED_LEN: usize = 96;

pub fn make_g1_from_compressed_slice(data: &[u8]) -> Option<G1Affine> {
    if data.len() != G1AFFINE_COMPRESSED_LEN {
        return None;
    }
    let mut bytes = [0u8; G1AFFINE_COMPRESSED_LEN];
    bytes.copy_from_slice(data);
    G1Affine::from_compressed(&bytes).into_option()
}

pub fn make_g2_from_compressed_slice(data: &[u8]) -> Option<G2Affine> {
    if data.len() != G2AFFINE_COMPRESSED_LEN {
        return None;
    }
    let mut bytes = [0u8; G2AFFINE_COMPRESSED_LEN];
    bytes.copy_from_slice(data);
    G2Affine::from_compressed(&bytes).into_option()
}

#[cfg(test)]
mod tests {
    use super::{make_g1_from_compressed_slice, make_g2_from_compressed_slice, verify_inner};
    use bls12_381::{G1Affine, G2Affine, Scalar};
    use pairing::group::Curve;

    #[test]
    fn decodes_valid_compressed_points() {
        assert!(make_g1_from_compressed_slice(&G1Affine::generator().to_compressed()).is_some());
        assert!(make_g2_from_compressed_slice(&G2Affine::generator().to_compressed()).is_some());
    }

    #[test]
    fn verifies_crumble_signature_system() {
        let signing_key = Scalar::from(11u64);
        let hashed_message = G1Affine::generator();
        let public_key = (G2Affine::generator() * signing_key).to_affine();
        let signature = (hashed_message * signing_key).to_affine();

        assert!(verify_inner(
            &hashed_message.to_compressed(),
            &public_key.to_compressed(),
            &signature.to_compressed(),
        ));

        let other_signature = (hashed_message * Scalar::from(13u64)).to_affine();
        assert!(!verify_inner(
            &hashed_message.to_compressed(),
            &public_key.to_compressed(),
            &other_signature.to_compressed(),
        ));
    }
}
