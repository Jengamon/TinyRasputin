use super::actions::Action;
use super::states::{GameState, RoundState, TerminalState};

pub trait PokerBot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize);
    fn handle_round_over(&mut self, gs: &GameState, ts: &TerminalState, player_index: usize);
    fn get_action(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) -> Action;
}
