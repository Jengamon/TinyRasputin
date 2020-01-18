use std::{fmt, error, thread, panic};
use std::sync::{mpsc, Arc, Mutex, atomic::{AtomicBool, Ordering}};
use crate::debug_println;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
    alive: Vec<Arc<AtomicBool>>,
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
            let mut alive_flags = Vec::with_capacity(size);

            let (sender, receiver) = mpsc::channel();
            let receiver = Arc::new(Mutex::new(receiver));

            for id in 0..size {
                let flag = Arc::new(AtomicBool::new(false));
                workers.push(Worker::new(id, Arc::clone(&receiver), flag.clone()));
                alive_flags.push(flag);
            }

            Ok(ThreadPool { workers, sender, alive: alive_flags })
        } else {
            Err(PoolCreationError::EmptyPool)
        }
    }

    pub fn execute<F>(&self, job_type: usize, f: F) where F: FnOnce() + Send + 'static {
        // Send the job to the queue
        let new_job: (Box<dyn FnBox + Send + 'static>, _) = (Box::new(f), job_type);

        for (id, flag) in self.alive.iter().enumerate() {
            let is_alive = flag.load(Ordering::SeqCst);
            if !is_alive {
                panic!("Worker {} panicked... Exiting,,,", id);
            }
        }

        self.sender.send(Message::NewJob(new_job)).unwrap();
    }

    fn shutdown(&mut self) {
        
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        debug_println!("Shutting down all workers!");

        for worker in &mut self.workers {
            debug_println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>, flag: Arc<AtomicBool>) -> Worker {
        let builder = thread::Builder::new()
            .name(format!("[Worker {}]", id));
        
        flag.store(true, Ordering::SeqCst);

        let thread = builder.spawn(move || {
            let result = panic::catch_unwind(move || {
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
            flag.store(false, Ordering::SeqCst);
            if let Err(err) = result {
                panic::resume_unwind(err);
            }
        });

        Worker {
            id,
            thread: thread.ok()
        }
    }
}