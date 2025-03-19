use bugreport::bugreport;
use bugreport::collector::{
    CompileTimeInformation, EnvironmentVariables, OperatingSystem, SoftwareVersion,
};
use bugreport::format::Markdown;
use clap::{Arg, ArgAction, ArgMatches, Command, command};
use clap_complete::{Shell, generate};
use miette::{Context, IntoDiagnostic};
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

const VALIDATE_CMD: &str = "validate";
const INFO_CMD: &str = "info";
const NUGET_CMD: &str = "nuget";
const JSON_CMD: &str = "json";
const COMPLETION_CMD: &str = "completion";
const BUGREPORT_CMD: &str = "bugreport";

const EXT_OPT: &str = "ext";
const RECURSIVELY_FLAG: &str = "recursively";
const SHOW_ERRORS_FLAG: &str = "showerrors";
const PRETTY_FLAG: &str = "pretty";
const TIME_FLAG: &str = "time";
const PROBLEMS_FLAG: &str = "problems";
const FAIL_FLAG: &str = "fail";
const MISMATCH_FLAG: &str = "mismatch";

const EXT_DESCR: &str = "Visual Studio solution extension";
const RECURSIVELY_DESCR: &str = "Scan directory recursively. False by default";
const SHOW_ERROR_ON_DIR_SCAN_DESCR: &str =
    "Output solution parsing errors while scanning directories. False by default";
const BENCHMARK_DESCR: &str = "Show scanning time in case of directory scanning. False by default";
const PATH_DESCR: &str = "Sets solution path or directory to analyze";
const DEFAULT_SOLUTION_EXT: &str = "sln";

