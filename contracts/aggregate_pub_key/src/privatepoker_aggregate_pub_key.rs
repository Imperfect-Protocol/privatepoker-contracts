use alloc::vec::Vec;

use alloy_primitives::{Address, Keccak256};
use alloy_sol_types::{SolCall, SolValue};
use privatepoker_common::lobby::{
    IPrivatePokerVerifySignature, MainLobby, TableAggregatePublicKeySet,
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
        aggregate_signature: Bytes,
    ) -> Result<(), Vec<u8>> {
        if aggregate_public_key.len() != G1AFFINE_COMPRESSED_LEN {
            return Err(b"INVALID_AGGREGATE_PUBLIC_KEY_LENGTH")?;
        }

        let sender = self.vm().msg_sender();
        let mut main_lobby = MainLobby::storage_slot();
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
        if !sender_can_set_table_key(sender, &table) {
            return Err(b"UNAUTHORIZED")?;
        }

        let verify_signature = main_lobby.facets.verify_signature.get();
        if verify_signature == Address::ZERO {
            return Err(b"VERIFY_SIGNATURE_NOT_SET")?;
        }

        let signed_digest =
            set_table_aggregate_public_key_digest(lobby_id, table_id, aggregate_public_key.clone());

        let verify = IPrivatePokerVerifySignature::verifySignatureCall {
            digest: signed_digest.to_vec().into(),
            aggregate_public_key: aggregate_public_key.0.clone().into(),
            aggregate_signature: aggregate_signature.0.clone().into(),
        };
        call_bool(
            self,
            verify_signature,
            verify.abi_encode(),
            b"INVALID_AGGREGATE_SIGNATURE",
        )?;

        table.aggregate_public_key.set_bytes(aggregate_public_key.0);

        stylus_core::log(self.vm(), TableAggregatePublicKeySet { lobby_id, table_id });
        Ok(())
    }
}

fn sender_can_set_table_key(sender: Address, table: &privatepoker_common::lobby::Table) -> bool {
    table.owner.get() == sender || sender_is_table_member(sender, table)
}

fn sender_is_table_member(sender: Address, table: &privatepoker_common::lobby::Table) -> bool {
    for index in 0..table.players.len() {
        let Some(player) = table.players.get(index) else {
            return false;
        };
        if player.address.get() == sender || player.operator.get() == sender {
            return true;
        }
    }
    false
}

pub fn set_table_aggregate_public_key_digest(
    lobby_id: U256,
    table_id: U256,
    aggregate_public_key: Bytes,
) -> [u8; 32] {
    let encoded = (lobby_id, table_id, aggregate_public_key).abi_encode();
    let mut k = Keccak256::new();
    k.update(encoded);
    k.finalize().0
}

fn call_bool(
    ctx: &mut PrivatePokerAggregatePubKey,
    to: Address,
    calldata: Vec<u8>,
    err: &[u8],
) -> Result<(), Vec<u8>> {
    let output = ctx
        .vm()
        .call(&ctx, to, &calldata)
        .map_err(|_| err.to_vec())?;
    let ok = bool::abi_decode(&output, true).map_err(|_| err.to_vec())?;
    if ok {
        Ok(())
    } else {
        Err(err.to_vec())
    }
}

pub const G1AFFINE_COMPRESSED_LEN: usize = 48;
pub const G2AFFINE_COMPRESSED_LEN: usize = 96;

#[cfg(test)]
mod tests {
    use super::{
        set_table_aggregate_public_key_digest, G1AFFINE_COMPRESSED_LEN, G2AFFINE_COMPRESSED_LEN,
    };
    use alloy_primitives::U256;
    use stylus_sdk::abi::Bytes;

    #[test]
    fn compressed_point_lengths_match_bls12_381() {
        assert_eq!(G1AFFINE_COMPRESSED_LEN, 48);
        assert_eq!(G2AFFINE_COMPRESSED_LEN, 96);
    }

    #[test]
    fn aggregate_public_key_digest_binds_lobby_table_and_key() {
        let aggregate_public_key = Bytes::from(vec![7u8; G1AFFINE_COMPRESSED_LEN]);
        let base = set_table_aggregate_public_key_digest(
            U256::from(1),
            U256::from(2),
            aggregate_public_key.clone(),
        );

        let changed_lobby = set_table_aggregate_public_key_digest(
            U256::from(9),
            U256::from(2),
            aggregate_public_key.clone(),
        );
        assert_ne!(base, changed_lobby);

        let changed_table = set_table_aggregate_public_key_digest(
            U256::from(1),
            U256::from(9),
            aggregate_public_key.clone(),
        );
        assert_ne!(base, changed_table);

        let changed_key = set_table_aggregate_public_key_digest(
            U256::from(1),
            U256::from(2),
            Bytes::from(vec![8u8; G1AFFINE_COMPRESSED_LEN]),
        );
        assert_ne!(base, changed_key);
    }
}
