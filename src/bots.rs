use super::skeleton::{
    bot::PokerBot,
    actions::{Action, ActionType},
    states::{GameState, RoundState, TerminalState}
};

pub struct TestBot {

}

impl Default for TestBot {
    fn default() -> TestBot {
        TestBot {

        }
    }
}

impl PokerBot for TestBot {
    fn handle_new_round(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) {
        println!("Starting new round. Progress: {:?}", gs);
    }

    fn handle_round_over(&mut self, gs: &GameState, ts: &TerminalState, player_index: usize) {
        println!("Round over! Progress: {:?}", gs);
    }

    fn get_action(&mut self, gs: &GameState, rs: &RoundState, player_index: usize) -> Action {
        println!("Current game state: {:?}", rs);
        let la = rs.legal_actions();
        if (la & ActionType::RAISE).bits() != 0 {
            println!("Raised by {}", rs.raise_bounds()[1]);
            Action::Raise(rs.raise_bounds()[1])
        } else {
            if (la & ActionType::CALL).bits() != 0 {
                Action::Call
            } else {
                Action::Check
            }
        }
    }
}
