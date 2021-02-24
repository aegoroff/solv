use clap::{App, Arg, SubCommand};
use solv::print::Print;
use std::time::Instant;

extern crate clap;
extern crate humantime;

fn main() {
    let app = build_cli();
    let matches = app.get_matches();

    let debug = matches.is_present("debug");

    if let Some(cmd) = matches.subcommand_matches("d") {
        if let Some(path) = cmd.value_of("PATH") {
            let now = Instant::now();
            solv::scan(path, debug);
            println!(
                "elapsed: {}",
                humantime::format_duration(now.elapsed()).to_string()
            );
        }
    }
    if let Some(cmd) = matches.subcommand_matches("s") {
        if let Some(path) = cmd.value_of("PATH") {
            let prn = Print::new(path);
            solv::parse(path, prn, debug);
        }
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
