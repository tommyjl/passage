mod command;
mod command_parser;
mod server;

use crate::server::{Server, ServerOptions};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    Server::new(ServerOptions {
        backlog: 128,
        only_v6: false,
        reuse_address: true,
        reuse_port: true,
        nodelay: true,
    })
    .run()
}
