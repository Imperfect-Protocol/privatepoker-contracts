use alloc::vec::Vec;

use alloy_primitives::Address;
use privatepoker_common::lobby::{HandshakeSignal, MainLobby};
use stylus_sdk::{abi::Bytes, alloy_primitives::U256, prelude::*, stylus_core};

#[storage]
#[entrypoint]
pub struct PrivatePokerSignal;

#[public]
impl PrivatePokerSignal {
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
        let main_lobby = MainLobby::storage_slot();
        let sender = self.vm().msg_sender();
        let lobby = main_lobby.lobbies.getter(lobby_id);
        if lobby.id.get() != lobby_id {
            return Err(b"INVALID_LOBBY")?;
        }
        let table = lobby.tables.get(table_id);
        if table.id.get() != table_id {
            return Err(b"INVALID_TABLE")?;
        }
        for i in 0..recipients.len() {
            let receipient = recipients[i];
            let player = table.players.get(i).ok_or_else(|| b"INVALID_PLAYER")?;
            if receipient != player.address.get() {
                Err(b"INVALID_RECEIPIENT")?;
            }
        }
        stylus_core::log(
            self.vm(),
            HandshakeSignal {
                sender,
                lobby_id,
                table_id,
                recipients,
                encrypted_data: encrypted_data
                    .into_iter()
                    .map(|v| v.to_vec().into())
                    .collect(),
            },
        );
        Ok(())
    }
}
