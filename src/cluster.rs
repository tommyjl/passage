use crate::command::NetCommand;
use crate::object::Object;
use crate::server::{ServerOptions, MESSAGE_MAX_SIZE};
use log::{debug, error, trace};
use socket2::{Domain, Socket, Type};
use std::error::Error;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

// TODO: Make nodes asynchronous
pub struct Cluster {
    _opt: ServerOptions,
    nodes: Vec<Socket>,
}

impl Cluster {
    pub fn new(opt: ServerOptions) -> Result<Self, Box<dyn Error>> {
        let mut nodes = Vec::new();
        while nodes.len() < opt.cluster_nodes.len() {
            let node_address: SocketAddr = opt.cluster_nodes[nodes.len()].parse()?;
            let mut node_socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
            if let Err(err) = node_socket.connect(&node_address.into()) {
                debug!("Failed to connect to node {:?}", node_address);
                error!("{}", err);
                thread::sleep(Duration::from_millis(opt.cluster_connect_timeout));
            } else {
                debug!("Successfully connected to node {:?}", node_address);

                let cmd = NetCommand::Leader(opt.cluster_password.clone());
                let obj: Object = cmd.into();
                let buf: Vec<u8> = obj.into();
                node_socket.write(&buf)?;

                nodes.push(node_socket);
            }
        }
        trace!("Connected to all cluster nodes");
        Ok(Cluster { _opt: opt, nodes })
    }

    pub fn relay(&mut self, out_buf: &[u8]) {
        let mut in_buf = [0u8; MESSAGE_MAX_SIZE];
        for node in self.nodes.iter_mut() {
            match node.write_all(out_buf) {
                Ok(_) => {
                    // TODO: Error handling
                    let size = node.read(&mut in_buf).unwrap();
                    if size == 0 {
                        panic!("Cluster node response was size 0");
                    }
                }
                Err(error) => {
                    match error.kind() {
                        // TODO: Reconnect
                        // io::ErrorKind::ConnectionRefused => todo!(),
                        // io::ErrorKind::ConnectionReset => todo!(),
                        // io::ErrorKind::ConnectionAborted => todo!(),
                        // io::ErrorKind::NotConnected => todo!(),
                        // io::ErrorKind::BrokenPipe => todo!(),
                        // io::ErrorKind::TimedOut => todo!(),
                        //
                        // TODO: Retry
                        // io::ErrorKind::Interrupted => todo!(),
                        //
                        _ => {
                            error!("Cluster relay failed to write to node: {:?}", error);
                            panic!()
                        }
                    };
                }
            };
        }
    }
}
