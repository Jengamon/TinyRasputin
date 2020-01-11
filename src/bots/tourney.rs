use tinyrasputin::skeleton::{
    bot::PokerBot,
    actions::{Action, ActionType},
    states::{STARTING_STACK, GameState, RoundState, TerminalState},
    cards::{CardValue, CardHandExt},
};
use tinyrasputin::engine::{
    showdown::{ShowdownEngine, ShowdownHand},
    relations::{generate_ordering, detect_cycles, resolve_cycles, RelationsExt},
};
use std::cmp::Ordering;
use std::collections::HashMap;
use rand::prelude::*;
use itertools::Itertools;

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
            ordering: generate_ordering(&vec![]),
            relations: vec![],
        }
    }
}

impl PokerBot for TourneyBot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) {
        println!("Round #{} time {}", gs.round_num, gs.game_clock);
        let new_order = generate_ordering(&self.relations);
        // TODO Add relations that we are sure of
        self.ordering = new_order;
        println!("Ordering: [{}]", new_order.iter().format(","));
        //println!("Round bot state: {:?}", self);
    }

    fn handle_round_over(&mut self, gs: &GameState, ts: &TerminalState, player_index: usize) {
        let my_delta = ts.deltas[player_index];
        let ref previous_state = ts.previous;
        let street = previous_state.street;
        let ref my_cards = previous_state.hands[player_index];
        let ref opp_cards = previous_state.hands[1 - player_index];
        let ref board_cards = previous_state.deck.0[..street as usize];
        println!("Cards: {} {} <{}>", my_cards.print(), opp_cards.print(), board_cards.iter().format(", "));
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
                Ordering::Less => println!("We lost! -> {}", my_delta < 0),
                Ordering::Greater => println!("We won! -> {}", my_delta > 0),
                Ordering::Equal => println!("We had a draw. -> {} ({})", my_delta == 0, my_delta)
            };

            let add_relationship = |relations: &mut Vec<(CardValue, CardValue)>, (a, b)| {
                if !relations.contains(&(a, b)) {
                    relations.push((a, b));
                }
                let cycles = detect_cycles(&relations);
                for cycle in cycles.iter() {
                    println!("Detected cycle {}", cycle.iter().format(" -> "));
                    for pair in cycle.windows(2) {
                        // println!("Removing relation {} -> {} due to cycle...", pair[0], pair[1]);
                        let position = relations.iter().position(|x| x == &(pair[0], pair[1]));
                        if let Some(position) = position {
                            relations.remove(position);
                        }
                    }
                }
                *relations = relations.remove_redundancies();
                for (a, b) in resolve_cycles(&relations, &cycles)/*.collect::<Vec<_>>().iter()*/ {
                    println!("Readding relation {} -> {}", a, b);
                    relations.push((a, b));
                }
            };

            let orel = self.relations.clone();
            
            if my_hand.is_same_type(&opp_hand) {
                // We only actually gain information if our hands are the same type

                if (my_hand.is_straight() || opp_hand.is_straight()) && gs.round_num < 500 {
                    // Straights are unreliable until we build up a rapport
                } else {
                    if my_delta > 0 && comp == Ordering::Less {
                        // Prediction: we lost, Actual: we won
                        let our_high = showdown_engine.highest_card(&my_hand.cards());
                        let opp_high = showdown_engine.highest_card(&opp_hand.cards());
                        if our_high.value() != opp_high.value() {
                            let ohi = self.ordering.iter().position(|x| *x == our_high.value()).unwrap();
                            let phi = self.ordering.iter().position(|x| *x == opp_high.value()).unwrap();
                            if ohi < phi { // Correct
                                println!("Adding (server win) relationship {} -> {}", opp_high.value(), our_high.value());
                                add_relationship(&mut self.relations, (opp_high.value(), our_high.value()));
                            } else {
                                unreachable!("Incorrect assumption (server win)");
                            }
                        }
                    } else if my_delta < 0 && comp == Ordering::Greater {
                        // Prediction: we won, Actual: we lost
                        let our_high = showdown_engine.highest_card(&my_hand.cards());
                        let opp_high = showdown_engine.highest_card(&opp_hand.cards());
                        if our_high.value() != opp_high.value() {
                            let ohi = self.ordering.iter().position(|x| *x == our_high.value()).unwrap();
                            let phi = self.ordering.iter().position(|x| *x == opp_high.value()).unwrap();
                            if ohi < phi {
                                unreachable!("Incorrect assumption (server lost)");
                            } else { // Correct
                                println!("Adding (server lost) relationship {} -> {}", our_high.value(), opp_high.value());
                                add_relationship(&mut self.relations, (our_high.value(), opp_high.value()));
                            }
                        }
                    } else if comp == Ordering::Equal && my_delta != 0 {
                        // Prediction: we drew, Actual: we won or lost
                        let my_highest_card = showdown_engine.highest_card(&my_hand.cards());
                        let opp_highest_card = showdown_engine.highest_card(&opp_hand.cards());
                        let board_highest = showdown_engine.highest_card(&board_cards);
                        if my_highest_card != opp_highest_card {
                            if my_delta > 0 {
                                println!("Adding (server win draw) relationship {} -> {}", board_highest.value(), my_highest_card.value());
                                add_relationship(&mut self.relations, (board_highest.value(), my_highest_card.value()));
                            } else {
                                println!("Adding (server lost draw) relationship {} -> {}", board_highest.value(), opp_highest_card.value());
                                add_relationship(&mut self.relations, (board_highest.value(), opp_highest_card.value()));
                            }
                        }
                    }
                }
            } else {
                // Try to gein information from when we think the hands are different
            }

            if orel != self.relations {
                // println!("Before simplifying:\n{}", self.relations.debug_relations());
                // self.relations = self.relations.remove_redundancies();
                println!("-\n{}", self.relations.debug_relations());
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
        let my_pip = rs.pips[player_index];
        let opp_pip = rs.pips[1 - player_index];
        let my_stack = rs.stacks[player_index];
        let opp_stack = rs.stacks[1 - player_index];
        let continue_cost = opp_pip - my_pip;
        let my_contrib = STARTING_STACK - my_stack;
        let opp_contrib = STARTING_STACK - opp_stack;
        let pot_total = my_contrib + opp_contrib;
        let mut rng = rand::thread_rng();

        let showdown_engine = ShowdownEngine::new(self.ordering);
        let mut cards = my_cards.to_vec();
        cards.extend(board_cards.iter());
        let best_hand = if gs.round_num > 500 {
            showdown_engine.process_hand(&cards)
        } else {
            showdown_engine.process_hand_no_straight(&cards)
        };
        println!("Hand: {}", showdown_engine.process_hand(&cards));

        // Check if that is the **only** action available to us
        if legal_actions == ActionType::CHECK {
            return Action::Check
        }

        let checkfold = || if (legal_actions & ActionType::CHECK).bits() != 0 {
            Action::Check
        } else {
            Action::Fold
        };

        let checkcall = || if (legal_actions & ActionType::CHECK).bits() != 0 {
            Action::Check
        } else {
            Action::Call
        };

        let raise_amount = |amount: f64| {
            let amount = amount as i64;
            let [rb_min, rb_max] = rs.raise_bounds();
            if amount < rb_min { rb_min }
            else if amount > rb_max { rb_max }
            else { amount }
        };

        // Always fold if all we detect is a "low" high card
        match best_hand {
            ShowdownHand::Flush(cards) if rs.street == 3 => if (legal_actions & ActionType::RAISE).bits() != 0 {
                Action::Raise(raise_amount(0.25 * pot_total as f64))
            } else {
                checkcall()
            },
            ShowdownHand::Flush(cards) if rs.street > 3 => if (legal_actions & ActionType::RAISE).bits() != 0 {
                if rng.gen_bool(0.6) {
                    Action::Raise(raise_amount(0.75 * my_stack as f64))
                } else {
                    Action::Raise(raise_amount(0.25 * my_stack as f64))
                }
            } else {
                checkcall()
            },
            ShowdownHand::HighCard(card) if gs.round_num > 200 => {
                // we have a pretty good idea of what's high and low, so fold if it's low
                if self.ordering.iter().position(|x| x == &card.value()).map(|x| x < 6).unwrap_or(false) {
                    Action::Fold
                } else {
                    checkcall()
                }
            },
            ShowdownHand::HighCard(card) => checkcall(),
            _ => checkfold() // Try to exit the game if we dont handle the hand
        }
    }
}

impl Drop for TourneyBot {
    fn drop(&mut self) {
        println!("Final relations:\n{}", self.relations[..].debug_relations());
        println!("Final ordering: [{}]", self.ordering.iter().format(","));
    }
}