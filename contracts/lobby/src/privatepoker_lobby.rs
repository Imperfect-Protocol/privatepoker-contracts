use alloc::{string::String, vec::Vec};

use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use privatepoker_common::lobby::{
    LobbyCreated, LobbyInfo, MainLobby, TableCreated, TableDetail, TableInfo, TablePlayerInfo,
};
use stylus_sdk::{abi::Bytes, alloy_primitives::U256, prelude::*, stylus_core};

#[storage]
#[entrypoint]
pub struct PrivatePokerLobby;

#[public]
impl PrivatePokerLobby {
    #[constructor]
    fn constructor(&mut self, initial_owner: Address) -> Result<(), Vec<u8>> {
        let mut main_lobby = MainLobby::storage_slot();
        main_lobby.owner.set(initial_owner);
        Ok(())
    }

    // --- ADMIN ACTIONS ---

    pub fn add_lobby(
        &mut self,
        id: U256,
        game_type: U256,
        flags: U256,
        name: String,
    ) -> Result<(), Vec<u8>> {
        let mut main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();
        if main_lobby.owner.get() != sender {
            return Err("NOT_ADMIN".into());
        }

        let mut new_lobby = main_lobby.lobbies.setter(id);
        new_lobby.id.set(id);
        new_lobby.game_type.set(game_type);
        new_lobby.flags.set(flags);
        new_lobby.total_players.set(U256::ZERO);
        new_lobby.total_volume.set(U256::ZERO);
        new_lobby.name.set_str(name.clone());
        main_lobby.lobby_ids.push(id);

        stylus_core::log(self.vm(), LobbyCreated { id, name });
        Ok(())
    }

    // --- USER ACTIONS ---

    pub fn create_table(
        &mut self,
        lobby_id: U256,
        table_id: U256,
        name: String,
        buy_in: U256,
        annonce_public_key: Bytes,
    ) -> Result<(), Vec<u8>> {
        let mut main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();
        let mut lobby = main_lobby.lobbies.setter(lobby_id);

        if lobby.id.get() == U256::ZERO {
            return Err("LOBBY_NOT_FOUND".into());
        }

        let mut table = lobby.tables.setter(table_id);
        table.owner.set(sender);
        table.id.set(table_id);
        table.flags.set(U256::ZERO);
        table.name.set_str(name.clone());
        table.buy_in.set(buy_in);
        table.annonce_public_key.set_bytes(annonce_public_key);

        let mut new_player = table.players.grow();
        new_player.address.set(sender);
        new_player.chips_remain.set(buy_in);

        lobby.table_ids.push(table_id);

        let total_players = lobby.total_players.get();
        lobby.total_players.set(total_players + U256::ONE);

        let total_volume = lobby.total_volume.get();
        lobby.total_volume.set(total_volume + buy_in);

        let mut player_tables = main_lobby.player_tables.setter(sender);
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
        Ok(())
    }

    // --- VIEW ACTIONS (For the UI) ---

    pub fn get_lobby_count(&self) -> U256 {
        let main_lobby = MainLobby::storage_slot();
        U256::from(main_lobby.lobby_ids.len())
    }

    pub fn get_lobby_at(&self, index: U256) -> Result<Bytes, Vec<u8>> {
        let main_lobby = MainLobby::storage_slot();
        let Some(lobby_id) = main_lobby.lobby_ids.get(index.to::<usize>()) else {
            return Err(b"INDEX_OUT_OF_RANGE")?;
        };

        let lobby = main_lobby.lobbies.getter(lobby_id);
        if lobby.id.get() != lobby_id {
            return Err(b"INVALID_LOBBY")?;
        }

        let lobby_name = lobby.name.get_string();
        let lobby_game_type = U256::from(lobby.game_type.get());
        let lobby_flags = U256::from(lobby.flags.get());
        let lobby_table_count = U256::from(lobby.table_ids.len());
        let lobby_player_count = U256::from(lobby.total_players.get());
        let lobby_total_volume = U256::from(lobby.total_volume.get());

        Ok(LobbyInfo {
            lobby_id,
            lobby_game_type,
            lobby_flags,
            lobby_table_count,
            lobby_player_count,
            lobby_total_volume,
            lobby_name,
        }
        .abi_encode()
        .into())
    }

    pub fn get_table_count(&self, lobby_id: U256) -> Result<U256, Vec<u8>> {
        let main_lobby = MainLobby::storage_slot();
        let lobby = main_lobby.lobbies.getter(lobby_id);
        if lobby.id.get() != lobby_id {
            return Err(b"INVALID_LOBBY")?;
        }

        Ok(U256::from(lobby.table_ids.len()))
    }

    pub fn get_tables_range(
        &self,
        lobby_id: U256,
        offset: U256,
        count: U256,
    ) -> Result<Vec<Bytes>, Vec<u8>> {
        let main_lobby = MainLobby::storage_slot();
        let lobby = main_lobby.lobbies.getter(lobby_id);
        if lobby.id.get() != lobby_id {
            return Err(b"INVALID_LOBBY")?;
        }

        let mut result = Vec::new();
        let first = offset.to::<usize>();
        let last = first + count.to::<usize>();

        for index in first..last {
            let Some(table_id) = lobby.table_ids.get(index) else {
                return Err(b"INDEX_OUT_OF_RANGE")?;
            };

            let table = lobby.tables.getter(table_id);
            if table.id.get() != table_id {
                return Err(b"INVALID_TABLE")?;
            }

            let table_name = table.name.get_string();
            let table_buyin = table.buy_in.get();
            let table_flags = U256::from(table.flags.get());
            let table_player_count = U256::from(table.players.len());
            let table_total_buyin = U256::from(table.total_buyin.get());

            result.push(
                TableInfo {
                    table_id,
                    table_flags,
                    table_buyin,
                    table_player_count,
                    table_total_buyin,
                    table_name,
                }
                .abi_encode()
                .into(),
            );
        }

        Ok(result)
    }

