use clap::{command, ArgAction, ArgMatches, Command};
use clap_complete::{generate, Shell};
use std::{
    io,
    time::{Duration, Instant},
};

#[macro_use]
extern crate clap;

const PATH: &str = "PATH";
const INFO_FLAG: &str = "info";

fn main() {
    let app = build_cli();
    let matches = app.get_matches();

    let debug = matches.get_flag("debug");

    match matches.subcommand() {
        Some(("d", cmd)) => scan_directory(cmd, debug),
        Some(("s", cmd)) => scan_file(cmd, debug),
        Some(("completion", cmd)) => print_completions(cmd),
        _ => {}
    }
}

fn scan_file(cmd: &ArgMatches, debug: bool) {
    if let Some(path) = cmd.get_one::<String>(PATH) {
        let is_info = cmd.get_flag(INFO_FLAG);
        let mut consumer = solv::new_consumer(debug, !is_info, false);
        solp::parse_file(path, consumer.as_consume());
    }
}

fn scan_directory(cmd: &ArgMatches, debug: bool) {
    let empty = String::default();
    if let Some(path) = cmd.get_one::<String>(PATH) {
        let now = Instant::now();
        let only_problems = cmd.get_flag("problems");
        let extension = cmd.get_one::<String>("ext").unwrap_or(&empty);

        let is_info = cmd.get_flag(INFO_FLAG);
        let mut consumer = solv::new_consumer(debug, !is_info, only_problems);
        let scanned = solp::scan(path, extension, consumer.as_consume());

        println!();

        print!("{consumer}");

        println!("{:>20} {}", "solutions scanned:", scanned);

        let duration = now.elapsed().as_millis();
        let duration = Duration::from_millis(duration as u64);
        println!(
            "{:>20} {}",
            "elapsed:",
            humantime::format_duration(duration)
        );
    }
}

fn print_completions(matches: &ArgMatches) {
    let mut cmd = build_cli();
    let bin_name = cmd.get_name().to_string();
    if let Some(generator) = matches.get_one::<Shell>("generator") {
        generate(*generator, &mut cmd, bin_name, &mut io::stdout());
    }
}

fn build_cli() -> Command {
    command!(crate_name!())
        .arg_required_else_help(true)
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .arg(
            arg!(-d --debug)
                .required(false)
                .action(ArgAction::SetTrue)
                .help("debug mode - just printing AST and parsing errors if any"),
        )
        .subcommand(
            Command::new("d")
                .aliases(["dir", "directory"])
                .about("Analyse all solutions within directory specified")
                .arg(
                    arg!([PATH])
                        .help("Sets directory path to find solutions")
                        .required(true),
                )
                .arg(
                    arg!(-i --info)
                        .required(false)
                        .action(ArgAction::SetTrue)
                        .help("show solutions info without validation"),
                )
                .arg(
                    arg!(-e --ext <EXTENSION>)
                        .required(false)
                        .default_value("sln")
                        .help("Visual Studio solution extension"),
                )
                .arg(
                    arg!(-p --problems)
                        .required(false)
                        .action(ArgAction::SetTrue)
                        .help(
                        "Show only solutions with problems. Correct solutions will not be shown.",
                    ),
                ),
        )
        .subcommand(
            Command::new("s")
                .aliases(["solution", "single"])
                .about("Analyse solution specified")
                .arg(
                    arg!(-i --info)
                        .required(false)
                        .action(ArgAction::SetTrue)
                        .help("show solution info without validation"),
                )
                .arg(
                    arg!([PATH])
                        .help("Sets solution path to analyze")
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("completion")
            .about("Generate the autocompletion script for the specified shell")
            .arg(
                arg!([generator])
                    .value_parser(value_parser!(Shell))
                    .required(true)
                    .index(1),
            )
        )
}
