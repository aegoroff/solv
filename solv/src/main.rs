use clap::{command, ArgAction, ArgMatches, Command};
use clap_complete::{generate, Shell};
use solp::Consume;
use solv::info::Info;
use solv::nuget::Nuget;
use solv::validate::Validate;
use std::fmt::Display;
use std::fs;
use std::{
    io,
    time::{Duration, Instant},
};

#[macro_use]
extern crate clap;

const PATH: &str = "PATH";
const EXT_DESCR: &str = "Visual Studio solution extension";
const DEFAULT_SOLUTION_EXT: &str = "sln";

fn main() {
    let app = build_cli();
    let matches = app.get_matches();

    match matches.subcommand() {
        Some(("validate", cmd)) => validate(cmd),
        Some(("info", cmd)) => info(cmd),
        Some(("nuget", cmd)) => nuget(cmd),
        Some(("completion", cmd)) => print_completions(cmd),
        _ => {}
    }
}

fn validate(cmd: &ArgMatches) {
    let only_problems = cmd.get_flag("problems");

    let mut consumer = Validate::new(only_problems);
    scan_path(cmd, &mut consumer);
}

fn info(cmd: &ArgMatches) {
    let mut consumer = Info::new();
    scan_path(cmd, &mut consumer);
}

fn nuget(cmd: &ArgMatches) {
    let only_mismatched = cmd.get_flag("mismatch");
    let fail_if_mismatched = cmd.get_flag("fail");
    let mut consumer = Nuget::new(only_mismatched);
    scan_path(cmd, &mut consumer);
    if consumer.mismatches_found && fail_if_mismatched {
        std::process::exit(exitcode::SOFTWARE);
    }
}

fn scan_path<C: Consume + Display>(cmd: &ArgMatches, consumer: &mut C) {
    if let Some(path) = cmd.get_one::<String>(PATH) {
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.is_dir() {
                let now = Instant::now();
                let empty = String::default();
                let extension = cmd.get_one::<String>("ext").unwrap_or(&empty);
                solp::parse_dir(path, extension, consumer);

                print!("{consumer}");

                let duration = now.elapsed().as_millis();
                let duration = Duration::from_millis(duration as u64);
                println!("{:>2} {}", "elapsed:", humantime::format_duration(duration));
            } else {
                solp::parse_file(path, consumer);
            }
        };
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
        .subcommand(
            Command::new("validate")
                .aliases(["va"])
                .about("Validates solutions within directory or file specified")
                .arg(
                    arg!([PATH])
                        .help("Sets solution path to analyze")
                        .required(true),
                )
                .arg(
                    arg!(-e --ext <EXTENSION>)
                        .required(false)
                        .default_value(DEFAULT_SOLUTION_EXT)
                        .help(EXT_DESCR),
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
            Command::new("info")
                .aliases(["i"])
                .about("Get information about found solutions")
                .arg(
                    arg!(-e --ext <EXTENSION>)
                        .required(false)
                        .default_value(DEFAULT_SOLUTION_EXT)
                        .help(EXT_DESCR),
                )
                .arg(
                    arg!([PATH])
                        .help("Sets solution path to analyze")
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("nuget")
                .aliases(["nu"])
                .about("Get nuget packages information within solutions")
                .arg(
                    arg!(-e --ext <EXTENSION>)
                        .required(false)
                        .default_value(DEFAULT_SOLUTION_EXT)
                        .help(EXT_DESCR),
                )
                .arg(
                    arg!(-m --mismatch)
                        .required(false)
                        .action(ArgAction::SetTrue)
                        .help(
                        "Show only mismatched packages if any. i.e. packages with different versions in the same solution",
                    ),
                )
                .arg(
                    arg!(-f --fail)
                        .required(false)
                        .action(ArgAction::SetTrue)
                        .help("Return not zero exit code if nuget mismatches found"),
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
