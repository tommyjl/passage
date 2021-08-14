use crate::command::Command;
use crate::db::{Database, HashMapDatabase};
use crate::object::parse;
use crate::thread_pool::ThreadPool;
use crate::wal::Wal;
use log::{debug, error, info, trace};
use nix::poll::{poll, PollFd, PollFlags};
use socket2::{Domain, Socket, Type};
use std::convert::TryFrom;
use std::error::Error;
use std::io;
use std::io::prelude::*;
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use std::time::Instant;

pub const MESSAGE_MAX_SIZE: usize = 512;

pub struct Server<P: ThreadPool> {
    opt: ServerOptions,
    pool: P,
    db: Arc<dyn Database>,
    wal: Arc<Wal>,
}

pub struct ServerOptions {
    pub thread_count: usize,
    pub backlog: i32,
    pub port: &'static str,

    // Socket options
    pub only_v6: bool,
    pub reuse_address: bool,
    pub reuse_port: bool,
    pub nodelay: bool,
}

impl ServerOptions {
    pub fn set_sockopts(&self, socket: &Socket) -> io::Result<()> {
        socket.set_reuse_address(self.reuse_address)?;
        trace!("SO_REUSEADDR = {}", socket.reuse_address()?);

        socket.set_reuse_port(self.reuse_port)?;
        trace!("SO_REUSEPORT = {}", socket.reuse_port()?);

        socket.set_nodelay(self.nodelay)?;
        trace!("TCP_NODELAY = {}", socket.nodelay()?);

        Ok(())
    }
}

impl<P: ThreadPool> Server<P> {
    pub fn new(options: ServerOptions, pool: P, wal: Arc<Wal>) -> Self {
        let time = Instant::now();
        let db: Arc<dyn Database> = Arc::new(HashMapDatabase::new());
        while let Some(cmd) = wal.read() {
            trace!("Replaying cmd = {:?}", cmd);
            let _response = db.execute(cmd);
        }
        trace!("Server init took {} ms", time.elapsed().as_millis());
        Self {
            opt: options,
            pool,
            db,
            wal,
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        let address: SocketAddr = format!("0.0.0.0:{}", self.opt.port).parse()?;
        let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
        self.opt.set_sockopts(&socket)?;
        socket.bind(&address.into())?;

        socket.listen(self.opt.backlog)?;
        trace!("Listening on {}:{}", address.ip(), address.port());

        let mut pollfds = vec![PollFd::new(socket.as_raw_fd(), PollFlags::POLLIN)];

        loop {
            let count = poll(&mut pollfds, -1)?;
            if count == 0 {
                continue;
            } else if count < 0 {
                error!("Poll returned {}", count);
                std::process::exit(1);
            }

            match pollfds[0].revents() {
                None => error!("No revents???"),
                Some(flags) => {
                    if !flags.intersects(PollFlags::POLLIN) {
                        error!("UNEXPECTED FLAG: {:?}", flags);
                    }

                    let (stream, _addr) = socket.accept()?;
                    self.opt.set_sockopts(&stream)?;

                    self.pool
                        .execute({
                            let stream: TcpStream = stream.into();
                            let db = self.db.clone();
                            let wal = self.wal.clone();
                            move || handle_client(stream, db, wal)
                        })
                        .unwrap();
                }
            }
        }
    }
}

fn handle_client(mut stream: TcpStream, db: Arc<dyn Database>, wal: Arc<Wal>) {
    let mut buf = [0; MESSAGE_MAX_SIZE];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Err(err) => {
                error!("TcpStream error: {}", err);
                break;
            }
            _ => (),
        };

        let mut cursor = io::Cursor::new(&buf[..]);
        let object = match parse(&mut cursor) {
            Ok(o) => o,
            Err(err) => {
                error!("Parse error: {}", err);
                break;
            }
        };

        let cmd = match Command::try_from(object) {
            Ok(o) => o,
            Err(err) => {
                error!("Invalid command: {}", err);
                break;
            }
        };

        info!("Incoming command: {:?}", cmd);
        wal.append(&cmd).unwrap();
        let response: Vec<u8> = db.execute(cmd).unwrap().into();
        if let Err(error) = stream.write(&response) {
            error!("Write: {}", error);
        }
    }

    if let Err(error) = stream.shutdown(Shutdown::Both) {
        let kind = error.kind();
        debug!("Shutdown: ErrorKind::{:?}", kind);
        if kind != io::ErrorKind::NotConnected {
            error!("Shutdown: {}", error);
        }
    }
}
