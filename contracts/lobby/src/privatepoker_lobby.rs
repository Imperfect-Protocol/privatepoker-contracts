use alloc::{string::String, vec::Vec};

use alloy_primitives::Address;
use privatepoker_common::{
    interfaces::{ChipTokenSet, LobbyCreated},
    lobby::MainLobby,
};
use stylus_sdk::{alloy_primitives::U256, prelude::*, stylus_core};

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

    pub fn set_chip_token(&mut self, chip_token: Address) -> Result<(), Vec<u8>> {
        let mut main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();
        if main_lobby.owner.get() != sender {
            return Err("NOT_ADMIN".into());
        }

        main_lobby.chip_token.set(chip_token);
        stylus_core::log(self.vm(), ChipTokenSet { chip_token });
        Ok(())
    }

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
        if new_lobby.id.get() != U256::ZERO {
            return Err("LOBBY_ALREADY_EXISTS".into());
        }
        new_lobby.table_ids.erase();
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

    pub fn remove_lobby(&mut self, id: U256) -> Result<(), Vec<u8>> {
        let mut main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();
        if main_lobby.owner.get() != sender {
            return Err("NOT_ADMIN".into());
        }

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

        let mut lobby = main_lobby.lobbies.setter(id);
        let len = lobby.table_ids.len();
        for i in 0..len {
            let table_id = lobby.table_ids.get(i).unwrap();
            let mut table = lobby.tables.setter(table_id);
            table.clear();
        }

        lobby.id.erase();
        lobby.game_type.erase();
        lobby.flags.erase();
        lobby.name.erase();
        lobby.total_volume.erase();
        lobby.total_players.erase();
        lobby.table_ids.erase();

        Ok(())
    }
}
