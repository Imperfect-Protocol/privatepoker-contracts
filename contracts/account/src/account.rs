use alloc::{vec, vec::Vec};

use alloy_primitives::{Address, U8};
use alloy_sol_types::{sol, SolCall, SolValue};
use privatepoker_common::{
    erc20,
    lobby::{
        AccountInfo, AccountUpdated, PrivatePokerAccountsStorage, PrivatePokerCashierStorage,
        PrivatePokerChipsStorage, SubscriptionPaid,
    },
};
use stylus_sdk::{abi::Bytes, alloy_primitives::U256, prelude::*, stylus_core};

sol! {
    interface IERC20Like {
        function balanceOf(address account) external view returns (uint256);
    }
}

const STARTER: u8 = 1;
const REGULAR: u8 = 2;
const PROFESSIONAL: u8 = 3;

const USDC_SCALE: u64 = 1_000_000;
const CHIPS_SCALE: u64 = 1_000_000;
const SUBSCRIPTION_SECONDS: u64 = 30 * 24 * 60 * 60;

#[storage]
#[entrypoint]
pub struct PrivatePokerAccount;

#[public]
impl PrivatePokerAccount {
    #[constructor]
    fn constructor(
        &mut self,
        initial_owner: Address,
        usdc: Address,
        chips: Address,
    ) -> Result<(), Vec<u8>> {
        if initial_owner == Address::ZERO {
            return Err(b"OWNER_ZERO".to_vec());
        }
        if usdc == Address::ZERO {
            return Err(b"USDC_ZERO".to_vec());
        }
        if chips == Address::ZERO {
            return Err(b"CHIPS_ZERO".to_vec());
        }

        let mut accounts = PrivatePokerAccountsStorage::storage_slot();
        accounts.owner.set(initial_owner);
        accounts.usdc.set(usdc);
        accounts.chips.set(chips);
        accounts.cashier.set(Address::ZERO);
        Ok(())
    }

    pub fn owner(&self) -> Address {
        PrivatePokerAccountsStorage::storage_slot().owner.get()
    }

    pub fn usdc(&self) -> Address {
        PrivatePokerAccountsStorage::storage_slot().usdc.get()
    }

    pub fn chips(&self) -> Address {
        PrivatePokerAccountsStorage::storage_slot().chips.get()
    }

    pub fn cashier(&self) -> Address {
        PrivatePokerAccountsStorage::storage_slot().cashier.get()
    }

    pub fn set_tokens(
        &mut self,
        usdc: Address,
        chips: Address,
        cashier: Address,
    ) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        if usdc == Address::ZERO {
            return Err(b"USDC_ZERO".to_vec());
        }
        if chips == Address::ZERO {
            return Err(b"CHIPS_ZERO".to_vec());
        }
        if cashier == Address::ZERO {
            return Err(b"CASHIER_ZERO".to_vec());
        }

