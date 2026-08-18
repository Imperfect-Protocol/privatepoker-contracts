use alloc::vec::Vec;

use alloy_primitives::Address;
use alloy_sol_types::SolCall;
use privatepoker_common::{
    calls::ContractCalls,
    interfaces::{IPrivatePokerVerifySignature, TableAggregatePublicKeySet},
    lobby::{MainLobby, PrivatePokerDiamond},
    poker::{set_table_aggregate_public_key_digest, G2AFFINE_COMPRESSED_LEN},
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

        let verify_signature = PrivatePokerDiamond::storage_slot().verify_signature.get();
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
        self.call_bool(
            verify_signature,
            &verify.abi_encode(),
            b"INVALID_AGGREGATE_SIGNATURE",
        )?;

        table.aggregate_public_key.set_bytes(aggregate_public_key.0);

        stylus_core::log(self.vm(), TableAggregatePublicKeySet { lobby_id, table_id });
        Ok(())
    }
}

pub const G1AFFINE_COMPRESSED_LEN: usize = 48;

#[cfg(test)]
mod tests {
    use super::G1AFFINE_COMPRESSED_LEN;
    use alloy_primitives::U256;
    use privatepoker_common::poker::{
        set_table_aggregate_public_key_digest, G2AFFINE_COMPRESSED_LEN,
    };
    use stylus_sdk::abi::Bytes;

    #[test]
    fn compressed_point_lengths_match_bls12_381() {
        assert_eq!(G1AFFINE_COMPRESSED_LEN, 48);
        assert_eq!(G2AFFINE_COMPRESSED_LEN, 96);
    }

    #[test]
    fn aggregate_public_key_digest_binds_lobby_table_and_key() {
        let aggregate_public_key = Bytes::from(vec![7u8; G2AFFINE_COMPRESSED_LEN]);
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
            Bytes::from(vec![8u8; G2AFFINE_COMPRESSED_LEN]),
        );
        assert_ne!(base, changed_key);
    }
}
