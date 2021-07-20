mod command;
mod command_parser;
mod server;

use crate::server::{Server, ServerOptions};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    Server::new(ServerOptions { backlog: 128 }).run()
}
