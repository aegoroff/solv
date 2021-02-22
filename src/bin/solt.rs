use clap::{App, Arg, SubCommand};

extern crate clap;

fn main() {
    let app = build_cli();
    let matches = app.get_matches();

    if let Some(cmd) = matches.subcommand_matches("d") {
        if let Some(path) = cmd.value_of("PATH") {
            solt_rs::scan(path);
        }
    }
    if let Some(cmd) = matches.subcommand_matches("s") {
        if let Some(path) = cmd.value_of("PATH") {
            solt_rs::parse(path);
        }
    }
}

fn build_cli() -> App<'static, 'static> {
    return App::new("solt")
        .version("0.1")
        .author("egoroff <egoroff@gmail.com>")
        .about("SOLution Tool that analyzes Microsoft Visual Studio solutions")
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
