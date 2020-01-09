use super::super::skeleton::{
    bot::PokerBot,
    actions::{Action, ActionType},
    states::{STARTING_STACK, GameState, RoundState, TerminalState}
};

pub struct Lesson2Bot {

}

impl Default for Lesson2Bot {
    fn default() -> Lesson2Bot {
        Lesson2Bot {
            
        }
    }
}

impl PokerBot for Lesson2Bot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) {

    }

    fn handle_round_over(&mut self, gs: &GameState, ts: &TerminalState, player_index: usize) {

    }

    fn get_action(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) -> Action {
        Action::Fold
    }
}