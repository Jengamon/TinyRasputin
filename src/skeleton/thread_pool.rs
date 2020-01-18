use std::{fmt, error, thread};
use std::sync::{mpsc, Arc, Mutex};
use crate::debug_println;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
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

            let (sender, receiver) = mpsc::channel();
            let receiver = Arc::new(Mutex::new(receiver));

            for id in 0..size {
                workers.push(Worker::new(id, Arc::clone(&receiver)));
            }

            Ok(ThreadPool { workers, sender })
        } else {
            Err(PoolCreationError::EmptyPool)
        }
    }

    pub fn execute<F>(&mut self, job_type: usize, f: F) where F: FnOnce() + Send + 'static {
        // Send the job to the queue
        let new_job: (Box<dyn FnBox + Send + 'static>, _) = (Box::new(f), job_type);

        // If this panics, we have no workers left,
        // so shutdown and panic
        if let Err(_) = self.sender.send(Message::NewJob(new_job)) {
            self.shutdown();
            panic!("All workers panicked");
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

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                debug_println!("Shutting down worker {}", worker.id);
                count += 1;
                // If a thread panicked, just print what it panicked with
                match thread.join() {
                    Ok(_) => debug_println!("Worker {} did not panic", worker.id),
                    // It is possible to panic with a non-Display error,
                    // but Debug is implemented for Any, so use that
                    Err(_) => debug_println!("Worker {} paniced", worker.id)
                };
            }
        }

        debug_println!("Shut down {} workers.", count);
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.shutdown()
    }
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let builder = thread::Builder::new()
            .name(format!("[Worker {}]", id));

        let thread = builder.spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv().unwrap();

                match message {
                    Message::NewJob((job, name)) => {
                        debug_println!("Worker {} received new job of type {}", id, name);

                        job.call_box();
                    },
                    Message::Terminate => {
                        debug_println!("Worker {} was told to terminate.", id);

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
