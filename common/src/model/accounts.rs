use alloc::{vec, vec::Vec};

use alloy_primitives::Address;
use alloy_primitives::U8;
use stylus_sdk::{
    alloy_primitives::U256,
    prelude::*,
    storage::{StorageAddress, StorageBytes, StorageMap, StorageU256, StorageU8, StorageVec},
};

use super::slots::PRIVATE_POKER_ACCOUNTS_SLOT;
use crate::storage::StorageSlot;

#[storage]
pub struct PlayerAccount {
    pub exists: StorageU256,
    pub player_address: StorageAddress,
    pub operator: StorageAddress,
    pub annonce_public_key: StorageBytes,
    pub encrypted_profile: StorageBytes,
    pub subscription_tier: StorageU8,
    pub subscription_paid_at: StorageU256,
    pub subscription_expires_at: StorageU256,
}

#[storage]
pub struct PrivatePokerAccountsStorage {
    pub owner: StorageAddress,
    pub usdc: StorageAddress,
    pub chips: StorageAddress,
    pub cashier: StorageAddress,
    pub accounts: StorageMap<Address, PlayerAccount>,
    pub operator_players: StorageMap<Address, StorageAddress>,
    pub players: StorageVec<StorageAddress>,
}

impl PrivatePokerAccountsStorage {
    #[inline]
    pub fn storage_slot() -> PrivatePokerAccountsStorage {
        StorageSlot::get_slot::<PrivatePokerAccountsStorage>(PRIVATE_POKER_ACCOUNTS_SLOT)
    }

    #[inline]
    pub fn operator_for_player(&self, player_address: Address) -> Result<Address, Vec<u8>> {
        let account = self.accounts.get(player_address);
        if account.exists.get() == U256::ZERO {
            return Err(b"ACCOUNT_MISSING".to_vec());
        }
        Ok(account.operator.get())
    }

    #[inline]
    pub fn owner_for_sender(&self, sender: Address) -> Address {
        let player = self.operator_players.get(sender);
        if player == Address::ZERO {
            sender
        } else {
            player
        }
    }

    #[inline]
    pub fn is_owner_or_operator(&self, sender: Address, owner: Address) -> bool {
        sender == owner || self.operator_players.get(sender) == owner
    }

    #[inline]
    pub fn write_account(
        &mut self,
        player_address: Address,
        operator: Address,
        annonce_public_key: &[u8],
        encrypted_profile: &[u8],
        subscription_tier: u8,
        paid_at: U256,
        expires_at: U256,
    ) {
        let mut account = self.accounts.setter(player_address);
        let is_new = account.exists.get() == U256::ZERO;
        if is_new {
            account.exists.set(U256::ONE);
            account.player_address.set(player_address);
            self.players.push(player_address);
        } else {
            let previous_operator = account.operator.get();
            if previous_operator != Address::ZERO
                && previous_operator != operator
                && self.operator_players.get(previous_operator) == player_address
            {
                self.operator_players.delete(previous_operator);
            }
        }

        account.operator.set(operator);
        account.annonce_public_key.set_bytes(annonce_public_key);
        account.encrypted_profile.set_bytes(encrypted_profile);
        account.subscription_tier.set(U8::from(subscription_tier));
        account.subscription_paid_at.set(paid_at);
        account.subscription_expires_at.set(expires_at);
        self.operator_players.setter(operator).set(player_address);
    }

    #[inline]
    pub fn update_account_profile(
        &mut self,
        player_address: Address,
        annonce_public_key: &[u8],
        encrypted_profile: &[u8],
    ) -> Result<Address, Vec<u8>> {
        let mut account = self.accounts.setter(player_address);
        if account.exists.get() == U256::ZERO {
            return Err(b"ACCOUNT_MISSING".to_vec());
        }
        let operator = account.operator.get();
        account.annonce_public_key.set_bytes(annonce_public_key);
        account.encrypted_profile.set_bytes(encrypted_profile);
        Ok(operator)
    }
}
