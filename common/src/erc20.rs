use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

use alloy_primitives::{Address, Uint, U256};
use alloy_sol_types::sol;
use stylus_sdk::{
    prelude::*,
    storage::{StorageAddress, StorageMap, StorageString, StorageU256, StorageU8},
};

#[storage]
pub struct Erc20Storage {
    pub owner: StorageAddress,
    pub name: StorageString,
    pub symbol: StorageString,
    pub decimals: StorageU8,
    pub total_supply: StorageU256,
    pub balances: StorageMap<Address, StorageU256>,
    pub allowances: StorageMap<Address, StorageMap<Address, StorageU256>>,
}

sol! {
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event Transfer(address indexed from, address indexed to, uint256 value);
}

pub fn init_token(
    token: &mut Erc20Storage,
    owner: Address,
    name: &str,
    symbol: &str,
    decimals: u8,
) {
    token.owner.set(owner);
    token.name.set_str(name.to_string());
    token.symbol.set_str(symbol.to_string());
    token.decimals.set(Uint::<8, 1>::from(decimals));
}

pub fn name(token: &Erc20Storage) -> String {
    token.name.get_string()
}

pub fn symbol(token: &Erc20Storage) -> String {
    token.symbol.get_string()
}

pub fn decimals(token: &Erc20Storage) -> u8 {
    token.decimals.get().to::<u8>()
}

pub fn owner(token: &Erc20Storage) -> Address {
    token.owner.get()
}

pub fn total_supply(token: &Erc20Storage) -> U256 {
    token.total_supply.get()
}

pub fn balance_of(token: &Erc20Storage, account: Address) -> U256 {
    token.balances.get(account)
}

pub fn allowance(token: &Erc20Storage, owner: Address, spender: Address) -> U256 {
    token.allowances.get(owner).get(spender)
}

pub fn approve(token: &mut Erc20Storage, owner: Address, spender: Address, value: U256) {
    token.allowances.setter(owner).insert(spender, value);
}

pub fn spend_allowance(
    token: &mut Erc20Storage,
    owner: Address,
    spender: Address,
    value: U256,
) -> Result<(), Vec<u8>> {
    let current = allowance(token, owner, spender);
    if current < value {
        return Err(b"ERC20_INSUFFICIENT_ALLOWANCE".to_vec());
    }
    token
        .allowances
        .setter(owner)
        .insert(spender, current - value);
    Ok(())
}

pub fn transfer(
    token: &mut Erc20Storage,
    from: Address,
    to: Address,
    value: U256,
) -> Result<(), Vec<u8>> {
    if to == Address::ZERO {
        return Err(b"ERC20_TRANSFER_TO_ZERO".to_vec());
    }

    let from_balance = token.balances.get(from);
    if from_balance < value {
        return Err(b"ERC20_INSUFFICIENT_BALANCE".to_vec());
    }

    token.balances.insert(from, from_balance - value);
    let to_balance = token.balances.get(to);
    token.balances.insert(to, to_balance + value);
    Ok(())
}

pub fn mint(token: &mut Erc20Storage, to: Address, value: U256) -> Result<(), Vec<u8>> {
    if to == Address::ZERO {
        return Err(b"ERC20_MINT_TO_ZERO".to_vec());
    }

    token.total_supply.set(token.total_supply.get() + value);
    token.balances.insert(to, token.balances.get(to) + value);
    Ok(())
}

pub fn burn(token: &mut Erc20Storage, from: Address, value: U256) -> Result<(), Vec<u8>> {
    let balance = token.balances.get(from);
    if balance < value {
        return Err(b"ERC20_INSUFFICIENT_BALANCE".to_vec());
    }

    token.balances.insert(from, balance - value);
    token.total_supply.set(token.total_supply.get() - value);
    Ok(())
}
