use alloc::{vec, vec::Vec};

use alloy_primitives::Address;
use alloy_sol_types::{sol, SolCall, SolValue};
use stylus_sdk::{
    alloy_primitives::U256, call::RawCall, prelude::*, storage::StorageAddress, stylus_core,
};

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
pub struct Cashier {
    owner: StorageAddress,
    usdc: StorageAddress,
    chips: StorageAddress,
}

#[public]
impl Cashier {
    #[constructor]
    fn constructor(
        &mut self,
        initial_owner: Address,
        usdc: Address,
        chips: Address,
    ) -> Result<(), Vec<u8>> {
        self.owner.set(initial_owner);
        self.usdc.set(usdc);
        self.chips.set(chips);
        Ok(())
    }

    pub fn owner(&self) -> Address {
        self.owner.get()
    }

    pub fn usdc(&self) -> Address {
        self.usdc.get()
    }

    pub fn asset(&self) -> Address {
        self.usdc.get()
    }

    pub fn chips(&self) -> Address {
        self.chips.get()
    }

    pub fn share(&self) -> Address {
        self.chips.get()
    }

    pub fn set_tokens(&mut self, usdc: Address, chips: Address) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        self.usdc.set(usdc);
        self.chips.set(chips);
        Ok(())
    }

    pub fn deposit(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        let receiver = self.vm().msg_sender();
        self.deposit_to(amount, receiver)?;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        let sender = self.vm().msg_sender();
        self.withdraw_to(amount, sender, sender)?;
        Ok(())
    }

    pub fn deposit_to(&mut self, assets: U256, receiver: Address) -> Result<U256, Vec<u8>> {
        let sender = self.vm().msg_sender();
        self.deposit_from(sender, receiver, assets)
    }

    pub fn mint(&mut self, shares: U256, receiver: Address) -> Result<U256, Vec<u8>> {
        let assets = self.preview_mint(shares);
        let sender = self.vm().msg_sender();
        self.deposit_from(sender, receiver, assets)
    }

    pub fn withdraw_to(
        &mut self,
        assets: U256,
        receiver: Address,
        owner: Address,
    ) -> Result<U256, Vec<u8>> {
        self.redeem_from(self.preview_withdraw(assets), receiver, owner)
    }

    pub fn redeem(
        &mut self,
        shares: U256,
        receiver: Address,
        owner: Address,
    ) -> Result<U256, Vec<u8>> {
        self.redeem_from(shares, receiver, owner)
    }

    pub fn total_assets(&self) -> Result<U256, Vec<u8>> {
        self.require_tokens_set()?;
        let balance = IERC20Like::balanceOfCall {
            account: self.vm().contract_address(),
        };
        call_u256(
            self.usdc.get(),
            balance.abi_encode(),
            b"USDC_BALANCE_FAILED",
        )
    }

    pub fn total_supply(&self) -> Result<U256, Vec<u8>> {
        self.require_tokens_set()?;
        let supply = IERC20Like::totalSupplyCall {};
        call_u256(
            self.chips.get(),
            supply.abi_encode(),
            b"CHIPS_SUPPLY_FAILED",
        )
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

    pub fn max_withdraw(&self, owner: Address) -> Result<U256, Vec<u8>> {
        self.require_tokens_set()?;
        let balance = IERC20Like::balanceOfCall { account: owner };
        call_u256(
            self.chips.get(),
            balance.abi_encode(),
            b"CHIPS_BALANCE_FAILED",
        )
    }

    pub fn max_redeem(&self, owner: Address) -> Result<U256, Vec<u8>> {
        self.max_withdraw(owner)
    }

    fn deposit_from(
        &mut self,
        sender: Address,
        receiver: Address,
        amount: U256,
    ) -> Result<U256, Vec<u8>> {
        if amount == U256::ZERO {
            return Err(b"ZERO_AMOUNT".to_vec());
        }
        if receiver == Address::ZERO {
            return Err(b"RECEIVER_ZERO".to_vec());
        }

        self.require_tokens_set()?;
        let cashier = self.vm().contract_address();
        let shares = self.preview_deposit(amount);

        let pull = IERC20Like::transferFromCall {
            from: sender,
            to: cashier,
            value: amount,
        };
        call_bool(
            self.usdc.get(),
            pull.abi_encode(),
            b"USDC_TRANSFER_FROM_FAILED",
        )?;

        let mint = IERC20Like::mintCall {
            to: receiver,
            value: shares,
        };
        call_bool(self.chips.get(), mint.abi_encode(), b"CHIPS_MINT_FAILED")?;

        stylus_core::log(
            self.vm(),
            Deposit {
                sender,
                owner: receiver,
                assets: amount,
                shares,
            },
        );

        Ok(shares)
    }

    fn redeem_from(
        &mut self,
        shares: U256,
        receiver: Address,
        owner: Address,
    ) -> Result<U256, Vec<u8>> {
        if shares == U256::ZERO {
            return Err(b"ZERO_AMOUNT".to_vec());
        }
        if receiver == Address::ZERO {
            return Err(b"RECEIVER_ZERO".to_vec());
        }

        self.require_tokens_set()?;
        let sender = self.vm().msg_sender();
        if owner != sender {
            return Err(b"OWNER_MUST_BE_SENDER".to_vec());
        }
        let assets = self.preview_redeem(shares);

        let burn = IERC20Like::burnCall {
            from: owner,
            value: shares,
        };
        call_bool(self.chips.get(), burn.abi_encode(), b"CHIPS_BURN_FAILED")?;

        let push = IERC20Like::transferCall {
            to: receiver,
            value: assets,
        };
        call_bool(self.usdc.get(), push.abi_encode(), b"USDC_TRANSFER_FAILED")?;

        stylus_core::log(
            self.vm(),
            Withdraw {
                sender,
                receiver,
                owner,
                assets,
                shares,
            },
        );

        Ok(assets)
    }

    fn only_owner(&self) -> Result<(), Vec<u8>> {
        if self.vm().msg_sender() != self.owner.get() {
            return Err(b"NOT_OWNER".to_vec());
        }
        Ok(())
    }

    fn require_tokens_set(&self) -> Result<(), Vec<u8>> {
        if self.usdc.get() == Address::ZERO {
            return Err(b"USDC_NOT_SET".to_vec());
        }
        if self.chips.get() == Address::ZERO {
            return Err(b"CHIPS_NOT_SET".to_vec());
        }
        Ok(())
    }
}

fn call_bool(to: Address, calldata: Vec<u8>, err: &[u8]) -> Result<(), Vec<u8>> {
    let output = unsafe { RawCall::new().flush_storage_cache().call(to, &calldata) }
        .map_err(|_| err.to_vec())?;
    if output.is_empty() {
        return Ok(());
    }
    let ok = bool::abi_decode(&output, true).map_err(|_| err.to_vec())?;
    if ok {
        Ok(())
    } else {
        Err(err.to_vec())
    }
}

fn call_u256(to: Address, calldata: Vec<u8>, err: &[u8]) -> Result<U256, Vec<u8>> {
    let output = unsafe { RawCall::new().flush_storage_cache().call(to, &calldata) }
        .map_err(|_| err.to_vec())?;
    U256::abi_decode(&output, true).map_err(|_| err.to_vec())
}
