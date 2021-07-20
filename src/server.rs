use crate::command::Command;
use log::{error, info, trace, warn};
use socket2::{Domain, Socket, Type};
use std::error::Error;
use std::io;
use std::io::prelude::*;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::thread;

const MESSAGE_MAX_SIZE: usize = 512;

pub struct Server {
    opt: ServerOptions,
}

pub struct ServerOptions {
    pub backlog: i32,
}

impl ServerOptions {
    pub fn set_sockopts(&self, socket: &Socket) -> io::Result<()> {
        if socket.only_v6()? {
            socket.set_only_v6(false)?;
        }
        trace!("IPV6_V6ONLY = {:?}", socket.only_v6()?);

        socket.set_reuse_address(true)?;
        trace!("SO_REUSEADDR = {:?}", socket.reuse_address()?);

        socket.set_reuse_port(true)?;
        trace!("SO_REUSEPORT = {:?}", socket.reuse_port()?);

        socket.set_nodelay(true)?;
        let nodelay = socket.nodelay()?;
        trace!("TCP_NODELAY = {}", nodelay);

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

        let listener: TcpListener = socket.into();
        for stream in listener.incoming() {
            let stream = stream?;

            let stream: Socket = stream.into();
            self.opt.set_sockopts(&stream)?;

            let stream: TcpStream = stream.into();
            thread::spawn(move || handle_client(stream));
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
