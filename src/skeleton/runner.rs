use std::net::{TcpStream, Shutdown, ToSocketAddrs};
use super::bot::PokerBot;
use std::io::{prelude::*, BufReader, ErrorKind};
use crate::into_cards;
use super::actions::{Action, ActionType};
use super::states::{SMALL_BLIND, BIG_BLIND, STARTING_STACK, GameState, RoundState, TerminalState, StateResult};
use super::cards::{Card, CardHand, CardDeck};
use std::time::{Duration, Instant};
use crate::debug_println;
use super::thread_pool::ThreadPool;
use std::sync::{
    atomic::{AtomicUsize, AtomicBool, Ordering},
    Arc, Mutex, RwLock,
    TryLockError,
    RwLockReadGuard, RwLockWriteGuard,
    MutexGuard,
    mpsc::channel,
};
#[cfg(feature = "debug_print")]
use itertools::Itertools;
use std::thread;
use approx::relative_eq;

const CONNECT_TIMEOUT: u64 = 10; // seconds
// const READ_TIMEOUT: u64 = 1; // microseconds
const WRITE_TIMEOUT: u64 = 1; // microseconds
// Pretty important due to how fast we are running.
// If a sent packet doesn't arrive in time, throw it away.
// const PACKET_TTL: u32 = 2; // seconds
const PLAYER_INDEX_LOAD_ORDERING: Ordering = Ordering::SeqCst;
const PLAYER_INDEX_STOR_ORDERING: Ordering = Ordering::SeqCst;
const MAX_THREAD_COUNT: usize = 16;
const SLEEP_DURATION: u64 = 1; // milliseconds
const COMP_TIME: u64 = 60; // microseconds

pub struct Runner {
    socket: Arc<Mutex<Socket>>,
    runner_start: Instant,
    thread_count: usize,
}

#[derive(Debug)]
struct Socket {
    stream: BufReader<TcpStream>,
    read_queue: Vec<ServerAction>,
    write_action: Vec<Action>,
    round_sent: AtomicBool,
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

// Actions that we should preserve the ordering for, so we
// push them into a queue, and have only one thread that controls them
#[derive(Debug)]
enum PreservedOrdering {
    Action(Action),
    Delta(i32),
    StartRound(CardHand),
    Reveal(CardHand),
    UpdateDeck(CardDeck),
    SetPlayerIndex(usize),
}

impl Socket {
    fn new(stream: BufReader<TcpStream>) -> Socket {
        Socket {
            stream,
            read_queue: vec![],
            write_action: vec![], // We always start off with checking to ack the server
            round_sent: AtomicBool::new(false),
        }
    }

    /// Returns an incoming message from the engine.
    fn receive(&mut self) -> Vec<ServerAction> {
        self.read_queue.drain(..).collect()
    }

    fn ping(&mut self) {
        debug_println!("[Socket] Will ping server");
        self.send(Action::Check);
    }

    /// Send an action message to the engine
    fn send(&mut self, action: Action) {
        let ref mut socket = self.stream;

        let code = match action {
            Action::Fold => "F".into(),
            Action::Call => "C".into(),
            Action::Check => "K".into(),
            Action::Raise(amt) => format!("R{}", amt)
        };

        let mut retries = 10;
        while self.round_sent.load(Ordering::SeqCst) {
            match writeln!(socket.get_mut(), "{}", code) {
                Ok(_) => break,
                Err(e) => if retries > 0 {
                    debug_println!("[Socket] Send error {}. Retrying...", e);
                    retries -= 1;
                } else {
                    panic!("[Socket] Server unresponsive. Panicing...")
                }
            }
            socket.get_mut().flush().unwrap();
        }

        self.round_sent.store(false, Ordering::SeqCst);

        Socket::check_for_socket_errors(socket.get_ref());
    }

    fn check_for_socket_errors(socket: &TcpStream) {
        // Check stream for errors. If there is one, disconnect.
        match socket.take_error() {
            Ok(Some(error)) => panic!("[Socket] Disconnecting because of stream error {}", error),
            Ok(None) => {}, // No stream error detected
            Err(e) => match e.kind() {
                ErrorKind::TimedOut | ErrorKind::WouldBlock => {}, // We don't care about these errors,
                kind => panic!("[Socket] Unexpected error when checking for socket errors ({:?}) {}", kind, e)
            }
        }
    }

