use crate::command::Command;
use log::{error, info, trace, warn};
use socket2::{Domain, Socket, Type};
use std::error::Error;
use std::io;
use std::io::prelude::*;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

const MESSAGE_MAX_SIZE: usize = 512;

pub struct Server {
    opt: ServerOptions,
}

pub struct ServerOptions {
    pub thread_count: usize,
    pub backlog: i32,

    // Socket options
    pub only_v6: bool,
    pub reuse_address: bool,
    pub reuse_port: bool,
    pub nodelay: bool,
}

impl ServerOptions {
    pub fn set_sockopts(&self, socket: &Socket) -> io::Result<()> {
        if socket.only_v6()? {
            socket.set_only_v6(self.only_v6)?;
        }
        trace!("IPV6_V6ONLY = {}", socket.only_v6()?);

        socket.set_reuse_address(self.reuse_address)?;
        trace!("SO_REUSEADDR = {}", socket.reuse_address()?);

        socket.set_reuse_port(self.reuse_port)?;
        trace!("SO_REUSEPORT = {}", socket.reuse_port()?);

        socket.set_nodelay(self.nodelay)?;
        trace!("TCP_NODELAY = {}", socket.nodelay()?);

        Ok(())
    }
}

impl Server {
    pub fn new(options: ServerOptions) -> Self {
        Self { opt: options }
    }

    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        let socket = Socket::new(Domain::IPV6, Type::STREAM, None)?;

        let address: SocketAddr = "[::1]:12345".parse()?;
        socket.bind(&address.into())?;

        self.opt.set_sockopts(&socket)?;

        socket.listen(self.opt.backlog)?;
        trace!("Listening on {}:{}", address.ip(), address.port());

        let pool = ThreadPool::new(self.opt.thread_count);

        let listener: TcpListener = socket.into();
        for stream in listener.incoming() {
            let stream = stream?;

            let stream: Socket = stream.into();
            self.opt.set_sockopts(&stream)?;

            let stream: TcpStream = stream.into();
            pool.execute(move || handle_client(stream)).unwrap();
        }

        Ok(())
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buf = [0; MESSAGE_MAX_SIZE];
    let len = stream.read(&mut buf).unwrap();

    match Command::parse(&buf[0..len]) {
        Ok(cmd) => info!("Incoming command: {:?}", cmd),
        Err(error) => error!("{}", error),
    }

    if let Err(error) = stream.write(b"Thank you for your patronage!\r\n") {
        warn!("Write: {}", error);
    }

    if let Err(error) = stream.shutdown(Shutdown::Both) {
        warn!("Shutdown: {}", error);
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

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
    fn new(rx: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
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
