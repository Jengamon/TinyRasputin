use super::{
    actions::{ActionType, Action},
    cards::{CardDeck, CardHand}
};
use std::cmp::{min, max};

pub const NUM_ROUNDS: i64 = 1000;
pub const STARTING_STACK: i64 = 200;
pub const BIG_BLIND: i64 = 2;
pub const SMALL_BLIND: i64 = 1;

#[derive(Debug, Clone, Copy)]
pub struct GameState {
    pub bankroll: i64,
    pub game_clock: f64,
    pub round_num: i64
}

/// Final state of a poker round corresponding to payoffs
#[derive(Debug, Clone)]
pub struct TerminalState {
    pub deltas: [i64; 2],
    pub previous: RoundState,
}

/// Encodes the game tree for one round of poker
#[derive(Debug, Clone)]
pub struct RoundState {
    pub button: i32,
    pub street: i32,
    pub pips: [i64; 2],
    pub stacks: [i64; 2],
    pub hands: [Option<CardHand>; 2],
    pub deck: CardDeck,
    pub previous: Option<Box<RoundState>>,
}

#[derive(Debug, Clone)]
pub enum StateResult {
    Round(RoundState),
    Terminal(TerminalState),
}

impl RoundState {
    /// Compares the players' hands and computes payoffs.
    pub fn showdown(&self) -> TerminalState {
        TerminalState { previous: self.clone(), deltas: [0, 0] }
    }

    /// Returns a mask which corresponds to the active player's legal moves.
    pub fn legal_actions(&self) -> ActionType {
        let active: usize = self.button as usize % 2;
        let continue_cost = self.pips[1 - active] - self.pips[active];
        if continue_cost == 0 {
            // We can only raise the stakes if both players can afford it
            let bets_forbidden = (self.stacks[0] == 0) | (self.stacks[1] == 0);
            if bets_forbidden { return ActionType::CHECK; }
            return ActionType::CHECK | ActionType::RAISE;
        }
        // continue_cost > 0
        // similarly, re-raising is only allowed if both players can afford it
        let raises_forbidden = (continue_cost == self.stacks[active]) | (self.stacks[1 - active] == 0);
        if raises_forbidden { return ActionType::FOLD | ActionType::CALL; }
        return ActionType::FOLD | ActionType::CALL | ActionType::RAISE;
    }

    /// Returns an array of the minimum and maximum legal raises
    pub fn raise_bounds(&self) -> [i64; 2] {
        let active: usize = self.button as usize % 2;
        let continue_cost = self.pips[1 - active] - self.pips[active];
        let max_contrib = min(self.stacks[active], self.stacks[1-active] + continue_cost);
        let min_contrib = min(max_contrib, continue_cost + max(continue_cost, BIG_BLIND));
        [self.pips[active] + min_contrib, self.pips[active] + max_contrib]
    }

    /// Resets the players' pips and advances the game tree to the next round of betting.
    pub fn proceed_street(&self) -> StateResult {
        if self.street == 5 {
            return StateResult::Terminal(self.showdown());
        }
        let new_street;
        if self.street == 0 {
            new_street = 3;
        } else {
            new_street = self.street + 1;
        }
        StateResult::Round(RoundState {
            button: 1,
            street: new_street,
            pips: [0, 0],
            stacks: self.stacks,
            hands: self.hands,
            deck: self.deck.clone(),
            previous: Some(Box::new(self.clone()))
        })
    }

    /// Advances the game tree by one action performed
    pub fn proceed(&self, action: Action) -> StateResult {
        let active: usize = self.button as usize % 2;
        match action {
            Action::Fold => {
                let delta;
                if active == 0 {
                    delta = self.stacks[0] - STARTING_STACK
                } else {
                    delta = STARTING_STACK - self.stacks[1]
                }
                StateResult::Terminal(TerminalState{
                    deltas: [delta, -delta],
                    previous: self.clone()
                })
            },
            Action::Call => {
                if self.button == 0 {
                    return StateResult::Round(RoundState {
                        button: 1,
                        street: 0,
                        pips: [BIG_BLIND, BIG_BLIND],
                        stacks: [STARTING_STACK - BIG_BLIND, STARTING_STACK - BIG_BLIND],
                        hands: self.hands,
                        deck: self.deck.clone(),
                        previous: Some(Box::new(self.clone()))
                    })
                }
                let mut new_pips = self.pips;
                let mut new_stacks = self.stacks;
                let contrib = new_pips[1 - active] - new_pips[active];
                new_stacks[active] -= contrib;
                new_pips[active] += contrib;
                let state = RoundState{
                    button: self.button + 1,
                    street: self.street,
                    pips: new_pips,
                    stacks: new_stacks,
                    hands: self.hands,
                    deck: self.deck.clone(),
                    previous: Some(Box::new(self.clone()))
                };
                state.proceed_street()
            },
            Action::Check => {
                if (self.street == 0 && self.button > 0) || self.button > 1 {
                    // both players acted
                    return self.proceed_street()
                }
                // let opponent act
                StateResult::Round(RoundState {
                    button: self.button + 1,
                    street: self.street,
                    pips: self.pips,
                    stacks: self.stacks,
                    hands: self.hands,
                    deck: self.deck.clone(),
                    previous: Some(Box::new(self.clone()))
                })
            },
            Action::Raise(ref amt) => {
                let mut new_pips = self.pips;
                let mut new_stacks = self.stacks;
                let contrib = amt - new_pips[active];
                new_stacks[active] -= contrib;
                new_pips[active] += contrib;
                StateResult::Round(RoundState {
                    button: self.button + 1,
                    street: self.street,
                    pips: new_pips,
                    stacks: new_stacks,
                    hands: self.hands,
                    deck: self.deck.clone(),
                    previous: Some(Box::new(self.clone()))
                })
            }
        }
    }
}
