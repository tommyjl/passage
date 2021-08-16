use crate::command::Command;
use crate::db::{Database, HashMapDatabase};
use crate::object::parse;
use crate::wal::Wal;
use log::{debug, error, info, trace};
use nix::poll::{poll, PollFd, PollFlags};
use socket2::{Domain, Socket, Type};
use std::convert::TryFrom;
use std::error::Error;
use std::io;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use std::time::Instant;

pub const MESSAGE_MAX_SIZE: usize = 512;

pub struct Server {
    opt: ServerOptions,
    db: Arc<dyn Database>,
    wal: Arc<Wal>,
}

pub struct ServerOptions {
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

impl Server {
    pub fn new(options: ServerOptions, wal: Arc<Wal>) -> Self {
        let time = Instant::now();
        let db: Arc<dyn Database> = Arc::new(HashMapDatabase::new());
        while let Some(cmd) = wal.read() {
            trace!("Replaying cmd = {:?}", cmd);
            let _response = db.execute(cmd);
        }
        trace!("Server init took {} ms", time.elapsed().as_millis());
        Self {
            opt: options,
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

        // The elements of the same index of pollfds, sockets, and bufs should
        // always correspond to the same file descriptor.
        let mut pollfds = vec![PollFd::new(socket.as_raw_fd(), PollFlags::POLLIN)];
        let mut sockets = vec![socket];
        let mut bufs = vec![[0u8; MESSAGE_MAX_SIZE]];

        let mut cleanup_ids: Vec<usize> = Vec::new();

        loop {
            let mut count = poll(&mut pollfds, -1)?;
            if count < 0 {
                error!("Poll returned {}", count);
                std::process::exit(1);
            }

            for i in 0..pollfds.len() {
                if count == 0 {
                    break;
                }

                let pollfd = &pollfds[i];
                let socket = &mut sockets[i];
                let buf = &mut bufs[i];

                if let Some(revents) = pollfd.revents() {
                    if revents.is_empty() {
                        continue;
                    }
                    count -= 1;

                    if i == 0 {
                        if !revents.intersects(PollFlags::POLLIN) {
                            error!("Invalid flag");
                            std::process::exit(1);
                        }

                        trace!("Incoming connection");
                        let (stream, _addr) = socket.accept()?;
                        self.opt.set_sockopts(&stream)?;

                        pollfds.push(PollFd::new(stream.as_raw_fd(), PollFlags::POLLIN));
                        sockets.push(stream);
                        bufs.push([0; MESSAGE_MAX_SIZE]);
                        trace!("Accepted connection");
                    } else {
                        if revents.intersects(PollFlags::POLLHUP) {
                            trace!("POLLHUP");
                            cleanup_ids.push(i);
                            continue;
                        }

                        if revents.intersects(PollFlags::POLLIN) {
                            let size = socket.read(buf)?;
                            if size == 0 {
                                debug!("read {} bytes", size);
                                continue;
                            }

                            let mut cursor = io::Cursor::new(&buf[..]);
                            let object = match parse(&mut cursor) {
                                Ok(o) => o,
                                Err(err) => {
                                    error!("Parse error: {}", err);
                                    break;
                                }
                            };

                            if cursor.position() as usize != size {
                                buf.rotate_left(size);
                            }

                            let cmd = match Command::try_from(object) {
                                Ok(o) => o,
                                Err(err) => {
                                    error!("Invalid command: {}", err);
                                    break;
                                }
                            };

                            info!("Incoming command: {:?}", cmd);
                            self.wal.append(&cmd).unwrap();
                            let response: Vec<u8> = self.db.execute(cmd).unwrap().into();

                            // TODO: Should PollFlags::POLLOUT be used?
                            if let Err(error) = socket.write(&response) {
                                error!("Write: {}", error);
                            }
                        }
                    }
                }
            }
            while let Some(i) = cleanup_ids.pop() {
                pollfds.remove(i);
                sockets.remove(i);
                bufs.remove(i);
            }
        }
    }
}
