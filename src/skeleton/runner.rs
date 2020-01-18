use std::net::{TcpStream, Shutdown, ToSocketAddrs};
use super::bot::PokerBot;
use std::io::{prelude::*, BufReader};
use crate::into_cards;
use super::actions::{Action, ActionType};
use super::states::{SMALL_BLIND, BIG_BLIND, STARTING_STACK, GameState, RoundState, TerminalState, StateResult};
use super::cards::{Card, CardHand, CardDeck};
use std::time::{Duration, Instant};
use crate::debug_println;
use super::thread_pool::ThreadPool;
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, LockResult};
use itertools::Itertools;

const CONNECT_TIMEOUT: u64 = 10;
const WRITE_TIMEOUT: u64 = 2;
const PLAYER_INDEX_ORDERING: Ordering = Ordering::SeqCst;
const THREAD_COUNT: usize = 3;
const SLEEP_DURATION: u64 = 300;

pub struct Runner {
    socket: Arc<Mutex<Socket>>,
    runner_start: Instant,
}

#[derive(Debug)]
struct Socket {
    stream: BufReader<TcpStream>,
    read_queue: Vec<ServerAction>,
    write_queue: Vec<Action>,
}

#[derive(Debug, Clone)]
enum ServerAction {
    SetGameClock(f32), // T
    SetPlayerIndex(usize), // P
    SetPlayerHand(CardHand), // H
    PlayFold, // F
    PlayCall, // C
    PlayCheck, // K
    PlayRaise(u32), // R
    UpdateDeck(CardDeck), // B
    RevealOpponentHand(CardHand), // O
    Delta(i32), // D
    Quit // Q
}

impl Socket {
    fn new(stream: BufReader<TcpStream>) -> Socket {
        Socket {
            stream,
            read_queue: vec![],
            write_queue: vec![],
        }
    }

    /// Returns an incoming message from the engine.
    fn receive(&mut self) -> Vec<ServerAction> {
        let mess = self.read_queue.drain(..).collect();
        mess
    }

    /// Send an action message to the engine
    fn send(&mut self, act: Action) {
        self.write_queue.push(act);
    }

    // Do all I/O processing here
    fn sync(&mut self) {
        let actions_to_write = self.write_queue.drain(..).collect::<Vec<_>>();
        let mut server_process = vec![];
        // Write as much as we can, then read all the actions we can
        let ref mut socket = self.stream;

        // Check stream for errors. If there is one, disconnect.
        if let Ok(Some(error)) = socket.get_ref().take_error() {
            panic!("[Runner] Disconnecting because of stream error {}", error);
        }

        // Read
        let mut s = String::new();
        if let Ok(data) = socket.read_line(&mut s) {
            for action in s.trim().split(" ").map(|x| x.trim().to_string()) {
                if !action.is_empty() {
                    server_process.push(action);
                }
            }
        } else {
            panic!("Socket read error")
        }

        // If we read any actions, dump any we were going to push
        if server_process.len() > 0 {
            debug_println!("[Runner] Read actions from socket [{}]", server_process.iter().format(", "));
        }

        debug_println!("[Runner] Syncing socket with actions [{}]", actions_to_write.iter().map(|x| format!("{:?}", x)).format(", "));

        // Ok, now we own the socket. write everything and then read. All shouldn't block for too long.
        for action in actions_to_write.into_iter() {
            let code = match action {
                Action::Fold => "F".into(),
                Action::Call => "C".into(),
                Action::Check => "K".into(),
                Action::Raise(amt) => format!("R{}", amt)
            };
            match writeln!(socket.get_mut(), "{}", code) {
                Err(e) => debug_println!("[Skelyboi] On writing action {}, socket errored with error {}", code, e),
                _ => {}
            }
        }

        // Flush
        match socket.get_mut().flush() {
            Err(e) => debug_println!("[Skelyboi] On flushing, socket errored with error {}", e),
            _ => {}
        };

        for action in server_process.into_iter() {
            let act = action.chars().nth(0).unwrap();
            let arg = action.chars().skip(1).collect::<String>();
            let server_action = match act {
                'T' => ServerAction::SetGameClock(arg.parse::<f32>().expect("Expected float for game clock")),
                'P' => ServerAction::SetPlayerIndex(arg.parse::<usize>().expect("Expected positive integer for player index")),
                'H' => {
                    let cards = into_cards!(arg);
                    assert!(cards.len() == 2, "Server sent too many cards for player hand");
                    ServerAction::SetPlayerHand(CardHand([cards[0], cards[1]]))
                },
                'F' => ServerAction::PlayFold,
                'C' => ServerAction::PlayCall,
                'K' => ServerAction::PlayCheck,
                'R' => ServerAction::PlayRaise(arg.parse::<u32>().expect("Expected positive integer for raise amount")),
                'B' => ServerAction::UpdateDeck(CardDeck(into_cards!(arg))),
                'O' => {
                    let cards = into_cards!(arg);
                    assert!(cards.len() == 2, "Server sent too many cards for player hand");
                    ServerAction::RevealOpponentHand(CardHand([cards[0], cards[1]]))
                },
                'D' => ServerAction::Delta(arg.parse::<i32>().expect("Expected integer for delta")),
                'Q' => ServerAction::Quit,
                c => panic!("Unknown server command {} with arg {}", c, arg)
            };
            self.read_queue.push(server_action);
        }
    }
}

