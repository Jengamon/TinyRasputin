mod skeleton;
mod bots;

use clap::{App, Arg};
use bots::*;
use skeleton::runner::Runner;
use std::net::Ipv4Addr;

fn main() -> std::io::Result<()> {
    // read in arguments
    let matches = App::new("TinyRasputin - A Rust PokerBot")
                    .version(env!("CARGO_PKG_VERSION"))
                    .author("Jengamon <uokwo@mit.edu>")
                    .about("Can play poker over the interwebz")
                    .arg(Arg::with_name("host")
                        .short("o")
                        .long("host")
                        .value_name("HOST_ADDR")
                        .help("Connects to specified host (IPv4 only)")
                        .takes_value(true))
                    .arg(Arg::with_name("port")
                        .help("Port to connect to")
                        .required(true)
                        .index(1))
                    .get_matches();

    let host = matches.value_of("host").unwrap_or("127.0.0.1");
    let port = matches.value_of("port").map(|x| x.parse::<u16>().expect("Expected integer for port number")).unwrap();

    println!("Connecting to {}:{}...", host, port);
    // Change the bot type here, and as long as it implements Default, it'll be built
    let bot: TestBot = Default::default();
    Runner::run_bot(bot, (host.parse::<Ipv4Addr>().expect("Expected IPv4 address for host"), port))
}
