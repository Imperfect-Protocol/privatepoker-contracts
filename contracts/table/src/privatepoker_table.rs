use alloc::{string::String, vec::Vec};

use alloy_primitives::Address;
use privatepoker_common::lobby::{clear_table, IPokerChips, MainLobby, PlayerJoined, TableCreated};
use stylus_sdk::{abi::Bytes, alloy_primitives::U256, prelude::*, stylus_core};

#[storage]
#[entrypoint]
pub struct PrivatePokerTable;

#[public]
impl PrivatePokerTable {
    pub fn create_table(
        &mut self,
        lobby_id: U256,
        table_id: U256,
        name: String,
        buy_in: U256,
        num_players: U256,
        player_address: Address,
        operator: Address,
        annonce_public_key: Bytes,
    ) -> Result<(), Vec<u8>> {
        if num_players < U256::from(2) {
            return Err(b"INVALID_NUM_PLAYERS")?;
        }
        let sender = self.vm().msg_sender();
        if sender != player_address && sender != operator {
            return Err(b"INVALID_PLAYER_OPERATOR")?;
        }
        self.collect_chip_buy_in(lobby_id, table_id, buy_in, false)?;

        let mut main_lobby = MainLobby::storage_slot();
        let mut lobby = main_lobby.lobbies.setter(lobby_id);

        if lobby.id.get() == U256::ZERO {
            return Err("LOBBY_NOT_FOUND".into());
        }

        let mut table = lobby.tables.setter(table_id);
        table.owner.set(player_address);
        table.id.set(table_id);
        table.flags.set(num_players);
        table.name.set_str(name.clone());
        table.buy_in.set(buy_in);
        table.total_buyin.set(buy_in);

        let mut new_player = table.players.grow();
        new_player.address.set(player_address);
        new_player.chips_remain.set(buy_in);
        new_player.annonce_public_key.set_bytes(annonce_public_key);
        new_player.operator.set(operator);

        lobby.table_ids.push(table_id);

        let total_players = lobby.total_players.get();
        lobby.total_players.set(total_players + U256::ONE);

        let total_volume = lobby.total_volume.get();
        lobby.total_volume.set(total_volume + buy_in);

        let mut player_tables = main_lobby.player_tables.setter(player_address);
        player_tables.grow().set(table_id);

        stylus_core::log(
            self.vm(),
            TableCreated {
                id: table_id,
                lobby_id,
                name,
                buy_in,
            },
        );
        stylus_core::log(
            self.vm(),
            PlayerJoined {
                player_address,
                lobby_id,
                table_id,
                player_name: String::new(),
                player_chips: buy_in,
            },
        );
        Ok(())
    }

    pub fn join_table(
        &mut self,
        lobby_id: U256,
        table_id: U256,
        player_address: Address,
        operator: Address,
        annonce_public_key: Bytes,
    ) -> Result<(), Vec<u8>> {
        let sender = self.vm().msg_sender();
        if sender != player_address && sender != operator {
            return Err(b"INVALID_PLAYER_OPERATOR")?;
        }
        let buy_in = self.get_join_buy_in(lobby_id, table_id)?;
        self.collect_chip_buy_in(lobby_id, table_id, buy_in, true)?;

        let mut main_lobby = MainLobby::storage_slot();

        let mut lobby = main_lobby.lobbies.setter(lobby_id);
        if lobby.id.get() == U256::ZERO {
            return Err("LOBBY_NOT_FOUND".into());
        }

        let mut table = lobby.tables.setter(table_id);
        if table.id.get() != table_id {
            return Err("TABLE_NOT_FOUND".into());
        }

        let num_players = table.players.len();
        let required_players = table.flags.get().to::<usize>();
        if required_players > 0 && num_players >= required_players {
            return Err("TABLE_FULL".into());
        }
        for i in 0..num_players {
            if table.players.get(i).unwrap().address.get() == player_address {
                return Err("ALREADY_SEATED".into());
            }
        }

        let mut new_player = table.players.grow();
        new_player.address.set(player_address);
        new_player.chips_remain.set(buy_in);
        new_player.annonce_public_key.set_bytes(annonce_public_key);
        new_player.operator.set(operator);

        let current_total_buyin = table.total_buyin.get();
        table.total_buyin.set(current_total_buyin + buy_in);

        let lobby_volume = lobby.total_volume.get();
        lobby.total_volume.set(lobby_volume + buy_in);

        let total_players = lobby.total_players.get();
        lobby.total_players.set(total_players + U256::ONE);

        let mut player_tables = main_lobby.player_tables.setter(player_address);
        player_tables.grow().set(table_id);

        stylus_core::log(
            self.vm(),
            PlayerJoined {
                player_address,
                lobby_id,
                table_id,
                player_name: String::new(),
                player_chips: buy_in,
            },
        );

        Ok(())
    }

