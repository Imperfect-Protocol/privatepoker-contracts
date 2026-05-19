use alloc::{format, vec::Vec};

use alloy_primitives::Address;
use alloy_sol_types::SolCall;
use privatepoker_common::lobby::{
    IPrivatePokerHandFacet, IPrivatePokerLobbyFacet, IPrivatePokerSpectateFacet,
    IPrivatePokerTableFacet, MainLobby,
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
    ) -> Result<(), Vec<u8>> {
        ensure_not_zero(lobby_facet, b"LOBBY_FACET_ZERO")?;
        ensure_not_zero(table_facet, b"TABLE_FACET_ZERO")?;
        ensure_not_zero(hand_facet, b"HAND_FACET_ZERO")?;
        ensure_not_zero(spectate_facet, b"SPECTATE_FACET_ZERO")?;

        let mut main_lobby = MainLobby::storage_slot();
        main_lobby.owner.set(initial_owner);
        main_lobby.facets.lobby.set(lobby_facet);
        main_lobby.facets.table.set(table_facet);
        main_lobby.facets.hand.set(hand_facet);
        main_lobby.facets.spectate.set(spectate_facet);
        Ok(())
    }

    pub fn lobby_facet(&self) -> Address {
        MainLobby::storage_slot().facets.lobby.get()
    }

    pub fn table_facet(&self) -> Address {
        MainLobby::storage_slot().facets.table.get()
    }

    pub fn hand_facet(&self) -> Address {
        MainLobby::storage_slot().facets.hand.get()
    }

    pub fn spectate_facet(&self) -> Address {
        MainLobby::storage_slot().facets.spectate.get()
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

        unsafe { Ok(self.vm().delegate_call(&self, facet, calldata)?) }
    }
}

fn facet_for_selector(selector: [u8; 4]) -> Result<Address, Vec<u8>> {
    let main_lobby = MainLobby::storage_slot();

    let facet = match facet_kind_for_selector(selector)? {
        FacetKind::Lobby => main_lobby.facets.lobby.get(),
        FacetKind::Table => main_lobby.facets.table.get(),
        FacetKind::Hand => main_lobby.facets.hand.get(),
        FacetKind::Spectate => main_lobby.facets.spectate.get(),
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

        IPrivatePokerSpectateFacet::getLobbyCountCall::SELECTOR
        | IPrivatePokerSpectateFacet::getLobbyAtCall::SELECTOR
        | IPrivatePokerSpectateFacet::getTableCountCall::SELECTOR
        | IPrivatePokerSpectateFacet::getTablesRangeCall::SELECTOR
        | IPrivatePokerSpectateFacet::getLobbyByIdCall::SELECTOR
        | IPrivatePokerSpectateFacet::getTableDetailCall::SELECTOR
        | IPrivatePokerSpectateFacet::getPlayerTablesCall::SELECTOR => FacetKind::Spectate,

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
    fn rejects_unknown_selector() {
        assert!(facet_kind_for_selector([0xde, 0xad, 0xbe, 0xef]).is_err());
    }
}
