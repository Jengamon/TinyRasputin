use tinyrasputin::skeleton::{
    bot::PokerBot,
    actions::{Action, ActionType},
    states::{STARTING_STACK, GameState, RoundState, TerminalState},
    cards::{CardValue, CardHandExt},
};
use tinyrasputin::engine::{
    showdown::{ShowdownEngine, Hand},
    probability::ProbabilityEngine,
    relations::{generate_ordering, detect_cycles, RelationsExt, relationships},
};
use std::cmp::Ordering;
use rand::prelude::*;
use itertools::Itertools;
use num_format::{Locale, ToFormattedString};
use std::cell::{RefCell, Cell};

const SAMPLE_GUESS_THRESHOLD: u64 = 1000;

pub struct TourneyBot {
    ordering: [CardValue; 13],
    prob_engine: ProbabilityEngine,
    // A relations cache, so we only recalculate when something changes
    relations: RefCell<Vec<(CardValue, CardValue)>>,
    relations_dirty: Cell<bool>,
}

impl Default for TourneyBot {
    fn default() -> TourneyBot {
        TourneyBot {
            ordering: generate_ordering(&vec![]),
            prob_engine: ProbabilityEngine::new(),
            relations: RefCell::new(vec![]),
            relations_dirty: Cell::new(false),
        }
    }
}

impl TourneyBot {
    fn add_relationship(&mut self, strength: f64, a: CardValue, b: CardValue) {
        println!("Saw relationship {} -> {} with strength {}...", a, b, strength);
        self.prob_engine.update(&a, &b, strength);
        self.relations_dirty.set(true);
    }

    fn relations(&self) -> Vec<(CardValue, CardValue)> {
        if self.relations_dirty.get() {
            self.relations_dirty.set(false);
            // Regenerate relations
            *self.relations.borrow_mut() = self.prob_engine.relations();
        }
        self.relations.borrow().iter().copied().collect()
    }
}

