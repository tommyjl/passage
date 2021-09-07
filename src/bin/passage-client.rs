use clap::Clap;
use passage::client::Client;

#[derive(Clap)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Get { key: String },
    Set { key: String, value: String },
    Remove { key: String },
}

fn main() {
    let opts = Opts::parse();
    let mut client = Client::new("127.0.0.1:12345");
    match opts.subcmd {
        SubCommand::Get { key } => {
            let obj = client.get(key).unwrap();
            let buf: Vec<u8> = obj.into();
            print!("{}", String::from_utf8(buf).unwrap());
        }
        SubCommand::Set { key, value } => {
            let buf = client.set(key, value);
            print!("{}", String::from_utf8(buf).unwrap());
        }
        SubCommand::Remove { key } => {
            let buf = client.remove(key);
            print!("{}", String::from_utf8(buf).unwrap());
        }
    };
}
