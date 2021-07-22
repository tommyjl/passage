use std::error::Error;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

pub type Job = Box<dyn FnOnce() + Send + 'static>;

pub trait ThreadPool {
    fn execute<F>(&self, job: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce() + Send + 'static;
}

pub struct ReceiverThreadPool {
    tx: mpsc::SyncSender<Message>,
    threads: Vec<thread::JoinHandle<()>>,
}

pub enum Message {
    Job(Job),
    Term,
}

impl ReceiverThreadPool {
    pub fn new(size: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Message>(1);
        let rx = Arc::new(Mutex::new(rx));

        let mut threads = Vec::with_capacity(size);
        for i in 0..size {
            let id = i;
            let rx = Arc::clone(&rx);
            let thread = thread::spawn(move || {
                loop {
                    log::debug!("Worker-{}: Ready for next order", id);
                    match rx.lock().unwrap().recv().unwrap() {
                        Message::Job(job) => job(),
                        Message::Term => break,
                    }
                }
                log::debug!("Worker-{}: Goodbye", id);
            });
            threads.push(thread);
        }

        Self { tx, threads }
    }
}

impl ThreadPool for ReceiverThreadPool {
    fn execute<F>(&self, job: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(job);
        self.tx.send(Message::Job(job))?;
        Ok(())
    }
}

impl Drop for ReceiverThreadPool {
    fn drop(&mut self) {
        for _ in 0..self.threads.len() {
            self.tx.send(Message::Term).unwrap();
        }
        while let Some(thread) = self.threads.pop() {
            thread.join().unwrap();
        }
    }
}
