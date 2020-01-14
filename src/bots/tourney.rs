use tinyrasputin::skeleton::{
    bot::PokerBot,
    actions::{Action, ActionType},
    states::{STARTING_STACK, GameState, RoundState, TerminalState},
    cards::{CardValue, CardHandExt},
};
use tinyrasputin::{
    engine::{
        showdown::{ShowdownEngine, Hand, PotentialHand},
        probability::ProbabilityEngine,
        relations::{generate_ordering, detect_cycles, RelationsExt, relationships},
        guess::{Guess},
    },
    skeleton::cards::Card,
};
use std::cmp::Ordering;
use rand::prelude::*;
use itertools::Itertools;
use num_format::{Locale, ToFormattedString};
use std::cell::{RefCell, Cell};
use std::borrow::Borrow;

const SAMPLE_GUESS_THRESHOLD: u64 = 1000;
const RAISE_HAPPY: f64 = 0.7;
const RAISE_CAUTIOUS: f64 = 0.3;

pub struct TourneyBot {
    ordering: [CardValue; 13],
    prob_engine: ProbabilityEngine,
    // A relations cache, so we only recalculate when something changes
    relations: RefCell<Vec<(CardValue, CardValue)>>,
    relations_dirty: Cell<bool>,

    // Learn your opponent to learn what you should do
    opponent_raise_count: i64,
    running_guess: Guess,
}

impl Default for TourneyBot {
    fn default() -> TourneyBot {
        TourneyBot {
            ordering: generate_ordering(&vec![]),
            prob_engine: ProbabilityEngine::new(),
            relations: RefCell::new(vec![]),
            relations_dirty: Cell::new(false),
            opponent_raise_count: 0,
            running_guess: Guess::new(),
        }
    }
}

impl TourneyBot {
    fn add_relationship<S>(&mut self, log_string: S, strength: f64, a: CardValue, b: CardValue) where S: Borrow<str> {
        self.running_guess.update(a, b, strength as f32);
        if self.prob_engine.update(log_string.borrow(), &a, &b, strength) {
            println!("[{}] Saw relationship {} -> {} with strength {}...", log_string.borrow(), a, b, strength);
            //self.relations_dirty.set(true);
            self.running_guess.update(a,b);
        }
    }

    fn relations(&self) -> Vec<(CardValue, CardValue)> {
        if self.relations_dirty.get() {
            // Regenerate relations
            *self.relations.borrow_mut() = self.prob_engine.relations();
            self.relations_dirty.set(false);
        }
        self.relations.borrow().iter().copied().collect()
    }
}

impl PokerBot for TourneyBot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) {
        println!("Round #{} time {} ({})", gs.round_num, gs.game_clock, gs.bankroll);
        let relations = self.relations();
        let sample_space_size = relations.possibilities();
        // println!("Sample space size -> {}", sample_space_size.to_formatted_string(&Locale::en));
        // In the unlikely event we actually calculate a "for certain" ordering, just keep it until we violate it enough
        if sample_space_size > SAMPLE_GUESS_THRESHOLD {
            // let new_order = generate_ordering(&relations);
            let new_order = generate_ordering(&vec![]);
            // TODO Add relations that we are sure of
            self.ordering = new_order;
        }
        // println!("Ordering: [{}]", self.ordering.iter().format(","));
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
        let print_prediction = |my_hand, opp_hand, delta| {
            let state = |n| if n > 0 {
                "won"
            } else if n < 0 {
                "lost"
            } else {
                "draw"
            };

            println!("locally predicted {}", state(delta));
            println!("server said {}", state(my_delta));
        };
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

