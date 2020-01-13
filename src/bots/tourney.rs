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

        checkfold()
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