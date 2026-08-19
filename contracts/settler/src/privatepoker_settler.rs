use alloc::vec::Vec;

use alloy_primitives::Address;
use privatepoker_common::{
    interfaces::HandSettled,
    lobby::{MainLobby, PrivatePokerChipsStorage},
    poker::{checked_sum, game_ended_winner_index, DIGEST_LEN, G2AFFINE_COMPRESSED_LEN},
};
use stylus_sdk::{abi::Bytes, alloy_primitives::U256, prelude::*, stylus_core};

#[storage]
#[entrypoint]
pub struct PrivatePokerSettler;

#[public]
impl PrivatePokerSettler {
    pub fn settle_hand(
        &mut self,
        lobby_id: U256,
        table_id: U256,
        hand_id: U256,
        pot_size: U256,
        pot_split: Vec<U256>,
        chips_balances: Vec<U256>,
        digest: Bytes,
        aggregate_public_key: Bytes,
    ) -> Result<(), Vec<u8>> {
        if hand_id == U256::ZERO {
            return Err(b"INVALID_HAND_ID")?;
        }
        if digest.len() != DIGEST_LEN {
            return Err(b"INVALID_DIGEST_LENGTH")?;
        }
        if aggregate_public_key.len() != G2AFFINE_COMPRESSED_LEN {
            return Err(b"INVALID_AGGREGATE_PUBLIC_KEY_LENGTH")?;
        }

        let sender = self.vm().msg_sender();
        let mut main_lobby = MainLobby::storage_slot();
        let owner = main_lobby.owner.get();
        let mut lobby = main_lobby.lobbies.setter(lobby_id);
        if lobby.id.get() != lobby_id {
            return Err(b"LOBBY_NOT_FOUND")?;
        }

        let mut table = lobby.tables.setter(table_id);
        if table.id.get() != table_id {
            return Err(b"TABLE_NOT_FOUND")?;
        }
        if table.current_hand.get() != hand_id {
            return Err(b"HAND_NOT_CURRENT")?;
        }
        if sender != owner && !table.has_operator(sender) {
            return Err(b"UNAUTHORIZED")?;
        }

        let num_players = table.flags.get().to::<usize>();
        if num_players == 0 || table.players.len() < num_players {
            return Err(b"TABLE_NOT_FULL")?;
        }
        if pot_split.len() != num_players {
            return Err(b"INVALID_POT_SPLIT_LENGTH")?;
        }
        if chips_balances.len() != num_players {
            return Err(b"INVALID_CHIPS_BALANCES_LENGTH")?;
        }
        if checked_sum(&pot_split)? != pot_size {
            return Err(b"INVALID_POT_SPLIT")?;
        }
        if checked_sum(&chips_balances)? != table.total_buyin.get() {
            return Err(b"INVALID_CHIPS_BALANCES")?;
        }
        let game_ended_winner_index = game_ended_winner_index(&chips_balances);

        let stored_aggregate_public_key = table.aggregate_public_key.get_bytes().to_vec();
        if stored_aggregate_public_key.is_empty() {
            return Err(b"TABLE_AGGREGATE_PUBLIC_KEY_NOT_SET")?;
        }
        if &aggregate_public_key.0 != stored_aggregate_public_key.as_slice() {
            return Err(b"AGGREGATE_PUBLIC_KEY_MISMATCH")?;
        }

        let mut hand = table.hands.setter(hand_id);
        if !hand.digest.get_bytes().is_empty() {
            return Err(b"HAND_ALREADY_SETTLED")?;
        }

        hand.pot_size.set(pot_size);
        unsafe {
            hand.pot_split.set_len(0);
        }
        for amount in pot_split {
            hand.pot_split.grow().set(amount);
        }
        hand.digest.set_bytes(digest.0.clone());

        for (index, balance) in chips_balances.iter().enumerate() {
            let mut player = table
                .players
                .setter(index)
                .ok_or_else(|| b"INVALID_PLAYER_INDEX")?;
            if game_ended_winner_index.is_some() {
                player.chips_remain.set(U256::ZERO);
            } else {
                player.chips_remain.set(*balance);
            }
        }

        if let Some(winner_index) = game_ended_winner_index {
            let winner = table
                .players
                .get(winner_index)
                .ok_or_else(|| b"INVALID_PLAYER_INDEX")?;
            let winner_address = winner.address.get();
            let payout = chips_balances[winner_index];
            let chip_token = main_lobby.chip_token.get();
            if chip_token == Address::ZERO {
                return Err(b"CHIP_TOKEN_NOT_SET")?;
            }

            let mut chips = PrivatePokerChipsStorage::storage_slot();
            let escrow_balance = chips.token.balances.get(chip_token);
            if escrow_balance < payout {
                return Err(b"CHIP_PAYOUT_FAILED")?;
            }
            chips
                .token
                .balances
                .insert(chip_token, escrow_balance - payout);
            let winner_balance = chips.token.balances.get(winner_address);
            chips
                .token
                .balances
                .insert(winner_address, winner_balance + payout);
            table.total_buyin.set(U256::ZERO);
        }

        stylus_core::log(
            self.vm(),
            HandSettled {
                lobby_id,
                table_id,
                hand_id,
                digest: digest.0.into(),
            },
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::U256;
    use privatepoker_common::poker::{checked_sum, game_ended_winner_index};

    #[test]
    fn checked_sum_rejects_overflow() {
        assert!(checked_sum(&[U256::MAX, U256::ONE]).is_err());
    }

    #[test]
    fn game_ended_requires_exactly_one_remaining_stack() {
        assert_eq!(
            game_ended_winner_index(&[U256::from(2000), U256::ZERO]),
            Some(0)
        );
        assert_eq!(
            game_ended_winner_index(&[U256::ZERO, U256::from(2000)]),
            Some(1)
        );
        assert_eq!(
            game_ended_winner_index(&[U256::from(1000), U256::from(1000)]),
            None
        );
        assert_eq!(game_ended_winner_index(&[U256::ZERO, U256::ZERO]), None);
    }
}
