use crate::server::{Server, ServerOptions};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    Server::new(ServerOptions { backlog: 128 }).run()
}

mod server {
    use log::{debug, info};
    use socket2::{Domain, Socket, Type};
    use std::error::Error;
    use std::io::prelude::*;
    use std::net::{SocketAddr, TcpListener, TcpStream};

    pub struct Server {
        opt: ServerOptions,
    }

    pub struct ServerOptions {
        pub backlog: i32,
    }

    impl Server {
        pub fn new(options: ServerOptions) -> Self {
            Self { opt: options }
        }

        pub fn run(&self) -> Result<(), Box<dyn Error>> {
            info!("Running!");
            let socket = Socket::new(Domain::IPV6, Type::STREAM, None)?;

            let address: SocketAddr = "[::1]:12345".parse()?;
            socket.bind(&address.into())?;

            if socket.only_v6()? {
                socket.set_only_v6(false)?;
            }
            socket.set_reuse_address(true)?;

            socket.listen(self.opt.backlog)?;
            debug!("Listening on {}:{}", address.ip(), address.port());

            let listener: TcpListener = socket.into();
            for stream in listener.incoming() {
                handle_client(stream?);
            }

            Ok(())
        }
    }

    fn handle_client(mut stream: TcpStream) {
        info!("Handling stream");
        let mut buf = [0; 1024];
        stream.read(&mut buf).unwrap();
        let buf_str: String = String::from_utf8(buf.to_vec()).unwrap();
        info!("Read '{}'", buf_str);
    }
}
