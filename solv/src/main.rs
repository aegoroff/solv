use clap::{Arg, ArgAction, ArgMatches, Command, command};
use clap_complete::{Shell, generate};
use color_eyre::eyre::{Context, Result};
use solp::Consume;
use solv::info::Info;
use solv::json::Json;
use solv::nuget::Nuget;
use solv::validate::Validate;
use std::fmt::Display;
use std::fs;
use std::io::{BufReader, Read};
use std::{
    io,
    time::{Duration, Instant},
};

#[macro_use]
extern crate clap;

#[cfg(target_os = "linux")]
use mimalloc::MiMalloc;

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const PATH: &str = "PATH";
const EXT_DESCR: &str = "Visual Studio solution extension";
const RECURSIVELY_FLAG: &str = "recursively";
const RECURSIVELY_DESCR: &str = "Scan directory recursively. False by default";
const BENCHMARK_DESCR: &str = "Show scanning time in case of directory scanning. False by default";
const PATH_DESCR: &str = "Sets solution path or directory to analyze";
const DEFAULT_SOLUTION_EXT: &str = "sln";

fn main() -> Result<()> {
    color_eyre::install()?;
    let app = build_cli();
    let matches = app.get_matches();

    match matches.subcommand() {
        Some(("validate", cmd)) => validate(cmd),
        Some(("info", cmd)) => info(cmd),
        Some(("nuget", cmd)) => nuget(cmd),
        Some(("json", cmd)) => json(cmd),
        Some(("completion", cmd)) => {
            print_completions(cmd);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate(cmd: &ArgMatches) -> Result<()> {
    let only_problems = cmd.get_flag("problems");

    let mut consumer = Validate::new(only_problems);
    scan_path(cmd, &mut consumer)
}

fn info(cmd: &ArgMatches) -> Result<()> {
    let mut consumer = Info::new();
    scan_path_or_stdin(cmd, &mut consumer)
}

fn nuget(cmd: &ArgMatches) -> Result<()> {
    let only_mismatched = cmd.get_flag("mismatch");
    let fail_if_mismatched = cmd.get_flag("fail");

    let mut consumer = Nuget::new(only_mismatched);
    let result = scan_path(cmd, &mut consumer);
    if consumer.mismatches_found && fail_if_mismatched {
        std::process::exit(exitcode::SOFTWARE);
    }
    result
}

fn json(cmd: &ArgMatches) -> Result<()> {
    let pretty = cmd.get_flag("pretty");
    let mut consumer = Json::new(pretty);
    scan_path_or_stdin(cmd, &mut consumer)
}

fn scan_path_or_stdin<C: Consume + Display>(cmd: &ArgMatches, consumer: &mut C) -> Result<()> {
    if cmd.get_one::<String>(PATH).is_some() {
        scan_path(cmd, consumer)
    } else {
        scan_stream(io::stdin(), consumer)
    }
}

#[allow(clippy::cast_possible_truncation)]
fn scan_path<C: Consume + Display>(cmd: &ArgMatches, consumer: &mut C) -> Result<()> {
    let now = Instant::now();
    if let Some(path) = cmd.get_one::<String>(PATH) {
        let metadata =
            fs::metadata(path).wrap_err_with(|| format!("Failed to use path: {path}"))?;
        if metadata.is_dir() {
            let empty = String::default();
            let extension = cmd.get_one::<String>("ext").unwrap_or(&empty);
            let recursively = cmd.get_flag(RECURSIVELY_FLAG);
            if recursively {
                solp::parse_dir_tree(path, extension, consumer);
            } else {
                solp::parse_dir(path, extension, consumer);
            }
        } else {
            solp::parse_file(path, consumer)?;
        }
        print!("{consumer}");

        if cmd.get_flag("time") {
            let duration = now.elapsed().as_millis();
            let duration = Duration::from_millis(duration as u64);
            println!(
                " {:>2} {}",
                "elapsed:",
                humantime::format_duration(duration)
            );
        }
    }
    Ok(())
}

fn scan_stream<C: Consume + Display, R: Read>(read: R, consumer: &mut C) -> Result<()> {
    let mut contents = String::new();
    let mut br = BufReader::new(read);
    br.read_to_string(&mut contents)
        .wrap_err_with(|| "Failed to read content from stream")?;
    let solution = solp::parse_str(&contents).wrap_err_with(|| "Failed to parse solution")?;
    consumer.ok(&solution);

    print!("{consumer}");

    Ok(())
}

fn print_completions(matches: &ArgMatches) {
    let mut cmd = build_cli();
    let bin_name = cmd.get_name().to_string();
    if let Some(generator) = matches.get_one::<Shell>("generator") {
        generate(*generator, &mut cmd, bin_name, &mut io::stdout());
    }
}

fn build_cli() -> Command {
    #![allow(non_upper_case_globals)]
    command!(crate_name!())
        .arg_required_else_help(true)
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .subcommand(validate_cmd())
        .subcommand(info_cmd())
        .subcommand(nuget_cmd())
        .subcommand(json_cmd())
        .subcommand(completion_cmd())
}

fn info_cmd() -> Command {
    Command::new("info")
        .aliases(["i"])
        .about("Get information about found solutions")
        .arg(extension_arg())
        .arg(recursively_arg())
        .arg(time_arg())
        .arg(path_arg())
}

fn validate_cmd() -> Command {
    Command::new("validate")
        .aliases(["va"])
        .about("Validates solutions within directory or file specified")
        .arg(extension_arg())
        .arg(
            arg!(-p - -problems)
                .required(false)
                .action(ArgAction::SetTrue)
                .help("Show only solutions with problems. Correct solutions will not be shown."),
        )
        .arg(recursively_arg())
        .arg(time_arg())
        .arg(path_arg().required(true))
}

fn nuget_cmd() -> Command {
    Command::new("nuget")
    .aliases(["nu"])
    .about("Get nuget packages information within solutions")
    .arg(extension_arg())
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
    .arg(recursively_arg())
    .arg(time_arg())
    .arg(path_arg().required(true))
}

fn json_cmd() -> Command {
    Command::new("json")
        .aliases(["j"])
        .about("Converts solution(s) into json")
        .arg(extension_arg())
        .arg(recursively_arg())
        .arg(time_arg())
        .arg(
            arg!(-p - -pretty)
                .required(false)
                .action(ArgAction::SetTrue)
                .help("Pretty-printed output. False by default"),
        )
        .arg(path_arg())
}

fn path_arg() -> Arg {
    arg!([PATH]).help(PATH_DESCR)
}

fn time_arg() -> Arg {
    arg!(-t - -time)
        .required(false)
        .action(ArgAction::SetTrue)
        .help(BENCHMARK_DESCR)
}

fn extension_arg() -> Arg {
    arg!(-e --ext <EXTENSION>)
        .required(false)
        .requires(PATH)
        .default_value(DEFAULT_SOLUTION_EXT)
        .help(EXT_DESCR)
}

fn recursively_arg() -> Arg {
    arg!(-r - -recursively)
        .required(false)
        .requires(PATH)
        .action(ArgAction::SetTrue)
        .help(RECURSIVELY_DESCR)
}

fn completion_cmd() -> Command {
    Command::new("completion")
        .about("Generate the autocompletion script for the specified shell")
        .arg(
            arg!([generator])
                .value_parser(value_parser!(Shell))
                .required(true)
                .index(1),
        )
}
