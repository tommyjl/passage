use passage::default_env;
use passage::server::{Server, ServerOptions};
use passage::thread_pool::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    default_env!("RUST_BACKTRACE", "1");
    default_env!("RUST_LOG", "trace");
    env_logger::init();

    let options = ServerOptions {
        thread_count: 2,
        backlog: 128,
        port: "12345",
        only_v6: false,
        reuse_address: true,
        reuse_port: true,
        nodelay: true,
    };
    let pool = ReceiverThreadPool::new(options.thread_count);
    Server::new(options, pool).run()
}
