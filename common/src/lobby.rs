use alloc::{vec, vec::Vec};

use alloy_primitives::{uint, Address, U32};
use alloy_sol_types::sol;
use stylus_sdk::{
    alloy_primitives::U256,
    keccak_const,
    prelude::*,
    storage::{StorageAddress, StorageBytes, StorageMap, StorageString, StorageU256, StorageVec},
};

#[storage]
pub struct TablePlayer {
    pub address: StorageAddress,
    pub chips_remain: StorageU256,
    pub annonce_public_key: StorageBytes,
}

#[storage]
pub struct Table {
    pub owner: StorageAddress,
    pub id: StorageU256,
    pub flags: StorageU256,
    pub name: StorageString,
    pub buy_in: StorageU256,
    pub players: StorageVec<TablePlayer>,
    pub total_buyin: StorageU256,
    pub current_hand: StorageU256,
    pub hand_start_ready_count: StorageU256,
    pub hand_start_ready: StorageMap<Address, StorageU256>,
}

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
    pub facets: PrivatePokerFacetAddresses,
}

#[storage]
pub struct PrivatePokerFacetAddresses {
    pub lobby: StorageAddress,
    pub table: StorageAddress,
    pub hand: StorageAddress,
    pub spectate: StorageAddress,
}

sol! {
    interface IPrivatePokerLobbyFacet {
        function setChipToken(address chip_token) external;
        function addLobby(uint256 id, uint256 game_type, uint256 flags, string name) external;
        function removeLobby(uint256 id) external;
    }

    interface IPrivatePokerTableFacet {
        function createTable(uint256 lobby_id, uint256 table_id, string name, uint256 buy_in, uint256 num_players, bytes annonce_public_key) external;
        function joinTable(uint256 lobby_id, uint256 table_id, bytes annonce_public_key) external;
        function removeTable(uint256 lobby_id, uint256 table_id) external;
    }

    interface IPrivatePokerHandFacet {
        function startHand(uint256 lobby_id, uint256 table_id) external;
    }

    interface IPrivatePokerSpectateFacet {
        function getLobbyCount() external view returns (uint256);
        function getLobbyAt(uint256 index) external view returns (bytes);
        function getTableCount(uint256 lobby_id) external view returns (uint256);
        function getTablesRange(uint256 lobby_id, uint256 offset, uint256 count) external view returns (bytes[]);
        function getLobbyById(uint256 lobby_id) external view returns (bytes);
        function getTableDetail(uint256 lobby_id, uint256 table_id) external view returns (bytes);
        function getPlayerTables(address player) external view returns (uint256[]);
    }

    interface IPrivatePokerSignal {
        function send_signal(uint256 lobby_id, uint256 table_id, address[] recipients, bytes[] encrypted_data) external;
    }

    struct LobbyInfo {
        uint256 lobby_id;
        uint256 lobby_game_type;
        uint256 lobby_flags;
        uint256 lobby_table_count;
        uint256 lobby_player_count;
        uint256 lobby_total_volume;
        string lobby_name;
    }

    struct TableInfo {
        uint256 table_id;
        uint256 table_flags;
        uint256 table_buyin;
        uint256 table_player_count;
        uint256 table_total_buyin;
        string table_name;
    }

    struct TablePlayerInfo {
        address player_address;
        uint256 player_chips;
        bytes player_annonce_public_key;
    }

    struct TableDetail {
        TableInfo info;
        TablePlayerInfo[] players;
    }

    event HandshakeSignal(address sender, uint256 lobby_id, uint256 table_id, address[] recipients, bytes[] encrypted_data);
    event HandStarted(uint256 lobby_id, uint256 table_id, uint256 seat_number, uint256 remain_count);
    event ChipTokenSet(address chip_token);
    event ChipsPaidOut(address recipient, uint256 amount);
    event LobbyCreated(uint256 id, string name);
    event TableCreated(uint256 id, uint256 lobby_id, string name, uint256 buy_in);
    event PlayerJoined(address player_address, uint256 lobby_id, uint256 table_id, string player_name, uint256 player_chips);
}

sol_interface! {
    interface IPokerChips {
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
    }
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

pub fn small_blind_for_buy_in(buy_in: U256) -> U256 {
    let hundred = U256::from(100);
    let blind = buy_in / hundred;
    if blind == U256::ZERO && buy_in > U256::ZERO {
        U256::ONE
    } else {
        blind
    }
}

pub fn clear_table(table: &mut Table) {
    table.owner.erase();
    table.id.erase();
    table.flags.erase();
    table.name.erase();
    table.buy_in.erase();
    table.total_buyin.erase();
    table.current_hand.erase();
    table.hand_start_ready_count.erase();
    unsafe {
        table.players.set_len(0);
    }
}
