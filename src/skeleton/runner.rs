use std::net::{TcpStream, ToSocketAddrs};
use super::bot::PokerBot;
use std::io::{prelude::*, BufReader};
use super::actions::{Action, ActionType};
use super::states::{SMALL_BLIND, BIG_BLIND, STARTING_STACK, GameState, RoundState, TerminalState, StateResult};
use super::cards::{Card, CardHand, CardDeck};
use std::time::{Duration};

const CONNECT_TIMEOUT: u64 = 10;
const READ_TIMEOUT: u64 = 10;
const WRITE_TIMEOUT: u64 = 1;

pub struct Runner<'a> {
    stream: BufReader<TcpStream>,
    bot: &'a mut Box<dyn PokerBot>,
}

impl<'a> Runner<'a> {
    /// Runs a PokerBot using the Runner
    pub fn run_bot<TS>(bot: &'a mut Box<dyn PokerBot>, addr: TS) -> std::io::Result<()> where TS: ToSocketAddrs {
        if let Some(addr) = addr.to_socket_addrs()?.nth(0) {
            let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(CONNECT_TIMEOUT))?;
            stream.set_nodelay(true).expect("set_nodelay call failed");
            stream.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT))).expect("read_timeout call failed");
            stream.set_write_timeout(Some(Duration::from_secs(WRITE_TIMEOUT))).expect("write_timeout call failed");
            // stream.set_nonblocking(true).expect("set_nonblocking call failed");
            let mut runner = Runner {
                stream: BufReader::new(stream),
                bot,
            };
            runner.run()
        } else {
            panic!("No addresses were sent to run on");
        }
    }

    /// Returns an incoming message from the engine.
    pub fn receive(&mut self) -> std::io::Result<Vec<String>> {
        // Check stream for errors. If there is one, disconnect.
        if let Some(error) = self.stream.get_ref().take_error()? {
            println!("[SkelyBoi] Disconnecting because of stream error {}", error);
            return Err(error)
        }

        let mut s = String::new();
        self.stream.read_line(&mut s)?;
        Ok(s.trim().split(" ").map(|x| x.trim().to_string()).collect::<_>())
    }

    /// Send an action message to the engine
    pub fn send(&mut self, act: Action) -> std::io::Result<()> {
        // Check stream for errors. If there is one, disconnect.
        if let Some(error) = self.stream.get_ref().take_error()? {
            println!("[SkelyBoi] Disconnecting because of stream error {}", error);
            return Err(error)
        }

        let code = match act {
            Action::Fold => "F".into(),
            Action::Call => "C".into(),
            Action::Check => "K".into(),
            Action::Raise(amt) => format!("R{}", amt)
        };
        writeln!(self.stream.get_mut(), "{}", code)?;
        Ok(())
        // self.stream.get_mut().flush()
    }

    /// Processes actions from the engine
    pub fn run(&mut self) -> std::io::Result<()> {
        let mut game_state = GameState {
            bankroll: 0,
            game_clock: 0.0,
            round_num: 1
        };
        let mut round_state = None;
        let mut terminal_state = None;
        let mut player_index = 0usize;
        loop {
            for clause in self.receive()? {
                let chr = clause.chars().nth(0).unwrap();
                let arg = clause.chars().skip(1).collect::<String>();
                // println!("[SkelyBoi] Received {}", clause);
                match chr {
                    // Set game clock
                    'T' => game_state = GameState {
                        bankroll: game_state.bankroll,
                        game_clock: arg.parse::<f64>().expect("Expected a float for game time"),
                        round_num: game_state.round_num
                    },
                    // Set player index (also referred to as "active")
                    'P' => player_index = arg.parse::<usize>().expect("Expected an unsigned integer for player index"),
                    // Set our hand
                    'H' => {
                        let mut hands = [None, None];
                        let proposed_hand = arg.split(",").collect::<Vec<_>>();
                        assert!(proposed_hand.len() == 2);
                        hands[player_index] = Some(CardHand([
                            proposed_hand[0].parse::<Card>().expect("Expected card in hand 1"), 
                            proposed_hand[1].parse::<Card>().expect("Expected card in hand 2")
                        ]));
                        let pips = [SMALL_BLIND, BIG_BLIND];
                        let stacks = [STARTING_STACK - SMALL_BLIND, STARTING_STACK - BIG_BLIND];
                        round_state = Some(RoundState {
                            button: 0,
                            street: 0,
                            pips,
                            stacks,
                            hands,
                            deck: CardDeck(vec![]),
                            previous: None
                        });
                        self.bot.handle_new_round(&game_state, &round_state.clone().unwrap(), player_index);
                    },
                    // A fold action
                    'F' => if let Some(rs) = round_state.clone() {
                        match rs.proceed(Action::Fold) {
                            StateResult::Round(r) => round_state = Some(r),
                            StateResult::Terminal(t) => {
                                terminal_state = Some(t);
                            }
                        }
                    } else {
                        panic!("Round state must exist for this action")
                    },
                    // A call action
                    'C' => if let Some(rs) = round_state.clone() {
                        match rs.proceed(Action::Call) {
                            StateResult::Round(r) => round_state = Some(r),
                            StateResult::Terminal(t) => {
                                terminal_state = Some(t);
                            }
                        }
                    } else {
                        panic!("Round state must exist for this action")
                    },
                    // A check action
                    'K' => if let Some(rs) = round_state.clone() {
                        match rs.proceed(Action::Check) {
                            StateResult::Round(r) => round_state = Some(r),
                            StateResult::Terminal(t) => {
                                terminal_state = Some(t);
                            }
                        }
                    } else {
                        panic!("Round state must exist for this action")
                    },
                    // A raise action
                    'R' => if let Some(rs) = round_state.clone() {
                        match rs.proceed(Action::Raise(arg.parse::<i64>().expect("Expected an integer for raise number"))) {
                            StateResult::Round(r) => round_state = Some(r),
                            StateResult::Terminal(t) => {
                                terminal_state = Some(t);
                            }
                        }
                    } else {
                        panic!("Round state must exist for this action")
                    },
                    // The deck was updated
                    'B' => if let Some(rs) = round_state.clone() {
                        round_state = Some(RoundState {
                            button: rs.button,
                            street: rs.street,
                            pips: rs.pips,
                            stacks: rs.stacks,
                            hands: rs.hands,
                            deck: CardDeck(arg.split(",").enumerate().map(|(i, x)| x.parse::<Card>().expect(&format!("Expected card in deck {}", i))).collect()),
                            previous: rs.previous
                        })
                    } else {
                        panic!("Round state must exist for this action")
                    },
                    // Reveal the opponent's hand
                    'O' => if let Some(rs) = round_state.clone() {
                        if let Some(prs) = rs.previous {
                            // backtrack
                            let mut revised_hands = prs.hands;
                            let prevised_hand = arg.split(",").collect::<Vec<_>>();
                            assert!(prevised_hand.len() == 2);
                            revised_hands[1 - player_index] = Some(CardHand([
                                prevised_hand[0].parse::<Card>().expect("Expected card in opponent hand 1"),
                                prevised_hand[1].parse::<Card>().expect("Expected card in opponent hand 2")
                            ]));
                            // rebuild history
                            let new_round_state = RoundState {
                                button: prs.button,
                                street: prs.street,
                                pips: prs.pips,
                                stacks: prs.stacks,
                                hands: revised_hands,
                                deck: prs.deck,
                                previous: prs.previous
                            };
                            round_state = None;
                            terminal_state = Some(TerminalState{
                                deltas: [0, 0],
                                previous: new_round_state
                            })
                        }
                    } else {
                        panic!("Round state must exist for this action")
                    },
                    // Delta has been calculated
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
                        round_state = None;
                    },
                    // End the game
                    'Q' => return Ok(()),
                    c => unreachable!("Invalid command sent from engine: {}", c)
                }
            }

            if let Some(round_state) = round_state.clone() {
                assert!(player_index == round_state.button as usize % 2);
                let bot_action = self.bot.get_action(&game_state, &round_state, player_index);
                let legal_actions = round_state.legal_actions();
                // Coerce the action to the next best action
                let action = match bot_action {
                    Action::Raise(raise) => if(legal_actions & ActionType::RAISE) == ActionType::RAISE {
                        Action::Raise(raise)
                    } else {
                        if(legal_actions & ActionType::CHECK) == ActionType::CHECK {
                            Action::Check
                        } else {
                            Action::Call
                        }
                    },
                    Action::Check => if (legal_actions & ActionType::CHECK) == ActionType::CHECK {
                        Action::Check
                    } else {
                        Action::Fold
                    },
                    Action::Call => if (legal_actions & ActionType::CHECK) == ActionType::CHECK {
                        Action::Check
                    } else {
                        Action::Call
                    },
                    Action::Fold => if (legal_actions & ActionType::CHECK) == ActionType::CHECK {
                        Action::Check
                    } else {
                        Action::Fold
                    }
                };
                if bot_action != action {
                    println!("[SkelyBoi] Coerced {:?} into {:?}", bot_action, action);
                }
                self.send(action)?
            } else {
                self.send(Action::Check)?
            }
        }
    }
}
