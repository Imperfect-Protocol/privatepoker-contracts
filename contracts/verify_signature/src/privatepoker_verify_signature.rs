use alloc::vec::Vec;

use alloy_primitives::{Keccak256, B256};
use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve};
use bls12_381::{Bls12, G1Affine, G1Projective, G2Affine};
use digest::generic_array::typenum::U64;
use digest::Output;
use pairing::{
    group::{Curve, Group},
    MultiMillerLoop,
};

use stylus_sdk::{abi::Bytes, prelude::*};

#[storage]
#[entrypoint]
pub struct PrivatePokerVerifySignature;

#[public]
impl PrivatePokerVerifySignature {
    pub fn verify_signature(
        &mut self,
        digest: Bytes,
        aggregate_public_key: Bytes,
        aggregate_signature: Bytes,
    ) -> Result<bool, Vec<u8>> {
        if digest.len() != DIGEST_LEN {
            return Err(b"INVALID_DIGEST_LENGTH")?;
        }
        if aggregate_public_key.len() != G2AFFINE_COMPRESSED_LEN {
            return Err(b"INVALID_AGGREGATE_PUBLIC_KEY_LENGTH")?;
        }
        if aggregate_signature.len() != G1AFFINE_COMPRESSED_LEN {
            return Err(b"INVALID_AGGREGATE_SIGNATURE_LENGTH")?;
        }

        Ok(verify_signature_inner(
            &digest.0,
            &aggregate_public_key.0,
            &aggregate_signature.0,
        ))
    }
}

fn verify_signature_inner(
    digest: &[u8],
    aggregate_public_key: &[u8],
    aggregate_signature: &[u8],
) -> bool {
    let Ok(pk) = make_g2_from_compressed_slice(aggregate_public_key) else {
        return false;
    };
    let Ok(sig) = make_g1_from_compressed_slice(aggregate_signature) else {
        return false;
    };

    let h = hash_to_curve(digest).to_affine();

    // e(sig, G1) * e(h, -PK) == 1
    // Using BLS12-381 standard pairing check
    let is_valid = Bls12::multi_miller_loop(&[
        (&sig, &G2Affine::generator().into()),
        (&G1Affine::from(h), &(-pk).into()),
    ])
    .final_exponentiation()
    .is_identity();

    is_valid.into()
}

pub const DIGEST_LEN: usize = 32;
pub const G1AFFINE_COMPRESSED_LEN: usize = 48;
pub const G2AFFINE_COMPRESSED_LEN: usize = 96;

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

pub struct Keccak256Hash(Keccak256);

impl digest::BlockInput for Keccak256Hash {
    type BlockSize = U64;
}

impl digest::Digest for Keccak256Hash {
    type OutputSize = U64;

    fn new() -> Self {
        Self(Keccak256::default())
    }

    fn output_size() -> usize {
        B256::len_bytes()
    }

    fn chain(mut self, data: impl AsRef<[u8]>) -> Self
    where
        Self: Sized,
    {
        self.0.update(data);
        self
    }
    fn update(&mut self, data: impl AsRef<[u8]>) {
        self.0.update(data);
    }

    fn finalize(self) -> Output<Self> {
        let res = self.0.finalize();
        let mut arr = digest::generic_array::GenericArray::default();
        #[cfg(test)]
        arr[..32].copy_from_slice(&res.0);
        #[cfg(not(test))]
        arr.copy_from_slice(&res.0);
        arr
    }

    fn reset(&mut self) {
        unimplemented!()
    }

    fn digest(_data: &[u8]) -> digest::generic_array::GenericArray<u8, Self::OutputSize> {
        unimplemented!()
    }

    fn finalize_reset(&mut self) -> Output<Self> {
        unimplemented!()
    }
}

pub fn hash_to_curve(message: &[u8]) -> G1Projective {
    let cs = b"BLS_SIG_BLS12381G2_XMD:KECCAK-256_SSWU_RO_";
    <G1Projective as HashToCurve<ExpandMsgXmd<Keccak256Hash>>>::hash_to_curve(message, cs)
}

#[cfg(test)]
mod tests {
    use super::{
        hash_to_curve, make_g1_from_compressed_slice, make_g2_from_compressed_slice,
        verify_signature_inner,
    };
    use bls12_381::{G1Affine, G1Projective, G2Affine, Scalar};
    use pairing::group::Curve;

    #[test]
    fn decodes_valid_compressed_points() {
        assert!(make_g1_from_compressed_slice(&G1Affine::generator().to_compressed()).is_ok());
        assert!(make_g2_from_compressed_slice(&G2Affine::generator().to_compressed()).is_ok());
    }

    #[test]
    fn hash_to_curve_matches_frontend_fixture() {
        let mut digest = [0u8; 32];
        digest[31] = 7;
        assert!(!bool::from(
            hash_to_curve(&digest).eq(&G1Projective::identity())
        ));
        assert_eq!(
            hex::encode(hash_to_curve(&digest).to_affine().to_compressed()),
            "b9eac4142eccb8241b24b3d2c3f7c658010e2378d95d492d1a8a8e3b50fae94d6d9bbf80c40b476ea97037004d2c1d04"
        );
    }

    #[test]
    fn verifies_hash_to_curve_signature() {
        let mut digest = [0u8; 32];
        digest[31] = 7;
        let signing_key = Scalar::from(11u64);
        let public_key = (G2Affine::generator() * signing_key).to_affine();
        let signature = (hash_to_curve(&digest) * signing_key).to_affine();

        assert!(verify_signature_inner(
            &digest,
            &public_key.to_compressed(),
            &signature.to_compressed()
        ));

        digest[31] = 8;
        assert!(!verify_signature_inner(
            &digest,
            &public_key.to_compressed(),
            &signature.to_compressed()
        ));
    }
}
