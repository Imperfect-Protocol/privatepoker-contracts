use alloc::{vec, vec::Vec};

use alloy_primitives::{uint, Address, U32};
use alloy_sol_types::sol;
use stylus_sdk::{
    alloy_primitives::U256,
    keccak_const,
    prelude::*,
    storage::{
        StorageAddress, StorageBytes, StorageMap, StorageString, StorageU256, StorageU8, StorageVec,
    },
};

use super::erc20;

#[storage]
pub struct TablePlayer {
    pub address: StorageAddress,
    pub chips_remain: StorageU256,
    pub annonce_public_key: StorageBytes,
    pub operator: StorageAddress,
}

#[storage]
pub struct Hand {
    pub pot_size: StorageU256,
    pub pot_split: StorageVec<StorageU256>,
    pub digest: StorageBytes,
    pub aggregate_signature: StorageBytes,
}

#[storage]
pub struct Table {
    pub owner: StorageAddress,
    pub id: StorageU256,
    pub flags: StorageU256,
    pub name: StorageString,
    pub buy_in: StorageU256,
    pub aggregate_public_key: StorageBytes,
    pub players: StorageVec<TablePlayer>,
    pub total_buyin: StorageU256,
    pub current_hand: StorageU256,
    pub hands: StorageMap<U256, Hand>,
    pub hand_start_ready_count: StorageU256,
    pub hand_start_ready: StorageMap<Address, StorageU256>,
    pub public_key_ready_count: StorageMap<U256, StorageU256>,
    pub public_key_ready: StorageMap<Address, StorageU256>,
}

#[storage]
pub struct Lobby {
    pub id: StorageU256,
    pub game_type: StorageU256,
    pub flags: StorageU256,
    pub name: StorageString,
    pub table_ids: StorageVec<StorageU256>,
    pub tables: StorageMap<U256, Table>,
    pub total_volume: StorageU256,
    pub total_players: StorageU256,
}

#[storage]
pub struct MainLobby {
    pub owner: StorageAddress,
    pub lobby_ids: StorageVec<StorageU256>,
    pub lobbies: StorageMap<U256, Lobby>,
    pub player_tables: StorageMap<Address, StorageVec<StorageU256>>,
    pub chip_token: StorageAddress,
    pub facets: PrivatePokerFacetAddresses,
}

#[storage]
pub struct PrivatePokerFacetAddresses {
    pub lobby: StorageAddress,
    pub table: StorageAddress,
    pub hand: StorageAddress,
    pub spectate: StorageAddress,
    pub account: StorageAddress,
    pub cashier: StorageAddress,
    pub chips: StorageAddress,
    pub settler: StorageAddress,
    pub aggregate_pub_key: StorageAddress,
    pub verify_signature: StorageAddress,
}

#[storage]
pub struct PrivatePokerChipsStorage {
    pub token: erc20::Erc20Storage,
    pub cashier: StorageAddress,
    pub lobby: StorageAddress,
    pub account: StorageAddress,
}

#[storage]
pub struct PrivatePokerCashierStorage {
    pub owner: StorageAddress,
    pub usdc: StorageAddress,
    pub chips: StorageAddress,
    pub accounted_assets: StorageU256,
}

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

