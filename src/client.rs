use crate::object;
use crate::object::{parse, Object};
use crate::server::MESSAGE_MAX_SIZE;
use socket2::{Domain, Socket, Type};
use std::error::Error;
use std::io;
use std::io::prelude::*;
use std::io::Cursor;
use std::net::SocketAddr;

type Result<T> = std::result::Result<T, ClientError>;

#[derive(Debug)]
pub enum ClientError {
    Io(io::Error),
    Object(object::Error),
}

impl Error for ClientError {}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Io(inner) => write!(f, "{}", inner),
            ClientError::Object(inner) => write!(f, "{}", inner),
        }
    }
}

impl From<io::Error> for ClientError {
    fn from(err: io::Error) -> Self {
        ClientError::Io(err)
    }
}

impl From<object::Error> for ClientError {
    fn from(err: object::Error) -> Self {
        ClientError::Object(err)
    }
}

pub struct Client {
    conn: Socket,
}

impl Client {
    pub fn new(addr: &str) -> Self {
        let addr: SocketAddr = addr.parse().unwrap();
        let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
        socket.connect(&addr.into()).unwrap();
        Self { conn: socket }
    }

    pub fn get(&mut self, key: String) -> Result<Object> {
        let msg = format!("*2\r\n+get\r\n+{}\r\n", key);
        self.conn.write(msg.as_bytes())?;

        let mut buf = [0; MESSAGE_MAX_SIZE];
        let len = self.conn.read(&mut buf)?;

        let mut cursor = Cursor::new(&buf[0..len]);
        let obj = parse(&mut cursor)?;

        Ok(obj)
    }

    pub fn set(&mut self, key: String, value: String) -> Result<Object> {
        let msg = format!("*3\r\n+set\r\n+{}\r\n+{}\r\n", key, value);
        self.conn.write(msg.as_bytes())?;

        let mut buf = [0; MESSAGE_MAX_SIZE];
        let len = self.conn.read(&mut buf)?;

        let mut cursor = Cursor::new(&buf[0..len]);
        let obj = parse(&mut cursor)?;

        Ok(obj)
    }

    pub fn remove(&mut self, key: String) -> Result<Object> {
        let msg = format!("*2\r\n+remove\r\n+{}\r\n", key);
        self.conn.write(msg.as_bytes())?;

        let mut buf = [0; MESSAGE_MAX_SIZE];
        let len = self.conn.read(&mut buf)?;

        let mut cursor = Cursor::new(&buf[0..len]);
        let obj = parse(&mut cursor)?;

        Ok(obj)
    }
}
