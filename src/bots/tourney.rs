use super::super::skeleton::{
    bot::PokerBot,
    actions::{Action, ActionType},
    states::{STARTING_STACK, GameState, RoundState, TerminalState},
    cards::{CardValue, CardHandExt},
};
use super::super::engine::{
    showdown::ShowdownEngine
};
use std::cmp::Ordering;
use std::collections::HashMap;
use rand::prelude::*;
use super::{generate_ordering, ensure_relation};

pub struct TourneyBot {
    wins_dict: HashMap<CardValue, u32>,
    showdowns_dict: HashMap<CardValue, u32>,
    ordering: [CardValue; 13],
    relations: Vec<(CardValue, CardValue)>,
}

impl Default for TourneyBot {
    fn default() -> TourneyBot {
        TourneyBot {
            wins_dict: HashMap::new(),
            showdowns_dict: HashMap::new(),
            ordering: generate_ordering(),
            relations: vec![(CardValue::Ace, CardValue::Two)],
        }
    }
}

impl PokerBot for TourneyBot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) {
        println!("Game state: {:?}", gs);
        let mut new_order = generate_ordering();
        // TODO Add relations that we are sure of
        for relation in self.relations.iter() {
            ensure_relation(&mut new_order, relation);
        }
        self.ordering = new_order;
        println!("Ordering: {:?}", new_order);
        //println!("Round bot state: {:?}", self);
    }

    fn handle_round_over(&mut self, gs: &GameState, ts: &TerminalState, player_index: usize) {
        let my_delta = ts.deltas[player_index];
        let ref previous_state = ts.previous;
        let street = previous_state.street;
        let ref my_cards = previous_state.hands[player_index];
        let ref opp_cards = previous_state.hands[1 - player_index];
        let ref board_cards = previous_state.deck.0[..street as usize];
        println!("Cards: {} {}", my_cards.print(), opp_cards.print());
        if opp_cards.is_some() {
            let opp_cards = opp_cards.unwrap().0;
            let my_cards = my_cards.unwrap().0;

            // showdown'd
            let showdown_engine = ShowdownEngine::new(self.ordering);
            let mut p_cards = my_cards.to_vec();
            let mut o_cards = opp_cards.to_vec();
            p_cards.extend(board_cards.iter());
            o_cards.extend(board_cards.iter());
            let my_hand = showdown_engine.process_hand(&p_cards);
            let opp_hand = showdown_engine.process_hand(&o_cards);
            println!("Hands: {} {}", my_hand, opp_hand);
            let comp = showdown_engine.compare_hands(&my_hand, &opp_hand);
            match comp {
                Ordering::Less => println!("We lost!"),
                Ordering::Greater => println!("We won!"),
                Ordering::Equal => println!("We had a draw.")
            };
            
            if my_hand.is_same_type(&opp_hand) {
                // We only actually gain information if our hands are the same type
                if my_delta > 0 && comp != Ordering::Greater {
                    // Prediction: we lost or drew, Actual: we won
                    let our_high = showdown_engine.highest_card(&my_hand.cards());
                    let opp_high = showdown_engine.highest_card(&opp_hand.cards());
                    if our_high.value() != opp_high.value() {
                        let ohi = self.ordering.iter().position(|x| *x == our_high.value()).unwrap();
                        let phi = self.ordering.iter().position(|x| *x == opp_high.value()).unwrap();
                        if ohi < phi { // Correct
                            println!("Adding (server win [err: o < p]) relationship {} -> {}", opp_high.value(), our_high.value());
                            self.relations.push((opp_high.value(), our_high.value()));
                        } else {
                            unreachable!("Incorrect assumption (server win)");
                        }
                    }
                } else if my_delta < 0 && comp != Ordering::Less {
                    // Prediction: we won or drew, Actual: we lost
                    let our_high = showdown_engine.highest_card(&my_hand.cards());
                    let opp_high = showdown_engine.highest_card(&opp_hand.cards());
                    if our_high.value() != opp_high.value() {
                        let ohi = self.ordering.iter().position(|x| *x == our_high.value()).unwrap();
                        let phi = self.ordering.iter().position(|x| *x == opp_high.value()).unwrap();
                        if ohi < phi {
                            unreachable!("Incorrect assumption (server lost)");
                        } else { // Correct
                            println!("Adding (server lost [err: p < o]) relationship {} -> {}", our_high.value(), opp_high.value());
                            self.relations.push((our_high.value(), opp_high.value()));
                        }
                    }
                }
            }
            
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
        let ref board_cards = rs.deck.0[..street as usize];
        let ref my_pip = rs.pips[player_index];
        let ref opp_pip = rs.pips[1 - player_index];
        let ref my_stack = rs.stacks[player_index];
        let ref opp_stack = rs.stacks[1 - player_index];
        let continue_cost = opp_pip - my_pip;
        let my_contrib = STARTING_STACK - my_stack;
        let opp_contrib = STARTING_STACK - opp_stack;
        let mut rng = rand::thread_rng();

        let showdown_engine = ShowdownEngine::new(self.ordering);
        let mut cards = my_cards.to_vec();
        cards.extend(board_cards.iter());
        let best_hand = showdown_engine.process_hand(&cards);
        println!("Best hand so far: {:?}", showdown_engine.process_hand(&cards));

        if (legal_actions & ActionType::RAISE).bits() != 0 {
            let [min_raise, max_raise] = rs.raise_bounds();
            let min_cost = min_raise - my_pip;
            let max_cost = max_raise - my_pip;
            let fc_winrate = self.wins_dict.get(&my_cards[0].value()).cloned().unwrap_or(1) as f32 / self.showdowns_dict.get(&my_cards[0].value()).cloned().unwrap_or(2) as f32;
            let sc_winrate = self.wins_dict.get(&my_cards[1].value()).cloned().unwrap_or(1) as f32 / self.showdowns_dict.get(&my_cards[1].value()).cloned().unwrap_or(2) as f32;
            if fc_winrate > 0.5 && sc_winrate > 0.5 {
                if rng.gen_bool(2.0 / 3.0) {
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
            if rng.gen_bool(5.0 / 6.0) {
                Action::Fold
            } else {
                Action::Call
            }
        }
    }
}