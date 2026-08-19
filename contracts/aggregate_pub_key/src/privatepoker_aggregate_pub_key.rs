use alloc::vec::Vec;

use privatepoker_common::{
    interfaces::TableAggregatePublicKeySet, lobby::MainLobby, poker::G2AFFINE_COMPRESSED_LEN,
};
use stylus_sdk::{abi::Bytes, alloy_primitives::U256, prelude::*, stylus_core};

#[storage]
#[entrypoint]
pub struct PrivatePokerAggregatePubKey;

#[public]
impl PrivatePokerAggregatePubKey {
    pub fn set_table_aggregate_public_key(
        &mut self,
        lobby_id: U256,
        table_id: U256,
        aggregate_public_key: Bytes,
    ) -> Result<(), Vec<u8>> {
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
        if table.current_hand.get() != U256::ZERO {
            return Err(b"TABLE_ALREADY_STARTED")?;
        }
        if sender != owner && !table.has_operator(sender) {
            return Err(b"UNAUTHORIZED")?;
        }

        table.aggregate_public_key.set_bytes(aggregate_public_key.0);

        stylus_core::log(self.vm(), TableAggregatePublicKeySet { lobby_id, table_id });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use privatepoker_common::poker::G2AFFINE_COMPRESSED_LEN;

    #[test]
    fn compressed_point_lengths_match_bls12_381() {
        assert_eq!(G2AFFINE_COMPRESSED_LEN, 96);
    }
}
