use alloc::{vec, vec::Vec};

use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use privatepoker_common::{
    interfaces::{LobbyInfo, TableDetail, TableInfo, TablePlayerInfo},
    lobby::MainLobby,
};
use stylus_sdk::{abi::Bytes, alloy_primitives::U256, prelude::*};

#[storage]
#[entrypoint]
pub struct PrivatePokerSpectate;

#[public]
impl PrivatePokerSpectate {
    pub fn get_lobby_count(&self) -> U256 {
        let main_lobby = MainLobby::storage_slot();
        U256::from(main_lobby.lobby_ids.len())
    }

    pub fn get_lobby_at(&self, index: U256) -> Result<Bytes, Vec<u8>> {
        let main_lobby = MainLobby::storage_slot();
        let Some(lobby_id) = main_lobby.lobby_ids.get(index.to::<usize>()) else {
            return Err(b"INDEX_OUT_OF_RANGE")?;
        };

        self.get_lobby_by_id(lobby_id)
    }

    pub fn get_table_count(&self, lobby_id: U256) -> Result<U256, Vec<u8>> {
        let main_lobby = MainLobby::storage_slot();
        let lobby = main_lobby.lobbies.getter(lobby_id);
        if lobby.id.get() != lobby_id {
            return Err(b"INVALID_LOBBY")?;
        }

        Ok(U256::from(lobby.active_table_count()))
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
            let Some(table_id) = lobby.active_table_id_at(index) else {
                return Err(b"INDEX_OUT_OF_RANGE")?;
            };

            let table = lobby.tables.getter(table_id);
            if table.id.get() != table_id {
                return Err(b"INVALID_TABLE")?;
            }

            result.push(
                TableInfo {
                    table_id,
                    table_flags: U256::from(table.flags.get()),
                    table_buyin: table.buy_in.get(),
                    table_player_count: U256::from(table.players.len()),
                    table_total_buyin: U256::from(table.total_buyin.get()),
                    table_current_hand: table.current_hand.get(),
                    table_name: table.name.get_string(),
                }
                .abi_encode()
                .into(),
            );
        }

        Ok(result)
    }

    pub fn get_lobby_by_id(&self, lobby_id: U256) -> Result<Bytes, Vec<u8>> {
        let main_lobby = MainLobby::storage_slot();
        let lobby = main_lobby.lobbies.getter(lobby_id);

        if lobby.id.get() != lobby_id {
            return Err(b"LOBBY_NOT_FOUND")?;
        }

        Ok(LobbyInfo {
            lobby_id,
            lobby_game_type: U256::from(lobby.game_type.get()),
            lobby_flags: U256::from(lobby.flags.get()),
            lobby_table_count: U256::from(lobby.active_table_count()),
            lobby_player_count: U256::from(lobby.total_players.get()),
            lobby_total_volume: U256::from(lobby.total_volume.get()),
            lobby_name: lobby.name.get_string(),
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
        let mut players = Vec::new();
        for index in 0..num_players {
            let Some(player) = table.players.get(index) else {
                return Err(b"INVALID_PLAYER_INDEX")?;
            };

            players.push(TablePlayerInfo {
                player_address: player.address.get(),
                operator: player.operator.get(),
                player_chips: player.chips_remain.get(),
                player_annonce_public_key: player.annonce_public_key.get_bytes().to_vec().into(),
            });
        }

        Ok(TableDetail {
            info: TableInfo {
                table_id,
                table_flags: U256::from(table.flags.get()),
                table_buyin: table.buy_in.get(),
                table_player_count: U256::from(num_players),
                table_total_buyin: U256::from(table.total_buyin.get()),
                table_current_hand: table.current_hand.get(),
                table_name: table.name.get_string(),
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
}
