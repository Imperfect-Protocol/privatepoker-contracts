use alloc::vec::Vec;

use alloy_primitives::U8;
use bls12_381::{G1Affine, G2Affine};
use pairing::{group::Group, MultiMillerLoop};
use stylus_sdk::{abi::Bytes, prelude::*};

#[storage]
#[entrypoint]
pub struct PrivatePokerVerifyUnmasking;

#[public]
impl PrivatePokerVerifyUnmasking {
    pub fn verify_unmasking(
        &mut self,
        num_players: U8,
        player_keys: Vec<Bytes>,
        shuffle_history: Vec<Vec<Bytes>>,
        unmasking_sequence_cards: Vec<Vec<Vec<Bytes>>>,
        unmasking_actors: Vec<U8>,
        unmasking_states: Vec<U8>,
    ) -> Result<(), Vec<u8>> {
        let mut v_player_keys = Vec::with_capacity(player_keys.len());
        for i in 0..player_keys.len() {
            let Ok(key) = make_g2_from_compressed_slice(&player_keys[i].0) else {
                return Err(b"INVALID_PLAYER_KEY")?;
            };
            v_player_keys.push(key);
        }

        let mut v_shuffle_history = Vec::new();
        for i in 0..shuffle_history.len() {
            let cards = &shuffle_history[i];
            let mut v_cards = Vec::with_capacity(cards.len());
            for j in 0..cards.len() {
                let Ok(v_card) = make_g1_from_compressed_slice(&cards[j].0) else {
                    return Err(b"INVALID_SHUFFLE_HISTORY")?;
                };
                v_cards.push(v_card);
            }
            v_shuffle_history.push(v_cards);
        }

        let mut v_unmasking_sequence_cards = Vec::with_capacity(unmasking_sequence_cards.len());
        for i in 0..unmasking_sequence_cards.len() {
            let unmasking_stage = &unmasking_sequence_cards[i];
            let mut v_unmasking_stage = Vec::with_capacity(unmasking_stage.len());
            for j in 0..unmasking_stage.len() {
                let player_cards = &unmasking_stage[j];
                let mut v_player_cards = Vec::with_capacity(player_cards.len());
                for k in 0..player_cards.len() {
                    let Ok(v_card) = make_g1_from_compressed_slice(&player_cards[k].0) else {
                        return Err(b"INVALID_SHUFFLE_HISTORY")?;
                    };
                    v_player_cards.push(v_card);
                }
                v_unmasking_stage.push(v_player_cards);
            }
            v_unmasking_sequence_cards.push(v_unmasking_stage);
        }

        verify_unmasking(
            num_players,
            v_player_keys,
            v_shuffle_history,
            v_unmasking_sequence_cards,
            unmasking_actors,
            unmasking_states,
        )?;

        Ok(())
    }
}

pub const POKER_HAND_STATE_UNMASK_HOLE_CARDS: u8 = 4;
pub const POKER_HAND_STATE_UNMASK_COMMUNITY_CARDS: u8 = 5;
pub const POKER_HAND_STATE_UNMASK_SHOWDOWN: u8 = 6;

