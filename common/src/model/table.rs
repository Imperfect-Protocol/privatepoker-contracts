use alloc::{vec, vec::Vec};

use alloy_primitives::Address;
use stylus_sdk::{
    alloy_primitives::U256,
    prelude::*,
    storage::{StorageAddress, StorageBytes, StorageMap, StorageString, StorageU256, StorageVec},
};

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

impl TablePlayer {
    #[inline]
    pub fn clear(&mut self) {
        self.address.erase();
        self.chips_remain.erase();
        self.annonce_public_key.erase();
        self.operator.erase();
    }
}

impl Hand {
    #[inline]
    pub fn clear(&mut self) {
        self.pot_size.erase();
        self.pot_split.erase();
        self.digest.erase();
        self.aggregate_signature.erase();
    }
}

impl Table {
    #[inline]
    pub fn clear(&mut self) {
        let player_count = self.players.len();
        for index in 0..player_count {
            if let Some(player) = self.players.get(index) {
                let player_address = player.address.get();
                self.hand_start_ready.delete(player_address);
                self.public_key_ready.delete(player_address);
            }
            if let Some(mut player) = self.players.setter(index) {
                player.clear();
            }
        }

        let current_hand = self.current_hand.get();
        let last_hand_to_clear = if current_hand == U256::ZERO {
            U256::ONE
        } else {
            current_hand
        };
        let mut hand_id = U256::ONE;
        while hand_id <= last_hand_to_clear {
            let mut hand = self.hands.setter(hand_id);
            hand.clear();
            self.public_key_ready_count.delete(hand_id);
            hand_id += U256::ONE;
        }

        self.owner.erase();
        self.id.erase();
        self.flags.erase();
        self.name.erase();
        self.buy_in.erase();
        self.aggregate_public_key.erase();
        self.total_buyin.erase();
        self.current_hand.erase();
        self.hand_start_ready_count.erase();
        unsafe {
            self.players.set_len(0);
        }
    }

    #[inline]
    pub fn has_operator(&self, operator: Address) -> bool {
        for index in 0..self.players.len() {
            let Some(player) = self.players.get(index) else {
                return false;
            };
            if player.operator.get() == operator {
                return true;
            }
        }
        false
    }
}

#[inline]
pub fn small_blind_for_buy_in(buy_in: U256) -> U256 {
    let hundred = U256::from(100);
    let blind = buy_in / hundred;
    if blind == U256::ZERO && buy_in > U256::ZERO {
        U256::ONE
    } else {
        blind
    }
}

#[inline]
pub fn clear_table_player(player: &mut TablePlayer) {
    player.clear();
}

#[inline]
pub fn clear_hand(hand: &mut Hand) {
    hand.clear();
}

#[inline]
pub fn clear_table(table: &mut Table) {
    table.clear();
}
