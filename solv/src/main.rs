use clap::{command, Command};
use std::time::Instant;

#[macro_use]
extern crate clap;

fn main() {
    let app = build_cli();
    let matches = app.get_matches();

    let debug = matches.contains_id("debug");
    let empty = String::default();

    if let Some(cmd) = matches.subcommand_matches("d") {
        if let Some(path) = cmd.get_one::<String>("PATH") {
            let now = Instant::now();
            let only_problems = cmd.contains_id("problems");
            let extension = cmd.get_one::<String>("ext").unwrap_or(&empty);

            let is_info = cmd.contains_id("info");
            let mut consumer = solv::new_consumer(debug, !is_info, only_problems);
            let scanned = solp::scan(path, extension, consumer.as_consume());

            println!();

            print!("{}", consumer);

            println!("{:>20} {}", "solutions scanned:", scanned);

            println!(
                "{:>20} {}",
                "elapsed:",
                humantime::format_duration(now.elapsed())
            );
        }
    }
    if let Some(cmd) = matches.subcommand_matches("s") {
        if let Some(path) = cmd.get_one::<String>("PATH") {
            let is_info = cmd.contains_id("info");
            let mut consumer = solv::new_consumer(debug, !is_info, false);
            solp::parse_file(path, consumer.as_consume());
        }
    }
}

fn build_cli() -> Command<'static> {
    return command!(crate_name!())
        .arg_required_else_help(true)
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .arg(
            arg!(-d --debug)
                .required(false)
                .takes_value(false)
                .help("debug mode - just printing AST and parsing errors if any"),
        )
        .subcommand(
            Command::new("d")
                .aliases(&["dir", "directory"])
                .about("Analyse all solutions within directory specified")
                .arg(
                    arg!([PATH])
                        .help("Sets directory path to find solutions")
                        .required(true),
                )
                .arg(
                    arg!(-i --info)
                        .required(false)
                        .takes_value(false)
                        .help("show solutions info without validation"),
                )
                .arg(
                    arg!(-e --ext <EXTENSION>)
                        .required(false)
                        .takes_value(true)
                        .default_value("sln")
                        .help("Visual Studio solution extension"),
                )
                .arg(
                    arg!(-p --problems)
                        .required(false)
                        .takes_value(false)
                        .help(
                        "Show only solutions with problems. Correct solutions will not be shown.",
                    ),
                ),
        )
        .subcommand(
            Command::new("s")
                .aliases(&["solution", "single"])
                .about("Analyse solution specified")
                .arg(
                    arg!(-i --info)
                        .required(false)
                        .takes_value(false)
                        .help("show solution info without validation"),
                )
                .arg(
                    arg!([PATH])
                        .help("Sets solution path to analyze")
                        .required(true),
                ),
        );
}
