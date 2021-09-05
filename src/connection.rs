use crate::cluster::Cluster;
use crate::command::{Command, NetCommand};
use crate::db::Database;
use crate::object::parse;
use crate::object::Object;
use crate::server::{ServerOptions, MESSAGE_MAX_SIZE};
use crate::wal::Wal;
use log::{debug, error, trace};
use socket2::{Domain, Socket, Type};
use std::convert::TryFrom;
use std::error::Error;
use std::io;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;
use std::sync::Arc;

#[derive(Debug)]
pub struct Connection {
    pub socket: Socket,
    pub buf: [u8; MESSAGE_MAX_SIZE],
    pub offset: usize,
    pub closed: bool,
    password: String,
    mode: ConnectionMode,
}

#[derive(PartialEq, Eq, Debug)]
enum ConnectionMode {
    Leader,
    Read,
    ReadWrite,
}

impl Connection {
    pub fn new_listener(opt: &ServerOptions) -> Result<Self, Box<dyn Error>> {
        let address: SocketAddr = format!("0.0.0.0:{}", opt.port).parse()?;
        let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
        opt.set_sockopts(&socket)?;
        socket.bind(&address.into())?;
        socket.listen(opt.backlog)?;
        trace!("Listening on {}:{}", address.ip(), address.port());
        Ok(Connection::new(socket, false, opt))
    }

    pub fn new(socket: Socket, read_only: bool, opt: &ServerOptions) -> Self {
        Self {
            socket,
            buf: [0u8; MESSAGE_MAX_SIZE],
            offset: 0,
            closed: false,
            mode: if read_only {
                ConnectionMode::Read
            } else {
                ConnectionMode::ReadWrite
            },
            password: opt.cluster_password.clone(),
        }
    }

    pub fn accept(&self, opt: &ServerOptions) -> io::Result<Self> {
        let (stream, _addr) = self.socket.accept()?;
        opt.set_sockopts(&stream)?;
        Ok(Connection::new(stream, opt.read_only, opt))
    }

    pub fn read(&mut self) -> io::Result<usize> {
        self.socket.read(&mut self.buf[self.offset..])
    }

    pub fn write_allowed(&self) -> bool {
        match self.mode {
            ConnectionMode::Leader => true,
            ConnectionMode::Read => false,
            ConnectionMode::ReadWrite => true,
        }
    }

    pub fn handle_incoming_command(
        &mut self,
        db: Arc<dyn Database>,
        wal: Arc<Wal>,
        cluster: &mut Option<Cluster>,
    ) -> Result<(), Box<dyn Error>> {
        let size = self.read()?;
        if size == 0 {
            trace!("read {} bytes", size);
            return Ok(());
        }

        let mut cursor = io::Cursor::new(&self.buf[..]);
        let mut offset = 0;

        while cursor.position() < size as u64 {
            let object = match parse(&mut cursor) {
                Ok(o) => o,
                Err(err) if matches!(err, crate::object::Error::Incomplete) => {
                    if offset == 0 {
                        trace!("Max message size exceeded");
                        self.closed = true;
                    }
                    break;
                }
                Err(err) => {
                    error!("Parse error: {}", err);
                    continue;
                }
            };

            if let Ok(net_cmd) = NetCommand::try_from(&object) {
                trace!("Handling network command! {:?}", net_cmd);
                match net_cmd {
                    NetCommand::Leader(ref password) => {
                        // TODO: Handle the password in a sane way. Should
                        // probably drop the connection if it was wrong, and
                        // the password should be hashed and fetched from some
                        // configuration.
                        if password == &self.password {
                            trace!("Connection is the leader node");
                            self.mode = ConnectionMode::Leader;
                        } else {
                            trace!("Incorrect password -- not leader node");
                            self.closed = true;
                        }
                    }
                }
            } else {
                let cmd = match Command::try_from(object) {
                    Ok(o) => o,
                    Err(err) => {
                        debug!("Invalid command: {}", err);
                        continue;
                    }
                };
                debug!("Incoming command: {:?}", cmd);

                let response_buf: Vec<u8> = if cmd.possibly_dirty() && !self.write_allowed() {
                    Object::Error("Read-only mode: Illegal command".to_string()).into()
                } else {
                    wal.append(&cmd).unwrap();
                    let response = db.execute(cmd).unwrap();
                    if response.is_dirty {
                        let buf = &self.buf[offset..cursor.position() as usize];
                        if let Some(cluster) = cluster {
                            cluster.relay(buf);
                        }
                    }
                    response.object.into()
                };

                if let Err(error) = self.socket.write(&response_buf) {
                    error!("Write: {}", error);
                }
            }
            offset = cursor.position() as usize;
        }

        if offset < size {
            self.buf.rotate_left(offset);
            self.offset = size - offset;
        } else {
            self.offset = 0;
        }

        Ok(())
    }
}

impl AsRawFd for Connection {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.socket.as_raw_fd()
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        trace!("Dropping connection");
    }
}
