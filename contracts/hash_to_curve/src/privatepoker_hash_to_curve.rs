use alloc::vec::Vec;

use pairing::group::Curve;
use stylus_sdk::{abi::Bytes, prelude::*};

use super::hash_to_curve;

#[storage]
#[entrypoint]
pub struct PrivatePokerHashToCurve;

#[public]
impl PrivatePokerHashToCurve {
    pub fn to_curve(&mut self, digest: Bytes) -> Bytes {
        let h = hash_to_curve::hash_to_curve(&digest.0).to_affine();

        let c = h.to_compressed();

        Bytes::from(c.to_vec())
    }
}