fn main() -> miette::Result<()> {
    let app = build_cli();
    let matches = app.get_matches();

    match matches.subcommand() {
        Some((VALIDATE_CMD, cmd)) => validate(cmd),
        Some((INFO_CMD, cmd)) => info(cmd),
        Some((NUGET_CMD, cmd)) => nuget(cmd),
        Some((JSON_CMD, cmd)) => json(cmd),
        Some((COMPLETION_CMD, cmd)) => {
            print_completions(cmd);
            Ok(())
        }
        Some((BUGREPORT_CMD, _)) => {
            print_bugreport();
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate(cmd: &ArgMatches) -> miette::Result<()> {
    let only_problems = cmd.get_flag(PROBLEMS_FLAG);

    let mut consumer = Validate::new(only_problems);
    scan_path(cmd, &mut consumer)
}

fn info(cmd: &ArgMatches) -> miette::Result<()> {
    let mut consumer = Info::new();
    scan_path_or_stdin(cmd, &mut consumer)
}

fn nuget(cmd: &ArgMatches) -> miette::Result<()> {
    let only_mismatched = cmd.get_flag(MISMATCH_FLAG);
    let fail_if_mismatched = cmd.get_flag(FAIL_FLAG);

    let mut consumer = Nuget::new(only_mismatched);
    let result = scan_path(cmd, &mut consumer);
    if consumer.mismatches_found && fail_if_mismatched {
        std::process::exit(exitcode::SOFTWARE);
    }
    result
}

fn json(cmd: &ArgMatches) -> miette::Result<()> {
    let pretty = cmd.get_flag(PRETTY_FLAG);
    let mut consumer = Json::new(pretty);
    scan_path_or_stdin(cmd, &mut consumer)
}

fn scan_path_or_stdin<C: Consume + Display>(
    cmd: &ArgMatches,
    consumer: &mut C,
) -> miette::Result<()> {
    if cmd.get_one::<String>(PATH).is_some() {
        scan_path(cmd, consumer)
    } else {
        scan_stream(io::stdin(), consumer)
    }
}

#[allow(clippy::cast_possible_truncation)]
fn scan_path<C: Consume + Display>(cmd: &ArgMatches, consumer: &mut C) -> miette::Result<()> {
    let now = Instant::now();
    if let Some(path) = cmd.get_one::<String>(PATH) {
        let metadata = fs::metadata(path)
            .into_diagnostic()
            .wrap_err_with(|| format!("Failed to use path: {path}"))?;
        if metadata.is_dir() {
            let empty = String::default();
            let extension = cmd.get_one::<String>(EXT_OPT).unwrap_or(&empty);
            let recursively = cmd.get_flag(RECURSIVELY_FLAG);
            let show_errors = cmd.get_flag(SHOW_ERRORS_FLAG);
            if recursively {
                solp::parse_dir_tree(path, extension, consumer, show_errors);
            } else {
                solp::parse_dir(path, extension, consumer, show_errors);
            }
        } else {
            solp::parse_file(path, consumer)?;
        }
        print!("{consumer}");

        if cmd.get_flag(TIME_FLAG) {
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

fn scan_stream<C: Consume + Display, R: Read>(read: R, consumer: &mut C) -> miette::Result<()> {
    let mut contents = String::new();
    let mut br = BufReader::new(read);
    br.read_to_string(&mut contents)
        .into_diagnostic()
        .wrap_err_with(|| "Failed to read content from stream")?;
    let solution = solp::parse_str(&contents)?;
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

fn print_bugreport() {
    bugreport!()
        .info(SoftwareVersion::default())
        .info(OperatingSystem::default())
        .info(EnvironmentVariables::list(&["SHELL", "TERM"]))
        .info(CompileTimeInformation::default())
        .print::<Markdown>();
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
        .subcommand(bugreport_cmd())
}

fn info_cmd() -> Command {
    Command::new(INFO_CMD)
        .aliases(["i"])
        .about("Get information about found solutions")
        .arg(extension_arg())
        .arg(recursively_arg())
        .arg(show_errors_on_dir_scan_arg())
        .arg(time_arg())
        .arg(path_arg())
}

fn validate_cmd() -> Command {
    Command::new(VALIDATE_CMD)
        .aliases(["va"])
        .about("Validates solutions within directory or file specified")
        .arg(extension_arg())
        .arg(
            Arg::new(PROBLEMS_FLAG)
                .long(PROBLEMS_FLAG)
                .short('p')
                .required(false)
                .action(ArgAction::SetTrue)
                .help("Show only solutions with problems. Correct solutions will not be shown."),
        )
        .arg(recursively_arg())
        .arg(show_errors_on_dir_scan_arg())
        .arg(time_arg())
        .arg(path_arg().required(true))
}

fn nuget_cmd() -> Command {
    Command::new(NUGET_CMD)
    .aliases(["nu"])
    .about("Get nuget packages information within solutions")
    .arg(extension_arg())
    .arg(
        Arg::new(MISMATCH_FLAG)
            .long(MISMATCH_FLAG)
            .short('m')
            .required(false)
            .action(ArgAction::SetTrue)
            .help(
            "Show only mismatched packages if any. i.e. packages with different versions in the same solution",
        ),
    )
    .arg(
        Arg::new(FAIL_FLAG)
            .long(FAIL_FLAG)
            .short('f')
            .required(false)
            .action(ArgAction::SetTrue)
            .help("Return not zero exit code if nuget mismatches found"),
    )
    .arg(recursively_arg())
    .arg(show_errors_on_dir_scan_arg())
    .arg(time_arg())
    .arg(path_arg().required(true))
}

fn json_cmd() -> Command {
    Command::new(JSON_CMD)
        .aliases(["j"])
        .about("Converts solution(s) into json")
        .arg(extension_arg())
        .arg(recursively_arg())
        .arg(show_errors_on_dir_scan_arg())
        .arg(time_arg())
        .arg(
            Arg::new(PRETTY_FLAG)
                .long(PRETTY_FLAG)
                .short('p')
                .required(false)
                .action(ArgAction::SetTrue)
                .help("Pretty-printed output. False by default"),
        )
        .arg(path_arg())
}

fn completion_cmd() -> Command {
    Command::new(COMPLETION_CMD)
        .about("Generate the autocompletion script for the specified shell")
        .arg(
            arg!([generator])
                .value_parser(value_parser!(Shell))
                .required(true)
                .index(1),
        )
}

fn bugreport_cmd() -> Command {
    Command::new(BUGREPORT_CMD)
        .about("Collect information about the system and the environment that users can send along with a bug report")
}

fn path_arg() -> Arg {
    arg!([PATH]).help(PATH_DESCR)
}

fn time_arg() -> Arg {
    Arg::new(TIME_FLAG)
        .long(TIME_FLAG)
        .short('t')
        .required(false)
        .action(ArgAction::SetTrue)
        .help(BENCHMARK_DESCR)
}

fn extension_arg() -> Arg {
    Arg::new(EXT_OPT)
        .long(EXT_OPT)
        .short('e')
        .value_name("EXTENSION")
        .required(false)
        .requires(PATH)
        .default_value(DEFAULT_SOLUTION_EXT)
        .help(EXT_DESCR)
}

fn recursively_arg() -> Arg {
    Arg::new(RECURSIVELY_FLAG)
        .long(RECURSIVELY_FLAG)
        .short('r')
        .required(false)
        .requires(PATH)
        .action(ArgAction::SetTrue)
        .help(RECURSIVELY_DESCR)
}

fn show_errors_on_dir_scan_arg() -> Arg {
    Arg::new(SHOW_ERRORS_FLAG)
        .long(SHOW_ERRORS_FLAG)
        .required(false)
        .requires(PATH)
        .action(ArgAction::SetTrue)
        .help(SHOW_ERROR_ON_DIR_SCAN_DESCR)
}
