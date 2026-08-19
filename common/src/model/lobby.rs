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
    pub open_table_ids: StorageVec<StorageU256>,
    pub running_table_ids: StorageVec<StorageU256>,
    pub completed_table_ids: StorageVec<StorageU256>,
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

impl Lobby {
    #[inline]
    pub fn active_table_count(&self) -> usize {
        self.open_table_ids.len() + self.running_table_ids.len()
    }

    #[inline]
    pub fn active_table_id_at(&self, index: usize) -> Option<U256> {
        let open_count = self.open_table_ids.len();
        if index < open_count {
            self.open_table_ids.get(index)
        } else {
            self.running_table_ids.get(index - open_count)
        }
    }

    #[inline]
    pub fn add_open_table(&mut self, table_id: U256) {
        self.open_table_ids.push(table_id);
    }

    #[inline]
    pub fn mark_table_running(&mut self, table_id: U256) -> bool {
        if Self::remove_table_id_from(&mut self.open_table_ids, table_id) {
            self.running_table_ids.push(table_id);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn mark_table_completed(&mut self, table_id: U256) -> bool {
        if Self::remove_table_id_from(&mut self.running_table_ids, table_id)
            || Self::remove_table_id_from(&mut self.open_table_ids, table_id)
        {
            self.completed_table_ids.push(table_id);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn remove_table_id(&mut self, table_id: U256) -> bool {
        Self::remove_table_id_from(&mut self.open_table_ids, table_id)
            || Self::remove_table_id_from(&mut self.running_table_ids, table_id)
            || Self::remove_table_id_from(&mut self.completed_table_ids, table_id)
    }

    #[inline]
    fn remove_table_id_from(ids: &mut StorageVec<StorageU256>, table_id: U256) -> bool {
        let len = ids.len();
        for index in 0..len {
            if ids.get(index).unwrap() == table_id {
                if index < len - 1 {
                    let last_val = ids.get(len - 1).unwrap();
                    ids.setter(index).unwrap().set(last_val);
                }
                ids.pop();
                return true;
            }
        }
        false
    }
}
