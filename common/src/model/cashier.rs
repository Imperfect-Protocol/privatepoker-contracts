use alloc::{vec, vec::Vec};

use stylus_sdk::{
    prelude::*,
    storage::{StorageAddress, StorageU256},
};

use super::slots::PRIVATE_POKER_CASHIER_SLOT;
use crate::storage::StorageSlot;

#[storage]
pub struct PrivatePokerCashierStorage {
    pub owner: StorageAddress,
    pub usdc: StorageAddress,
    pub chips: StorageAddress,
    pub accounted_assets: StorageU256,
}

impl PrivatePokerCashierStorage {
    #[inline]
    pub fn storage_slot() -> PrivatePokerCashierStorage {
        StorageSlot::get_slot::<PrivatePokerCashierStorage>(PRIVATE_POKER_CASHIER_SLOT)
    }
}
