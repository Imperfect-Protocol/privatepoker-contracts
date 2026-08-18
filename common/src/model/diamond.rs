use alloc::{vec, vec::Vec};

use stylus_sdk::{prelude::*, storage::StorageAddress};

use super::slots::PRIVATE_POKER_DIAMOND_SLOT;
use crate::storage::StorageSlot;

#[storage]
pub struct PrivatePokerDiamond {
    pub lobby: StorageAddress,
    pub table: StorageAddress,
    pub hand: StorageAddress,
    pub spectate: StorageAddress,
    pub account: StorageAddress,
    pub cashier: StorageAddress,
    pub chips: StorageAddress,
    pub settler: StorageAddress,
    pub aggregate_pub_key: StorageAddress,
    pub verify_signature: StorageAddress,
}

impl PrivatePokerDiamond {
    #[inline]
    pub fn storage_slot() -> PrivatePokerDiamond {
        StorageSlot::get_slot::<PrivatePokerDiamond>(PRIVATE_POKER_DIAMOND_SLOT)
    }
}
