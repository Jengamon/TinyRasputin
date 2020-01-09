use super::super::skeleton::{
    bot::PokerBot,
    actions::{Action, ActionType},
    states::{STARTING_STACK, GameState, RoundState, TerminalState}
};

use std::collections::HashMap;


#[derive(Debug)]
pub struct Lesson1Bot {
    wins_dict: HashMap<String, u32>,
    showdowns_dict: HashMap<String, u32>,
}

impl Default for Lesson1Bot {
    fn default() -> Lesson1Bot {
        let mut wins_dict = HashMap::new();
        let mut showdowns_dict = HashMap::new();
        for card in ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"].iter() {
            wins_dict.insert(card.to_string(), 1);
            showdowns_dict.insert(card.to_string(), 2);
        }
        Lesson1Bot {
            wins_dict,
            showdowns_dict
        }
    }
}

impl PokerBot for Lesson1Bot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) {
        println!("Round bot state: {:?}", self);
    }

    fn handle_round_over(&mut self, gs: &GameState, ts: &TerminalState, player_index: usize) {
        let my_delta = ts.deltas[player_index];
        let ref previous_state = ts.previous;
        let street = previous_state.street;
        let ref my_cards = previous_state.hands[player_index];
        let ref opp_cards = previous_state.hands[1 - player_index];
        println!("Cards: {:?} {:?}", my_cards, opp_cards);
        if opp_cards != &["".to_string(), "".to_string()] {
            // showdown'd
            if my_delta > 0 {
                // we won
                self.wins_dict.get_mut(&my_cards[0][..=0]).map(|x| *x += 1);
                self.wins_dict.get_mut(&my_cards[1][..=0]).map(|x| *x += 1);
            }
            self.showdowns_dict.get_mut(&my_cards[0][..=0]).map(|x| *x += 1);
            self.showdowns_dict.get_mut(&my_cards[1][..=0]).map(|x| *x += 1);
            if my_delta < 0 {
                self.wins_dict.get_mut(&opp_cards[0][..=0]).map(|x| *x += 1);
                self.wins_dict.get_mut(&opp_cards[1][..=0]).map(|x| *x += 1);
            }
            self.showdowns_dict.get_mut(&opp_cards[0][..=0]).map(|x| *x += 1);
            self.showdowns_dict.get_mut(&opp_cards[1][..=0]).map(|x| *x += 1);
        }

    }

    fn get_action(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) -> Action {
        let legal_actions = rs.legal_actions();
        let street = rs.street;
        let ref my_cards = rs.hands[player_index];
        let ref board_cards = &rs.deck[..street as usize];
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
            let fc_winrate = self.wins_dict.get(&my_cards[0][..=0]).cloned().unwrap_or(1) as f32 / self.showdowns_dict.get(&my_cards[0][..=0]).cloned().unwrap_or(2) as f32;
            let sc_winrate = self.wins_dict.get(&my_cards[1][..=0]).cloned().unwrap_or(1) as f32 / self.showdowns_dict.get(&my_cards[1][..=0]).cloned().unwrap_or(2) as f32;
            if fc_winrate > 0.5 && sc_winrate > 0.5 {
                return Action::Raise(min_raise)
            }
        }
        if (legal_actions & ActionType::CHECK).bits() != 0 {
            return Action::Check
        }
        Action::Call
    }
}