    // Do all I/O processing here
    fn sync(&mut self) {
        let mut server_process = vec![];
        // Write as much as we can, then read all the actions we can
        let ref mut socket = self.stream;

        // Read
        let mut s = String::new();

        // Make read and write non-blocking, so we don't run out of time if our opponent does
        match socket.read_line(&mut s) {
            Ok(_) => {},
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => {},
                kind => panic!("[Socket] Unexpected read error ({:?}) {}", kind, e)
            }
        }

        for action in s.trim().split(" ").map(|x| x.trim().to_string()) {
            if !action.is_empty() {
                server_process.push(action);
            }
        }

        Socket::check_for_socket_errors(socket.get_ref());

        if server_process.len() > 0 {
            // self.round_sent = false;
            debug_println!("[Socket] Read actions from socket [{}]", server_process.iter().format(", "));
        }

        Socket::check_for_socket_errors(socket.get_ref());

        // Process server strings into ServerAction objects
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
                c => panic!("[Socket] Unknown server command {} with arg {}", c, arg)
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
    pub fn run_bot<TS>(bot: Box<dyn PokerBot + Send + Sync>, addr: TS, thread_count: usize) -> std::io::Result<()> where TS: ToSocketAddrs {
        if let Some(addr) = addr.to_socket_addrs()?.nth(0) {
            let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(CONNECT_TIMEOUT))?;
            stream.set_nodelay(true).expect("set_nodelay call failed");
            // stream.set_read_timeout(Some(Duration::from_micros(READ_TIMEOUT))).expect("read_timeout call failed");
            stream.set_write_timeout(Some(Duration::from_micros(WRITE_TIMEOUT))).expect("write_timeout call failed");
            // stream.set_ttl(PACKET_TTL).expect("set_ttl call failed");
            // stream.set_nonblocking(true).expect("set_nonblocking call failed");
            let mut runner = Runner {
                socket: Arc::new(Mutex::new(Socket::new(BufReader::new(stream)))),
                runner_start: Instant::now(),
                thread_count,
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
    fn poll_until_write<'a, T>(device: &'a Arc<RwLock<T>>, device_id: &'static str) -> RwLockWriteGuard<'a, T> {
        loop {
            match device.try_write() {
                Ok(guard) => return guard,
                Err(try_error) => match try_error {
                    TryLockError::WouldBlock => {}, // Just try again
                    TryLockError::Poisoned(e) => panic!("Resource {} poisoned. Invalid state.", device_id),
                }
            }
        }
    }

    // This function polls for read access
    fn poll_until_read<'a, T>(device: &'a Arc<RwLock<T>>, device_id: &'static str) -> RwLockReadGuard<'a, T> {
        loop {
            match device.try_read() {
                Ok(guard) => return guard,
                Err(try_error) => match try_error {
                    TryLockError::WouldBlock => {}, // Just try again
                    TryLockError::Poisoned(e) => panic!("Resource {} poisoned. Invalid state.", device_id),
                }
            }
        }
    }

    // Put bot and socket lock error-handling code in one place
    // Is basically the same code as poll_until_* but for Mutexed stuff
    fn lock_device<'a, T>(device: &'a Arc<Mutex<T>>, device_id: &'static str) -> MutexGuard<'a, T> {
        loop {
            match device.try_lock() {
                Ok(guard) => return guard,
                Err(try_error) => match try_error {
                    TryLockError::WouldBlock => {}, // Just try again
                    TryLockError::Poisoned(e) => panic!("Device {} poisoned. Shutting down.", device_id)
                }
            }
        }
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
        let mut pool = if self.thread_count <= MAX_THREAD_COUNT {
            ThreadPool::new(self.thread_count).unwrap()
        } else {
            panic!("Attempted to make {} threads, which is too many.", self.thread_count);
        };

        let (action_sender, action_receiver) = channel();
        let action_receiver = Arc::new(Mutex::new(action_receiver));
        //let barrier = Arc::new(Barrier::new(2));
        let mut state_change = false;

        loop {
            {
                let socket = self.socket.clone();
                pool.execute(88, move || {
                    Runner::lock_device(&socket, "socket").sync();
                });
            }

            // Read from the server
            {
                let mut socket = Runner::lock_device(&self.socket, "socket");
                // Read the server messages and then react to them by changing our state
                let clauses = socket.receive();
                for clause in clauses.into_iter() {
                    state_change = true;
                    // Runner::lock_device(&self.socket, "socket").write_action = None; // Clear the current action.
                    //let chr = clause.chars().nth(0).unwrap();
                    //let arg = clause.chars().skip(1).collect::<String>();
                    // Get our current state.
                    let (game_state, round_state, terminal_state, bot, player_index) = (game_state.clone(), round_state.clone(), terminal_state.clone(), bot.clone(), player_index.clone());
                    // The main runner code is entirely run in thread pools! We reserve the main thread for getting the action from our bot and
                    // either sending or receiving it, otherwise we should update entirely asyncrously
                    let action_sender = action_sender.clone();
                    match clause.clone() {
                        // Set game clock
                        ServerAction::SetGameClock(clock) => {
                            let mut game_state = Runner::poll_until_write(&game_state, "game");
                            debug_println!("[Runner] Setting game clock to {:.3}", clock);
                            *game_state = GameState {
                                bankroll: game_state.bankroll,
                                game_clock: clock,
                                round_num: game_state.round_num
                            };
                        },
                        // Set player index (also referred to as "active")
                        ServerAction::SetPlayerIndex(index) => action_sender.send(PreservedOrdering::SetPlayerIndex(index)).unwrap(),
                        // Set our hand
                        ServerAction::SetPlayerHand(hand) => action_sender.send(PreservedOrdering::StartRound(hand)).unwrap(),
                        // Since the server doesn't tell us who did what, we have to preserve that information
                        // By preserving the order of actions, so we push them to a queue and run them all in order

                        // A fold action
                        ServerAction::PlayFold => action_sender.send(PreservedOrdering::Action(Action::Fold)).unwrap(),
                        // A call action
                        ServerAction::PlayCall => action_sender.send(PreservedOrdering::Action(Action::Call)).unwrap(),
                        // A check action
                        ServerAction::PlayCheck => action_sender.send(PreservedOrdering::Action(Action::Check)).unwrap(),
                        // A raise action
                        ServerAction::PlayRaise(by) => action_sender.send(PreservedOrdering::Action(Action::Raise(by))).unwrap(),
                        // The deck was updated
                        ServerAction::UpdateDeck(deck) => action_sender.send(PreservedOrdering::UpdateDeck(deck)).unwrap(),
                        // Reveal the opponent's hand
                        ServerAction::RevealOpponentHand(hand) => action_sender.send(PreservedOrdering::Reveal(hand)).unwrap(),
                        // Delta has been calculated
                        ServerAction::Delta(delta) => action_sender.send(PreservedOrdering::Delta(delta)).unwrap(),
                        // End the game
                        ServerAction::Quit => {pool.shutdown(); return},
                    }
                }
            }



            if state_change {
                // Run actions in the action_queue
                {
                    // let barrier = barrier.clone();
                    let action_receiver = action_receiver.clone();
                    let (game_state, round_state, terminal_state, bot, player_index, socket) =
                        (game_state.clone(), round_state.clone(), terminal_state.clone(), bot.clone(), player_index.clone(), self.socket.clone());
                    pool.execute(69, move || {
                        let mut round_state = Runner::poll_until_write(&round_state, "round");
                        let mut game_state = Runner::poll_until_write(&game_state, "game");
                        let mut terminal_state = Runner::poll_until_write(&terminal_state, "terminal");
                        let mut bot = Runner::lock_device(&bot, "bot");
                        let action_queue = Runner::lock_device(&action_receiver, "actions");
                        // Receive as many actions as possible, but don't block on it.
                        while let Ok(action) = action_queue.try_recv() {
                            debug_println!("[Runner] Running action {:?}", action);
                            match action {
                                PreservedOrdering::Action(act) => {
                                    if let Some(ref rs) = *round_state {
                                        match rs.proceed(act) {
                                            StateResult::Round(r) => *round_state = Some(r),
                                            StateResult::Terminal(t) => {
                                                *terminal_state = Some(t);
                                            }
                                        }
                                    } else {
                                        panic!("Round state must exist for action {:?}", action);
                                    }
                                },
                                PreservedOrdering::Delta(delta) => {
                                    debug_println!("[Runner] Setting player deltas and ending round");
                                    assert!(terminal_state.is_some());
                                    let player_index_ = player_index.load(PLAYER_INDEX_LOAD_ORDERING);
                                    if let Some(ref tstate) = *terminal_state {
                                        let mut deltas = [-delta, -delta];
                                        deltas[player_index_] = delta;
                                        let term = TerminalState{
                                            deltas,
                                            previous: tstate.previous.clone()
                                        };
                                        *game_state = GameState {
                                            bankroll: game_state.bankroll + delta as i64,
                                            game_clock: game_state.game_clock,
                                            round_num: game_state.round_num
                                        };
                                        bot.handle_round_over(&*game_state, &term, player_index_);
                                        *terminal_state = Some(term);
                                        *game_state = GameState {
                                            bankroll: game_state.bankroll,
                                            game_clock: game_state.game_clock,
                                            round_num: game_state.round_num + 1
                                        };
                                        *round_state = None;
                                    }
                                },
                                PreservedOrdering::StartRound(hand) => {
                                    let player_index_ = player_index.load(PLAYER_INDEX_LOAD_ORDERING);
                                    let mut hands = [None, None];
                                    hands[player_index_] = Some(hand);
                                    let pips = [SMALL_BLIND, BIG_BLIND];
                                    let stacks = [STARTING_STACK - SMALL_BLIND, STARTING_STACK - BIG_BLIND];
                                    // debug_println!("[Runner] Setting player's hand and starting round");
                                    let round = RoundState {
                                        button: 0,
                                        street: 0,
                                        pips,
                                        stacks,
                                        hands,
                                        deck: CardDeck(vec![]),
                                        previous: None
                                    };
                                    bot.handle_new_round(&*game_state, &round, player_index_);
                                    *round_state = Some(round);
                                },
                                PreservedOrdering::Reveal(hand) => {
                                    let player_index_ = player_index.load(PLAYER_INDEX_LOAD_ORDERING);
                                    if let Some(ref prs) = *round_state {
                                        let mut revised_hands = prs.hands;
                                        revised_hands[1 - player_index_] = Some(hand);
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
                                        *terminal_state = Some(TerminalState{
                                            deltas: [0, 0],
                                            previous: new_round_state
                                        });
                                    } else {
                                        panic!("Round state must exists for reveal")
                                    }
                                },
                                PreservedOrdering::UpdateDeck(deck) => {
                                    if let Some(ref rs) = *round_state {
                                        *round_state = Some(RoundState {
                                            button: rs.button,
                                            street: deck.0.len() as u32,
                                            pips: rs.pips,
                                            stacks: rs.stacks,
                                            hands: rs.hands,
                                            deck,
                                            previous: rs.previous.clone()
                                        })
                                    } else {
                                        panic!("Round state must exist for this action")
                                    }
                                },
                                PreservedOrdering::SetPlayerIndex(index) => {
                                    player_index.store(index, PLAYER_INDEX_STOR_ORDERING)
                                },
                            }
                        }

                        //barrier.wait();
                    })
                }
                {
                    let socket = self.socket.clone();
                    // let barrier = barrier.clone();
                    let (game_state, round_state, bot, player_index) = (game_state.clone(), round_state.clone(), bot.clone(), player_index.clone());
                    pool.execute(9, move || {
                        // Acquire the round state if it is available, but DO NOT BLOCK ( but maybe block the socket for a bit... )
                        // let mut socket = socket.lock().unwrap();
                        //barrier.wait();
                        let mut socket = Runner::lock_device(&socket, "socket");
                        let round_state = Runner::poll_until_read(&round_state, "round");
                        let game_state = Runner::poll_until_read(&game_state, "game");
                        // We haven't tried to send anything


                        if let Some(ref round_state) = *round_state {
                            let player_index = player_index.load(PLAYER_INDEX_LOAD_ORDERING);
                            assert!(player_index == round_state.button as usize % 2);
                            // if we can make an action, do so, unless we already have done so.
                            if !socket.round_sent.load(Ordering::SeqCst) {
                                socket.round_sent.store(true, Ordering::Relaxed);
                                let mut bot = Runner::lock_device(&bot, "bot");
                                let bot_action = bot.get_action(&*game_state, round_state, player_index);
                                let legal_actions = round_state.legal_actions();
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
                                socket.send(action);
                            }
                        } else {
                            if !socket.round_sent.load(Ordering::SeqCst) {
                                socket.round_sent.store(true, Ordering::SeqCst);
                                socket.ping();
                            }
                        }
                    });
                }
            }

            state_change = false;

            // Check of we timed out before letting anything lock or poll_until_write
            {
                let game_state = Runner::poll_until_read(&game_state, "game");
                let round_state = Runner::poll_until_read(&round_state, "round");
                if (relative_eq!(game_state.game_clock, 0.0, epsilon = 0.001)  && game_state.round_num > 1)
                    || Instant::now() - self.runner_start > Duration::from_secs(COMP_TIME)
                    || game_state.round_num == 1001 && round_state.is_none() {
                    debug_println!("Game over. Check for errors.");
                    return;
                }
            }

            // Let the computer rest for a bit
            thread::sleep(Duration::from_micros(SLEEP_DURATION));
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        let runtime = Instant::now() - self.runner_start;
        println!("[Runner] Ran for {:?}", runtime);
    }
}