    // --- DIRECT LOOKUP ACTIONS ---

    pub fn get_lobby_by_id(&self, lobby_id: U256) -> Result<Bytes, Vec<u8>> {
        let main_lobby = MainLobby::storage_slot();
        let lobby = main_lobby.lobbies.getter(lobby_id);

        if lobby.id.get() != lobby_id {
            return Err(b"LOBBY_NOT_FOUND")?;
        }

        let lobby_name = lobby.name.get_string();
        let lobby_game_type = U256::from(lobby.game_type.get());
        let lobby_flags = U256::from(lobby.flags.get());
        let lobby_table_count = U256::from(lobby.table_ids.len());
        let lobby_player_count = U256::from(lobby.total_players.get());
        let lobby_total_volume = U256::from(lobby.total_volume.get());

        Ok(LobbyInfo {
            lobby_id,
            lobby_game_type,
            lobby_flags,
            lobby_table_count,
            lobby_player_count,
            lobby_total_volume,
            lobby_name,
        }
        .abi_encode()
        .into())
    }

    pub fn get_table_detail(&self, lobby_id: U256, table_id: U256) -> Result<Bytes, Vec<u8>> {
        let main_lobby = MainLobby::storage_slot();
        let lobby = main_lobby.lobbies.getter(lobby_id);

        if lobby.id.get() != lobby_id {
            return Err(b"LOBBY_NOT_FOUND")?;
        }

        let table = lobby.tables.getter(table_id);
        if table.id.get() != table_id {
            return Err(b"TABLE_NOT_FOUND")?;
        }

        let num_players = table.players.len();

        let table_name = table.name.get_string();
        let table_buyin = table.buy_in.get();
        let table_flags = U256::from(table.flags.get());
        let table_player_count = U256::from(num_players);
        let table_total_buyin = U256::from(table.total_buyin.get());

        let mut players = Vec::new();
        for index in 0..num_players {
            let Some(player) = table.players.get(index) else {
                return Err(b"INVALID_PLAYER_INDEX")?;
            };

            players.push(TablePlayerInfo {
                player_address: player.address.get(),
                player_chips: player.chips_remain.get(),
            });
        }

        Ok(TableDetail {
            info: TableInfo {
                table_id,
                table_flags,
                table_buyin,
                table_player_count,
                table_total_buyin,
                table_name,
            },
            players,
        }
        .abi_encode()
        .into())
    }

    pub fn get_player_tables(&self, player: Address) -> Result<Vec<U256>, Vec<u8>> {
        let main_lobby = MainLobby::storage_slot();

        let player_tables = main_lobby.player_tables.get(player);
        let mut result = Vec::new();

        for index in 0..player_tables.len() {
            let Some(table_id) = player_tables.get(index) else {
                return Err(b"INVALID_TABLE_INDEX")?;
            };
            result.push(table_id);
        }

        Ok(result)
    }

    // --- REMOVAL ACTIONS ---

    /// Removes a lobby and all associated data. Restricted to Admin.
    pub fn remove_lobby(&mut self, id: U256) -> Result<(), Vec<u8>> {
        let mut main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();
        if main_lobby.owner.get() != sender {
            return Err("NOT_ADMIN".into());
        }

        // 1. Remove from the ID tracker (Swap and Pop)
        let mut found = false;
        let len = main_lobby.lobby_ids.len();
        for i in 0..len {
            if main_lobby.lobby_ids.get(i).unwrap() == id {
                if i < len - 1 {
                    let last_val = main_lobby.lobby_ids.get(len - 1).unwrap();
                    main_lobby.lobby_ids.setter(i).unwrap().set(last_val);
                }
                main_lobby.lobby_ids.pop();
                found = true;
                break;
            }
        }

        if !found {
            return Err("LOBBY_NOT_FOUND".into());
        }

        // 2. Clear the storage map entry
        // In Stylus, we "clear" by overwriting with default/zero values
        let mut lobby = main_lobby.lobbies.setter(id);
        lobby.id.erase();
        lobby.name.erase();

        // Remove all tables in the lobby
        // Note: this can cause DOS if list is long!
        let len = lobby.table_ids.len();
        for i in 0..len {
            let table_id = lobby.table_ids.get(i).unwrap();
            let mut table = lobby.tables.setter(table_id);
            table.owner.erase();
            table.id.erase();
            table.flags.erase();
            table.name.erase();
            table.buy_in.erase();
            table.annonce_public_key.erase();
            unsafe {
                table.players.set_len(0);
            }
            table.total_buyin.erase();
        }

        lobby.table_ids.erase();

        Ok(())
    }

    /// Removes a specific table from a lobby. Restricted to Table Owner or Admin.
    pub fn remove_table(&mut self, lobby_id: U256, table_id: U256) -> Result<(), Vec<u8>> {
        let mut main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();
        let mut lobby = main_lobby.lobbies.setter(lobby_id);
        let table = lobby.tables.getter(table_id);

        if table.owner.get() != sender && main_lobby.owner.get() != sender {
            return Err("UNAUTHORIZED".into());
        }

        // 1. Remove from lobby's table_ids (Swap and Pop)
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

        // 2. Clear the table data
        let mut table = lobby.tables.setter(table_id);
        table.owner.erase();
        table.id.erase();
        table.flags.erase();
        table.name.erase();
        table.buy_in.erase();
        table.annonce_public_key.erase();
        unsafe {
            table.players.set_len(0);
        }
        table.total_buyin.erase();

        Ok(())
    }
}
