use alloc::vec::Vec;

use privatepoker_common::lobby::{HandStarted, MainLobby};
use stylus_sdk::{alloy_primitives::U256, prelude::*, stylus_core};

#[storage]
#[entrypoint]
pub struct PrivatePokerHand;

#[public]
impl PrivatePokerHand {
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

        let seated_players = table.players.len();
        if seated_players == 0 {
            return Err(b"NO_PLAYERS")?;
        }
        let num_players = table.flags.get().to::<usize>();
        if num_players == 0 {
            return Err(b"INVALID_NUM_PLAYERS")?;
        }
        if seated_players < num_players {
            return Err(b"TABLE_NOT_FULL")?;
        }
        if table.aggregate_public_key.get_bytes().is_empty() {
            return Err(b"TABLE_AGGREGATE_PUBLIC_KEY_NOT_SET")?;
        }

        let mut seat_number = None;

        for index in 0..seated_players {
            let player = table
                .players
                .get(index)
                .ok_or_else(|| b"INVALID_PLAYER_INDEX")?;
            if player.address.get() == sender || player.operator.get() == sender {
                seat_number = Some(index);
                break;
            }
        }

        let seat_number = seat_number.ok_or_else(|| b"SENDER_NOT_SEATED")?;

        let ready_marker = table.current_hand.get() + U256::ONE;
        if table.hand_start_ready.get(sender) == ready_marker {
            return Err(b"ALREADY_READY")?;
        }

        table.hand_start_ready.insert(sender, ready_marker);

        let ready_count = table.hand_start_ready_count.get() + U256::ONE;
        let remain_count = U256::from(num_players) - ready_count;
        if ready_count < U256::from(num_players) {
            table.hand_start_ready_count.set(ready_count);
            stylus_core::log(
                self.vm(),
                HandStarted {
                    lobby_id,
                    table_id,
                    seat_number: U256::from(seat_number),
                    remain_count,
                },
            );
            return Ok(());
        }

        table.hand_start_ready_count.set(U256::ZERO);
        table.current_hand.set(ready_marker);

        stylus_core::log(
            self.vm(),
            HandStarted {
                lobby_id,
                table_id,
                seat_number: U256::from(seat_number),
                remain_count,
            },
        );

        Ok(())
    }
}
