use alloc::{string::String, vec::Vec};

use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use privatepoker_common::lobby::{
    ChipTokenSet, HandStarted, LobbyCreated, LobbyInfo, MainLobby, TableCreated, TableDetail,
    TableInfo, TablePlayerInfo,
};
use stylus_sdk::{abi::Bytes, alloy_primitives::U256, prelude::*, stylus_core};

sol_interface! {
    interface IPokerChips {
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
    }
}

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
        self.collect_chip_buy_in(lobby_id, table_id, buy_in, false)?;

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
        table.total_buyin.set(buy_in);

        let mut new_player = table.players.grow();
        new_player.address.set(sender);
        new_player.chips_remain.set(buy_in);
        new_player.annonce_public_key.set_bytes(annonce_public_key);

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

    pub fn join_table(
        &mut self,
        lobby_id: U256,
        table_id: U256,
        annonce_public_key: Bytes,
    ) -> Result<(), Vec<u8>> {
        let buy_in = self.get_join_buy_in(lobby_id, table_id)?;
        self.collect_chip_buy_in(lobby_id, table_id, buy_in, true)?;

        let mut main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();

        let mut lobby = main_lobby.lobbies.setter(lobby_id);
        if lobby.id.get() == U256::ZERO {
            return Err("LOBBY_NOT_FOUND".into());
        }

        let mut table = lobby.tables.setter(table_id);
        if table.id.get() != table_id {
            return Err("TABLE_NOT_FOUND".into());
        }

        // 1. Prevent double-seating
        let num_players = table.players.len();
        for i in 0..num_players {
            if table.players.get(i).unwrap().address.get() == sender {
                return Err("ALREADY_SEATED".into());
            }
        }

        // 2. Seat the player and give them their chips
        let mut new_player = table.players.grow();
        new_player.address.set(sender);
        new_player.chips_remain.set(buy_in);
        new_player.annonce_public_key.set_bytes(annonce_public_key);

        // 3. Update the global accounting
        let current_total_buyin = table.total_buyin.get();
        table.total_buyin.set(current_total_buyin + buy_in);

        let lobby_volume = lobby.total_volume.get();
        lobby.total_volume.set(lobby_volume + buy_in);

        let total_players = lobby.total_players.get();
        lobby.total_players.set(total_players + U256::ONE);

        // 4. Add table to player's active tables index
        let mut player_tables = main_lobby.player_tables.setter(sender);
        player_tables.grow().set(table_id);

        Ok(())
    }

    pub fn start_hand(&mut self, lobby_id: U256, table_id: U256) -> Result<(), Vec<u8>> {
        let mut main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();

        let mut lobby = main_lobby.lobbies.setter(lobby_id);
        if lobby.id.get() != lobby_id {
            return Err(b"LOBBY_NOT_FOUND")?;
        }

        let mut table = lobby.tables.setter(table_id);
        if table.id.get() != table_id {
            return Err(b"TABLE_NOT_FOUND")?;
        }

        let num_players = table.players.len();
        if num_players == 0 {
            return Err(b"NO_PLAYERS")?;
        }

        let mut sender_is_seated = false;
        let first_player = table
            .players
            .get(0)
            .ok_or_else(|| b"INVALID_PLAYER_INDEX")?
            .address
            .get();

        for index in 0..num_players {
            let player = table
                .players
                .get(index)
                .ok_or_else(|| b"INVALID_PLAYER_INDEX")?;
            if player.address.get() == sender {
                sender_is_seated = true;
                break;
            }
        }
        if !sender_is_seated {
            return Err(b"SENDER_NOT_SEATED")?;
        }

        let ready_marker = table.current_hand.get() + U256::ONE;
        if table.hand_start_ready.get(sender) == ready_marker {
            return Err(b"ALREADY_READY")?;
        }

        table.hand_start_ready.insert(sender, ready_marker);

        let ready_count = table.hand_start_ready_count.get() + U256::ONE;
        if ready_count < U256::from(num_players) {
            table.hand_start_ready_count.set(ready_count);
            return Ok(());
        }

        table.hand_start_ready_count.set(U256::ZERO);
        table.current_hand.set(ready_marker);

        let small_blind = small_blind_for_buy_in(table.buy_in.get());
        let big_blind = small_blind * U256::from(2);

        stylus_core::log(
            self.vm(),
            HandStarted {
                lobby_id,
                table_id,
                next_player: first_player,
                small_blind,
                big_blind,
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

        self.get_lobby_by_id(lobby_id)
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
                player_annonce_public_key: player.annonce_public_key.get_bytes().to_vec().into(),
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
            clear_table(&mut table);
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

    /// Removes a specific table from a lobby. Restricted to Table Owner or Admin.
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

fn small_blind_for_buy_in(buy_in: U256) -> U256 {
    let hundred = U256::from(100);
    let blind = buy_in / hundred;
    if blind == U256::ZERO && buy_in > U256::ZERO {
        U256::ONE
    } else {
        blind
    }
}

fn clear_table(table: &mut privatepoker_common::lobby::Table) {
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

impl PrivatePokerLobby {
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