sol! {
    interface IPrivatePokerLobbyFacet {
        function setChipToken(address chip_token) external;
        function addLobby(uint256 id, uint256 game_type, uint256 flags, string name) external;
        function removeLobby(uint256 id) external;
    }

    interface IPrivatePokerTableFacet {
        function createTable(uint256 lobby_id, uint256 table_id, string name, uint256 buy_in, uint256 num_players, address player_address, address operator, bytes annonce_public_key) external;
        function joinTable(uint256 lobby_id, uint256 table_id, address player_address, address operator, bytes annonce_public_key) external;
        function removeTable(uint256 lobby_id, uint256 table_id) external;
    }

    interface IPrivatePokerHandFacet {
        function startHand(uint256 lobby_id, uint256 table_id) external;
    }

    interface IPrivatePokerAggregatePubKeyFacet {
        function setTableAggregatePublicKey(uint256 lobby_id, uint256 table_id, bytes aggregate_public_key, bytes aggregate_signature) external;
    }

    interface IPrivatePokerSettlerFacet {
        function settleHand(uint256 lobby_id, uint256 table_id, uint256 hand_id, uint256 pot_size, uint256[] pot_split, uint256[] chips_balances, bytes digest, bytes aggregate_signature) external;
    }

    interface IPrivatePokerVerifySignature {
        function verifySignature(bytes digest, bytes aggregate_public_key, bytes aggregate_signature) external returns (bool);
    }

    interface IPrivatePokerSpectateFacet {
        function getLobbyCount() external view returns (uint256);
        function getLobbyAt(uint256 index) external view returns (bytes);
        function getTableCount(uint256 lobby_id) external view returns (uint256);
        function getTablesRange(uint256 lobby_id, uint256 offset, uint256 count) external view returns (bytes[]);
        function getLobbyById(uint256 lobby_id) external view returns (bytes);
        function getTableDetail(uint256 lobby_id, uint256 table_id) external view returns (bytes);
        function getPlayerTables(address player) external view returns (uint256[]);
    }

    interface IPrivatePokerAccountFacet {
        function subscribe(address player_address, address operator, bytes annonce_public_key, bytes encrypted_profile, uint8 subscription_tier) external;
        function updateAccount(address player_address, address operator, bytes annonce_public_key, bytes encrypted_profile) external;
        function subscriptionPrice(uint8 tier) external view returns (uint256);
        function subscriptionChips(uint8 tier) external view returns (uint256);
        function getAccount(address player_address) external view returns (bytes);
        function isSubscriptionActive(address player_address) external view returns (bool);
        function accountCount() external view returns (uint256);
        function accountAt(uint256 index) external view returns (address);
    }

    interface IPrivatePokerCashierFacet {
        function owner() external view returns (address);
        function usdc() external view returns (address);
        function asset() external view returns (address);
        function chips() external view returns (address);
        function share() external view returns (address);
        function depositFrom(address payer, address receiver, uint256 assets, uint256 shares) external returns (uint256);
        function totalAssets() external view returns (uint256);
        function totalSupply() external view returns (uint256);
        function convertToShares(uint256 assets) external pure returns (uint256);
        function convertToAssets(uint256 shares) external pure returns (uint256);
        function previewDeposit(uint256 assets) external pure returns (uint256);
        function previewMint(uint256 shares) external pure returns (uint256);
        function previewWithdraw(uint256 assets) external pure returns (uint256);
        function previewRedeem(uint256 shares) external pure returns (uint256);
        function maxDeposit(address receiver) external pure returns (uint256);
        function maxMint(address receiver) external pure returns (uint256);
        function maxWithdraw(address owner) external view returns (uint256);
        function maxRedeem(address owner) external view returns (uint256);
    }

    interface IPrivatePokerChipsFacet {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
        function owner() external view returns (address);
        function cashier() external view returns (address);
        function lobby() external view returns (address);
        function account() external view returns (address);
        function totalSupply() external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
        function allowance(address owner, address spender) external view returns (uint256);
        function approve(address spender, uint256 value) external returns (bool);
        function transfer(address to, uint256 value) external returns (bool);
        function transferFrom(address from, address to, uint256 value) external returns (bool);
        function mint(address to, uint256 value) external returns (bool);
        function burn(address from, uint256 value) external returns (bool);
    }

    struct LobbyInfo {
        uint256 lobby_id;
        uint256 lobby_game_type;
        uint256 lobby_flags;
        uint256 lobby_table_count;
        uint256 lobby_player_count;
        uint256 lobby_total_volume;
        string lobby_name;
    }

    struct TableInfo {
        uint256 table_id;
        uint256 table_flags;
        uint256 table_buyin;
        uint256 table_player_count;
        uint256 table_total_buyin;
        string table_name;
    }

    struct TablePlayerInfo {
        address player_address;
        address operator;
        uint256 player_chips;
        bytes player_annonce_public_key;
    }

    struct TableDetail {
        TableInfo info;
        TablePlayerInfo[] players;
    }

    struct AccountInfo {
        address player_address;
        address operator;
        bytes annonce_public_key;
        bytes encrypted_profile;
        uint8 subscription_tier;
        uint256 subscription_paid_at;
        uint256 subscription_expires_at;
    }

    event HandStarted(uint256 lobby_id, uint256 table_id, uint256 seat_number, uint256 remain_count);
    event TableAggregatePublicKeySet(uint256 lobby_id, uint256 table_id);
    event HandSettled(uint256 lobby_id, uint256 table_id, uint256 hand_id, bytes digest);
    event ChipTokenSet(address chip_token);
    event ChipsPaidOut(address recipient, uint256 amount);
    event LobbyCreated(uint256 id, string name);
    event TableCreated(uint256 id, uint256 lobby_id, string name, uint256 buy_in);
    event PlayerJoined(address player_address, uint256 lobby_id, uint256 table_id, string player_name, uint256 player_chips);
    event AccountUpdated(address player_address, address operator);
    event SubscriptionPaid(address player_address, uint8 subscription_tier, uint256 usdc_amount, uint256 chips_amount, uint256 paid_at, uint256 expires_at);
}

sol_interface! {
    interface IPokerChips {
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
    }
}

use super::storage::StorageSlot;

pub const VERSION_NUMBER: U32 = uint!(1_U32);

pub const MAIN_LOBBY_SLOT: U256 = {
    const HASH: [u8; 32] = keccak_const::Keccak256::new()
        .update(b"PrivatePoker.MainLobby")
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

impl MainLobby {
    pub fn storage_slot() -> MainLobby {
        StorageSlot::get_slot::<MainLobby>(MAIN_LOBBY_SLOT)
    }
}

impl PrivatePokerChipsStorage {
    pub fn storage_slot() -> PrivatePokerChipsStorage {
        StorageSlot::get_slot::<PrivatePokerChipsStorage>(PRIVATE_POKER_CHIPS_SLOT)
    }
}

impl PrivatePokerCashierStorage {
    pub fn storage_slot() -> PrivatePokerCashierStorage {
        StorageSlot::get_slot::<PrivatePokerCashierStorage>(PRIVATE_POKER_CASHIER_SLOT)
    }
}

impl PrivatePokerAccountsStorage {
    pub fn storage_slot() -> PrivatePokerAccountsStorage {
        StorageSlot::get_slot::<PrivatePokerAccountsStorage>(PRIVATE_POKER_ACCOUNTS_SLOT)
    }
}

pub fn small_blind_for_buy_in(buy_in: U256) -> U256 {
    let hundred = U256::from(100);
    let blind = buy_in / hundred;
    if blind == U256::ZERO && buy_in > U256::ZERO {
        U256::ONE
    } else {
        blind
    }
}

pub fn clear_table(table: &mut Table) {
    table.owner.erase();
    table.id.erase();
    table.flags.erase();
    table.name.erase();
    table.buy_in.erase();
    table.aggregate_public_key.erase();
    table.total_buyin.erase();
    table.current_hand.erase();
    table.hand_start_ready_count.erase();
    unsafe {
        table.players.set_len(0);
    }
}