            let my_hand = showdown_engine.process_hand_no_straight(&p_cards);
            let opp_hand = showdown_engine.process_hand_no_straight(&o_cards);
            let (winner, loser) = if my_delta > 0 {
                (my_hand.clone(), opp_hand.clone())
            } else if my_delta < 0 {
                (opp_hand.clone(), my_hand.clone())
            } else {
                (my_hand.clone(), opp_hand.clone())
            };
            match (my_hand.showdown(), opp_hand.showdown()) {
                (Some(my_showdown_hand), Some(opp_showdown_hand)) => {
                    // We detected something other than high card for both hands
                    let delta = match showdown_engine.compare_potential_hands(&my_hand, &opp_hand) {
                        Ordering::Greater => 1,
                        Ordering::Less => -1,
                        Ordering::Equal => 0
                    };

                    print_prediction(&my_hand, &opp_hand, delta);

                    let (actual_winner, actual_loser) = if delta.signum() * my_delta.signum() > 0 {
                        // We were correct
                        match delta {
                            -1 => (opp_showdown_hand, my_showdown_hand),
                            _ => (my_showdown_hand, opp_showdown_hand)
                        }
                    } else {
                        // We were incorrect or drew
                        match my_delta.signum() {
                            1 => (my_showdown_hand, opp_showdown_hand),
                            _ => (opp_showdown_hand, my_showdown_hand)
                        }
                    };

                    println!("Winner hand: {} Loser hand: {}", actual_winner, actual_loser);

                    if my_delta != 0 {
                        match (actual_winner, actual_loser) {
                            // Same hand type relations
                            (Hand::Pair(winner), Hand::Pair(loser)) => self.add_relationship("pair -> pair", 0.9, showdown_engine.highest_card_value(loser), showdown_engine.highest_card_value(winner)),
                            (Hand::ThreeOfAKind(winner), Hand::ThreeOfAKind(loser)) => self.add_relationship("3k -> 3k", 0.9, showdown_engine.highest_card_value(loser), showdown_engine.highest_card_value(winner)),
                            (Hand::FourOfAKind(winner), Hand::FourOfAKind(loser)) => self.add_relationship("4k -> 4k", 0.9, showdown_engine.highest_card_value(loser), showdown_engine.highest_card_value(winner)),
                            (Hand::FullHouse(winner), Hand::FullHouse(loser)) => {
                                add_relationship("fh -> fh", 0.9, )
                            },
                            (_, _) => {}
                        }
                    }
                },
                (None, Some(_)) => {
                    print_prediction(&my_hand, &opp_hand, -1);

                    if my_delta > 0 {
                        // We are wrong
                    } else {
                        // We are right
                    }
                },
                (Some(_), None) => {
                    print_prediction(&my_hand, &opp_hand, 1);

                    if my_delta < 0 {
                        // We are wrong
                    } else {
                        // We are right
                    }
                },
                (None, None) => {
                    // We only detected high cards. This relationship is very unlikely. Rating 1 / 12
                    // let (winner, loser) = (winner.cards().iter().copied().nth(0).unwrap(), loser.cards().iter().copied().nth(0).unwrap());
                    let (winner_hand, loser_hand) = if my_delta > 0 {
                        (my_cards, opp_cards)
                    } else {
                        (opp_cards, my_cards)
                    };

                    let delta = match showdown_engine.compare_potential_hands(&my_hand, &opp_hand) {
                        Ordering::Greater => 1,
                        Ordering::Less => -1,
                        Ordering::Equal => 0
                    };

                    print_prediction(&my_hand, &opp_hand, delta);

                    if my_delta != 0 {
                        for winning_card in winner_hand.into_iter() {
                            for losing_card in loser_hand.into_iter() {
                                // self.add_relationship("hc -> hc", 0.25, losing_card.value(), winning_card.value());
                                self.add_relationship("hc -> hc", 0.25, losing_card.value(), winning_card.value());
                            }
                        }
                    } else { // In case of a draw, the high card is a board card, but this is very unlikely
                        let high_card = showdown_engine.highest_card(board_cards);
                        for card in winner_hand.into_iter().chain(loser_hand.into_iter()) {
                            self.add_relationship("(draw) hc -> hc", my_delta.signum() as f64 * 0.1, card.value(), high_card.value())
                        }
                    }
                }
            }
        }
    }

    fn get_action(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) -> Action {
        // todo!()
        let legal_actions = rs.legal_actions();

        let street = rs.street;
        let ref board_cards = rs.deck.0[..street as usize];
        let ref my_cards = rs.hands[player_index].unwrap().0;
        let my_pip = rs.pips[player_index];
        let opp_pip = rs.pips[1 - player_index];
        let my_stack = rs.stacks[player_index];
        let opp_stack = rs.stacks[1 - player_index];
        let continue_cost = opp_pip - my_pip;
        let my_contrib = STARTING_STACK - my_stack;
        let opp_contrib = STARTING_STACK - opp_stack;
        let pot_total = my_contrib + opp_contrib;
        let mut rng = rand::thread_rng();

        // if opp_pip > my_pip {
        //     // Detect an opponent raise?
        //     println!("Opponent raised, not us");
        //     self.opponent_raise_count += 1;
        // }

        // println!("Pot {} my stack {} opp stack {} CC {}", pot_total, my_stack, opp_stack, continue_cost);
        println!("My cards [{}]", my_cards.iter().format(", "));
        println!("Board cards [{}]", board_cards.iter().format(", "));
        let showdown_engine = ShowdownEngine::new(self.ordering);

        let my_best = showdown_engine.process_hand(&vec![my_cards.iter().as_slice(), board_cards].into_iter().flat_map(|x: &[Card]| x.iter().copied()).collect::<Vec<Card>>());
        let raise: f64 = if let Some(board_cards) = Some(board_cards).filter(|x| !x.is_empty()) {
            // Flop, Turn, or River (we have some board information)
            let board_best = showdown_engine.process_hand(board_cards);
            println!("({}) board best {} my best {}", if street == 3 { "flop" } else if street == 4 { "turn" } else { "river" }, board_best, my_best);
            let hand_relationship = showdown_engine.compare_potential_hands(&my_best, &board_best);
            match hand_relationship {
                Ordering::Greater => {
                    // Our hand beats the board
                    rng.gen_range(0.5, 2.0) * pot_total as f64
                },
                Ordering::Equal => {
                    // Our best *is* the board
                    0.0
                },
                Ordering::Less => {
                    // The board beats our hand
                    0.0
                }
            }
        } else {
            // Pre-Flop (we have no board information)
            println!("(pre-flop) my best {}", my_best);
            match my_best {
                PotentialHand::Hand(hand) => { // We already have a hand (which means we have pocket pairs)
                    rng.gen_range(0.7, 1.5) * pot_total as f64
                },
                hand => {
                    // Generate a percent from what we think
                    let possibilities = self.relations().possibilities();
                    let card_score: usize = my_cards.into_iter().map(|x| self.ordering.iter().position(|y| y == &x.value()).unwrap()).sum();
                    let strength = card_score as f64 / 24.0;
                    let balance = possibilities as f64 / 6_227_020_800.0f64;
                    if strength > 12.0 {
                        let strength = strength - 12.0;
                        rng.gen_range(if balance == 1.0 { 0.0 } else { balance } * strength, strength) * pot_total as f64
                    } else {
                        // Is it worth it? Right now, we only bet on high cards as long as the continue cost is less than 50 and our stack is more than 100
                        if continue_cost < 50 && my_stack > 100 {
                            rng.gen_range(0.0, 50.0)
                        } else {
                            0.0
                        }
                    }
                }
            }
        };

        println!("Raise worth: {}", raise);

        let raise = raise as i64;

        let act = |fail_action| if (legal_actions & ActionType::CHECK) == ActionType::CHECK {
            Action::Check
        } else {
            fail_action
        };

        if raise > continue_cost {
            // Bound the raise
            let [rb_min, rb_max] = rs.raise_bounds();
            let raise = if (raise as i64) > rb_max {
                rb_max
            } else if (raise as i64) < rb_min {
                rb_min
            } else {
                raise as i64
            };

            // We think this hand is worth it!
            if (legal_actions & ActionType::RAISE) == ActionType::RAISE {
                Action::Raise(raise)
            } else {
                act(Action::Call)
            }
        } else {
            // Gain some data points
            if gs.round_num > 100 {
                let opr = (self.opponent_raise_count as f64 / gs.round_num as f64).min(1.0);
                println!("Opponent Raise Percent: {}%", opr * 100.0);
                if opr > RAISE_HAPPY {
                    // Our opponent is relatively raise happy. Them raising tells us nothing. ignore them and call their bluffs
                    if rng.gen_bool(opr) {
                        act(Action::Call)
                    } else {
                        // welp, we gotta hedge our bets somewhere
                        act(Action::Fold)
                    }
                } else if opr < RAISE_CAUTIOUS {
                    // Our oppnent is relatively raise cautious. Them raising means they got something. Be cautious.
                    if rng.gen_bool(opr) {
                        act(Action::Fold)
                    } else {
                        act(Action::Call)
                    }
                } else {
                    // Our opponent is a normal goddamn person. Act normal. Flip a coin. That our their an allin bot, and we don't have enough information
                    if rand::random() {
                        act(Action::Fold)
                    } else {
                        act(Action::Call)
                    }
                }
            } else {
                // Assume the worst
                if (legal_actions & ActionType::RAISE) == ActionType::RAISE {
                    Action::Raise(raise)
                } else {
                    act(Action::Fold)
                }
            }
        }
    }
}

impl Drop for TourneyBot {
    fn drop(&mut self) {
        let relations = self.relations();
        println!("Final relations:\n{}", relations.debug_relations());
        println!("Final guess:\n{:?}", self.running_guess);
        println!("Final ordering: [{}]", self.ordering.iter().format(""));
        println!("Probability engine:\n{}", self.prob_engine.probabilities().into_iter().map(|((a, b), p)| format!("{} -> {} P({})", a, b, p)).format("\n"));
        let ignored_rules = self.prob_engine.inconsistent_rule_names();
        for rule in ignored_rules {
            println!("You should check rule [{}] for inconsistencies.", rule);
        }
    }
}
