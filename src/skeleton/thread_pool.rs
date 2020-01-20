use std::{fmt, error, thread};
use std::sync::{mpsc, Arc, Mutex};
use crate::debug_println;
use std::sync::atomic::{AtomicBool, Ordering};
use std::panic;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
    flags: Vec<Arc<AtomicBool>>,
    // receiver: Arc<Mutex<mpsc::Receiver<Message>>>,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

enum Message {
    NewJob(Job),
    Terminate,
}

type Job = (Box<dyn FnBox + Send + 'static>, usize);

#[derive(Debug)]
pub enum PoolCreationError {
    EmptyPool,
}

impl fmt::Display for PoolCreationError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PoolCreationError::EmptyPool => write!(fmt, "attempted to create a pool of size 0"),
        }
    }
}

impl error::Error for PoolCreationError {}

impl ThreadPool {
    pub fn new(size: usize) -> Result<ThreadPool, PoolCreationError> {
        if size > 0 {
            let mut workers = Vec::with_capacity(size);
            let mut flags = Vec::with_capacity(size);

            let (sender, receiver) = mpsc::channel();
            let receiver = Arc::new(Mutex::new(receiver));

            for id in 0..size {
                let flag = Arc::new(AtomicBool::new(true));
                workers.push(Worker::new(id, Arc::clone(&receiver), flag.clone()));
                flags.push(flag);
            }

            Ok(ThreadPool { workers, sender, flags })//, receiver })
        } else {
            Err(PoolCreationError::EmptyPool)
        }
    }

    pub fn execute<F>(&mut self, job_type: usize, f: F) where F: FnOnce() + Send + 'static {
        // Send the job to the queue
        let new_job: (Box<dyn FnBox + Send + 'static>, _) = (Box::new(f), job_type);

        // If a worker crashes, we should reboot it.
        for (i, flag) in self.flags.iter().enumerate() {
            let flag_ = flag.load(Ordering::SeqCst);
            if !flag_ {
                // self.shutdown();
                if let Some(thread) = self.workers[i].thread.take() {
                    panic!("[ThreadPool] Worker {} panicked. Killing all workers...", i);
                }
                // self.workers[i] = Worker::new(i, Arc::clone(&self.receiver), flag.clone());
            }
        }

        // If this panics, we have no workers left,
        // so shutdown and panic
        if let Err(_) = self.sender.send(Message::NewJob(new_job)) {
            panic!("All workers panicked or closed. Unrecoverable errors.");
        }
    }

    pub fn shutdown(&mut self) {
        for _ in &mut self.workers {
            // Here we don't care about send errors
            // If we send, great.
            // If not, we don't care, cause that means everyone is dead.
            // We just want to end and merge all threads
            if let Ok(_) = self.sender.send(Message::Terminate) {
                // do nothing
            }
        }

        let mut count = 0;

        debug_println!("[ThreadPool] Sent terminate to all workers");

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                debug_println!("[ThreadPool] Shutting down worker {}", worker.id);
                count += 1;
                // If a thread panicked, just print what it panicked with
                match thread.join() {
                    Ok(_) => debug_println!("[ThreadPool] Worker {} did not panic", worker.id),
                    // It is possible to panic with a non-Display error,
                    // but Debug is implemented for Any, so use that
                    Err(_) => debug_println!("[ThreadPool] Worker {} paniced", worker.id)
                };
            }
        }

        debug_println!("[ThreadPool] Shut down {} workers.", count);
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>, flag: Arc<AtomicBool>) -> Worker {
        let builder = thread::Builder::new()
            .name(format!("[Worker {}]", id));

        let thread = builder.spawn(move || {
            // Get the default handler
            let default_hook = panic::take_hook();

            panic::set_hook(Box::new(move |p| {
                // Add some notification stuff so we report to the main thread we crashed
                flag.store(false, Ordering::SeqCst);
                // Panic with the big boi
                default_hook(p);
            }));

            loop {
                let message = receiver.lock().unwrap().recv().unwrap();

                match message {
                    Message::NewJob((job, name)) => {
                        debug_println!("[Worker] Worker {} received new job of type {}", id, name);

                        job.call_box();
                    },
                    Message::Terminate => {
                        debug_println!("[Worker] Worker {} was told to terminate.", id);

                        break;
                    },
                }
            }
        });

        Worker {
            id,
            thread: thread.ok()
        }
    }
}
