use alloc::{format, vec::Vec};

use alloy_primitives::Address;
use alloy_sol_types::SolCall;
use privatepoker_common::{
    calls::ContractCalls,
    erc20,
    interfaces::{
        IPrivatePokerAccountFacet, IPrivatePokerAggregatePubKeyFacet, IPrivatePokerCashierFacet,
        IPrivatePokerChipsFacet, IPrivatePokerHandFacet, IPrivatePokerLobbyFacet,
        IPrivatePokerSettlerFacet, IPrivatePokerSpectateFacet, IPrivatePokerTableFacet,
    },
    lobby::{
        MainLobby, PrivatePokerAccountsStorage, PrivatePokerCashierStorage,
        PrivatePokerChipsStorage, PrivatePokerDiamond as DiamondStorage,
    },
};
use stylus_sdk::{prelude::*, ArbResult};

#[storage]
#[entrypoint]
pub struct PrivatePokerDiamond;

#[public]
impl PrivatePokerDiamond {
    #[constructor]
    fn constructor(
        &mut self,
        initial_owner: Address,
        lobby_facet: Address,
        table_facet: Address,
        hand_facet: Address,
        spectate_facet: Address,
        account_facet: Address,
        cashier_facet: Address,
        chips_facet: Address,
        settler_facet: Address,
        aggregate_pub_key_facet: Address,
        verify_signature: Address,
        usdc: Address,
    ) -> Result<(), Vec<u8>> {
        ensure_not_zero(lobby_facet, b"LOBBY_FACET_ZERO")?;
        ensure_not_zero(table_facet, b"TABLE_FACET_ZERO")?;
        ensure_not_zero(hand_facet, b"HAND_FACET_ZERO")?;
        ensure_not_zero(spectate_facet, b"SPECTATE_FACET_ZERO")?;
        ensure_not_zero(account_facet, b"ACCOUNT_FACET_ZERO")?;
        ensure_not_zero(cashier_facet, b"CASHIER_FACET_ZERO")?;
        ensure_not_zero(chips_facet, b"CHIPS_FACET_ZERO")?;
        ensure_not_zero(settler_facet, b"SETTLER_FACET_ZERO")?;
        ensure_not_zero(aggregate_pub_key_facet, b"AGGREGATE_PUB_KEY_FACET_ZERO")?;
        ensure_not_zero(verify_signature, b"VERIFY_SIGNATURE_ZERO")?;
        ensure_not_zero(usdc, b"USDC_ZERO")?;

        let diamond = self.vm().contract_address();

        let mut main_lobby = MainLobby::storage_slot();
        main_lobby.owner.set(initial_owner);
        main_lobby.chip_token.set(diamond);

        let mut diamond_storage = DiamondStorage::storage_slot();
        diamond_storage.lobby.set(lobby_facet);
        diamond_storage.table.set(table_facet);
        diamond_storage.hand.set(hand_facet);
        diamond_storage.spectate.set(spectate_facet);
        diamond_storage.account.set(account_facet);
        diamond_storage.cashier.set(cashier_facet);
        diamond_storage.chips.set(chips_facet);
        diamond_storage.settler.set(settler_facet);
        diamond_storage
            .aggregate_pub_key
            .set(aggregate_pub_key_facet);
        diamond_storage.verify_signature.set(verify_signature);

        let mut chips = PrivatePokerChipsStorage::storage_slot();
        erc20::init_token(
            &mut chips.token,
            initial_owner,
            "Private Poker Chips",
            "CHIPS",
            6,
        );
        chips.cashier.set(diamond);
        chips.lobby.set(diamond);
        chips.account.set(diamond);

        let mut cashier = PrivatePokerCashierStorage::storage_slot();
        cashier.owner.set(initial_owner);
        cashier.usdc.set(usdc);
        cashier.chips.set(diamond);

        let mut accounts = PrivatePokerAccountsStorage::storage_slot();
        accounts.owner.set(initial_owner);
        accounts.usdc.set(usdc);
        accounts.chips.set(diamond);
        accounts.cashier.set(diamond);
        Ok(())
    }

