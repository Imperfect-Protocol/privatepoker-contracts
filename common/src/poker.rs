use alloc::vec::Vec;

use stylus_sdk::alloy_primitives::U256;

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