    pub fn remove_table(&mut self, lobby_id: U256, table_id: U256) -> Result<(), Vec<u8>> {
        let mut main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();
        let mut lobby = main_lobby.lobbies.setter(lobby_id);
        if lobby.id.get() != lobby_id {
            return Err("LOBBY_NOT_FOUND".into());
        }

        let table = lobby.tables.getter(table_id);
        if table.owner.get() != sender && main_lobby.owner.get() != sender {
            return Err("UNAUTHORIZED".into());
        }

        let mut found = false;
        let len = lobby.table_ids.len();
        for i in 0..len {
            if lobby.table_ids.get(i).unwrap() == table_id {
                if i < len - 1 {
                    let last_val = lobby.table_ids.get(len - 1).unwrap();
                    lobby.table_ids.setter(i).unwrap().set(last_val);
                }
                lobby.table_ids.pop();
                found = true;
                break;
            }
        }

        if !found {
            return Err("TABLE_NOT_FOUND".into());
        }

        let mut table = lobby.tables.setter(table_id);
        clear_table(&mut table);

        Ok(())
    }
}

impl PrivatePokerTable {
    fn get_join_buy_in(&self, lobby_id: U256, table_id: U256) -> Result<U256, Vec<u8>> {
        let main_lobby = MainLobby::storage_slot();
        let lobby = main_lobby.lobbies.getter(lobby_id);
        if lobby.id.get() == U256::ZERO {
            return Err("LOBBY_NOT_FOUND".into());
        }

        let table = lobby.tables.getter(table_id);
        if table.id.get() != table_id {
            return Err("TABLE_NOT_FOUND".into());
        }

        Ok(table.buy_in.get())
    }

    fn collect_chip_buy_in(
        &mut self,
        lobby_id: U256,
        table_id: U256,
        buy_in: U256,
        table_must_exist: bool,
    ) -> Result<(), Vec<u8>> {
        if buy_in == U256::ZERO {
            return Ok(());
        }

        let main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();
        let lobby = main_lobby.lobbies.getter(lobby_id);
        if lobby.id.get() == U256::ZERO {
            return Err("LOBBY_NOT_FOUND".into());
        }

        let table = lobby.tables.getter(table_id);
        if table_must_exist {
            if table.id.get() != table_id {
                return Err("TABLE_NOT_FOUND".into());
            }

            let num_players = table.players.len();
            for i in 0..num_players {
                if table.players.get(i).unwrap().address.get() == sender {
                    return Err("ALREADY_SEATED".into());
                }
            }
        } else if table.id.get() == table_id {
            return Err("TABLE_ALREADY_EXISTS".into());
        }

        let chip_token = main_lobby.chip_token.get();
        if chip_token == Address::ZERO {
            return Err(b"CHIP_TOKEN_NOT_SET")?;
        }

        let lobby_address = self.vm().contract_address();
        let transferred = IPokerChips::new(chip_token)
            .transfer_from(&mut *self, sender, lobby_address, buy_in)
            .map_err(|_| b"CHIP_TRANSFER_FROM_FAILED".to_vec())?;
        if !transferred {
            return Err(b"CHIP_TRANSFER_FROM_REJECTED")?;
        }

        Ok(())
    }
}