    pub fn lobby_facet(&self) -> Address {
        DiamondStorage::storage_slot().lobby.get()
    }

    pub fn table_facet(&self) -> Address {
        DiamondStorage::storage_slot().table.get()
    }

    pub fn hand_facet(&self) -> Address {
        DiamondStorage::storage_slot().hand.get()
    }

    pub fn spectate_facet(&self) -> Address {
        DiamondStorage::storage_slot().spectate.get()
    }

    pub fn account_facet(&self) -> Address {
        DiamondStorage::storage_slot().account.get()
    }

    pub fn cashier_facet(&self) -> Address {
        DiamondStorage::storage_slot().cashier.get()
    }

    pub fn chips_facet(&self) -> Address {
        DiamondStorage::storage_slot().chips.get()
    }

    pub fn settler_facet(&self) -> Address {
        DiamondStorage::storage_slot().settler.get()
    }

    pub fn aggregate_pub_key_facet(&self) -> Address {
        DiamondStorage::storage_slot().aggregate_pub_key.get()
    }

    pub fn verify_signature(&self) -> Address {
        DiamondStorage::storage_slot().verify_signature.get()
    }

    #[payable]
    #[fallback]
    fn fallback(&mut self, calldata: &[u8]) -> ArbResult {
        if calldata.len() < 4 {
            return Err(b"CALLDATA_TOO_SHORT".to_vec());
        }

        let mut selector = [0u8; 4];
        selector.copy_from_slice(&calldata[0..4]);
        let facet = facet_for_selector(selector)?;

        unsafe { self.delegate_call_raw(facet, calldata) }
    }
}

