use alloc::{string::String, vec, vec::Vec};

use alloy_primitives::Address;
use alloy_primitives::U8;
use stylus_sdk::{
    alloy_primitives::U256,
    prelude::*,
    storage::{StorageAddress, StorageBytes, StorageMap, StorageString, StorageU256, StorageU8, StorageVec},
};

use super::slots::PRIVATE_POKER_ACCOUNTS_SLOT;
use crate::storage::StorageSlot;

pub const ACCOUNT_STATUS_UNVERIFIED: U256 = U256::from_limbs([1, 0, 0, 0]);
pub const ACCOUNT_STATUS_VERIFIED: U256 = U256::from_limbs([2, 0, 0, 0]);
pub const ACCOUNT_STATUS_SUSPENDED: U256 = U256::from_limbs([4, 0, 0, 0]);
pub const ACCOUNT_STATUS_BANNED: U256 = U256::from_limbs([8, 0, 0, 0]);
pub const ACCOUNT_STATUS_DELETED: U256 = U256::from_limbs([16, 0, 0, 0]);

#[storage]
pub struct PlayerAccount {
    pub flags: StorageU256,
    pub status_changed_at: StorageU256,
    pub player_address: StorageAddress,
    pub operator: StorageAddress,
    pub display_name: StorageString,
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
        if account.flags.get() == U256::ZERO {
            return Err(b"ACCOUNT_MISSING".to_vec());
        }
        Ok(account.operator.get())
    }

    #[inline]
    pub fn account_status(&self, player_address: Address) -> U256 {
        self.accounts.get(player_address).flags.get()
    }

    #[inline]
    pub fn account_status_changed_at(&self, player_address: Address) -> U256 {
        self.accounts.get(player_address).status_changed_at.get()
    }

    #[inline]
    pub fn create_account(
        &mut self,
        player_address: Address,
        display_name: String,
        encrypted_profile: &[u8],
        now: U256,
    ) -> Result<(), Vec<u8>> {
        let mut account = self.accounts.setter(player_address);
        if account.flags.get() != U256::ZERO {
            return Err(b"ACCOUNT_EXISTS".to_vec());
        }

        account.flags.set(ACCOUNT_STATUS_UNVERIFIED);
        account.status_changed_at.set(now);
        account.player_address.set(player_address);
        account.display_name.set_str(display_name);
        account.encrypted_profile.set_bytes(encrypted_profile);
        self.players.push(player_address);
        Ok(())
    }

    #[inline]
    pub fn set_account_status(
        &mut self,
        player_address: Address,
        flags: U256,
        now: U256,
    ) -> Result<(), Vec<u8>> {
        let mut account = self.accounts.setter(player_address);
        if account.flags.get() == U256::ZERO {
            return Err(b"ACCOUNT_MISSING".to_vec());
        }
        if flags == U256::ZERO {
            return Err(b"INVALID_STATUS".to_vec());
        }

        account.flags.set(flags);
        account.status_changed_at.set(now);
        Ok(())
    }

    #[inline]
    pub fn require_verified(&self, player_address: Address) -> Result<(), Vec<u8>> {
        if self.account_status(player_address) != ACCOUNT_STATUS_VERIFIED {
            return Err(b"ACCOUNT_NOT_VERIFIED".to_vec());
        }
        Ok(())
    }

    #[inline]
    pub fn player_for_sender(&self, sender: Address) -> Address {
        let player = self.operator_players.get(sender);
        if player == Address::ZERO {
            sender
        } else {
            player
        }
    }

    #[inline]
    pub fn write_account(
        &mut self,
        player_address: Address,
        operator: Address,
        display_name: String,
        annonce_public_key: &[u8],
        encrypted_profile: &[u8],
        subscription_tier: u8,
        paid_at: U256,
        expires_at: U256,
    ) -> Result<(), Vec<u8>> {
        let mut account = self.accounts.setter(player_address);
        if account.flags.get() == U256::ZERO {
            return Err(b"ACCOUNT_MISSING".to_vec());
        }

        let previous_operator = account.operator.get();
        if previous_operator != Address::ZERO
            && previous_operator != operator
            && self.operator_players.get(previous_operator) == player_address
        {
            self.operator_players.delete(previous_operator);
        }

        account.operator.set(operator);
        account.display_name.set_str(display_name);
        account.annonce_public_key.set_bytes(annonce_public_key);
        account.encrypted_profile.set_bytes(encrypted_profile);
        account.subscription_tier.set(U8::from(subscription_tier));
        account.subscription_paid_at.set(paid_at);
        account.subscription_expires_at.set(expires_at);
        self.operator_players.setter(operator).set(player_address);
        Ok(())
    }

    #[inline]
    pub fn update_account_profile(
        &mut self,
        player_address: Address,
        display_name: String,
        annonce_public_key: &[u8],
        encrypted_profile: &[u8],
    ) -> Result<Address, Vec<u8>> {
        let mut account = self.accounts.setter(player_address);
        if account.flags.get() == U256::ZERO {
            return Err(b"ACCOUNT_MISSING".to_vec());
        }
        let operator = account.operator.get();
        account.display_name.set_str(display_name);
        account.annonce_public_key.set_bytes(annonce_public_key);
        account.encrypted_profile.set_bytes(encrypted_profile);
        Ok(operator)
    }
}
