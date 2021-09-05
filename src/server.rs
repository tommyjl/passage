use crate::cluster::Cluster;
use crate::connection::Connection;
use crate::db::{Database, HashMapDatabase};
use crate::wal::Wal;
use log::trace;
use nix::poll::{poll, PollFd, PollFlags};
use socket2::Socket;
use std::error::Error;
use std::io;
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
    pub cluster_password: String,
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

pub struct Server {
    opt: ServerOptions,
    db: Arc<dyn Database>,
    wal: Arc<Wal>,
    cluster: Option<Cluster>,

    // Each Connection has a corresponding PollFd on the same index.
    connections: Vec<Connection>,
    pollfds: Vec<PollFd>,
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
            cluster: None,
            pollfds: Vec::new(),
            connections: Vec::new(),
        }
    }

    fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        let listener = Connection::new_listener(&self.opt)?;
        let fd = listener.as_raw_fd();

        self.connections.push(listener);
        self.pollfds.push(PollFd::new(fd, PollFlags::POLLIN));

        Ok(())
    }

    fn start_cluster(&mut self) -> Result<(), Box<dyn Error>> {
        let cluster = Cluster::new(self.opt.clone())?;
        self.cluster = Some(cluster);
        Ok(())
    }

    fn accept_connection(&mut self, i: usize) -> Result<(), Box<dyn Error>> {
        let connection = self.connections[i].accept(&self.opt)?;
        let fd = connection.as_raw_fd();

        self.connections.push(connection);
        self.pollfds.push(PollFd::new(fd, PollFlags::POLLIN));

        Ok(())
    }

    fn close_connection(&mut self, i: usize) {
        self.connections[i].closed = true;
    }

    fn cleanup_closed(&mut self) {
        trace!("Cleaning up!");
        for i in (0..self.connections.len()).rev() {
            if self.connections[i].closed {
                self.pollfds.remove(i);
                self.connections.remove(i);
            }
        }
    }

    fn respond_to_command(&mut self, i: usize) -> Result<(), Box<dyn Error>> {
        self.connections[i].handle_incoming_command(
            self.db.clone(),
            self.wal.clone(),
            &mut self.cluster,
        )
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.listen()?;
        self.start_cluster()?;

        loop {
            let mut poll_count = poll(&mut self.pollfds, -1)?;
            for i in 0..self.pollfds.len() {
                if poll_count == 0 {
                    break;
                }
                if let Some(event) = ServerEvent::check(i, &self.pollfds[i]) {
                    trace!("New event: {:?}", event);
                    poll_count -= 1;
                    match event {
                        ServerEvent::IncomingConnection => self.accept_connection(i)?,
                        ServerEvent::CloseConnection => self.close_connection(i),
                        ServerEvent::IncomingCommand => self.respond_to_command(i)?,
                    }
                }
            }
            self.cleanup_closed();
        }
    }
}
