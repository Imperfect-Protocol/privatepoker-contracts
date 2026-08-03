use alloc::vec::Vec;

use alloy_primitives::{Address, Bytes as AlloyBytes, U8};
use alloy_sol_types::SolCall;
use privatepoker_common::lobby::{
    HandStarted, HandVerified, IPrivatePokerVerifyShuffle, IPrivatePokerVerifyUnmasking, MainLobby,
    PublicKeySubmitted,
};
use stylus_sdk::{abi::Bytes, alloy_primitives::U256, prelude::*, stylus_core};

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

        let mut seat_number = None;

        for index in 0..seated_players {
            let player = table
                .players
                .get(index)
                .ok_or_else(|| b"INVALID_PLAYER_INDEX")?;
            if player.address.get() == sender {
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

    pub fn submit_public_key(
        &mut self,
        lobby_id: U256,
        table_id: U256,
        hand_id: U256,
        player: U8,
        public_key: Bytes,
        masked_before: Vec<Bytes>,
        masked_after: Vec<Bytes>,
        traces: Vec<Bytes>,
        player_keys: Vec<Bytes>,
        shuffle_history: Vec<Vec<Bytes>>,
        unmasking_sequence_cards: Vec<Vec<Vec<Bytes>>>,
        unmasking_actors: Vec<U8>,
        unmasking_states: Vec<U8>,
    ) -> Result<(), Vec<u8>> {
        if hand_id == U256::ZERO {
            return Err(b"INVALID_HAND_ID")?;
        }

        let sender = self.vm().msg_sender();
        let (verify_shuffle, verify_unmasking, num_players, player_index) = {
            let mut main_lobby = MainLobby::storage_slot();
            let verify_shuffle = main_lobby.facets.verify_shuffle.get();
            let verify_unmasking = main_lobby.facets.verify_unmasking.get();
            require_contract(verify_shuffle, b"VERIFY_SHUFFLE_ZERO")?;
            require_contract(verify_unmasking, b"VERIFY_UNMASKING_ZERO")?;

            let mut lobby = main_lobby.lobbies.setter(lobby_id);
            if lobby.id.get() != lobby_id {
                return Err(b"LOBBY_NOT_FOUND")?;
            }

            let table = lobby.tables.setter(table_id);
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

            let player_index = player.to::<usize>();
            if player_index >= num_players {
                return Err(b"INVALID_PLAYER_INDEX")?;
            }

            let table_player = table
                .players
                .get(player_index)
                .ok_or_else(|| b"INVALID_PLAYER_INDEX")?;
            if table_player.address.get() != sender && table_player.operator.get() != sender {
                return Err(b"SENDER_NOT_PLAYER")?;
            }

            if table.public_key_ready.get(sender) == hand_id {
                return Err(b"ALREADY_SUBMITTED")?;
            }

            (verify_shuffle, verify_unmasking, num_players, player_index)
        };

        call_verify_shuffle(
            self,
            verify_shuffle,
            masked_before,
            masked_after,
            public_key,
            traces,
        )?;
        call_verify_unmasking(
            self,
            verify_unmasking,
            U8::from(num_players as u8),
            player_keys,
            shuffle_history,
            unmasking_sequence_cards,
            unmasking_actors,
            unmasking_states,
        )?;

        let mut main_lobby = MainLobby::storage_slot();
        let mut lobby = main_lobby.lobbies.setter(lobby_id);
        let mut table = lobby.tables.setter(table_id);

        if table.public_key_ready.get(sender) == hand_id {
            return Err(b"ALREADY_SUBMITTED")?;
        }
        table.public_key_ready.insert(sender, hand_id);

        let ready_count = table.public_key_ready_count.get(hand_id) + U256::ONE;
        table.public_key_ready_count.insert(hand_id, ready_count);

        let remain_count = U256::from(num_players) - ready_count;
        stylus_core::log(
            self.vm(),
            PublicKeySubmitted {
                lobby_id,
                table_id,
                hand_id,
                seat_number: U256::from(player_index),
                remain_count,
            },
        );

        if ready_count >= U256::from(num_players) {
            stylus_core::log(
                self.vm(),
                HandVerified {
                    lobby_id,
                    table_id,
                    hand_id,
                },
            );
        }

        Ok(())
    }
}

fn call_verify_shuffle(
    ctx: &mut PrivatePokerHand,
    to: Address,
    masked_before: Vec<Bytes>,
    masked_after: Vec<Bytes>,
    pk: Bytes,
    traces: Vec<Bytes>,
) -> Result<(), Vec<u8>> {
    let calldata = IPrivatePokerVerifyShuffle::verifyShuffleCall {
        masked_before: to_alloy_bytes_vec(masked_before),
        masked_after: to_alloy_bytes_vec(masked_after),
        pk: to_alloy_bytes(pk),
        traces: to_alloy_bytes_vec(traces),
    }
    .abi_encode();
    call_void(ctx, to, calldata, b"SHUFFLE_VERIFICATION_FAILED")
}

fn call_verify_unmasking(
    ctx: &mut PrivatePokerHand,
    to: Address,
    num_players: U8,
    player_keys: Vec<Bytes>,
    shuffle_history: Vec<Vec<Bytes>>,
    unmasking_sequence_cards: Vec<Vec<Vec<Bytes>>>,
    unmasking_actors: Vec<U8>,
    unmasking_states: Vec<U8>,
) -> Result<(), Vec<u8>> {
    let calldata = IPrivatePokerVerifyUnmasking::verifyUnmaskingCall {
        num_players: num_players.to::<u8>(),
        player_keys: to_alloy_bytes_vec(player_keys),
        shuffle_history: to_alloy_bytes_matrix(shuffle_history),
        unmasking_sequence_cards: to_alloy_bytes_cube(unmasking_sequence_cards),
        unmasking_actors: to_u8_vec(unmasking_actors),
        unmasking_states: to_u8_vec(unmasking_states),
    }
    .abi_encode();
    call_void(ctx, to, calldata, b"UNMASKING_VERIFICATION_FAILED")
}

fn call_void(
    ctx: &mut PrivatePokerHand,
    to: Address,
    calldata: Vec<u8>,
    default_error: &[u8],
) -> Result<(), Vec<u8>> {
    match ctx.vm().call(&ctx, to, &calldata) {
        Ok(_) => Ok(()),
        Err(stylus_core::calls::errors::Error::Revert(revert)) if revert.is_empty() => {
            Err(default_error.to_vec())
        }
        Err(stylus_core::calls::errors::Error::Revert(revert)) => Err(revert),
        Err(_) => Err(default_error.to_vec()),
    }
}

fn require_contract(address: Address, err: &[u8]) -> Result<(), Vec<u8>> {
    if address == Address::ZERO {
        return Err(err.to_vec());
    }
    Ok(())
}

fn to_alloy_bytes(bytes: Bytes) -> AlloyBytes {
    AlloyBytes::from(bytes.0)
}

fn to_alloy_bytes_vec(items: Vec<Bytes>) -> Vec<AlloyBytes> {
    items.into_iter().map(to_alloy_bytes).collect()
}

fn to_alloy_bytes_matrix(items: Vec<Vec<Bytes>>) -> Vec<Vec<AlloyBytes>> {
    items.into_iter().map(to_alloy_bytes_vec).collect()
}

fn to_alloy_bytes_cube(items: Vec<Vec<Vec<Bytes>>>) -> Vec<Vec<Vec<AlloyBytes>>> {
    items.into_iter().map(to_alloy_bytes_matrix).collect()
}

fn to_u8_vec(items: Vec<U8>) -> Vec<u8> {
    items.into_iter().map(|item| item.to::<u8>()).collect()
}
