use alloc::vec::Vec;

use alloy_primitives::{Keccak256, B256};
use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve};
use bls12_381::{Bls12, G1Affine, G2Affine, G2Projective, Gt};
use digest::generic_array::typenum::U64;
use digest::Output;
use pairing::{group::Curve, MultiMillerLoop};

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
        if aggregate_public_key.len() != G1AFFINE_COMPRESSED_LEN {
            return Err(b"INVALID_AGGREGATE_PUBLIC_KEY_LENGTH")?;
        }
        if aggregate_signature.len() != G2AFFINE_COMPRESSED_LEN {
            return Err(b"INVALID_AGGREGATE_SIGNATURE_LENGTH")?;
        }

        Ok(verify(
            &digest.0,
            &aggregate_public_key.0,
            &aggregate_signature.0,
        ))
    }
}

pub fn verify(digest: &[u8], aggregate_public_key: &[u8], aggregate_signature: &[u8]) -> bool {
    let Ok(pk) = make_g1_from_compressed_slice(aggregate_public_key) else {
        return false;
    };
    let Ok(sig) = make_g2_from_compressed_slice(aggregate_signature) else {
        return false;
    };

    let h = hash_to_curve_2(digest);
    let h_prepared = h.to_affine().into();
    let signature_prepared = sig.into();
    let gen_neg = -G1Affine::generator();

    let product = Bls12::multi_miller_loop(&[(&pk, &h_prepared), (&gen_neg, &signature_prepared)])
        .final_exponentiation();

    product == Gt::identity()
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

pub fn hash_to_curve_2(message: &[u8]) -> G2Projective {
    let cs = b"BLS_SIG_BLS12381G1_XMD:KECCAK-256_SSWU_RO_";
    <G2Projective as HashToCurve<ExpandMsgXmd<Keccak256Hash>>>::hash_to_curve(message, cs)
}

#[cfg(test)]
mod tests {
    use super::{
        hash_to_curve_2, make_g1_from_compressed_slice, make_g2_from_compressed_slice, verify,
        PrivatePokerVerifySignature,
    };
    use bls12_381::{G1Affine, G2Affine, G2Projective, Scalar};
    use pairing::group::Curve;
    use stylus_sdk::abi::Bytes;
    use stylus_sdk::testing::TestVM;

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
            hash_to_curve_2(&digest).eq(&G2Projective::identity())
        ));
        assert_eq!(
            hex::encode(hash_to_curve_2(&digest).to_affine().to_compressed()),
            "982e8197ed181571d1fb7c8674a6190dd79ea4ca49206301d9acc81e058a89c68ffb2279ae6a0cd5515e8b82dbf4e72001b4c90ee2ea0a3e3a86867cee118c77eb04c7bae3c6ee2cab7c269bef6c6d5cc24461b8f1724b1da16f368fef6d1818"
        );
    }

    #[test]
    fn verifies_hash_to_curve_signature() {
        let mut digest = [0u8; 32];
        digest[31] = 7;
        let signing_key = Scalar::from(11u64);
        let public_key = (G1Affine::generator() * signing_key).to_affine();
        let signature = (hash_to_curve_2(&digest) * signing_key).to_affine();

        assert!(verify(
            &digest,
            &public_key.to_compressed(),
            &signature.to_compressed()
        ));

        digest[31] = 8;
        assert!(!verify(
            &digest,
            &public_key.to_compressed(),
            &signature.to_compressed()
        ));
    }

    #[test]
    fn contract_entrypoint_verifies_hash_to_curve_signature() {
        let vm = TestVM::default();
        let mut contract = PrivatePokerVerifySignature::from(&vm);
        let mut digest = [0u8; 32];
        digest[31] = 7;
        let signing_key = Scalar::from(11u64);
        let public_key = (G1Affine::generator() * signing_key).to_affine();
        let signature = (hash_to_curve_2(&digest) * signing_key).to_affine();

        assert_eq!(
            contract
                .verify_signature(
                    Bytes::from(digest.to_vec()),
                    Bytes::from(public_key.to_compressed().to_vec()),
                    Bytes::from(signature.to_compressed().to_vec()),
                )
                .unwrap(),
            true
        );

        digest[31] = 8;
        assert_eq!(
            contract
                .verify_signature(
                    Bytes::from(digest.to_vec()),
                    Bytes::from(public_key.to_compressed().to_vec()),
                    Bytes::from(signature.to_compressed().to_vec()),
                )
                .unwrap(),
            false
        );
    }
}