fn facet_for_selector(selector: [u8; 4]) -> Result<Address, Vec<u8>> {
    let diamond = DiamondStorage::storage_slot();

    let facet = match facet_kind_for_selector(selector)? {
        FacetKind::Lobby => diamond.lobby.get(),
        FacetKind::Table => diamond.table.get(),
        FacetKind::Hand => diamond.hand.get(),
        FacetKind::Spectate => diamond.spectate.get(),
        FacetKind::Account => diamond.account.get(),
        FacetKind::Cashier => diamond.cashier.get(),
        FacetKind::Chips => diamond.chips.get(),
        FacetKind::Settler => diamond.settler.get(),
        FacetKind::AggregatePubKey => diamond.aggregate_pub_key.get(),
    };

    ensure_not_zero(facet, b"FACET_NOT_INSTALLED")?;
    Ok(facet)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FacetKind {
    Lobby,
    Table,
    Hand,
    Spectate,
    Account,
    Cashier,
    Chips,
    Settler,
    AggregatePubKey,
}

fn facet_kind_for_selector(selector: [u8; 4]) -> Result<FacetKind, Vec<u8>> {
    Ok(match selector {
        IPrivatePokerLobbyFacet::setChipTokenCall::SELECTOR
        | IPrivatePokerLobbyFacet::addLobbyCall::SELECTOR
        | IPrivatePokerLobbyFacet::removeLobbyCall::SELECTOR => FacetKind::Lobby,

        IPrivatePokerTableFacet::createTableCall::SELECTOR
        | IPrivatePokerTableFacet::joinTableCall::SELECTOR
        | IPrivatePokerTableFacet::removeTableCall::SELECTOR => FacetKind::Table,

        IPrivatePokerHandFacet::startHandCall::SELECTOR => FacetKind::Hand,

        IPrivatePokerAggregatePubKeyFacet::setTableAggregatePublicKeyCall::SELECTOR => {
            FacetKind::AggregatePubKey
        }

        IPrivatePokerSettlerFacet::settleHandCall::SELECTOR => FacetKind::Settler,

        IPrivatePokerSpectateFacet::getLobbyCountCall::SELECTOR
        | IPrivatePokerSpectateFacet::getLobbyAtCall::SELECTOR
        | IPrivatePokerSpectateFacet::getTableCountCall::SELECTOR
        | IPrivatePokerSpectateFacet::getTablesRangeCall::SELECTOR
        | IPrivatePokerSpectateFacet::getLobbyByIdCall::SELECTOR
        | IPrivatePokerSpectateFacet::getTableDetailCall::SELECTOR
        | IPrivatePokerSpectateFacet::getPlayerTablesCall::SELECTOR => FacetKind::Spectate,

        IPrivatePokerAccountFacet::subscribeCall::SELECTOR
        | IPrivatePokerAccountFacet::updateAccountCall::SELECTOR
        | IPrivatePokerAccountFacet::subscriptionPriceCall::SELECTOR
        | IPrivatePokerAccountFacet::subscriptionChipsCall::SELECTOR
        | IPrivatePokerAccountFacet::getAccountCall::SELECTOR
        | IPrivatePokerAccountFacet::isSubscriptionActiveCall::SELECTOR
        | IPrivatePokerAccountFacet::accountCountCall::SELECTOR
        | IPrivatePokerAccountFacet::accountAtCall::SELECTOR => FacetKind::Account,

        IPrivatePokerCashierFacet::ownerCall::SELECTOR
        | IPrivatePokerCashierFacet::usdcCall::SELECTOR
        | IPrivatePokerCashierFacet::assetCall::SELECTOR
        | IPrivatePokerCashierFacet::chipsCall::SELECTOR
        | IPrivatePokerCashierFacet::shareCall::SELECTOR
        | IPrivatePokerCashierFacet::depositFromCall::SELECTOR
        | IPrivatePokerCashierFacet::totalAssetsCall::SELECTOR
        | IPrivatePokerCashierFacet::totalSupplyCall::SELECTOR
        | IPrivatePokerCashierFacet::convertToSharesCall::SELECTOR
        | IPrivatePokerCashierFacet::convertToAssetsCall::SELECTOR
        | IPrivatePokerCashierFacet::previewDepositCall::SELECTOR
        | IPrivatePokerCashierFacet::previewMintCall::SELECTOR
        | IPrivatePokerCashierFacet::previewWithdrawCall::SELECTOR
        | IPrivatePokerCashierFacet::previewRedeemCall::SELECTOR
        | IPrivatePokerCashierFacet::maxDepositCall::SELECTOR
        | IPrivatePokerCashierFacet::maxMintCall::SELECTOR
        | IPrivatePokerCashierFacet::maxWithdrawCall::SELECTOR
        | IPrivatePokerCashierFacet::maxRedeemCall::SELECTOR => FacetKind::Cashier,

        IPrivatePokerChipsFacet::nameCall::SELECTOR
        | IPrivatePokerChipsFacet::symbolCall::SELECTOR
        | IPrivatePokerChipsFacet::decimalsCall::SELECTOR
        | IPrivatePokerChipsFacet::cashierCall::SELECTOR
        | IPrivatePokerChipsFacet::lobbyCall::SELECTOR
        | IPrivatePokerChipsFacet::accountCall::SELECTOR
        | IPrivatePokerChipsFacet::balanceOfCall::SELECTOR
        | IPrivatePokerChipsFacet::allowanceCall::SELECTOR
        | IPrivatePokerChipsFacet::approveCall::SELECTOR
        | IPrivatePokerChipsFacet::transferCall::SELECTOR
        | IPrivatePokerChipsFacet::transferFromCall::SELECTOR
        | IPrivatePokerChipsFacet::mintCall::SELECTOR
        | IPrivatePokerChipsFacet::burnCall::SELECTOR => FacetKind::Chips,

        _ => {
            return Err(format!("FUNCTION_NOT_FOUND: 0x{}", hex::encode(selector)).into_bytes());
        }
    })
}

fn ensure_not_zero(address: Address, error: &[u8]) -> Result<(), Vec<u8>> {
    if address == Address::ZERO {
        Err(error.to_vec())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_lobby_selectors_to_lobby_facet() {
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerLobbyFacet::setChipTokenCall::SELECTOR),
            Ok(FacetKind::Lobby)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerLobbyFacet::addLobbyCall::SELECTOR),
            Ok(FacetKind::Lobby)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerLobbyFacet::removeLobbyCall::SELECTOR),
            Ok(FacetKind::Lobby)
        );
    }

    #[test]
    fn routes_table_selectors_to_table_facet() {
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerTableFacet::createTableCall::SELECTOR),
            Ok(FacetKind::Table)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerTableFacet::joinTableCall::SELECTOR),
            Ok(FacetKind::Table)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerTableFacet::removeTableCall::SELECTOR),
            Ok(FacetKind::Table)
        );
    }

    #[test]
    fn routes_hand_selectors_to_hand_facet() {
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerHandFacet::startHandCall::SELECTOR),
            Ok(FacetKind::Hand)
        );
    }

    #[test]
    fn routes_aggregate_pub_key_selectors_to_aggregate_pub_key_facet() {
        assert_eq!(
            facet_kind_for_selector(
                IPrivatePokerAggregatePubKeyFacet::setTableAggregatePublicKeyCall::SELECTOR
            ),
            Ok(FacetKind::AggregatePubKey)
        );
    }

    #[test]
    fn routes_settler_selectors_to_settler_facet() {
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerSettlerFacet::settleHandCall::SELECTOR),
            Ok(FacetKind::Settler)
        );
    }

    #[test]
    fn routes_spectate_selectors_to_spectate_facet() {
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerSpectateFacet::getLobbyCountCall::SELECTOR),
            Ok(FacetKind::Spectate)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerSpectateFacet::getLobbyAtCall::SELECTOR),
            Ok(FacetKind::Spectate)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerSpectateFacet::getTableCountCall::SELECTOR),
            Ok(FacetKind::Spectate)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerSpectateFacet::getTablesRangeCall::SELECTOR),
            Ok(FacetKind::Spectate)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerSpectateFacet::getLobbyByIdCall::SELECTOR),
            Ok(FacetKind::Spectate)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerSpectateFacet::getTableDetailCall::SELECTOR),
            Ok(FacetKind::Spectate)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerSpectateFacet::getPlayerTablesCall::SELECTOR),
            Ok(FacetKind::Spectate)
        );
    }

    #[test]
    fn routes_account_selectors_to_account_facet() {
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerAccountFacet::subscribeCall::SELECTOR),
            Ok(FacetKind::Account)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerAccountFacet::getAccountCall::SELECTOR),
            Ok(FacetKind::Account)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerAccountFacet::isSubscriptionActiveCall::SELECTOR),
            Ok(FacetKind::Account)
        );
    }

    #[test]
    fn routes_cashier_selectors_to_cashier_facet() {
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerCashierFacet::assetCall::SELECTOR),
            Ok(FacetKind::Cashier)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerCashierFacet::depositFromCall::SELECTOR),
            Ok(FacetKind::Cashier)
        );
    }

    #[test]
    fn routes_chips_selectors_to_chips_facet() {
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerChipsFacet::balanceOfCall::SELECTOR),
            Ok(FacetKind::Chips)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerChipsFacet::approveCall::SELECTOR),
            Ok(FacetKind::Chips)
        );
        assert_eq!(
            facet_kind_for_selector(IPrivatePokerChipsFacet::transferFromCall::SELECTOR),
            Ok(FacetKind::Chips)
        );
    }

    #[test]
    fn rejects_unknown_selector() {
        assert!(facet_kind_for_selector([0xde, 0xad, 0xbe, 0xef]).is_err());
    }
}
