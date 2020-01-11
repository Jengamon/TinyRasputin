mod bots;

use clap::{App, Arg};
use bots::*;
use tinyrasputin::skeleton::{runner::Runner, bot::PokerBot};
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
                    .arg(Arg::with_name("bot")
                        .short("b")
                        .long("bot")
                        .value_name("BOT_VERSION")
                        .help("Selects which bot version to use [can be: test, l1, l2, tourney]")
                        .takes_value(true))
                    .arg(Arg::with_name("port")
                        .help("Port to connect to")
                        .required(true)
                        .index(1))
                    .get_matches();

    let host = matches.value_of("host").unwrap_or("127.0.0.1");
    let port = matches.value_of("port").map(|x| x.parse::<u16>().expect("Expected integer for port number")).unwrap();
    let botv = matches.value_of("bot").unwrap_or("tourney");

    println!("Connecting to {}:{}...", host, port);
    println!("Attempting to run bot version {}...", botv);
    // Change the bot type here, and as long as it implements Default, it'll be built
    let mut bot: Box<dyn PokerBot> = match botv {
        "test" => Box::new(TestBot::default()),
        "l1" => Box::new(Lesson1Bot::default()),
        "l2" => Box::new(Lesson2Bot::default()),
        "tourney" => Box::new(TourneyBot::default()),
        _ => panic!("Invalid bot version: {}", botv)
    };
    Runner::run_bot(&mut bot, (host.parse::<Ipv4Addr>().expect("Expected IPv4 address for host"), port))
}
