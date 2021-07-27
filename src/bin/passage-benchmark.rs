use clap::Clap;
use log::info;
use passage::client::Client;
use passage::default_env;
use std::time::Instant;

#[derive(Clap)]
struct Opts {
    #[clap(long, short, default_value = "1000")]
    requests: i32,

    #[clap(subcommand)]
    kind: BenchKind,
}

#[derive(Clap)]
enum BenchKind {
    Get,
    Set,
}

impl std::fmt::Display for BenchKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            BenchKind::Get => "get",
            BenchKind::Set => "set",
        };
        write!(f, "{}", kind)
    }
}

fn bench_get(client: &mut Client) {
    let _ = client.get("drink".to_string());
}

fn bench_set(client: &mut Client) {
    let _ = client.set("drink".to_string(), "water".to_string());
}

fn main() {
    default_env!("RUST_LOG", "trace");
    env_logger::init();

    let opts = Opts::parse();
    let bench_fn = match opts.kind {
        BenchKind::Get => bench_get,
        BenchKind::Set => bench_set,
    };

    let time = Instant::now();
    let mut client = Client::new("127.0.0.1:12345");
    for _ in 0..opts.requests {
        bench_fn(&mut client);
    }
    let elapsed = time.elapsed().as_millis();

    info!(
        "{} {} requests took {} ms",
        opts.requests, opts.kind, elapsed
    );

    let avg = (elapsed as f64) / (opts.requests as f64);
    info!("Average: {} ms", avg);
}
