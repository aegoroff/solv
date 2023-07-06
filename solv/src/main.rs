use clap::{command, ArgAction, ArgMatches, Command};
use clap_complete::{generate, Shell};
use solp::Consume;
use solv::info::Info;
use solv::nuget::Nuget;
use solv::validate::Validate;
use std::fs;
use std::{
    io,
    time::{Duration, Instant},
};

#[macro_use]
extern crate clap;

const PATH: &str = "PATH";

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

    let consumer = Validate::new(only_problems);
    scan_path(cmd, consumer);
}

fn info(cmd: &ArgMatches) {
    let consumer = Info::new();
    scan_path(cmd, consumer);
}

fn nuget(cmd: &ArgMatches) {
    let consumer = Nuget::new();
    scan_path(cmd, consumer);
}

fn scan_path<C: Consume>(cmd: &ArgMatches, mut consumer: C) {
    if let Some(path) = cmd.get_one::<String>(PATH) {
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.is_dir() {
                let now = Instant::now();
                let empty = String::default();
                let extension = cmd.get_one::<String>("ext").unwrap_or(&empty);
                let scanned = solp::scan(path, extension, &mut consumer);
                println!("{:>20} {}", "solutions scanned:", scanned);

                let duration = now.elapsed().as_millis();
                let duration = Duration::from_millis(duration as u64);
                println!(
                    "{:>20} {}",
                    "elapsed:",
                    humantime::format_duration(duration)
                );
            } else {
                solp::parse_file(path, &mut consumer);
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
            Command::new("info")
                .aliases(["i"])
                .about("Get information about found solutions")
                .arg(
                    arg!(-e --ext <EXTENSION>)
                        .required(false)
                        .default_value("sln")
                        .help("Visual Studio solution extension"),
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
                        .default_value("sln")
                        .help("Visual Studio solution extension"),
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
