use std::error::Error;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

pub type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    _tx: mpsc::SyncSender<Job>,
    _workers: Vec<Worker>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Job>(1);
        let rx = Arc::new(Mutex::new(rx));

        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&rx)));
        }

        Self {
            _tx: tx,
            _workers: workers,
        }
    }

    pub fn execute<F>(&self, job: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(job);
        self._tx.send(job)?;
        Ok(())
    }
}

pub struct Worker {
    _thread: thread::JoinHandle<()>,
}

impl Worker {
    pub fn new(rx: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || {
            log::debug!("working");
            loop {
                let job = rx.lock().unwrap().recv().unwrap();
                job();
            }
        });
        Self { _thread: thread }
    }
}
