use alloc::{string::String, vec::Vec};

use alloy_primitives::Address;
use privatepoker_common::{
    erc20,
    interfaces::{PlayerJoined, TableCreated},
    lobby::{MainLobby, PrivatePokerAccountsStorage, PrivatePokerChipsStorage},
};
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
        annonce_public_key: Bytes,
    ) -> Result<(), Vec<u8>> {
        if num_players < U256::from(2) {
            return Err(b"INVALID_NUM_PLAYERS")?;
        }
        let operator =
            PrivatePokerAccountsStorage::storage_slot().operator_for_player(player_address)?;
        let sender = self.vm().msg_sender();
        let owner = MainLobby::storage_slot().owner.get();
        if sender != operator && sender != owner {
            return Err(b"NOT_OPERATOR_OR_OWNER")?;
        }
        self.collect_chip_buy_in(lobby_id, table_id, player_address, buy_in, false)?;

        let mut main_lobby = MainLobby::storage_slot();
        let mut lobby = main_lobby.lobbies.setter(lobby_id);

        if lobby.id.get() == U256::ZERO {
            return Err("LOBBY_NOT_FOUND".into());
        }

        let mut table = lobby.tables.setter(table_id);
        table.clear();

        table.created_by.set(player_address);
        table.id.set(table_id);
        table.flags.set(num_players);
        table.name.set_str(name.clone());
        table.buy_in.set(buy_in);
        table.total_buyin.set(buy_in);

        table.current_hand.set(U256::ZERO);
        table.hand_start_ready_count.set(U256::ZERO);
        table.aggregate_public_key.erase();

        let mut new_player = table.players.grow();
        new_player.address.set(player_address);
        new_player.chips_remain.set(buy_in);
        new_player.annonce_public_key.set_bytes(annonce_public_key);
        new_player.operator.set(operator);

        lobby.add_open_table(table_id);

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
        annonce_public_key: Bytes,
    ) -> Result<(), Vec<u8>> {
        let operator =
            PrivatePokerAccountsStorage::storage_slot().operator_for_player(player_address)?;
        let sender = self.vm().msg_sender();
        let owner = MainLobby::storage_slot().owner.get();
        if sender != operator && sender != owner {
            return Err(b"NOT_OPERATOR_OR_OWNER")?;
        }
        let buy_in = self.get_join_buy_in(lobby_id, table_id)?;
        self.collect_chip_buy_in(lobby_id, table_id, player_address, buy_in, true)?;

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
        let table_is_full = required_players > 0 && table.players.len() >= required_players;

        let lobby_volume = lobby.total_volume.get();
        lobby.total_volume.set(lobby_volume + buy_in);

        let total_players = lobby.total_players.get();
        lobby.total_players.set(total_players + U256::ONE);

        let mut player_tables = main_lobby.player_tables.setter(player_address);
        player_tables.grow().set(table_id);

        if table_is_full {
            lobby.mark_table_running(table_id);
        }

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
        let owner = main_lobby.owner.get();
        if sender != owner {
            let operator = PrivatePokerAccountsStorage::storage_slot()
                .operator_for_player(table.created_by.get())?;
            if sender != operator {
                return Err("UNAUTHORIZED".into());
            }
        }

        if !lobby.remove_table_id(table_id) {
            return Err("TABLE_NOT_FOUND".into());
        }

        let mut table = lobby.tables.setter(table_id);
        table.clear();

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
        player_address: Address,
        buy_in: U256,
        table_must_exist: bool,
    ) -> Result<(), Vec<u8>> {
        if buy_in == U256::ZERO {
            return Ok(());
        }

        let main_lobby = MainLobby::storage_slot();
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
                if table.players.get(i).unwrap().address.get() == player_address {
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

        let mut chips = PrivatePokerChipsStorage::storage_slot();
        erc20::spend_allowance(&mut chips.token, player_address, chip_token, buy_in)
            .map_err(|_| b"CHIP_TRANSFER_FROM_FAILED".to_vec())?;
        erc20::transfer(&mut chips.token, player_address, chip_token, buy_in)
            .map_err(|_| b"CHIP_TRANSFER_FROM_FAILED".to_vec())?;
        stylus_core::log(
            self.vm(),
            erc20::Transfer {
                from: player_address,
                to: chip_token,
                value: buy_in,
            },
        );

        Ok(())
    }
}
