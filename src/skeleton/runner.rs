use std::net::{TcpStream, ToSocketAddrs};
use super::bot::PokerBot;
use std::io::prelude::*;
use super::actions::Action;
use super::states::{SMALL_BLIND, BIG_BLIND, STARTING_STACK, GameState, RoundState, TerminalState, StateResult};

pub struct Runner {
    stream: TcpStream,
    bot: Box<dyn PokerBot>,
}

impl Runner {
    pub fn run_bot<PB: 'static, TS>(bot: PB, addr: TS) -> std::io::Result<()> where PB: PokerBot, TS: ToSocketAddrs {
        let stream = TcpStream::connect(addr.to_socket_addrs().unwrap().nth(0).unwrap())?;
        let mut runner = Runner {
            stream,
            bot: Box::new(bot)
        };
        runner.run()
    }

    /// Returns an incoming message from the engine.
    pub fn receive(&mut self) -> std::io::Result<Vec<String>> {
        let mut s = String::new();
        self.stream.read_to_string(&mut s)?;
        Ok(s.trim().split(" ").map(|x| x.to_string()).collect::<_>())
    }

    pub fn send(&mut self, act: Action) -> std::io::Result<()> {
        let code = match act {
            Action::Fold => "F".into(),
            Action::Call => "C".into(),
            Action::Check => "K".into(),
            Action::Raise(amt) => format!("R{}", amt)
        };
        write!(self.stream, "{}", code)
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        let mut game_state = GameState {
            bankroll: 0,
            game_clock: 0.0,
            round_num: 1
        };
        let mut round_state = None;
        let mut terminal_state = None;
        let mut player_index = 0usize;
        let mut round_flag = true;
        loop {
            for clause in self.receive()? {
                let char = clause.chars().nth(0).unwrap();
                let arg = clause.chars().skip(1).collect::<String>();
                match char {
                    'T' => game_state = GameState {
                        bankroll: game_state.bankroll,
                        game_clock: arg.parse::<f64>().expect("Expected a float for game time"),
                        round_num: game_state.round_num
                    },
                    'P' => player_index = arg.parse::<usize>().expect("Expected an unsigned integer for player index"),
                    'H' => {
                        let mut hands = [["".to_string(), "".to_string()], ["".to_string(), "".to_string()]];
                        let proposed_hand = arg.split(",").collect::<Vec<_>>();
                        assert!(proposed_hand.len() == 2);
                        hands[player_index][0] = proposed_hand[0].to_string();
                        hands[player_index][1] = proposed_hand[1].to_string();
                        let pips = [SMALL_BLIND, BIG_BLIND];
                        let stacks = [STARTING_STACK - SMALL_BLIND, STARTING_STACK - BIG_BLIND];
                        round_state = Some(RoundState {
                            button: 0,
                            street: 0,
                            pips,
                            stacks,
                            hands,
                            deck: vec![],
                            previous: None
                        });
                        if round_flag {
                            self.bot.handle_new_round(&game_state, &round_state.clone().unwrap(), player_index);
                            round_flag = false;
                        }
                    },
                    'F' => if let Some(rs) = round_state.clone() {
                        match rs.proceed(Action::Fold) {
                            StateResult::Round(r) => round_state = Some(r),
                            StateResult::Terminal(t) => terminal_state = Some(t),
                        }
                    },
                    'C' => if let Some(rs) = round_state.clone() {
                        match rs.proceed(Action::Call) {
                            StateResult::Round(r) => round_state = Some(r),
                            StateResult::Terminal(t) => terminal_state = Some(t),
                        }
                    },
                    'K' => if let Some(rs) = round_state.clone() {
                        match rs.proceed(Action::Check) {
                            StateResult::Round(r) => round_state = Some(r),
                            StateResult::Terminal(t) => terminal_state = Some(t),
                        }
                    },
                    'R' => if let Some(rs) = round_state.clone() {
                        match rs.proceed(Action::Raise(arg.parse::<i64>().expect("Expected an integer for raise number"))) {
                            StateResult::Round(r) => round_state = Some(r),
                            StateResult::Terminal(t) => terminal_state = Some(t),
                        }
                    },
                    'B' => if let Some(rs) = round_state.clone() {
                        round_state = Some(RoundState {
                            button: rs.button,
                            street: rs.street,
                            pips: rs.pips,
                            stacks: rs.stacks,
                            hands: rs.hands.clone(),
                            deck: arg.split(",").map(|x| x.to_string()).collect(),
                            previous: rs.previous
                        })
                    },
                    'O' => if let Some(rs) = round_state.clone() {
                        if let Some(prs) = rs.previous {
                            // backtrack
                            let mut revised_hands = prs.hands;
                            let prevised_hands = arg.split(",").collect::<Vec<_>>();
                            assert!(prevised_hands.len() == 2);
                            revised_hands[1 - player_index][0] = prevised_hands[0].to_string();
                            revised_hands[1 - player_index][1] = prevised_hands[1].to_string();
                            // rebuild history
                            let new_round_state = RoundState {
                                button: prs.button,
                                street: prs.street,
                                pips: prs.pips,
                                stacks: prs.stacks,
                                hands: revised_hands,
                                deck: prs.deck.clone(),
                                previous: prs.previous.clone()
                            };
                            round_state = Some(new_round_state.clone());
                            terminal_state = Some(TerminalState{
                                deltas: [0, 0],
                                previous: new_round_state
                            })
                        }
                    },
                    'D' => {
                        assert!(terminal_state.is_some());
                        let delta = arg.parse::<i64>().expect("Expected an integer when calculating deltas");
                        let mut deltas = [-delta, -delta];
                        deltas[player_index] = delta;
                        terminal_state = Some(TerminalState{
                            deltas,
                            previous: terminal_state.unwrap().previous
                        });
                        game_state = GameState {
                            bankroll: game_state.bankroll + delta,
                            game_clock: game_state.game_clock,
                            round_num: game_state.round_num
                        };
                        self.bot.handle_round_over(&game_state, &terminal_state.clone().unwrap(), player_index);
                        game_state = GameState {
                            bankroll: game_state.bankroll,
                            game_clock: game_state.game_clock,
                            round_num: game_state.round_num + 1
                        };
                        round_flag = true;
                    },
                    'Q' => return Ok(()),
                    _ => unreachable!()
                }
            }
            if round_flag { // ack the engine
                self.send(Action::Check)?
            } else {
                if let Some(round_state) = round_state.clone() {
                    assert!(player_index == round_state.button as usize % 2);
                    let action = self.bot.get_action(&game_state, &round_state, player_index);
                    self.send(action)?
                } else {
                    unreachable!("Error in server message: No round state")
                }
            }
        }
    }
}
