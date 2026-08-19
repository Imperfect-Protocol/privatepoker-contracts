use alloc::{vec, vec::Vec};

use alloy_primitives::{Address, Bytes as AlloyBytes};
use alloy_sol_types::{SolCall, SolValue};
use privatepoker_common::{
    calls::ContractCalls,
    interfaces::{
        IPrivatePokerAggregatePubKeyFacet, IPrivatePokerHashToCurve, IPrivatePokerSettlerFacet,
        IPrivatePokerVerifySignature,
    },
    poker::{G1AFFINE_COMPRESSED_LEN, G2AFFINE_COMPRESSED_LEN},
};
use stylus_sdk::{abi::Bytes, prelude::*, storage::StorageAddress};

const SELECTOR_LEN: usize = 4;
const ABI_WORD_LEN: usize = 32;
const AGGREGATE_PUBLIC_KEY_ARG_INDEX: usize = 2;
const SETTLE_AGGREGATE_PUBLIC_KEY_ARG_INDEX: usize = 7;

#[storage]
#[entrypoint]
pub struct PrivatePokerSignatory {
    hash_to_curve: StorageAddress,
    verify_signature: StorageAddress,
}

#[public]
impl PrivatePokerSignatory {
    #[constructor]
    fn constructor(
        &mut self,
        hash_to_curve: Address,
        verify_signature: Address,
    ) -> Result<(), Vec<u8>> {
        if hash_to_curve == Address::ZERO || verify_signature == Address::ZERO {
            return Err(vec![1]);
        }
        self.hash_to_curve.set(hash_to_curve);
        self.verify_signature.set(verify_signature);
        Ok(())
    }

    pub fn verify_signed_calldata(&mut self, signed_calldata: Bytes) -> Result<bool, Vec<u8>> {
        if signed_calldata.len() < SELECTOR_LEN + G1AFFINE_COMPRESSED_LEN {
            return Err(vec![2]);
        }
        let signature_offset = signed_calldata.len() - G1AFFINE_COMPRESSED_LEN;
        let actual_calldata = &signed_calldata[..signature_offset];
        let signature = &signed_calldata[signature_offset..];
        if actual_calldata.len() < SELECTOR_LEN {
            return Err(vec![3]);
        }

        let mut selector = [0u8; SELECTOR_LEN];
        selector.copy_from_slice(&actual_calldata[..SELECTOR_LEN]);
        let public_key_arg_index = match selector {
            IPrivatePokerAggregatePubKeyFacet::setTableAggregatePublicKeyCall::SELECTOR => {
                AGGREGATE_PUBLIC_KEY_ARG_INDEX
            }
            IPrivatePokerSettlerFacet::settleHandCall::SELECTOR => {
                SETTLE_AGGREGATE_PUBLIC_KEY_ARG_INDEX
            }
            _ => return Err(vec![4]),
        };

        let Some(aggregate_public_key) = dynamic_bytes_arg(actual_calldata, public_key_arg_index)
        else {
            return Err(vec![5]);
        };
        if aggregate_public_key.len() != G2AFFINE_COMPRESSED_LEN {
            return Err(vec![6]);
        }

        let hashed_message = self.hash_to_curve(actual_calldata)?;
        if hashed_message.len() != G1AFFINE_COMPRESSED_LEN {
            return Err(vec![7]);
        }

        self.verify_signature(&hashed_message, aggregate_public_key, signature)
    }
}

impl PrivatePokerSignatory {
    fn hash_to_curve(&mut self, actual_calldata: &[u8]) -> Result<Vec<u8>, Vec<u8>> {
        let hash_to_curve = self.hash_to_curve.get();
        if hash_to_curve == Address::ZERO {
            return Err(vec![8]);
        }

        let call = IPrivatePokerHashToCurve::toCurveCall {
            digest: actual_calldata.to_vec().into(),
        };
        let output = self.call_bytes(hash_to_curve, &call.abi_encode(), &[9])?;
        AlloyBytes::abi_decode(&output, true)
            .map(|bytes| bytes.to_vec())
            .map_err(|_| vec![10])
    }

    fn verify_signature(
        &mut self,
        hashed_message: &[u8],
        aggregate_public_key: &[u8],
        signature: &[u8],
    ) -> Result<bool, Vec<u8>> {
        let verify_signature = self.verify_signature.get();
        if verify_signature == Address::ZERO {
            return Err(vec![11]);
        }

        let call = IPrivatePokerVerifySignature::verifySignatureCall {
            hashed_message: hashed_message.to_vec().into(),
            aggregate_public_key: aggregate_public_key.to_vec().into(),
            aggregate_signature: signature.to_vec().into(),
        };
        let output = self.call_bytes(verify_signature, &call.abi_encode(), &[12])?;
        bool::abi_decode(&output, true).map_err(|_| vec![13])
    }
}

fn dynamic_bytes_arg(calldata: &[u8], arg_index: usize) -> Option<&[u8]> {
    let args = calldata.get(SELECTOR_LEN..)?;
    let head_pos = arg_index.checked_mul(ABI_WORD_LEN)?;
    let offset = read_usize_word(args.get(head_pos..head_pos.checked_add(ABI_WORD_LEN)?)?)?;
    let len_pos = offset;
    let len = read_usize_word(args.get(len_pos..len_pos.checked_add(ABI_WORD_LEN)?)?)?;
    let bytes_start = len_pos.checked_add(ABI_WORD_LEN)?;
    let bytes_end = bytes_start.checked_add(len)?;
    args.get(bytes_start..bytes_end)
}

fn read_usize_word(word: &[u8]) -> Option<usize> {
    if word.len() != ABI_WORD_LEN || word[..24].iter().any(|byte| *byte != 0) {
        return None;
    }

    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&word[24..]);
    Some(u64::from_be_bytes(bytes) as usize)
}

#[cfg(test)]
mod tests {
    use super::{dynamic_bytes_arg, AGGREGATE_PUBLIC_KEY_ARG_INDEX};
    use alloy_primitives::{Bytes as AlloyBytes, U256};
    use alloy_sol_types::SolCall;
    use privatepoker_common::{
        interfaces::IPrivatePokerAggregatePubKeyFacet, poker::G2AFFINE_COMPRESSED_LEN,
    };

    #[test]
    fn extracts_public_key_arg_by_raw_offsets() {
        let public_key = vec![7u8; G2AFFINE_COMPRESSED_LEN];
        let call = IPrivatePokerAggregatePubKeyFacet::setTableAggregatePublicKeyCall {
            lobby_id: U256::from(1),
            table_id: U256::from(2),
            aggregate_public_key: AlloyBytes::copy_from_slice(&public_key),
        };
        let calldata = call.abi_encode();

        assert_eq!(
            dynamic_bytes_arg(&calldata, AGGREGATE_PUBLIC_KEY_ARG_INDEX),
            Some(public_key.as_slice())
        );
    }
}
