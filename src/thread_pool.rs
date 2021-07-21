use std::error::Error;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

pub type Job = Box<dyn FnOnce() + Send + 'static>;

pub trait ThreadPool {
    fn new(size: usize) -> Self
    where
        Self: Sized;

    fn execute<F>(&self, job: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce() + Send + 'static;
}

pub struct ReceiverThreadPool {
    tx: mpsc::SyncSender<Job>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl ThreadPool for ReceiverThreadPool {
    fn new(size: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Job>(1);
        let rx = Arc::new(Mutex::new(rx));

        let mut threads = Vec::with_capacity(size);
        for _ in 0..size {
            let rx = Arc::clone(&rx);
            let thread = thread::spawn(move || {
                log::debug!("working");
                loop {
                    let job = rx.lock().unwrap().recv().unwrap();
                    job();
                }
            });
            threads.push(thread);
        }

        Self { tx, threads }
    }

    fn execute<F>(&self, job: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(job);
        self.tx.send(job)?;
        Ok(())
    }
}
