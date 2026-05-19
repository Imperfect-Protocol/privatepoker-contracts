use alloc::vec::Vec;

use alloy_primitives::Address;
use privatepoker_common::lobby::HandshakeSignal;
use stylus_sdk::{
    abi::Bytes, alloy_primitives::U256, prelude::*, storage::StorageAddress, stylus_core,
};

#[storage]
#[entrypoint]
pub struct PrivatePokerSignal {
    lobby: StorageAddress,
}

#[public]
impl PrivatePokerSignal {
    #[constructor]
    fn constructor(&mut self, lobby: Address) -> Result<(), Vec<u8>> {
        if lobby == Address::ZERO {
            return Err(b"LOBBY_ZERO".to_vec());
        }
        self.lobby.set(lobby);
        Ok(())
    }

    pub fn lobby(&self) -> Address {
        self.lobby.get()
    }

    // --- DECENTRALISED P2P ---

    /// Joiner calls this to "knock" on the Host's door.
    /// Data is the WebRTC Offer encrypted with the Host's BLS Public Key.
    pub fn send_signal(
        &mut self,
        lobby_id: U256,
        table_id: U256,
        recipients: Vec<Address>,
        encrypted_data: Vec<Bytes>,
    ) -> Result<(), Vec<u8>> {
        let sender = self.vm().msg_sender();

        if recipients.len() != encrypted_data.len() {
            return Err(b"SIGNAL_LENGTH_MISMATCH")?;
        }
        if recipients.is_empty() {
            return Err(b"SIGNAL_EMPTY")?;
        }

        let encrypted_data = encrypted_data
            .into_iter()
            .map(|v| v.to_vec().into())
            .collect::<Vec<alloy_primitives::Bytes>>();

        stylus_core::log(
            self.vm(),
            HandshakeSignal {
                sender,
                lobby_id,
                table_id,
                recipients,
                encrypted_data,
            },
        );
        Ok(())
    }
}
