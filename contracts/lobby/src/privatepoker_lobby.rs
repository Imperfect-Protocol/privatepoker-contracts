use alloc::{string::String, vec::Vec};

use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use privatepoker_common::lobby::{LobbyCreated, LobbyInfo, MainLobby, TableCreated, TableInfo};
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

    pub fn add_lobby(&mut self, id: U256, name: String) -> Result<(), Vec<u8>> {
        let mut main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();
        if main_lobby.owner.get() != sender {
            return Err("NOT_ADMIN".into());
        }

        let mut new_lobby = main_lobby.lobbies.setter(id);
        new_lobby.id.set(id);
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
        annonce_public_key: String,
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
        table.name.set_str(name.clone());
        table.buy_in.set(buy_in);
        table.is_active.set(true);
        table.annonce_public_key.set_str(annonce_public_key);

        lobby.table_ids.push(table_id);

        stylus_core::log(
            self.vm(),
            TableCreated {
                id: table_id,
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
        let table_count = U256::from(lobby.table_ids.len());

        Ok(LobbyInfo {
            lobby_id,
            lobby_name,
            table_count,
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

    pub fn get_table_at(&self, lobby_id: U256, index: U256) -> Result<Bytes, Vec<u8>> {
        let main_lobby = MainLobby::storage_slot();
        let lobby = main_lobby.lobbies.getter(lobby_id);
        if lobby.id.get() != lobby_id {
            return Err(b"INVALID_LOBBY")?;
        }

        let Some(table_id) = lobby.table_ids.get(index) else {
            return Err(b"INDEX_OUT_OF_RANGE")?;
        };

        let table = lobby.tables.getter(table_id);
        if table.id.get() != table_id {
            return Err(b"INVALID_TABLE")?;
        }

        let table_name = table.name.get_string();
        let table_buyin = table.buy_in.get();

        Ok(TableInfo {
            table_id,
            table_name,
            table_buyin,
        }
        .abi_encode()
        .into())
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
            table.name.erase();
            table.buy_in.erase();
            table.is_active.erase();
            table.annonce_public_key.erase();
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
        table.name.erase();
        table.buy_in.erase();
        table.is_active.erase();
        table.annonce_public_key.erase();

        Ok(())
    }
}