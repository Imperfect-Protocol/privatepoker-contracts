use alloc::{vec, vec::Vec};

use stylus_sdk::{prelude::*, storage::StorageAddress};

use super::slots::PRIVATE_POKER_CHIPS_SLOT;
use crate::{erc20, storage::StorageSlot};

#[storage]
pub struct PrivatePokerChipsStorage {
    pub token: erc20::Erc20Storage,
    pub cashier: StorageAddress,
    pub lobby: StorageAddress,
    pub account: StorageAddress,
}

impl PrivatePokerChipsStorage {
    #[inline]
    pub fn storage_slot() -> PrivatePokerChipsStorage {
        StorageSlot::get_slot::<PrivatePokerChipsStorage>(PRIVATE_POKER_CHIPS_SLOT)
    }
}
