mod command;
mod command_parser;
mod db;
mod server;
mod thread_pool;

use thread_pool::*;

use crate::server::{Server, ServerOptions};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let options = ServerOptions {
        thread_count: 2,
        backlog: 128,
        only_v6: false,
        reuse_address: true,
        reuse_port: true,
        nodelay: true,
    };
    let pool = ReceiverThreadPool::new(options.thread_count);
    Server::new(options, pool).run()
}
