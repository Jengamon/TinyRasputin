use super::super::skeleton::{
    bot::PokerBot,
    actions::{Action, ActionType},
    states::{STARTING_STACK, GameState, RoundState, TerminalState},
    cards::{CardValue, CardHandExt},
};
use std::collections::HashMap;
use rand::prelude::*;

pub struct TourneyBot {
    wins_dict: HashMap<CardValue, u32>,
    showdowns_dict: HashMap<CardValue, u32>,
}

impl Default for TourneyBot {
    fn default() -> TourneyBot {
        TourneyBot {
            wins_dict: HashMap::new(),
            showdowns_dict: HashMap::new()
        }
    }
}

impl PokerBot for TourneyBot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) {
        println!("Game state: {:?}", gs);
        //println!("Round bot state: {:?}", self);
    }

    fn handle_round_over(&mut self, gs: &GameState, ts: &TerminalState, player_index: usize) {
        let my_delta = ts.deltas[player_index];
        let ref previous_state = ts.previous;
        let street = previous_state.street;
        let ref my_cards = previous_state.hands[player_index];
        let ref opp_cards = previous_state.hands[1 - player_index];
        println!("Cards: {} {}", my_cards.print(), opp_cards.print());
        if opp_cards.is_some() {
            let opp_cards = opp_cards.unwrap().0;
            let my_cards = my_cards.unwrap().0;
            // showdown'd
            if my_delta > 0 {
                // we won
                *self.wins_dict.entry(my_cards[0].value()).or_insert(1) += 1;
                *self.wins_dict.entry(my_cards[1].value()).or_insert(1) += 1;
            }
            *self.showdowns_dict.entry(my_cards[0].value()).or_insert(2) += 1;
            *self.showdowns_dict.entry(my_cards[1].value()).or_insert(2) += 1;
            if my_delta < 0 {
                *self.wins_dict.entry(opp_cards[0].value()).or_insert(1) += 1;
                *self.wins_dict.entry(opp_cards[1].value()).or_insert(1) += 1;
            }
            *self.showdowns_dict.entry(opp_cards[0].value()).or_insert(2) += 1;
            *self.showdowns_dict.entry(opp_cards[1].value()).or_insert(2) += 1;
        }

    }

    fn get_action(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) -> Action {
        let legal_actions = rs.legal_actions();
        let street = rs.street;
        let ref my_cards = rs.hands[player_index].unwrap().0;
        let ref board_cards = &rs.deck.0[..street as usize];
        let ref my_pip = rs.pips[player_index];
        let ref opp_pip = rs.pips[1 - player_index];
        let ref my_stack = rs.stacks[player_index];
        let ref opp_stack = rs.stacks[1 - player_index];
        let continue_cost = opp_pip - my_pip;
        let my_contrib = STARTING_STACK - my_stack;
        let opp_contrib = STARTING_STACK - opp_stack;
        if (legal_actions & ActionType::RAISE).bits() != 0 {
            let [min_raise, max_raise] = rs.raise_bounds();
            let min_cost = min_raise - my_pip;
            let max_cost = max_raise - my_pip;
            let fc_winrate = self.wins_dict.get(&my_cards[0].value()).cloned().unwrap_or(1) as f32 / self.showdowns_dict.get(&my_cards[0].value()).cloned().unwrap_or(2) as f32;
            let sc_winrate = self.wins_dict.get(&my_cards[1].value()).cloned().unwrap_or(1) as f32 / self.showdowns_dict.get(&my_cards[1].value()).cloned().unwrap_or(2) as f32;
            if fc_winrate > 0.5 && sc_winrate > 0.5 {
                if rand::random() {
                    return Action::Raise(min_raise)
                }
            }
        }
        if (legal_actions & ActionType::CHECK).bits() != 0 {
            return Action::Check
        }
        if gs.round_num < 100 || (gs.round_num > 100 && gs.bankroll > 0)  {
            Action::Call
        } else {
            if rand::random() || rand::random() || rand::random() {
                Action::Fold
            } else {
                Action::Call
            }
        }
    }
}