use alloc::{vec, vec::Vec};

use alloy_primitives::Address;
use alloy_sol_types::{sol, SolCall};
use privatepoker_common::{
    calls::ContractCalls,
    erc20,
    lobby::{PrivatePokerCashierStorage, PrivatePokerChipsStorage},
};
use stylus_sdk::{alloy_primitives::U256, prelude::*, stylus_core};

sol! {
    interface IERC20Like {
        function balanceOf(address account) external view returns (uint256);
        function totalSupply() external view returns (uint256);
        function transferFrom(address from, address to, uint256 value) external returns (bool);
        function transfer(address to, uint256 value) external returns (bool);
        function mint(address to, uint256 value) external returns (bool);
        function burn(address from, uint256 value) external returns (bool);
    }

    event Deposit(address indexed sender, address indexed owner, uint256 assets, uint256 shares);
    event Withdraw(address indexed sender, address indexed receiver, address indexed owner, uint256 assets, uint256 shares);
}

#[storage]
#[entrypoint]
pub struct Cashier;

#[public]
impl Cashier {
    #[constructor]
    fn constructor(
        &mut self,
        initial_owner: Address,
        usdc: Address,
        chips: Address,
    ) -> Result<(), Vec<u8>> {
        let mut cashier = PrivatePokerCashierStorage::storage_slot();
        cashier.owner.set(initial_owner);
        cashier.usdc.set(usdc);
        cashier.chips.set(chips);
        Ok(())
    }

    pub fn owner(&self) -> Address {
        PrivatePokerCashierStorage::storage_slot().owner.get()
    }

    pub fn usdc(&self) -> Address {
        PrivatePokerCashierStorage::storage_slot().usdc.get()
    }

    pub fn asset(&self) -> Address {
        self.usdc()
    }

    pub fn chips(&self) -> Address {
        PrivatePokerCashierStorage::storage_slot().chips.get()
    }

    pub fn share(&self) -> Address {
        self.chips()
    }

    pub fn set_tokens(&mut self, usdc: Address, chips: Address) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        let mut cashier = PrivatePokerCashierStorage::storage_slot();
        cashier.usdc.set(usdc);
        cashier.chips.set(chips);
        Ok(())
    }

    pub fn deposit_from(
        &mut self,
        payer: Address,
        receiver: Address,
        assets: U256,
        shares: U256,
    ) -> Result<U256, Vec<u8>> {
        self.require_diamond_call()?;
        self.deposit_from_internal(payer, receiver, assets, shares)
    }

    pub fn total_assets(&mut self) -> Result<U256, Vec<u8>> {
        self.require_tokens_set()?;
        let balance = IERC20Like::balanceOfCall {
            account: self.vm().contract_address(),
        };
        self.call_u256(self.usdc(), &balance.abi_encode(), b"USDC_BALANCE_FAILED")
    }

    pub fn total_supply(&mut self) -> Result<U256, Vec<u8>> {
        self.require_tokens_set()?;
        let supply = IERC20Like::totalSupplyCall {};
        self.call_u256(self.chips(), &supply.abi_encode(), b"CHIPS_SUPPLY_FAILED")
    }

    pub fn convert_to_shares(&self, assets: U256) -> U256 {
        assets
    }

    pub fn convert_to_assets(&self, shares: U256) -> U256 {
        shares
    }

    pub fn preview_deposit(&self, assets: U256) -> U256 {
        self.convert_to_shares(assets)
    }

    pub fn preview_mint(&self, shares: U256) -> U256 {
        self.convert_to_assets(shares)
    }

    pub fn preview_withdraw(&self, assets: U256) -> U256 {
        self.convert_to_shares(assets)
    }

    pub fn preview_redeem(&self, shares: U256) -> U256 {
        self.convert_to_assets(shares)
    }

    pub fn max_deposit(&self, _receiver: Address) -> U256 {
        U256::MAX
    }

    pub fn max_mint(&self, _receiver: Address) -> U256 {
        U256::MAX
    }

    pub fn max_withdraw(&mut self, owner: Address) -> Result<U256, Vec<u8>> {
        self.require_tokens_set()?;
        let balance = IERC20Like::balanceOfCall { account: owner };
        self.call_u256(self.chips(), &balance.abi_encode(), b"CHIPS_BALANCE_FAILED")
    }

    pub fn max_redeem(&mut self, owner: Address) -> Result<U256, Vec<u8>> {
        self.max_withdraw(owner)
    }

    fn deposit_from_internal(
        &mut self,
        payer: Address,
        receiver: Address,
        amount: U256,
        shares: U256,
    ) -> Result<U256, Vec<u8>> {
        if amount == U256::ZERO {
            return Err(b"ZERO_AMOUNT".to_vec());
        }
        if shares == U256::ZERO {
            return Err(b"ZERO_SHARES".to_vec());
        }
        if payer == Address::ZERO {
            return Err(b"PAYER_ZERO".to_vec());
        }
        if receiver == Address::ZERO {
            return Err(b"RECEIVER_ZERO".to_vec());
        }

        self.require_tokens_set()?;
        let diamond = self.vm().contract_address();
        let accounted_assets = PrivatePokerCashierStorage::storage_slot()
            .accounted_assets
            .get();
        if payer == diamond {
            let balance = IERC20Like::balanceOfCall { account: diamond };
            let balance =
                self.call_u256(self.usdc(), &balance.abi_encode(), b"USDC_BALANCE_FAILED")?;
            if balance < accounted_assets + amount {
                return Err(b"USDC_NOT_RECEIVED".to_vec());
            }
        } else {
            let pull = IERC20Like::transferFromCall {
                from: payer,
                to: diamond,
                value: amount,
            };
            self.call_optional_bool(
                self.usdc(),
                &pull.abi_encode(),
                b"USDC_TRANSFER_FROM_FAILED",
            )?;
        }
        PrivatePokerCashierStorage::storage_slot()
            .accounted_assets
            .set(accounted_assets + amount);

        let mut chips = PrivatePokerChipsStorage::storage_slot();
        erc20::mint(&mut chips.token, receiver, shares)?;
        stylus_core::log(
            self.vm(),
            erc20::Transfer {
                from: Address::ZERO,
                to: receiver,
                value: shares,
            },
        );

        stylus_core::log(
            self.vm(),
            Deposit {
                sender: self.vm().msg_sender(),
                owner: receiver,
                assets: amount,
                shares,
            },
        );

        Ok(shares)
    }

    fn only_owner(&self) -> Result<(), Vec<u8>> {
        if self.vm().msg_sender() != self.owner() {
            return Err(b"NOT_OWNER".to_vec());
        }
        Ok(())
    }

    fn require_diamond_call(&self) -> Result<(), Vec<u8>> {
        if self.vm().msg_sender() != self.vm().contract_address() {
            return Err(b"DIAMOND_ONLY".to_vec());
        }
        Ok(())
    }

    fn require_tokens_set(&self) -> Result<(), Vec<u8>> {
        if self.usdc() == Address::ZERO {
            return Err(b"USDC_NOT_SET".to_vec());
        }
        if self.chips() == Address::ZERO {
            return Err(b"CHIPS_NOT_SET".to_vec());
        }
        Ok(())
    }
}
