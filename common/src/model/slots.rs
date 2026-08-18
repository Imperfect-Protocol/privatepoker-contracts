use alloy_primitives::{uint, U32};
use stylus_sdk::{alloy_primitives::U256, keccak_const};

pub const VERSION_NUMBER: U32 = uint!(1_U32);

pub const MAIN_LOBBY_SLOT: U256 = {
    const HASH: [u8; 32] = keccak_const::Keccak256::new()
        .update(b"PrivatePoker.MainLobby")
        .finalize();
    U256::from_be_bytes(HASH).wrapping_sub(uint!(1_U256))
};

pub const PRIVATE_POKER_DIAMOND_SLOT: U256 = {
    const HASH: [u8; 32] = keccak_const::Keccak256::new()
        .update(b"PrivatePoker.Diamond")
        .finalize();
    U256::from_be_bytes(HASH).wrapping_sub(uint!(1_U256))
};

pub const PRIVATE_POKER_CHIPS_SLOT: U256 = {
    const HASH: [u8; 32] = keccak_const::Keccak256::new()
        .update(b"PrivatePoker.Chips")
        .finalize();
    U256::from_be_bytes(HASH).wrapping_sub(uint!(1_U256))
};

pub const PRIVATE_POKER_CASHIER_SLOT: U256 = {
    const HASH: [u8; 32] = keccak_const::Keccak256::new()
        .update(b"PrivatePoker.Cashier")
        .finalize();
    U256::from_be_bytes(HASH).wrapping_sub(uint!(1_U256))
};

pub const PRIVATE_POKER_ACCOUNTS_SLOT: U256 = {
    const HASH: [u8; 32] = keccak_const::Keccak256::new()
        .update(b"PrivatePoker.Accounts")
        .finalize();
    U256::from_be_bytes(HASH).wrapping_sub(uint!(1_U256))
};
