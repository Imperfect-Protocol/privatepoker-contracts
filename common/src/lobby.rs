use alloc::{vec, vec::Vec};

use alloy_primitives::{uint, U32};
use alloy_sol_types::sol;
use stylus_sdk::{
    alloy_primitives::U256,
    keccak_const,
    prelude::*,
    storage::{StorageAddress, StorageBool, StorageMap, StorageString, StorageU256, StorageVec},
};

#[storage]
pub struct Table {
    pub owner: StorageAddress,
    pub id: StorageU256,
    pub name: StorageString,
    pub buy_in: StorageU256,
    pub is_active: StorageBool,
    pub annonce_public_key: StorageString,
}

#[storage]
pub struct Lobby {
    pub id: StorageU256,
    pub name: StorageString,
    pub table_ids: StorageVec<StorageU256>,
    pub tables: StorageMap<U256, Table>,
}

#[storage]
pub struct MainLobby {
    pub owner: StorageAddress,
    pub lobby_ids: StorageVec<StorageU256>,
    pub lobbies: StorageMap<U256, Lobby>,
}

sol! {
    struct LobbyInfo {
        uint256 lobby_id;
        string lobby_name;
        uint256 table_count;
    }

    struct TableInfo {
        uint256 table_id;
        string table_name;
        uint256 table_buyin;
    }

    event HandshakeSignal(address sender, uint256 lobby_id, uint256 table_id, address recipient, bytes encrypted_data);
    event LobbyCreated(uint256 id, string name);
    event TableCreated(uint256 id, string name, uint256 buy_in);
}

use super::storage::StorageSlot;

pub const VERSION_NUMBER: U32 = uint!(1_U32);

pub const MAIN_LOBBY_SLOT: U256 = {
    const HASH: [u8; 32] = keccak_const::Keccak256::new()
        .update(b"PrivatePoker.MainLobby")
        .finalize();
    U256::from_be_bytes(HASH).wrapping_sub(uint!(1_U256))
};

impl MainLobby {
    pub fn storage_slot() -> MainLobby {
        StorageSlot::get_slot::<MainLobby>(MAIN_LOBBY_SLOT)
    }
}
