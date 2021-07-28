use crate::server::MESSAGE_MAX_SIZE;
use socket2::{Domain, Socket, Type};
use std::io::prelude::*;
use std::net::SocketAddr;

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

    pub fn get(&mut self, key: String) -> Vec<u8> {
        let msg = format!("get {}\r\n", key);
        let _len = self.conn.write(msg.as_bytes()).unwrap();

        let mut buf = [0; MESSAGE_MAX_SIZE];
        let len = self.conn.read(&mut buf).unwrap();

        buf[0..len].to_vec()
    }

    pub fn set(&mut self, key: String, value: String) -> Vec<u8> {
        let msg = format!("set {} {}\r\n", key, value);
        let _len = self.conn.write(msg.as_bytes()).unwrap();

        let mut buf = [0; MESSAGE_MAX_SIZE];
        let len = self.conn.read(&mut buf).unwrap();

        buf[0..len].to_vec()
    }

    pub fn remove(&mut self, key: String) -> Vec<u8> {
        let msg = format!("remove {}\r\n", key);
        let _len = self.conn.write(msg.as_bytes()).unwrap();

        let mut buf = [0; MESSAGE_MAX_SIZE];
        let len = self.conn.read(&mut buf).unwrap();

        buf[0..len].to_vec()
    }
}
