mod print;

use crate::print::{Info, Validate};
use clap::{App, Arg, SubCommand};
use solp::Consume;
use std::time::Instant;

extern crate clap;
extern crate humantime;
extern crate solp;
#[macro_use]
extern crate prettytable;

fn main() {
    let app = build_cli();
    let matches = app.get_matches();

    let debug = matches.is_present("debug");

    if let Some(cmd) = matches.subcommand_matches("d") {
        if let Some(path) = cmd.value_of("PATH") {
            let now = Instant::now();
            let only_problems = cmd.is_present("problems");
            let extension = cmd.value_of("ext").unwrap_or("");

            let is_info = cmd.is_present("info");
            let consumer = new_consumer(debug, !is_info, only_problems);
            let scanned = solp::scan(path, extension, &*consumer);

            println!();
            println!("{:>20} {}", "solutions scanned:", scanned);

            println!(
                "{:>20} {}",
                "elapsed:",
                humantime::format_duration(now.elapsed()).to_string()
            );
        }
    }
    if let Some(cmd) = matches.subcommand_matches("s") {
        if let Some(path) = cmd.value_of("PATH") {
            let is_info = cmd.is_present("info");
            let consumer = new_consumer(debug, !is_info, false);
            solp::parse(path, &*consumer);
        }
    }
}

fn new_consumer(debug: bool, only_validate: bool, only_problems: bool) -> Box<dyn Consume> {
    if only_validate {
        Validate::new_box(debug, only_problems)
    } else {
        Info::new_box(debug)
    }
}

fn build_cli() -> App<'static, 'static> {
    return App::new("solv")
        .version("0.1")
        .author("egoroff <egoroff@gmail.com>")
        .about("SOLution Validation tool that analyzes Microsoft Visual Studio solutions")
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .short("d")
                .takes_value(false)
                .help("debug mode - just printing AST and parsing errors if any")
                .required(false),
        )
        .subcommand(
            SubCommand::with_name("d")
                .aliases(&["dir", "directory"])
                .about("Analyse all solutions within directory specified")
                .arg(
                    Arg::with_name("PATH")
                        .help("Sets directory path to find solutions")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("info")
                        .long("info")
                        .short("i")
                        .takes_value(false)
                        .help("show solutions info without validation")
                        .required(false),
                )
                .arg(
                    Arg::with_name("ext")
                        .long("ext")
                        .short("e")
                        .takes_value(true)
                        .default_value("sln")
                        .help("Visual Studio solution extension")
                        .required(false),
                )
                .arg(
                    Arg::with_name("problems")
                        .long("problems")
                        .short("p")
                        .takes_value(false)
                        .help("Show only solutions with problems. Correct solutions will not be shown.")
                        .required(false),
                ),
        )
        .subcommand(
            SubCommand::with_name("s")
                .aliases(&["solution", "single"])
                .about("Analyse solution specified")
                .arg(
                    Arg::with_name("info")
                        .long("info")
                        .short("i")
                        .takes_value(false)
                        .help("show solution info without validation")
                        .required(false),
                )
                .arg(
                    Arg::with_name("PATH")
                        .help("Sets solution path to analyze")
                        .required(true)
                        .index(1),
                ),
        );
}
