use crate::cluster::Cluster;
use crate::command::Command;
use crate::db::{Database, DatabaseResponse, HashMapDatabase};
use crate::object::parse;
use crate::object::Object;
use crate::wal::Wal;
use log::{debug, error, trace};
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

#[derive(Clone)]
pub struct ServerOptions {
    pub backlog: i32,
    pub port: u32,
    pub read_only: bool,

    // Socket options
    pub only_v6: bool,
    pub reuse_address: bool,
    pub reuse_port: bool,
    pub nodelay: bool,

    // Cluster options
    pub cluster_nodes: Vec<String>,
    pub cluster_connect_timeout: u64,
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

#[derive(Debug)]
enum ServerEvent {
    IncomingConnection,
    CloseConnection,
    IncomingCommand,
}

impl ServerEvent {
    fn check(index: usize, pollfd: &PollFd) -> Option<Self> {
        pollfd.revents().and_then(|revents| {
            // The 0-index is the listener, everything else is a connection
            if index == 0 && revents.intersects(PollFlags::POLLIN) {
                Some(ServerEvent::IncomingConnection)
            } else if revents.intersects(PollFlags::POLLHUP) {
                Some(ServerEvent::CloseConnection)
            } else if revents.intersects(PollFlags::POLLIN) {
                Some(ServerEvent::IncomingCommand)
            } else {
                None
            }
        })
    }
}

struct SocketHandle {
    socket: Socket,
    buf: [u8; MESSAGE_MAX_SIZE],
    offset: usize,
}

impl SocketHandle {
    fn new(socket: Socket) -> Self {
        Self {
            socket,
            buf: [0u8; MESSAGE_MAX_SIZE],
            offset: 0,
        }
    }
}

pub struct Server {
    opt: ServerOptions,
    db: Arc<dyn Database>,
    wal: Arc<Wal>,
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
        let socket = self.listen()?;
        let mut cluster = Cluster::new(self.opt.clone())?;

        // The elements of the same index of pollfds and socket handles should
        // always correspond to the same file descriptor.
        let mut pollfds = vec![PollFd::new(socket.as_raw_fd(), PollFlags::POLLIN)];
        let mut handles = vec![SocketHandle::new(socket)];

        let mut cleanup_ids: Vec<usize> = Vec::new();

        loop {
            let mut count = poll(&mut pollfds, -1)?;
            for i in 0..pollfds.len() {
                if count == 0 {
                    break;
                }
                if let Some(event) = ServerEvent::check(i, &pollfds[i]) {
                    trace!("New event: {:?}", event);
                    count -= 1;
                    let handle = &mut handles[i];
                    match event {
                        ServerEvent::IncomingConnection => {
                            let (stream, _addr) = handle.socket.accept()?;
                            self.opt.set_sockopts(&stream)?;

                            pollfds.push(PollFd::new(stream.as_raw_fd(), PollFlags::POLLIN));
                            handles.push(SocketHandle::new(stream));
                        }
                        ServerEvent::CloseConnection => cleanup_ids.push(i),
                        ServerEvent::IncomingCommand => {
                            let size = handle.socket.read(&mut handle.buf[handle.offset..])?;
                            if size == 0 {
                                debug!("read {} bytes", size);
                                continue;
                            }

                            let mut cursor = io::Cursor::new(&handle.buf[..]);
                            let mut offset = 0;
                            while cursor.position() < size as u64 {
                                offset = cursor.position() as usize;

                                let object = match parse(&mut cursor) {
                                    Ok(o) => o,
                                    Err(err) if matches!(err, crate::object::Error::Incomplete) => {
                                        if offset == 0 {
                                            trace!("Max message size exceeded");
                                            cleanup_ids.push(i);
                                        }
                                        break;
                                    }
                                    Err(err) => {
                                        error!("Parse error: {}", err);
                                        continue;
                                    }
                                };

                                let cmd = match Command::try_from(object) {
                                    Ok(o) => o,
                                    Err(err) => {
                                        debug!("Invalid command: {}", err);
                                        continue;
                                    }
                                };

                                debug!("Incoming command: {:?}", cmd);

                                let response = if self.opt.read_only && cmd.possibly_dirty() {
                                    DatabaseResponse {
                                        object: Object::Error(
                                            "Read-only mode: Illegal command".to_string(),
                                        ),
                                        is_dirty: false,
                                    }
                                } else {
                                    self.wal.append(&cmd).unwrap();
                                    self.db.execute(cmd).unwrap()
                                };

                                if response.is_dirty {
                                    let buf = &handle.buf[0..cursor.position() as usize];
                                    cluster.relay(buf);
                                }

                                let response_buf: Vec<u8> = response.object.into();
                                if let Err(error) = handle.socket.write(&response_buf) {
                                    error!("Write: {}", error);
                                }
                            }

                            if offset < size {
                                handle.buf.rotate_left(offset);
                                handle.offset = size - offset;
                            } else {
                                handle.offset = 0;
                            }
                        }
                    }
                }
            }
            while let Some(i) = cleanup_ids.pop() {
                pollfds.remove(i);
                handles.remove(i);
            }
        }
    }

    fn listen(&self) -> Result<Socket, Box<dyn Error>> {
        let address: SocketAddr = format!("0.0.0.0:{}", self.opt.port).parse()?;
        let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
        self.opt.set_sockopts(&socket)?;
        socket.bind(&address.into())?;
        socket.listen(self.opt.backlog)?;
        trace!("Listening on {}:{}", address.ip(), address.port());
        Ok(socket)
    }
}