pub fn verify_unmasking(
    num_players: U8,
    player_keys: Vec<G2Affine>,
    shuffle_history: Vec<Vec<G1Affine>>,
    unmasking_sequence_cards: Vec<Vec<Vec<G1Affine>>>,
    unmasking_actors: Vec<U8>,
    unmasking_states: Vec<U8>,
) -> Result<Option<usize>, Vec<u8>> {
    if player_keys.len() != num_players.to::<usize>() {
        return Err(b"NUM_PLAYERS_MISMATCH")?;
    }

    let final_shuffled_deck = shuffle_history
        .last()
        .ok_or_else(|| b"NO_SHUFFLE_HISTORY")?;

    let num_players = num_players.to::<usize>();
    let mut deck_idx = 0;

    let mut tracked_hole_cards: Vec<Vec<bls12_381::G1Affine>> = Vec::new();
    for _ in 0..num_players {
        tracked_hole_cards.push(final_shuffled_deck[deck_idx..deck_idx + 2].to_vec());
        deck_idx += 2;
    }

    let mut tracked_community_cards: Vec<Vec<bls12_381::G1Affine>> = vec![
        final_shuffled_deck[deck_idx..deck_idx + 3].to_vec(),
        final_shuffled_deck[deck_idx + 3..deck_idx + 4].to_vec(),
        final_shuffled_deck[deck_idx + 4..deck_idx + 5].to_vec(),
    ];

    let mut comm_round_idx = 0;
    let mut comm_unmask_count = 0;

    // 1. Prepare G2 points once for the entire batch to save CPU cycles
    let neg_g2_gen = -bls12_381::G2Affine::generator();
    let neg_g2_prepared = bls12_381::G2Prepared::from(neg_g2_gen);

    let mut prepared_pks = Vec::new();
    for pk in &player_keys {
        prepared_pks.push(bls12_381::G2Prepared::from(*pk));
    }

    // We will collect all peeling actions here: (unmasked, masked, action_player)
    let mut audit_trail = Vec::new();

    let unmasking_len = unmasking_states.len();
    if unmasking_len != unmasking_actors.len() || unmasking_len != unmasking_sequence_cards.len() {
        return Err(b"UNMASKING_LENGTH_MISMATCH")?;
    }

    // 2. Replay history and collect the trace instead of verifying immediately
    for i in 0..unmasking_states.len() {
        let (action_player, state_type, submitted_cards) = (
            &unmasking_actors[i],
            &unmasking_states[i],
            &unmasking_sequence_cards[i],
        );
        match state_type.to::<u8>() {
            POKER_HAND_STATE_UNMASK_HOLE_CARDS => {
                for target_player in 0..num_players {
                    if target_player == action_player.to::<usize>() {
                        continue;
                    }
                    let before = &tracked_hole_cards[target_player];
                    let after = submitted_cards[target_player].clone();

                    for (b, a) in before.iter().zip(after.iter()) {
                        audit_trail.push((*a, *b, action_player.to::<usize>()));
                    }
                    tracked_hole_cards[target_player] = after;
                }
            }
            POKER_HAND_STATE_UNMASK_COMMUNITY_CARDS => {
                let before = &tracked_community_cards[comm_round_idx];
                let after = submitted_cards[0].clone();

                for (b, a) in before.iter().zip(after.iter()) {
                    audit_trail.push((*a, *b, action_player.to::<usize>()));
                }
                tracked_community_cards[comm_round_idx] = after;

                comm_unmask_count += 1;
                if comm_unmask_count == num_players {
                    comm_unmask_count = 0;
                    comm_round_idx += 1;
                }
            }
            POKER_HAND_STATE_UNMASK_SHOWDOWN => {
                let target_player = action_player.to::<usize>();
                let before = &tracked_hole_cards[target_player];
                let after = submitted_cards[target_player].clone();

                for (b, a) in before.iter().zip(after.iter()) {
                    audit_trail.push((*a, *b, action_player.to::<usize>()));
                }
                tracked_hole_cards[target_player] = after;
            }
            _ => {}
        }
    }

    // 3. Build the giant batch for the Miller Loop
    let mut miller_terms = Vec::with_capacity(audit_trail.len() * 2);
    for (unmasked, masked, action_player) in &audit_trail {
        miller_terms.push((unmasked, &prepared_pks[*action_player]));
        miller_terms.push((masked, &neg_g2_prepared));
    }

    // 4. The Optimistic Batch Execution (O(1) final exponentiation for the whole game)
    let is_valid: bool = bls12_381::Bls12::multi_miller_loop(&miller_terms)
        .final_exponentiation()
        .is_identity()
        .into();

    if is_valid {
        // The game was perfectly fair.
        return Ok(None);
    }

    // 5. Fallback: The batch failed. Someone cheated.
    // We run the individual checks to find out exactly who it was.
    for (unmasked, masked, action_player) in audit_trail {
        let is_match: bool = bls12_381::Bls12::multi_miller_loop(&[
            (&unmasked, &prepared_pks[action_player]),
            (&masked, &neg_g2_prepared),
        ])
        .final_exponentiation()
        .is_identity()
        .into();

        if !is_match {
            return Ok(Some(action_player));
        }
    }

    Ok(None)
}

pub const G1AFFINE_COMPRESSED_LEN: usize = 48;
pub const G2AFFINE_COMPRESSED_LEN: usize = 96;

pub fn make_g2_from_compressed_slice(data: &[u8]) -> Result<G2Affine, &'static str> {
    if data.len() != G2AFFINE_COMPRESSED_LEN {
        return Err("INVALID_G2_COMPRESSED_LENGTH");
    }
    let mut bytes = [0u8; G2AFFINE_COMPRESSED_LEN];
    bytes.copy_from_slice(data);
    G2Affine::from_compressed(&bytes)
        .into_option()
        .ok_or("G2_DECODE_ERROR")
}

pub fn make_g1_from_compressed_slice(data: &[u8]) -> Result<G1Affine, &'static str> {
    if data.len() != G1AFFINE_COMPRESSED_LEN {
        return Err("INVALID_G1_COMPRESSED_LENGTH");
    }
    let mut bytes = [0u8; G1AFFINE_COMPRESSED_LEN];
    bytes.copy_from_slice(data);
    G1Affine::from_compressed(&bytes)
        .into_option()
        .ok_or("G1_DECODE_ERROR")
}
