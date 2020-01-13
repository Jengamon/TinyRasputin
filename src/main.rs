mod bots;

use clap::{App, Arg, SubCommand, AppSettings};
use bots::*;
use tinyrasputin::{
    engine::relations::{RelationsExt},
    skeleton::{runner::Runner, bot::PokerBot, cards::CardValue}
};
use itertools::Itertools;
use std::net::Ipv4Addr;

fn main() -> std::io::Result<()> {
    // read in arguments
    let matches = App::new("TinyRasputin - A Rust PokerBot")
                    .setting(AppSettings::SubcommandRequiredElseHelp)
                    .version(env!("CARGO_PKG_VERSION"))
                    .author("Jengamon <uokwo@mit.edu>")
                    .about("Can play poker over the interwebz or analyze its results")
                    .subcommand(SubCommand::with_name("analyze")
                        .about("Analyze a result file using various commands")
                        .version(env!("CARGO_PKG_VERSION"))
                        .author("Jengamon <uokwo@mit.edu>")
                        .arg(Arg::with_name("path")
                            .required(true)
                            .index(1)))
                    .subcommand(SubCommand::with_name("bot")
                        .about("Run a bot included in this package")
                        .version(env!("CARGO_PKG_VERSION"))
                        .author("Jengamon <uokwo@mit.edu>")
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
                            .index(1)))
                    .get_matches();

    if let Some(matches) = matches.subcommand_matches("analyze") {
        use std::io::{BufReader, BufRead};
        use std::fs::File;
        // We want to analyze something
        let file = BufReader::new(File::open(matches.value_of("path").unwrap())?);
        let mut lines: Vec<_> = file.lines().filter_map(Result::ok).collect();

        fn read_cv_array<C>(mut chars: C) -> (Vec<CardValue>, C) where C: Iterator<Item=char> {
            assert!(chars.next() == Some('['));
            let mut values = vec![];
            while let Some(c) = chars.next() {
                if c == ']' { break }
                values.push(c.to_string().parse::<CardValue>().unwrap());
            }
            (values, chars)
        }

        while !lines.is_empty() {
            let line: String = lines.remove(0).trim().to_string().to_ascii_lowercase();
            if line.starts_with("#") {
                continue
            }
            let relations: Vec<_> = lines.drain(0..13).flat_map(|x| {
                let mut chars = x.chars();
                assert!(chars.next() == Some('|'));
                let value = chars.next().map(|x| x.to_string().parse::<CardValue>().unwrap()).unwrap();
                assert!(chars.next() == Some('|'));
                let (pre, chars) = read_cv_array(chars);
                let (post, _) = read_cv_array(chars);
                pre.into_iter().map(|x| (x, value)).chain(post.into_iter().map(|x| (value, x))).collect::<Vec<_>>()
            }).collect::<Vec<_>>().remove_redundancies();
            let simplified = relations.simplify();

            let commands = line.split(",").map(|x| x.trim().to_string());
            for command in commands {
                match &command[..] {
                    "simplify" => {
                        println!("{}", simplified.debug_relations());
                    },
                    "count" => {
                        println!("{}", relations.len());
                    },
                    "possibilities" => {
                        println!("{}", simplified.possibilities());
                    },
                    "check" => {
                        let order = lines.remove(0).chars().map(|c| c.to_string().parse::<CardValue>().unwrap()).collect::<Vec<_>>();
                        assert!(order.len() == 13);
                        println!("Correctness check for {}", order.iter().format(" -> "));
                        println!("Rule count: {} ({})", relations.len(), simplified.len());
                        let violations = simplified.iter().filter(|(a, b)| {
                            let a_index = order.iter().position(|x| x == a);
                            let b_index = order.iter().position(|x| x == b);
                            b_index < a_index
                        }).collect::<Vec<_>>();
                        for (a, b) in violations.iter() {
                            println!("Rule violation: {} -> {}", a, b);
                        }
                        println!("Correctness: {}%", 100.0 * (1.0 - (violations.len() as f64 / simplified.len() as f64)));
                        println!("{}", relations.possibilities());
                    },
                    c => println!("Invalid command {}", c)
                }
            }
        }
        Ok(())
    } else if let Some(matches) = matches.subcommand_matches("bot") {
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
    } else {
        unreachable!()
    }
}
