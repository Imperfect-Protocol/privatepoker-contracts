use alloc::vec;

use alloy_sol_types::sol;
use stylus_sdk::prelude::*;

sol! {
    interface IPrivatePokerLobbyFacet {
        function setChipToken(address chip_token) external;
        function addLobby(uint256 id, uint256 game_type, uint256 flags, string name) external;
        function removeLobby(uint256 id) external;
    }

    interface IPrivatePokerTableFacet {
        function createTable(uint256 lobby_id, uint256 table_id, string name, uint256 buy_in, uint256 num_players, address player_address, bytes annonce_public_key) external;
        function joinTable(uint256 lobby_id, uint256 table_id, address player_address, bytes annonce_public_key) external;
        function removeTable(uint256 lobby_id, uint256 table_id) external;
    }

    interface IPrivatePokerHandFacet {
        function startHand(uint256 lobby_id, uint256 table_id) external;
    }

    interface IPrivatePokerAggregatePubKeyFacet {
        function setTableAggregatePublicKey(uint256 lobby_id, uint256 table_id, bytes aggregate_public_key) external;
    }

    interface IPrivatePokerSettlerFacet {
        function settleHand(uint256 lobby_id, uint256 table_id, uint256 hand_id, uint256 pot_size, uint256[] pot_split, uint256[] chips_balances, bytes digest, bytes aggregate_public_key) external returns (uint256);
    }

    interface IPrivatePokerSignatory {
        function verifySignedCalldata(bytes signed_calldata) external returns (bool);
    }

    interface IPrivatePokerHashToCurve {
        function toCurve(bytes digest) external returns (bytes);
    }

    interface IPrivatePokerVerifySignature {
        function verifySignature(bytes hashed_message, bytes aggregate_public_key, bytes aggregate_signature) external returns (bool);
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
        function subscribe(address player_address, address operator, string display_name, bytes annonce_public_key, bytes encrypted_profile, uint8 subscription_tier) external;
        function updateAccount(address player_address, string display_name, bytes annonce_public_key, bytes encrypted_profile) external;
        function createAccount(address player_address, string display_name, bytes encrypted_profile) external;
        function setAccountStatus(address player_address, uint256 flags) external;
        function getAccountStatus(address player_address) external view returns (uint256);
        function accountStatusChangedAt(address player_address) external view returns (uint256);
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
        uint256 table_current_hand;
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
        uint256 flags;
        uint256 status_changed_at;
        string display_name;
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
    event AccountStatusChanged(address player_address, uint256 flags);
    event SubscriptionPaid(address player_address, uint8 subscription_tier, uint256 usdc_amount, uint256 chips_amount, uint256 paid_at, uint256 expires_at);
}

sol_interface! {
    interface IPokerChips {
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
    }
}