impl PokerBot for TourneyBot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) {
        println!("Round #{} time {}", gs.round_num, gs.game_clock);
        let relations = self.relations();
        let sample_space_size = relations.possibilities();
        println!("Sample space size -> {}", sample_space_size.to_formatted_string(&Locale::en));
        // In the unlikely event we actually calculate a "for certain" ordering, just keep it until we violate it enough
        if sample_space_size > SAMPLE_GUESS_THRESHOLD {
            let new_order = generate_ordering(&relations);
            // TODO Add relations that we are sure of
            self.ordering = new_order;
        }
        println!("Ordering: [{}]", self.ordering.iter().format(","));
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
            // We can see our opponents cards, so we got to showdown
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
            match (my_hand.showdown(), opp_hand.showdown()) {
                (Some(my_hand), Some(opp_hand)) => {
                    // We detected something other than high card for both hands
                },
                (None, Some(_)) | (Some(_), None) => {

                },
                (None, None) => {
                    // We only detected high cards. This relationship is very unlikely. Rating 1 / 12
                    let (my_card, opp_card) = (my_hand.cards()[0], opp_hand.cards()[0]);
                    let (winner, loser) = if my_delta > 0 {
                        (my_card, opp_card)
                    } else {
                        (opp_card, my_card)
                    }

                    if my_delta != 0 {
                        println!("Attempting to preserve relationship {} -> {} with P(1 / 12)")
                    }
                }
            }
        }
    }

    fn get_action(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) -> Action {
        // todo!()
        let legal_actions = rs.legal_actions();
        let checkfold = || if (legal_actions & ActionType::CHECK) == ActionType::CHECK {
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



        if opp_pip > 10 {
            match best_hand {
                ShowdownHand::Flush(cards) | ShowdownHand::FourOfAKind(cards) | ShowdownHand::FullHouse(cards) => if (legal_actions & ActionType::RAISE).bits() != 0 {
                    Action::Raise(raise_amount(rng.gen_range(0.5,1.5) * pot_total as f64))
                } else {
                    checkcall()
                },
                _ => checkfold()
            }
        } else {
            // Always fold if all we detect is a "low" high card
            match best_hand {
                // Our ace in the hole: predicting straights
                ShowdownHand::Straight(cards) if gs.round_num > 500 => if (legal_actions & ActionType::RAISE).bits() != 0 {
                    Action::Raise(raise_amount(0.5 * pot_total as f64)) // Don't bet too much on it being a straight
                } else {
                    checkcall()
                },
                // Flushes
                ShowdownHand::Flush(cards) if rs.street == 3 => if (legal_actions & ActionType::RAISE).bits() != 0 {
                    Action::Raise(raise_amount(0.25 * pot_total as f64))
                } else {
                    checkcall()
                },
                ShowdownHand::Flush(cards) if rs.street > 3 => if (legal_actions & ActionType::RAISE).bits() != 0 {
                    if rng.gen_bool(0.8) {
                        Action::Raise(raise_amount(rng.gen_range(0.75,2.5) * my_stack as f64))
                    } else {
                        Action::Raise(raise_amount(0.25 * my_stack as f64))
                    }
                } else {
                    checkcall()
                },
                ShowdownHand::FourOfAKind(cards) => if (legal_actions & ActionType::RAISE).bits() != 0 {
                    Action::Raise(raise_amount(0.5 * pot_total as f64)) // Don't bet too much on it being a straight
                } else {
                    checkcall()
                },
                ShowdownHand::FullHouse(cards) => if (legal_actions & ActionType::RAISE).bits() != 0 {
                    Action::Raise(raise_amount(0.5 * pot_total as f64)) // Don't bet too much on it being a straight
                } else {
                    checkcall()
                },
                ShowdownHand::ThreeOfAKind(cards) => if (legal_actions & ActionType::RAISE).bits() != 0 {
                    Action::Raise(raise_amount(0.5 * pot_total as f64)) // Don't bet too much on it being a straight
                } else {
                    checkcall()
                },
                ShowdownHand::TwoPair(cards) => if (legal_actions & ActionType::RAISE).bits() != 0 {
                    Action::Raise(raise_amount(0.5 * pot_total as f64)) // Don't bet too much on it being a straight
                } else {
                    checkcall()
                },
                // Shadow pair
                ShowdownHand::Pair(cards) if rs.street == 0 => if (legal_actions & ActionType::RAISE).bits() != 0 {
                    let value = cards[0].value();
                    let strength = self.ordering.iter().position(|x| x == &value).unwrap();
                    if (gs.round_num > 200) {
                        Action::Raise(raise_amount(0.3 * pot_total as f64))
                    } else if rng.gen_bool(strength as f64 / 13.0 + 0.05) {
                        Action::Raise(raise_amount(rng.gen_range(0.8,1.4) * pot_total as f64))
                    } else {
                        let amount = rng.gen_range(0.15,0.35) * (strength as f64 / 13.0) * pot_total as f64;
                        let [_, mx] = rs.raise_bounds();
                        if amount < mx as f64 && rand::random() {
                            Action::Raise(raise_amount(amount))
                        } else {
                            checkcall() // It isn't a very strong hand
                        }
                    }
                } else {
                    checkcall()
                },
                ShowdownHand::Pair(cards) => if (legal_actions & ActionType::RAISE).bits() != 0 {
                    Action::Raise(raise_amount(rng.gen_range(0.4,1.0) * pot_total as f64)) // Don't bet too much on it being a straight
                } else {
                    checkcall()
                },
                ShowdownHand::HighCard(card) if gs.round_num > 200 => {
                    // we have a pretty good idea of what's high and low, so fold if it's low
                    if self.ordering.iter().position(|x| x == &card.value()).map(|x| x < 10).unwrap_or(false) {
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
}

impl Drop for TourneyBot {
    fn drop(&mut self) {
        let relations = self.relations();
        println!("Final relations:\n{}", relations.debug_relations());
        println!("Final ordering: [{}]", self.ordering.iter().format(""));
        println!("Probability engine:\n{}", self.prob_engine.probabilities().into_iter().map(|((a, b), p)| format!("{} -> {} P({})", a, b, p)).format("\n"));
    }
}
