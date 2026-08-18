use alloc::{string::String, vec, vec::Vec};

use alloy_primitives::Address;
use privatepoker_common::{
    erc20,
    lobby::{PrivatePokerAccountsStorage, PrivatePokerChipsStorage},
};
use stylus_sdk::{alloy_primitives::U256, prelude::*, stylus_core};

#[storage]
#[entrypoint]
pub struct Chips;

#[public]
impl Chips {
    #[constructor]
    fn constructor(&mut self, initial_owner: Address) -> Result<(), Vec<u8>> {
        let mut chips = PrivatePokerChipsStorage::storage_slot();
        erc20::init_token(
            &mut chips.token,
            initial_owner,
            "Private Poker Chips",
            "CHIPS",
            6,
        );
        Ok(())
    }

    pub fn name(&self) -> String {
        let chips = PrivatePokerChipsStorage::storage_slot();
        erc20::name(&chips.token)
    }

    pub fn symbol(&self) -> String {
        let chips = PrivatePokerChipsStorage::storage_slot();
        erc20::symbol(&chips.token)
    }

    pub fn decimals(&self) -> u8 {
        let chips = PrivatePokerChipsStorage::storage_slot();
        erc20::decimals(&chips.token)
    }

    pub fn owner(&self) -> Address {
        let chips = PrivatePokerChipsStorage::storage_slot();
        erc20::owner(&chips.token)
    }

    pub fn cashier(&self) -> Address {
        PrivatePokerChipsStorage::storage_slot().cashier.get()
    }

    pub fn lobby(&self) -> Address {
        PrivatePokerChipsStorage::storage_slot().lobby.get()
    }

    pub fn account(&self) -> Address {
        PrivatePokerChipsStorage::storage_slot().account.get()
    }

    pub fn total_supply(&self) -> U256 {
        let chips = PrivatePokerChipsStorage::storage_slot();
        erc20::total_supply(&chips.token)
    }

    pub fn balance_of(&self, account: Address) -> U256 {
        let chips = PrivatePokerChipsStorage::storage_slot();
        erc20::balance_of(&chips.token, account)
    }

    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        let chips = PrivatePokerChipsStorage::storage_slot();
        erc20::allowance(&chips.token, owner, spender)
    }

    pub fn set_cashier(&mut self, cashier: Address) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        PrivatePokerChipsStorage::storage_slot()
            .cashier
            .set(cashier);
        Ok(())
    }

    pub fn set_lobby(&mut self, lobby: Address) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        PrivatePokerChipsStorage::storage_slot().lobby.set(lobby);
        Ok(())
    }

    pub fn set_account(&mut self, account: Address) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        PrivatePokerChipsStorage::storage_slot()
            .account
            .set(account);
        Ok(())
    }

    pub fn approve(&mut self, spender: Address, value: U256) -> Result<bool, Vec<u8>> {
        let sender = self.vm().msg_sender();
        let accounts = PrivatePokerAccountsStorage::storage_slot();
        let owner = accounts.owner_for_sender(sender);
        self.require_diamond(spender)?;
        if !accounts.is_owner_or_operator(sender, owner) {
            return Err(b"NOT_OWNER_OR_OPERATOR".to_vec());
        }

        let mut chips = PrivatePokerChipsStorage::storage_slot();
        erc20::approve(&mut chips.token, owner, spender, value);
        stylus_core::log(
            self.vm(),
            erc20::Approval {
                owner,
                spender,
                value,
            },
        );
        Ok(true)
    }

    pub fn transfer(&mut self, to: Address, value: U256) -> Result<bool, Vec<u8>> {
        let sender = self.vm().msg_sender();
        let from = PrivatePokerAccountsStorage::storage_slot().owner_for_sender(sender);
        self.require_transfer_allowed(sender, from, to)?;

        let mut chips = PrivatePokerChipsStorage::storage_slot();
        erc20::transfer(&mut chips.token, from, to, value)?;
        stylus_core::log(self.vm(), erc20::Transfer { from, to, value });
        Ok(true)
    }

    pub fn transfer_from(
        &mut self,
        from: Address,
        to: Address,
        value: U256,
    ) -> Result<bool, Vec<u8>> {
        let spender = self.vm().msg_sender();
        self.require_transfer_allowed(spender, from, to)?;

        let mut chips = PrivatePokerChipsStorage::storage_slot();
        erc20::spend_allowance(&mut chips.token, from, spender, value)?;
        erc20::transfer(&mut chips.token, from, to, value)?;
        stylus_core::log(self.vm(), erc20::Transfer { from, to, value });
        Ok(true)
    }

    pub fn mint(&mut self, to: Address, value: U256) -> Result<bool, Vec<u8>> {
        self.require_diamond(self.vm().msg_sender())?;

        let mut chips = PrivatePokerChipsStorage::storage_slot();
        erc20::mint(&mut chips.token, to, value)?;
        stylus_core::log(
            self.vm(),
            erc20::Transfer {
                from: Address::ZERO,
                to,
                value,
            },
        );
        Ok(true)
    }

    pub fn burn(&mut self, from: Address, value: U256) -> Result<bool, Vec<u8>> {
        self.require_diamond(self.vm().msg_sender())?;

        let mut chips = PrivatePokerChipsStorage::storage_slot();
        erc20::burn(&mut chips.token, from, value)?;
        stylus_core::log(
            self.vm(),
            erc20::Transfer {
                from,
                to: Address::ZERO,
                value,
            },
        );
        Ok(true)
    }

    fn only_owner(&self) -> Result<(), Vec<u8>> {
        let chips = PrivatePokerChipsStorage::storage_slot();
        if self.vm().msg_sender() != erc20::owner(&chips.token) {
            return Err(b"NOT_OWNER".to_vec());
        }
        Ok(())
    }

    fn diamond(&self) -> Address {
        self.vm().contract_address()
    }

    fn require_diamond(&self, address: Address) -> Result<(), Vec<u8>> {
        if address != self.diamond() {
            return Err(b"DIAMOND_ONLY".to_vec());
        }
        Ok(())
    }

    fn require_transfer_allowed(
        &self,
        spender: Address,
        from: Address,
        to: Address,
    ) -> Result<(), Vec<u8>> {
        let diamond = self.diamond();
        let chips = PrivatePokerChipsStorage::storage_slot();

        let owner_to_diamond = spender == from && to == diamond;
        let operator_to_diamond =
            PrivatePokerAccountsStorage::storage_slot().owner_for_sender(spender) == from
                && to == diamond;
        let diamond_pull_buyin = spender == diamond && to == diamond;
        let diamond_payout = spender == diamond && from == diamond;

        if owner_to_diamond || operator_to_diamond || diamond_pull_buyin || diamond_payout {
            return Ok(());
        }

        if spender == erc20::owner(&chips.token) {
            return Ok(());
        }

        Err(b"CHIPS_NON_TRANSFERABLE".to_vec())
    }
}
