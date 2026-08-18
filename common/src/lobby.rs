use alloc::{vec, vec::Vec};

use alloy_primitives::{uint, Address, U32};
use stylus_sdk::{
    alloy_primitives::U256,
    keccak_const,
    prelude::*,
    storage::{
        StorageAddress, StorageBytes, StorageMap, StorageString, StorageU256, StorageU8, StorageVec,
    },
};

use super::erc20;
pub use super::interfaces::*;

#[storage]
pub struct TablePlayer {
    pub address: StorageAddress,
    pub chips_remain: StorageU256,
    pub annonce_public_key: StorageBytes,
    pub operator: StorageAddress,
}

#[storage]
pub struct Hand {
    pub pot_size: StorageU256,
    pub pot_split: StorageVec<StorageU256>,
    pub digest: StorageBytes,
    pub aggregate_signature: StorageBytes,
}

#[storage]
pub struct Table {
    pub owner: StorageAddress,
    pub id: StorageU256,
    pub flags: StorageU256,
    pub name: StorageString,
    pub buy_in: StorageU256,
    pub aggregate_public_key: StorageBytes,
    pub players: StorageVec<TablePlayer>,
    pub total_buyin: StorageU256,
    pub current_hand: StorageU256,
    pub hands: StorageMap<U256, Hand>,
    pub hand_start_ready_count: StorageU256,
    pub hand_start_ready: StorageMap<Address, StorageU256>,
    pub public_key_ready_count: StorageMap<U256, StorageU256>,
    pub public_key_ready: StorageMap<Address, StorageU256>,
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
    pub account: StorageAddress,
    pub cashier: StorageAddress,
    pub chips: StorageAddress,
    pub settler: StorageAddress,
    pub aggregate_pub_key: StorageAddress,
    pub verify_signature: StorageAddress,
}

#[storage]
pub struct PrivatePokerChipsStorage {
    pub token: erc20::Erc20Storage,
    pub cashier: StorageAddress,
    pub lobby: StorageAddress,
    pub account: StorageAddress,
}

#[storage]
pub struct PrivatePokerCashierStorage {
    pub owner: StorageAddress,
    pub usdc: StorageAddress,
    pub chips: StorageAddress,
    pub accounted_assets: StorageU256,
}

#[storage]
pub struct PlayerAccount {
    pub exists: StorageU256,
    pub player_address: StorageAddress,
    pub operator: StorageAddress,
    pub annonce_public_key: StorageBytes,
    pub encrypted_profile: StorageBytes,
    pub subscription_tier: StorageU8,
    pub subscription_paid_at: StorageU256,
    pub subscription_expires_at: StorageU256,
}

#[storage]
pub struct PrivatePokerAccountsStorage {
    pub owner: StorageAddress,
    pub usdc: StorageAddress,
    pub chips: StorageAddress,
    pub cashier: StorageAddress,
    pub accounts: StorageMap<Address, PlayerAccount>,
    pub operator_players: StorageMap<Address, StorageAddress>,
    pub players: StorageVec<StorageAddress>,
}

use super::storage::StorageSlot;

pub const VERSION_NUMBER: U32 = uint!(1_U32);

pub const MAIN_LOBBY_SLOT: U256 = {
    const HASH: [u8; 32] = keccak_const::Keccak256::new()
        .update(b"PrivatePoker.MainLobby")
        .finalize();
    U256::from_be_bytes(HASH).wrapping_sub(uint!(1_U256))
};

pub const PRIVATE_POKER_CHIPS_SLOT: U256 = {
    const HASH: [u8; 32] = keccak_const::Keccak256::new()
        .update(b"PrivatePoker.Chips")
        .finalize();
    U256::from_be_bytes(HASH).wrapping_sub(uint!(1_U256))
};

pub const PRIVATE_POKER_CASHIER_SLOT: U256 = {
    const HASH: [u8; 32] = keccak_const::Keccak256::new()
        .update(b"PrivatePoker.Cashier")
        .finalize();
    U256::from_be_bytes(HASH).wrapping_sub(uint!(1_U256))
};

pub const PRIVATE_POKER_ACCOUNTS_SLOT: U256 = {
    const HASH: [u8; 32] = keccak_const::Keccak256::new()
        .update(b"PrivatePoker.Accounts")
        .finalize();
    U256::from_be_bytes(HASH).wrapping_sub(uint!(1_U256))
};

impl MainLobby {
    pub fn storage_slot() -> MainLobby {
        StorageSlot::get_slot::<MainLobby>(MAIN_LOBBY_SLOT)
    }
}

impl PrivatePokerChipsStorage {
    pub fn storage_slot() -> PrivatePokerChipsStorage {
        StorageSlot::get_slot::<PrivatePokerChipsStorage>(PRIVATE_POKER_CHIPS_SLOT)
    }
}

impl PrivatePokerCashierStorage {
    pub fn storage_slot() -> PrivatePokerCashierStorage {
        StorageSlot::get_slot::<PrivatePokerCashierStorage>(PRIVATE_POKER_CASHIER_SLOT)
    }
}

impl PrivatePokerAccountsStorage {
    pub fn storage_slot() -> PrivatePokerAccountsStorage {
        StorageSlot::get_slot::<PrivatePokerAccountsStorage>(PRIVATE_POKER_ACCOUNTS_SLOT)
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

pub fn clear_table_player(player: &mut TablePlayer) {
    player.address.erase();
    player.chips_remain.erase();
    player.annonce_public_key.erase();
    player.operator.erase();
}

pub fn clear_hand(hand: &mut Hand) {
    hand.pot_size.erase();
    hand.pot_split.erase();
    hand.digest.erase();
    hand.aggregate_signature.erase();
}

pub fn clear_table(table: &mut Table) {
    let player_count = table.players.len();
    for index in 0..player_count {
        if let Some(player) = table.players.get(index) {
            let player_address = player.address.get();
            table.hand_start_ready.delete(player_address);
            table.public_key_ready.delete(player_address);
        }
        if let Some(mut player) = table.players.setter(index) {
            clear_table_player(&mut player);
        }
    }

    let current_hand = table.current_hand.get();
    let last_hand_to_clear = if current_hand == U256::ZERO {
        U256::ONE
    } else {
        current_hand
    };
    let mut hand_id = U256::ONE;
    while hand_id <= last_hand_to_clear {
        let mut hand = table.hands.setter(hand_id);
        clear_hand(&mut hand);
        table.public_key_ready_count.delete(hand_id);
        hand_id += U256::ONE;
    }

    table.owner.erase();
    table.id.erase();
    table.flags.erase();
    table.name.erase();
    table.buy_in.erase();
    table.aggregate_public_key.erase();
    table.total_buyin.erase();
    table.current_hand.erase();
    table.hand_start_ready_count.erase();
    unsafe {
        table.players.set_len(0);
    }
}
