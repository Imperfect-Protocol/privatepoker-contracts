use alloc::{string::String, vec, vec::Vec};

use alloy_primitives::Address;
use privatepoker_common::erc20;
use stylus_sdk::{alloy_primitives::U256, prelude::*, stylus_core};

#[storage]
#[entrypoint]
pub struct TestUsdc {
    token: erc20::Erc20Storage,
}

#[public]
impl TestUsdc {
    #[constructor]
    fn constructor(&mut self, initial_owner: Address) -> Result<(), Vec<u8>> {
        erc20::init_token(&mut self.token, initial_owner, "Test USDC", "tUSDC", 6);
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

    pub fn total_supply(&self) -> U256 {
        erc20::total_supply(&self.token)
    }

    pub fn balance_of(&self, account: Address) -> U256 {
        erc20::balance_of(&self.token, account)
    }

    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        erc20::allowance(&self.token, owner, spender)
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
        erc20::spend_allowance(&mut self.token, from, spender, value)?;
        erc20::transfer(&mut self.token, from, to, value)?;
        stylus_core::log(self.vm(), erc20::Transfer { from, to, value });
        Ok(true)
    }

    pub fn mint(&mut self, to: Address, value: U256) -> Result<bool, Vec<u8>> {
        let sender = self.vm().msg_sender();
        if sender != erc20::owner(&self.token) {
            return Err(b"NOT_OWNER".to_vec());
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

    pub fn faucet(&mut self, value: U256) -> Result<bool, Vec<u8>> {
        let to = self.vm().msg_sender();
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
}
