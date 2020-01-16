use tinyrasputin::skeleton::{
    bot::PokerBot,
    actions::{Action, ActionType},
    states::{STARTING_STACK, GameState, RoundState, TerminalState},
    cards::{CardValue, Card, CardHandExt}
};

use std::collections::HashMap;
use rand::prelude::*;


#[derive(Debug)]
pub struct Lesson2Bot {
    wins_dict: HashMap<CardValue, u32>,
    showdowns_dict: HashMap<CardValue, u32>,
}

impl Default for Lesson2Bot {
    fn default() -> Lesson2Bot {
        let wins_dict = HashMap::new();
        let showdowns_dict = HashMap::new();
        Lesson2Bot {
            wins_dict,
            showdowns_dict
        }
    }
}

impl PokerBot for Lesson2Bot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) {
        println!("Game state: {:?} {:?}", gs, rs);
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
        let my_pip = rs.pips[player_index];
        let opp_pip = rs.pips[1 - player_index];
        let my_stack = rs.stacks[player_index];
        let opp_stack = rs.stacks[1 - player_index];
        let continue_cost = opp_pip - my_pip;
        let my_contrib = STARTING_STACK - my_stack;
        let opp_contrib = STARTING_STACK - opp_stack;
        if legal_actions == ActionType::CHECK {
            return Action::Check
        }
        if (legal_actions & ActionType::RAISE).bits() != 0 {
            let [min_raise, max_raise] = rs.raise_bounds();
            let min_cost = min_raise - my_pip;
            let max_cost = max_raise - my_pip;
            let mut rng = rand::thread_rng();
            let pot_total = my_contrib + opp_contrib;
            let p = (continue_cost as f64) / (pot_total as f64 + continue_cost as f64);
            let mut bet_amount = (0.75 * pot_total as f64) as u32;
            if bet_amount < min_raise {
                bet_amount = min_raise;
            } else if bet_amount > max_raise {
                bet_amount = max_raise;
            }
            if bet_amount > my_stack {
                bet_amount = my_stack;
            }

            println!("P: {} Pot total: {} CC: {}", p, pot_total, continue_cost);

            let agree_counts = my_cards.iter().map(|card| board_cards.iter().filter(|x| x.value() == card.value()).count()).collect::<Vec<_>>();

            if agree_counts.iter().any(|x| *x > 1) {
                return Action::Raise(bet_amount)
            }

            let fc_winrate = self.wins_dict.get(&my_cards[0].value()).cloned().unwrap_or(1) as f64 / self.showdowns_dict.get(&my_cards[0].value()).cloned().unwrap_or(2) as f64;
            let sc_winrate = self.wins_dict.get(&my_cards[1].value()).cloned().unwrap_or(1) as f64 / self.showdowns_dict.get(&my_cards[1].value()).cloned().unwrap_or(2) as f64;
            if agree_counts[0] == 1 {
                if rng.gen_bool(fc_winrate) {
                    return Action::Raise(bet_amount)
                }
            }
            if agree_counts[1] == 1 {
                if rng.gen_bool(sc_winrate) {
                    return Action::Raise(bet_amount)
                }
            }
            if my_cards[0] == my_cards[1] {
                if fc_winrate > p {
                    if rng.gen_bool(fc_winrate) {
                        return Action::Raise(bet_amount)
                    }
                }
            }
        }
        if (legal_actions & ActionType::CHECK).bits() != 0 {
            return Action::Check
        }
        Action::Call
    }
}