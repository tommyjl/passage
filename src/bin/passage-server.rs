use clap::{crate_authors, crate_version, Clap};
use passage::default_env;
use passage::server::{Server, ServerOptions};
use passage::thread_pool::*;
use passage::wal::Wal;
use std::error::Error;
use std::sync::Arc;

#[derive(Clap)]
#[clap(version = crate_version!(), author = crate_authors!())]
struct Opts {
    #[clap(long, default_value = "wal.txt")]
    log_file: String,

    #[clap(long)]
    fsync: bool,

    #[clap(long, default_value = "2")]
    thread_count: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    default_env!("RUST_BACKTRACE", "1");
    default_env!("RUST_LOG", "trace");
    env_logger::init();

    let opts = Opts::parse();

    let options = ServerOptions {
        thread_count: opts.thread_count,
        backlog: 128,
        port: "12345",
        only_v6: false,
        reuse_address: true,
        reuse_port: true,
        nodelay: true,
    };
    let pool = ReceiverThreadPool::new(options.thread_count);
    let wal = Arc::new(Wal::new(&opts.log_file, opts.fsync).unwrap());
    Server::new(options, pool, wal).run()
}
