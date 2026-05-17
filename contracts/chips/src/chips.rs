use alloc::{string::String, vec, vec::Vec};

use alloy_primitives::Address;
use privatepoker_common::erc20;
use stylus_sdk::{alloy_primitives::U256, prelude::*, storage::StorageAddress, stylus_core};

#[storage]
#[entrypoint]
pub struct Chips {
    token: erc20::Erc20Storage,
    cashier: StorageAddress,
    lobby: StorageAddress,
}

#[public]
impl Chips {
    #[constructor]
    fn constructor(&mut self, initial_owner: Address) -> Result<(), Vec<u8>> {
        erc20::init_token(
            &mut self.token,
            initial_owner,
            "Private Poker Chips",
            "CHIPS",
            6,
        );
        Ok(())
    }

    pub fn name(&self) -> String {
        erc20::name(&self.token)
    }

    pub fn symbol(&self) -> String {
        erc20::symbol(&self.token)
    }

    pub fn decimals(&self) -> u8 {
        erc20::decimals(&self.token)
    }

    pub fn owner(&self) -> Address {
        erc20::owner(&self.token)
    }

    pub fn cashier(&self) -> Address {
        self.cashier.get()
    }

    pub fn lobby(&self) -> Address {
        self.lobby.get()
    }

    pub fn total_supply(&self) -> U256 {
        erc20::total_supply(&self.token)
    }

    pub fn balance_of(&self, account: Address) -> U256 {
        erc20::balance_of(&self.token, account)
    }

    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        erc20::allowance(&self.token, owner, spender)
    }

    pub fn set_cashier(&mut self, cashier: Address) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        self.cashier.set(cashier);
        Ok(())
    }

    pub fn set_lobby(&mut self, lobby: Address) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        self.lobby.set(lobby);
        Ok(())
    }

    pub fn approve(&mut self, spender: Address, value: U256) -> Result<bool, Vec<u8>> {
        let sender = self.vm().msg_sender();
        erc20::approve(&mut self.token, sender, spender, value);
        stylus_core::log(
            self.vm(),
            erc20::Approval {
                owner: sender,
                spender,
                value,
            },
        );
        Ok(true)
    }

    pub fn transfer(&mut self, to: Address, value: U256) -> Result<bool, Vec<u8>> {
        let sender = self.vm().msg_sender();
        self.require_transfer_allowed(sender, sender, to)?;
        erc20::transfer(&mut self.token, sender, to, value)?;
        stylus_core::log(
            self.vm(),
            erc20::Transfer {
                from: sender,
                to,
                value,
            },
        );
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
        erc20::spend_allowance(&mut self.token, from, spender, value)?;
        erc20::transfer(&mut self.token, from, to, value)?;
        stylus_core::log(self.vm(), erc20::Transfer { from, to, value });
        Ok(true)
    }

    pub fn mint(&mut self, to: Address, value: U256) -> Result<bool, Vec<u8>> {
        let sender = self.vm().msg_sender();
        if sender != self.cashier.get() && sender != erc20::owner(&self.token) {
            return Err(b"NOT_MINTER".to_vec());
        }
        erc20::mint(&mut self.token, to, value)?;
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
        let sender = self.vm().msg_sender();
        if sender != self.cashier.get() && sender != erc20::owner(&self.token) {
            return Err(b"NOT_BURNER".to_vec());
        }
        erc20::burn(&mut self.token, from, value)?;
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
        if self.vm().msg_sender() != erc20::owner(&self.token) {
            return Err(b"NOT_OWNER".to_vec());
        }
        Ok(())
    }

    fn require_transfer_allowed(
        &self,
        spender: Address,
        from: Address,
        to: Address,
    ) -> Result<(), Vec<u8>> {
        let lobby = self.lobby.get();
        if lobby == Address::ZERO {
            return Err(b"LOBBY_NOT_SET".to_vec());
        }

        let owner_to_lobby = spender == from && to == lobby;
        let lobby_pull_buyin = spender == lobby && to == lobby;
        let lobby_payout = spender == lobby && from == lobby;
        if owner_to_lobby || lobby_pull_buyin || lobby_payout {
            return Ok(());
        }

        Err(b"CHIPS_NON_TRANSFERABLE".to_vec())
    }
}