        let mut accounts = PrivatePokerAccountsStorage::storage_slot();
        accounts.usdc.set(usdc);
        accounts.chips.set(chips);
        accounts.cashier.set(cashier);
        Ok(())
    }

    pub fn subscription_price(&self, tier: u8) -> Result<U256, Vec<u8>> {
        match tier {
            STARTER => Ok(U256::from(10_u64 * USDC_SCALE)),
            REGULAR => Ok(U256::from(50_u64 * USDC_SCALE)),
            PROFESSIONAL => Ok(U256::from(200_u64 * USDC_SCALE)),
            _ => Err(b"BAD_TIER".to_vec()),
        }
    }

    pub fn subscription_chips(&self, tier: u8) -> Result<U256, Vec<u8>> {
        match tier {
            STARTER => Ok(U256::from(1_000_u64 * CHIPS_SCALE)),
            REGULAR => Ok(U256::from(10_000_u64 * CHIPS_SCALE)),
            PROFESSIONAL => Ok(U256::from(100_000_u64 * CHIPS_SCALE)),
            _ => Err(b"BAD_TIER".to_vec()),
        }
    }

    pub fn subscribe(
        &mut self,
        player_address: Address,
        operator: Address,
        annonce_public_key: Bytes,
        encrypted_profile: Bytes,
        subscription_tier: u8,
    ) -> Result<(), Vec<u8>> {
        self.require_player_or_operator(player_address, operator)?;
        self.require_addresses(player_address, operator)?;

        let usdc_amount = self.subscription_price(subscription_tier)?;
        let chips_amount = self.subscription_chips(subscription_tier)?;
        let paid_at = U256::from(self.vm().block_timestamp());
        let expires_at = paid_at + U256::from(SUBSCRIPTION_SECONDS);

        self.deposit_subscription(
            self.vm().contract_address(),
            player_address,
            usdc_amount,
            chips_amount,
        )?;
        self.write_account(
            player_address,
            operator,
            annonce_public_key,
            encrypted_profile,
            subscription_tier,
            paid_at,
            expires_at,
        );

        stylus_core::log(
            self.vm(),
            SubscriptionPaid {
                player_address,
                subscription_tier,
                usdc_amount,
                chips_amount,
                paid_at,
                expires_at,
            },
        );

        Ok(())
    }

    pub fn update_account(
        &mut self,
        player_address: Address,
        operator: Address,
        annonce_public_key: Bytes,
        encrypted_profile: Bytes,
    ) -> Result<(), Vec<u8>> {
        self.require_player_or_existing_operator(player_address, operator)?;
        self.require_addresses(player_address, operator)?;

        let mut accounts = PrivatePokerAccountsStorage::storage_slot();
        let mut account = accounts.accounts.setter(player_address);
        if account.exists.get() == U256::ZERO {
            return Err(b"ACCOUNT_MISSING".to_vec());
        }

        account.operator.set(operator);
        account
            .annonce_public_key
            .set_bytes(annonce_public_key.as_ref());
        account
            .encrypted_profile
            .set_bytes(encrypted_profile.as_ref());
        accounts
            .operator_players
            .setter(operator)
            .set(player_address);

        stylus_core::log(
            self.vm(),
            AccountUpdated {
                player_address,
                operator,
            },
        );
        Ok(())
    }

    pub fn get_account(&self, player_address: Address) -> Result<Bytes, Vec<u8>> {
        let accounts = PrivatePokerAccountsStorage::storage_slot();
        let account = accounts.accounts.get(player_address);
        if account.exists.get() == U256::ZERO {
            return Err(b"ACCOUNT_MISSING".to_vec());
        }

        let info = AccountInfo {
            player_address: account.player_address.get(),
            operator: account.operator.get(),
            annonce_public_key: account.annonce_public_key.get_bytes().into(),
            encrypted_profile: account.encrypted_profile.get_bytes().into(),
            subscription_tier: account.subscription_tier.get().to::<u8>(),
            subscription_paid_at: account.subscription_paid_at.get(),
            subscription_expires_at: account.subscription_expires_at.get(),
        };

        Ok(info.abi_encode().into())
    }

    pub fn is_subscription_active(&self, player_address: Address) -> bool {
        let accounts = PrivatePokerAccountsStorage::storage_slot();
        let account = accounts.accounts.get(player_address);
        if account.exists.get() == U256::ZERO {
            return false;
        }
        account.subscription_expires_at.get() > U256::from(self.vm().block_timestamp())
    }

    pub fn account_count(&self) -> U256 {
        U256::from(PrivatePokerAccountsStorage::storage_slot().players.len())
    }

    pub fn account_at(&self, index: U256) -> Result<Address, Vec<u8>> {
        let accounts = PrivatePokerAccountsStorage::storage_slot();
        let index = usize::try_from(index).map_err(|_| b"INDEX_TOO_BIG".to_vec())?;
        if index >= accounts.players.len() {
            return Err(b"INDEX_OUT_OF_BOUNDS".to_vec());
        }
        Ok(accounts.players.get(index).unwrap())
    }

    fn write_account(
        &mut self,
        player_address: Address,
        operator: Address,
        annonce_public_key: Bytes,
        encrypted_profile: Bytes,
        subscription_tier: u8,
        paid_at: U256,
        expires_at: U256,
    ) {
        let mut accounts = PrivatePokerAccountsStorage::storage_slot();
        let mut account = accounts.accounts.setter(player_address);
        let is_new = account.exists.get() == U256::ZERO;
        if is_new {
            account.exists.set(U256::ONE);
            account.player_address.set(player_address);
            accounts.players.push(player_address);
        } else {
            let previous_operator = account.operator.get();
            if previous_operator != Address::ZERO
                && previous_operator != operator
                && accounts.operator_players.get(previous_operator) == player_address
            {
                accounts.operator_players.delete(previous_operator);
            }
        }

        account.operator.set(operator);
        account
            .annonce_public_key
            .set_bytes(annonce_public_key.as_ref());
        account
            .encrypted_profile
            .set_bytes(encrypted_profile.as_ref());
        account.subscription_tier.set(U8::from(subscription_tier));
        account.subscription_paid_at.set(paid_at);
        account.subscription_expires_at.set(expires_at);
        accounts
            .operator_players
            .setter(operator)
            .set(player_address);

        stylus_core::log(
            self.vm(),
            AccountUpdated {
                player_address,
                operator,
            },
        );
    }

    fn deposit_subscription(
        &mut self,
        payer: Address,
        receiver: Address,
        assets: U256,
        shares: U256,
    ) -> Result<(), Vec<u8>> {
        if payer != self.vm().contract_address() {
            return Err(b"PAYER_NOT_DIAMOND".to_vec());
        }
        if receiver == Address::ZERO {
            return Err(b"RECEIVER_ZERO".to_vec());
        }
        if assets == U256::ZERO {
            return Err(b"ZERO_AMOUNT".to_vec());
        }
        if shares == U256::ZERO {
            return Err(b"ZERO_SHARES".to_vec());
        }

        let usdc = PrivatePokerCashierStorage::storage_slot().usdc.get();
        if usdc == Address::ZERO {
            return Err(b"USDC_NOT_SET".to_vec());
        }

        let diamond = self.vm().contract_address();
        let accounted_assets = PrivatePokerCashierStorage::storage_slot()
            .accounted_assets
            .get();
        let balance = IERC20Like::balanceOfCall { account: diamond };
        let balance = call_u256(self, usdc, balance.abi_encode(), b"USDC_BALANCE_FAILED")?;
        if balance < accounted_assets + assets {
            return Err(b"USDC_NOT_RECEIVED".to_vec());
        }

        PrivatePokerCashierStorage::storage_slot()
            .accounted_assets
            .set(accounted_assets + assets);

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
        Ok(())
    }

    fn only_owner(&self) -> Result<(), Vec<u8>> {
        if self.vm().msg_sender() != self.owner() {
            return Err(b"NOT_OWNER".to_vec());
        }
        Ok(())
    }

    fn require_addresses(&self, player_address: Address, operator: Address) -> Result<(), Vec<u8>> {
        if player_address == Address::ZERO {
            return Err(b"PLAYER_ZERO".to_vec());
        }
        if operator == Address::ZERO {
            return Err(b"OPERATOR_ZERO".to_vec());
        }
        Ok(())
    }

    fn require_player_or_operator(
        &self,
        player_address: Address,
        operator: Address,
    ) -> Result<(), Vec<u8>> {
        let sender = self.vm().msg_sender();
        if sender != player_address && sender != operator {
            return Err(b"NOT_PLAYER_OR_OPERATOR".to_vec());
        }
        Ok(())
    }

    fn require_player_or_existing_operator(
        &self,
        player_address: Address,
        operator: Address,
    ) -> Result<(), Vec<u8>> {
        let sender = self.vm().msg_sender();
        let accounts = PrivatePokerAccountsStorage::storage_slot();
        let account = accounts.accounts.get(player_address);
        let current_operator = account.operator.get();
        if sender != player_address && sender != operator && sender != current_operator {
            return Err(b"NOT_PLAYER_OR_OPERATOR".to_vec());
        }
        Ok(())
    }
}

fn call_u256(
    ctx: &mut PrivatePokerAccount,
    to: Address,
    calldata: Vec<u8>,
    err: &[u8],
) -> Result<U256, Vec<u8>> {
    let output = ctx
        .vm()
        .call(&ctx, to, &calldata)
        .map_err(|_| err.to_vec())?;
    U256::abi_decode(&output, true).map_err(|_| err.to_vec())
}
