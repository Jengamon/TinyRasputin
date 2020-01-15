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
// use num_format::{Locale, ToFormattedString};
use std::cell::{RefCell, Cell};
use std::borrow::Borrow;

// const SAMPLE_GUESS_THRESHOLD: u64 = 1000;
const RAISE_HAPPY: f64 = 0.7;
const RAISE_CAUTIOUS: f64 = 0.3;
const FILE_BYTE_SIZE: usize = 524288;

pub struct TourneyBot {
    ordering: [CardValue; 13],
    prob_engine: ProbabilityEngine,
    // A relations cache, so we only recalculate when something changes
    relations: RefCell<Vec<(CardValue, CardValue)>>,
    relations_dirty: Cell<bool>,

    // Learn your opponent to learn what you should do
    opponent_raise_count: i64,
    running_guess: Guess,

    // How many bytes have we output?
    byte_count: usize,
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
            byte_count: 0,
        }
    }
}

impl TourneyBot {
    fn add_relationship<S>(&mut self, log_string: S, strength: f64, a: CardValue, b: CardValue) where S: Borrow<str> {
        self.running_guess.update(a, b, strength as f32);
        if self.prob_engine.update(log_string.borrow(), &a, &b, strength) {
            self.debug_print(format!("[{}] Saw relationship {} -> {} with strength {:.2}...", log_string.borrow(), a, b, strength), 0.3);
            self.relations_dirty.set(true);
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

    fn debug_print(&mut self, string: String, necessity: f64) {
        let bytes_reserved = self.internal_state().len();
        let bytes_to_take = string.len();
        // let mut rng = rand::thread_rng();
        let bytes_remaining = FILE_BYTE_SIZE - bytes_reserved - self.byte_count - 1;
        if bytes_to_take as f64 / bytes_remaining as f64 <= necessity {
            self.byte_count += bytes_to_take;
            println!("{}", string);
        }
    }

    fn internal_state(&self) -> String {
        use std::fmt::Write;

        let mut string = String::new();
        let relations = self.relations();
        writeln!(string, "({:.2}%)\n{}", 100.0 * (1.0 - (self.relations().possibilities() as f64 / 6227020800.0)), relations.debug_relations()).unwrap();
        writeln!(string, "{:?}", self.running_guess).unwrap();
        writeln!(string, "{}", self.prob_engine.probabilities().into_iter().map(|((a, b), p)| format!("{} -> {} P({:.4})", a, b, p)).format("\n")).unwrap();
        writeln!(string, "Detected OPR {:.2}%", self.opponent_raise_count as f64 / 1000.0).unwrap();
        let ignored_rules = self.prob_engine.inconsistent_rule_names();
        for rule in ignored_rules {
            writeln!(string, "You should check rule [{}] for inconsistencies.", rule).unwrap();
        }
        string
    }
}

impl PokerBot for TourneyBot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) {
        self.debug_print(format!("Round #{} {:.0}", gs.round_num, gs.game_clock), gs.round_num as f64 / 1000.0);
        // let relations = self.relations();
        // let sample_space_size = relations.possibilities();
        // println!("Sample space size -> {}", sample_space_size.to_formatted_string(&Locale::en));
        // In the unlikely event we actually calculate a "for certain" ordering, just keep it until we violate it enough
        if true { // sample_space_size > SAMPLE_GUESS_THRESHOLD {
            // let new_order = generate_ordering(&relations);
            let new_order = generate_ordering(&self.relations());
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
        // println!("Cards: {} {} <{}>", my_cards.print(), opp_cards.print(), board_cards.iter().format(", "));
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
            let mut print_prediction = |my_hand, opp_hand, delta| {
                let state = |n: i64| if n > 0 {
                    "won"
                } else if n < 0 {
                    "lost"
                } else {
                    "draw"
                };
    
                // println!("locally predicted {}", state(delta));
                // println!("server said {}", state(my_delta));
                self.debug_print(format!("expected {} ({})", state(delta), delta.signum() == my_delta.signum()), 0.5);
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

                    self.debug_print(format!("Winner hand: {} Loser hand: {}", actual_winner, actual_loser), 0.5);

                    if my_delta != 0 {
                        match (actual_winner, actual_loser) {
                            // Same hand type relations
                            (Hand::Pair(winner), Hand::Pair(loser)) => self.add_relationship("pair -> pair", 0.9, showdown_engine.highest_card_value(loser), showdown_engine.highest_card_value(winner)),
                            (Hand::ThreeOfAKind(winner), Hand::ThreeOfAKind(loser)) => self.add_relationship("3k -> 3k", 0.9, showdown_engine.highest_card_value(loser), showdown_engine.highest_card_value(winner)),
                            (Hand::FourOfAKind(winner), Hand::FourOfAKind(loser)) => self.add_relationship("4k -> 4k", 0.9, showdown_engine.highest_card_value(loser), showdown_engine.highest_card_value(winner)),
                            (Hand::FullHouse(winner), Hand::FullHouse(loser)) => {
                                let (winner_triple_value, winner_pair_value) = {
                                    let values = ShowdownEngine::values(winner.iter());
                                    assert!(values.len() == 2);
                                    let counts: Vec<_> = values.iter().map(|x| ShowdownEngine::count(winner.iter().copied(), x)).collect();
                                    // There are only 5 cards max, so one must be higher
                                    if counts[0] > counts[1] {
                                        (values[0], values[1])
                                    } else {
                                        (values[1], values[0])
                                    }
                                };
                                let (loser_triple_value, loser_pair_value) = {
                                    let values = ShowdownEngine::values(loser.iter());
                                    assert!(values.len() == 2);
                                    let counts: Vec<_> = values.iter().map(|x| ShowdownEngine::count(winner.iter().copied(), x)).collect();
                                    if counts[0] > counts[1] {
                                        (values[0], values[1])
                                    } else {
                                        (values[1], values[0])
                                    }
                                };
                                if winner_triple_value != loser_triple_value {
                                    self.add_relationship("fh -> fh (ltv -> wtv)", 0.9, loser_triple_value, winner_triple_value);
                                } else {
                                    self.add_relationship("fh -> fh (lpv -> wpv)", 0.9, loser_pair_value, winner_pair_value);
                                }
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

        if continue_cost > 1 && self.opponent_raise_count < gs.round_num {
            self.opponent_raise_count += 1;
        }

        let order_confidence = 1.0 - (self.relations().possibilities() as f64 / 6227020800.0);

        // println!("Pot {} my stack {} opp stack {} CC {}", pot_total, my_stack, opp_stack, continue_cost);
        // println!("My cards [{}]", my_cards.iter().format(", "));
        // println!("Board cards [{}]", board_cards.iter().format(", "));
        let showdown_engine = ShowdownEngine::new(self.ordering);

        let my_best = showdown_engine.process_hand(&vec![my_cards.iter().as_slice(), board_cards].into_iter().flat_map(|x: &[Card]| x.iter().copied()).collect::<Vec<Card>>());
        let raise: f64 = if let Some(board_cards) = Some(board_cards).filter(|x| !x.is_empty()) {
            // Flop, Turn, or River (we have some board information)
            let board_best = showdown_engine.process_hand(board_cards);
            self.debug_print(format!("({}) board best {} my best {}", if street == 3 { "flop" } else if street == 4 { "turn" } else { "river" }, board_best, my_best), 0.5);
            let hand_relationship = showdown_engine.compare_potential_hands(&my_best, &board_best);
            match hand_relationship {
                Ordering::Greater => {
                    // Our hand beats the board
                    match (my_best, board_best) {
                        (PotentialHand::Hand(Hand::FourOfAKind(hand)), _) | (PotentialHand::Hand(Hand::ThreeOfAKind(hand)), _) => {
                            //let high_value = ShowdownEngine::values(hand.cards().                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        into_iter()).into_iter().max_by(|a, b| showdown_engine.value_order(&a, &b)).unwrap();
                            //if self.running_guess.predicted_value(ShowEdown::Engine)
                            pot_total as f64
                        },// rng.gen_range(0.5, 2.0) * pot_total as f64, // We are relatively confident in this hand
                        (PotentialHand::Hand(Hand::FullHouse(_)), _) => { // Don't play full houses yet
                            0.0
                        },
                        (PotentialHand::Hand(Hand::Pair(pair)), PotentialHand::HighCard(card)) => {
                            // Look at guess to find the relative strength of our hand
                            let pair_value = ShowdownEngine::values(pair.iter())[0];
                            let high_card_value = card.value();
                            if self.running_guess.predicted_value(pair_value) - self.running_guess.predicted_value(high_card_value) > 4.0 {
                                rng.gen_range(0.5, 0.8) * pot_total as f64
                            } else {
                                0.0 // We aren't really confident in this pair
                            }
                        },
                        _ => 0.0 // check-fold
                    }
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
            self.debug_print(format!("(pre-flop) my best {}", my_best), 0.1);
            match my_best {
                PotentialHand::Hand(hand) => { // We already have a hand (which means we have pocket pairs)
                    let max_hand_value = ShowdownEngine::values(hand.cards().into_iter()).into_iter().max_by(|a, b| showdown_engine.value_order(&a, &b)).unwrap();
                    if self.running_guess.predicted_value(max_hand_value) - 7.0 > 3.0 || rng.gen_bool(0.5 * (1.0 - order_confidence)) {
                        // We are slightly confident this is high
                        rng.gen_range(0.1 * order_confidence as f64, 0.25 * order_confidence as f64) * pot_total as f64
                    } else {
                        0.0
                    }
                },
                hand => {
                    // Look at our guess to predict approx how strong the hand is
                    let max_hand_value = ShowdownEngine::values(hand.cards().into_iter()).into_iter().max_by(|a, b| showdown_engine.value_order(&a, &b)).unwrap();
                    if self.running_guess.predicted_value(max_hand_value) - 7.0 > 5.0 {
                        // We are slightly confident this is high
                        rng.gen_range(0.0, 0.25 * order_confidence) * pot_total as f64
                    } else {
                        0.0
                    }
                }
            }
        };

        self.debug_print(format!("Raise worth: {}", raise), 0.5);

        let raise = raise as u64;

        let act = |fail_action| if (legal_actions & ActionType::CHECK) == ActionType::CHECK {
            Action::Check
        } else {
            fail_action
        };

        if raise > continue_cost {
            // Bound the raise
            let [rb_min, rb_max] = rs.raise_bounds();
            let raise = if (raise as u64) > rb_max {
                rb_max
            } else if (raise as u64) < rb_min {
                rb_min
            } else {
                raise as u64
            };

            // We think this hand is worth it!
            if (legal_actions & ActionType::RAISE) == ActionType::RAISE {
                if continue_cost < raise {
                    act(Action::Call)
                } else {
                    Action::Raise(raise)
                }
            } else {
                act(Action::Call)
            }
        } else {
            // Gain some data points
            if gs.round_num > 100 {
                let opr = (self.opponent_raise_count as f64 / gs.round_num as f64).min(1.0);
                self.debug_print(format!("Opponent Raise Percent: {:.2}%", opr * 100.0), 0.1);
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
                    if continue_cost > 0 {
                        Action::Raise(raise)
                    } else {
                        act(Action::Call)
                    }
                } else {
                    act(Action::Fold)
                }
            }
        }
    }
}

impl Drop for TourneyBot {
    fn drop(&mut self) {
        println!("{}", self.internal_state());
    }
}