// Shutdown the socket even if we panic, and right when we panic
impl Drop for Socket {
    fn drop(&mut self) {
        match self.stream.get_mut().shutdown(Shutdown::Both) {
            Ok(()) => debug_println!("Successfully shut down socket"),
            // We don't really care about errors here, as our goal is simply to end the socket
            Err(_) => {}
        }
    }
}

impl Runner {
    /// Runs a PokerBot using the Runner
    pub fn run_bot<TS>(bot: Box<dyn PokerBot + Send + Sync>, addr: TS) -> std::io::Result<()> where TS: ToSocketAddrs {
        if let Some(addr) = addr.to_socket_addrs()?.nth(0) {
            let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(CONNECT_TIMEOUT))?;
            stream.set_nodelay(true).expect("set_nodelay call failed");
            // stream.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT))).expect("read_timeout call failed");
            stream.set_write_timeout(Some(Duration::from_secs(WRITE_TIMEOUT))).expect("write_timeout call failed");
            // stream.set_nonblocking(true).expect("set_nonblocking call failed");
            let mut runner = Runner {
                socket: Arc::new(Mutex::new(Socket::new(BufReader::new(stream)))),
                runner_start: Instant::now(),
            };
            Ok(runner.run(bot))
        } else {
            panic!("No addresses were sent to run on");
        }
    }

    // We never want to block access to state when we have write access to the bot, as
    // that is asking for a lockup to happen, so we have some functions that continually query
    // whether the device (piece of state) is actually ready for bot access
    // This function polls for unique access
    fn poll_until_write<T>(device: Arc<RwLock<T>>, device_id: &'static str) -> LockResult<RwLockWriteGuard<T>> {
        todo!()
    }

    // This function polls for read access
    fn poll_until_read<T>(device: Arc<RwLock<T>>, device_id: &'static str) -> LockResult<RwLockReadGuard<T>> {
        todo!()
    }

    /// Processes actions from the engine and never returns when called
    fn run(&mut self, bot: Box<dyn PokerBot + Send + Sync>) {
        let game_state = Arc::new(RwLock::new(GameState {
            bankroll: 0,
            game_clock: 0.0,
            round_num: 1
        }));
        let round_state: Arc<RwLock<Option<RoundState>>> = Arc::new(RwLock::new(None));
        let terminal_state: Arc<RwLock<Option<TerminalState>>> = Arc::new(RwLock::new(None));
        let bot = Arc::new(Mutex::new(bot)); // Wrap the bot in a read-write lock
        let player_index = Arc::new(AtomicUsize::new(0usize));
        let mut pool = ThreadPool::new(THREAD_COUNT).unwrap();
        loop {
            {
                let (socket, player_index) = (self.socket.clone(), player_index.clone());
                let (round_state, bot, game_state) = (round_state.clone(), bot.clone(), game_state.clone());

                pool.execute(9, move || {
                    // Acquire the round state if it is available, but DO NOT BLOCK ( but maybe block the socket for a bit... )
                    // let mut socket = socket.lock().unwrap();
                    let mut bot = bot.lock().unwrap();
                    let round_state = Runner::poll_until_read(round_state, "round").unwrap();
                    let game_state = Runner::poll_until_read(game_state, "game").unwrap();
                    // Check if there even is a round state
                    if let Some(ref round_state) = *round_state {
                        // Clone the current copy of round state
                        let player_index = player_index.load(PLAYER_INDEX_ORDERING);
                        assert!(player_index == round_state.button as usize % 2);
                        let bot_action = bot.get_action(&*game_state, round_state, player_index);
                        let legal_actions = round_state.legal_actions();
                        // Coerce the action to the next best action
                        let action = match bot_action {
                            Action::Raise(raise) => if (legal_actions & ActionType::RAISE) == ActionType::RAISE {
                                let [rb_min, rb_max] = round_state.raise_bounds();
                                if raise > rb_min && raise < rb_max {
                                    Action::Raise(raise)
                                } else {
                                    if(legal_actions & ActionType::CHECK) == ActionType::CHECK {
                                        Action::Check
                                    } else {
                                        Action::Call
                                    }
                                }
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
                            println!("[Runner] Coerced {:?} into {:?}. Check bot for error.", bot_action, action);
                        }
                        let mut socket = socket.lock().unwrap();
                        socket.send(action);
                    }
                });
            }

            let clauses = self.socket.clone().lock().unwrap().receive();
            for clause in clauses.into_iter() {
                //let chr = clause.chars().nth(0).unwrap();
                //let arg = clause.chars().skip(1).collect::<String>();
                // Get our current state.
                let (game_state, round_state, terminal_state, bot, player_index) = (game_state.clone(), round_state.clone(), terminal_state.clone(), bot.clone(), player_index.clone());
                // The main runner code is entirely run in thread pools! We reserve the main thread for getting the action from our bot and
                // either sending or receiving it, otherwise we should update entirely asyncrously
                debug_println!("[Runner] Received {:?}", clause);
                match clause.clone() {
                    // Set game clock
                    ServerAction::SetGameClock(clock) => pool.execute(0, move || {
                        let mut game_state = game_state.write().unwrap();
                        *game_state = GameState {
                            bankroll: game_state.bankroll,
                            game_clock: clock,
                            round_num: game_state.round_num
                        };
                    }),
                    // Set player index (also referred to as "active")
                    ServerAction::SetPlayerIndex(index) => player_index.store(index, PLAYER_INDEX_ORDERING),
                    // Set our hand
                    ServerAction::SetPlayerHand(hand) => pool.execute(1, move || {
                        let mut hands = [None, None];
                        let player_index = player_index.load(PLAYER_INDEX_ORDERING);
                        hands[player_index] = Some(hand);
                        let pips = [SMALL_BLIND, BIG_BLIND];
                        let stacks = [STARTING_STACK - SMALL_BLIND, STARTING_STACK - BIG_BLIND];
                        let mut bot = bot.lock().unwrap();
                        let mut round_state = Runner::poll_until_write(round_state, "round").unwrap();
                        let game_state = Runner::poll_until_read(game_state, "game").unwrap();
                        let round = RoundState {
                            button: 0,
                            street: 0,
                            pips,
                            stacks,
                            hands,
                            deck: CardDeck(vec![]),
                            previous: None
                        };
                        bot.handle_new_round(&*game_state, &round, player_index);
                        *round_state = Some(round);
                    }),
                    // A fold action
                    ServerAction::PlayFold => pool.execute(2, move || {
                        let mut round_state = round_state.write().unwrap();
                        let mut terminal_state = terminal_state.write().unwrap();
                        if let Some(ref rs) = *round_state {
                            match rs.proceed(Action::Fold) {
                                StateResult::Round(r) => *round_state = Some(r),
                                StateResult::Terminal(t) => {
                                    *terminal_state = Some(t);
                                }
                            }
                        } else {
                            panic!("Round state must exist for fold action")
                        }
                    }),
                    // A call action
                    ServerAction::PlayCall => pool.execute(3, move || {
                        let mut round_state = round_state.write().unwrap();
                        let mut terminal_state = terminal_state.write().unwrap();
                        if let Some(ref rs) = *round_state {
                            match rs.proceed(Action::Call) {
                                StateResult::Round(r) => *round_state = Some(r),
                                StateResult::Terminal(t) => {
                                    *terminal_state = Some(t);
                                }
                            }
                        } else {
                            panic!("Round state must exist for call action")
                        }
                    }),
                    // A check action
                    ServerAction::PlayCheck => pool.execute(4, move || {
                        let mut round_state = round_state.write().unwrap();
                        let mut terminal_state = terminal_state.write().unwrap();
                        if let Some(ref rs) = *round_state {
                            match rs.proceed(Action::Check) {
                                StateResult::Round(r) => *round_state = Some(r),
                                StateResult::Terminal(t) => {
                                    *terminal_state = Some(t);
                                }
                            }
                        } else {
                            panic!("Round state must exist for check action")
                        }
                    }),
                    // A raise action
                    ServerAction::PlayRaise(by) => pool.execute(5, move || {
                        let mut round_state = round_state.write().unwrap();
                        let mut terminal_state = terminal_state.write().unwrap();
                        if let Some(ref rs) = *round_state {
                            match rs.proceed(Action::Raise(by)) {
                                StateResult::Round(r) => *round_state = Some(r),
                                StateResult::Terminal(t) => {
                                    *terminal_state = Some(t);
                                }
                            }
                        } else {
                            panic!("Round state must exist for this action")
                        }
                    }),
                    // The deck was updated
                    ServerAction::UpdateDeck(deck) => pool.execute(6, move || {
                        let mut round_state = round_state.write().unwrap();
                        if let Some(ref rs) = *round_state {
                            *round_state = Some(RoundState {
                                button: rs.button,
                                street: rs.street,
                                pips: rs.pips,
                                stacks: rs.stacks,
                                hands: rs.hands,
                                deck,
                                previous: rs.previous.clone()
                            })
                        } else {
                            panic!("Round state must exist for this action")
                        }
                    }),
                    // Reveal the opponent's hand
                    ServerAction::RevealOpponentHand(hand) => pool.execute(7, move || {
                        let mut round_state = round_state.write().unwrap();
                        let mut terminal_state = terminal_state.write().unwrap();
                        if let Some(ref rs) = *round_state {
                            if let Some(ref prs) = rs.previous {
                                let player_index = player_index.load(PLAYER_INDEX_ORDERING);
                                // backtrack
                                let mut revised_hands = prs.hands;
                                revised_hands[1 - player_index] = Some(hand);
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
                                *round_state = None;
                                *terminal_state = Some(TerminalState{
                                    deltas: [0, 0],
                                    previous: new_round_state
                                })
                            }
                        } else {
                            panic!("Round state must exist for reveal action")
                        }
                    }),
                    // Delta has been calculated
                    ServerAction::Delta(delta) => pool.execute(8, move || {
                        let mut bot = bot.lock().unwrap();
                        let mut round_state = Runner::poll_until_write(round_state, "round").unwrap();
                        let mut game_state = Runner::poll_until_write(game_state, "game").unwrap();
                        let mut terminal_state = Runner::poll_until_write(terminal_state, "terminal").unwrap();
                        let player_index = player_index.load(PLAYER_INDEX_ORDERING);
                        assert!(terminal_state.is_some());
                        if let Some(ref tstate) = *terminal_state {
                            let mut deltas = [-delta, -delta];
                            deltas[player_index] = delta;
                            let term = TerminalState{
                                deltas,
                                previous: tstate.previous.clone()
                            };
                            *game_state = GameState {
                                bankroll: game_state.bankroll + delta as i64,
                                game_clock: game_state.game_clock,
                                round_num: game_state.round_num
                            };
                            bot.handle_round_over(&*game_state, &term, player_index);
                            *terminal_state = Some(term);
                            *game_state = GameState {
                                bankroll: game_state.bankroll,
                                game_clock: game_state.game_clock,
                                round_num: game_state.round_num + 1
                            };
                            *round_state = None;
                        } else {
                            unreachable!("No previous terminal state before delta command. Round should not be over...")
                        }
                    }),
                    // End the game
                    ServerAction::Quit => return,
                }
            }

            self.socket.lock().unwrap().sync();
            // std::thread::sleep(Duration::from_millis(SLEEP_DURATION));
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        let runtime = Instant::now() - self.runner_start;
        println!("[Runner] Ran for {:?}", runtime);
    }
}
