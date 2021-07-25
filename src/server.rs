use crate::command::Command;
use crate::db::{Database, HashMapDatabase};
use crate::thread_pool::ThreadPool;
use crate::wal::Wal;
use log::{error, info, trace, warn};
use socket2::{Domain, Socket, Type};
use std::error::Error;
use std::io;
use std::io::prelude::*;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;

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
    pub fn new(options: ServerOptions, pool: P) -> Self {
        let db: Arc<dyn Database> = Arc::new(HashMapDatabase::new());
        let wal = Arc::new(Wal::new().unwrap());
        while let Some(cmd) = wal.read() {
            let _response = handle_command(cmd, &db);
        }
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

        socket.bind(&address.into())?;

        self.opt.set_sockopts(&socket)?;

        socket.listen(self.opt.backlog)?;
        trace!("Listening on {}:{}", address.ip(), address.port());

        let listener: TcpListener = socket.into();
        for stream in listener.incoming() {
            let stream = stream?;

            let stream: Socket = stream.into();
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

        Ok(())
    }
}

fn handle_client(mut stream: TcpStream, db: Arc<dyn Database>, wal: Arc<Wal>) {
    let mut buf = [0; MESSAGE_MAX_SIZE];
    let len = stream.read(&mut buf).unwrap();

    match Command::parse(&buf[0..len]) {
        Ok(cmd) => {
            info!("Incoming command: {:?}", cmd);
            wal.append(&cmd).unwrap();
            let response = handle_command(cmd, &db);
            if let Err(error) = stream.write(&response) {
                warn!("Write: {}", error);
            }
        }
        Err(error) => error!("{}", error),
    }

    if let Err(error) = stream.shutdown(Shutdown::Both) {
        warn!("Shutdown: {}", error);
    }
}

fn handle_command(cmd: Command, db: &Arc<dyn Database>) -> Vec<u8> {
    match cmd {
        Command::Get(key) => db
            .get(key.into())
            .map(|v| {
                format!("Ok: {}\r\n", String::from_utf8(v).unwrap())
                    .as_bytes()
                    .to_owned()
            })
            .unwrap_or(b"Err: Not found\r\n".to_vec()),
        Command::Set(key, value) => db
            .set(key.into(), value.into())
            .map(|v| {
                format!("Ok: {}\r\n", String::from_utf8(v).unwrap())
                    .as_bytes()
                    .to_owned()
            })
            .unwrap_or(b"Err: Not found\r\n".to_vec()),
        Command::Remove(key) => db
            .remove(key.into())
            .map(|v| {
                format!("Ok: {}\r\n", String::from_utf8(v).unwrap())
                    .as_bytes()
                    .to_owned()
            })
            .unwrap_or(b"Err: Not found\r\n".to_vec()),
    }
}
