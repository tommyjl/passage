use clap::{crate_authors, crate_version, Clap};
use passage::default_env;
use passage::server::{Server, ServerOptions};
use passage::wal::Wal;
use std::error::Error;
use std::sync::Arc;

#[derive(Clap)]
#[clap(version = crate_version!(), author = crate_authors!())]
struct Opts {
    #[clap(long, default_value = "wal.txt")]
    log_file: String,

    #[clap(long, default_value = "1234")]
    cluster_password: String,

    #[clap(long)]
    cluster_nodes: Vec<String>,

    #[clap(long)]
    fsync: bool,

    #[clap(long)]
    read_only: bool,

    #[clap(short, long, default_value = "12345")]
    port: u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    default_env!("RUST_BACKTRACE", "1");
    default_env!("RUST_LOG", "trace");
    env_logger::init();

    let opts = Opts::parse();

    let options = ServerOptions {
        backlog: 128,
        port: opts.port,
        read_only: opts.read_only,
        only_v6: false,
        reuse_address: true,
        reuse_port: true,
        nodelay: true,
        cluster_password: opts.cluster_password,
        cluster_nodes: opts.cluster_nodes,
        cluster_connect_timeout: 1000,
    };
    let wal = Arc::new(Wal::new(&opts.log_file, opts.fsync).unwrap());
    Server::new(options, wal).run()
}
