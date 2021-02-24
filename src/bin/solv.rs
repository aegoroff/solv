use clap::{App, Arg, SubCommand};
use solv::print::{DanglingSearch, Info};
use solv::Consume;
use std::time::Instant;

extern crate clap;
extern crate humantime;

fn main() {
    let app = build_cli();
    let matches = app.get_matches();

    let debug = matches.is_present("debug");
    let only_validate = matches.is_present("validate");

    if let Some(cmd) = matches.subcommand_matches("d") {
        if let Some(path) = cmd.value_of("PATH") {
            let now = Instant::now();
            let only_problems = cmd.is_present("problems");

            let consumer: Box<dyn Consume> = new_consumer(debug, only_validate, only_problems);
            solv::scan(path, &consumer);

            println!(
                "elapsed: {}",
                humantime::format_duration(now.elapsed()).to_string()
            );
        }
    }
    if let Some(cmd) = matches.subcommand_matches("s") {
        if let Some(path) = cmd.value_of("PATH") {
            let consumer: Box<dyn Consume> = new_consumer(debug, only_validate, false);
            solv::parse(path, &consumer);
        }
    }
}

fn new_consumer(debug: bool, only_validate: bool, only_problems: bool) -> Box<dyn Consume> {
    return if only_validate {
        DanglingSearch::new(debug, only_problems)
    } else {
        Info::new(debug)
    };
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
        .arg(
            Arg::with_name("validate")
                .long("onlyvalidate")
                .short("v")
                .takes_value(false)
                .help("only validate solution without printing info")
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
                    Arg::with_name("PATH")
                        .help("Sets solution path to analyze")
                        .required(true)
                        .index(1),
                ),
        );
}
