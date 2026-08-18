use alloc::vec::Vec;

use alloy_primitives::{Bytes as AlloyBytes, Keccak256};
use alloy_sol_types::SolValue;
use stylus_sdk::{abi::Bytes, alloy_primitives::U256};

pub const DIGEST_LEN: usize = 32;
pub const G1AFFINE_COMPRESSED_LEN: usize = 48;
pub const G2AFFINE_COMPRESSED_LEN: usize = 96;

#[inline]
pub fn checked_sum(values: &[U256]) -> Result<U256, Vec<u8>> {
    values.iter().try_fold(U256::ZERO, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| b"U256_OVERFLOW".to_vec())
    })
}

#[inline]
pub fn game_ended_winner_index(chips_balances: &[U256]) -> Option<usize> {
    let mut winner_index = None;
    for (index, balance) in chips_balances.iter().enumerate() {
        if *balance == U256::ZERO {
            continue;
        }
        if winner_index.is_some() {
            return None;
        }
        winner_index = Some(index);
    }
    winner_index
}

#[inline]
pub fn set_table_aggregate_public_key_digest(
    lobby_id: U256,
    table_id: U256,
    aggregate_public_key: Bytes,
) -> [u8; 32] {
    let encoded = (lobby_id, table_id, aggregate_public_key).abi_encode();
    let mut k = Keccak256::new();
    k.update(encoded);
    k.finalize().0
}

#[inline]
pub fn settlement_signature_digest(
    lobby_id: U256,
    table_id: U256,
    hand_id: U256,
    pot_size: U256,
    pot_split: &[U256],
    chips_balances: &[U256],
    digest: &[u8],
) -> [u8; 32] {
    let encoded = (
        lobby_id,
        table_id,
        hand_id,
        pot_size,
        pot_split.to_vec(),
        chips_balances.to_vec(),
        AlloyBytes::copy_from_slice(digest),
    )
        .abi_encode();

    let mut k = Keccak256::new();
    k.update(encoded);
    k.finalize().0
}
