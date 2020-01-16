use tinyrasputin::skeleton::{
    bot::PokerBot,
    actions::{Action, ActionType},
    states::{STARTING_STACK, GameState, RoundState, TerminalState}
};

pub struct EmptyBot {

}

impl Default for EmptyBot {
    fn default() -> EmptyBot {
        EmptyBot {

        }
    }
}

impl PokerBot for EmptyBot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) {

    }

    fn handle_round_over(&mut self, gs: &GameState, ts: &TerminalState, player_index: usize) {

    }

    fn get_action(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) -> Action {
        Action::Fold
    }
}