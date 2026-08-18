use alloc::{vec, vec::Vec};

use alloy_primitives::Address;
use stylus_sdk::{
    alloy_primitives::U256,
    prelude::*,
    storage::{StorageAddress, StorageMap, StorageString, StorageU256, StorageVec},
};

use super::{slots::MAIN_LOBBY_SLOT, table::Table};
use crate::storage::StorageSlot;

#[storage]
pub struct Lobby {
    pub id: StorageU256,
    pub game_type: StorageU256,
    pub flags: StorageU256,
    pub name: StorageString,
    pub table_ids: StorageVec<StorageU256>,
    pub tables: StorageMap<U256, Table>,
    pub total_volume: StorageU256,
    pub total_players: StorageU256,
}

#[storage]
pub struct MainLobby {
    pub owner: StorageAddress,
    pub lobby_ids: StorageVec<StorageU256>,
    pub lobbies: StorageMap<U256, Lobby>,
    pub player_tables: StorageMap<Address, StorageVec<StorageU256>>,
    pub chip_token: StorageAddress,
}

impl MainLobby {
    #[inline]
    pub fn storage_slot() -> MainLobby {
        StorageSlot::get_slot::<MainLobby>(MAIN_LOBBY_SLOT)
    }
}
