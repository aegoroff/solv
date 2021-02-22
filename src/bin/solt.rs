use clap::{App, Arg};

extern crate clap;

fn main() {
    let app = build_cli();
    let matches = app.get_matches();

    match matches.value_of("PATH") {
        Some(path) => solt_rs::scan(path),
        None => {}
    }
}

fn build_cli() -> App<'static, 'static> {
    return App::new("solt")
        .version("0.1")
        .author("egoroff <egoroff@gmail.com>")
        .about("SOLution Tool that analyzes Microsoft Visual Studio solutions")
        .arg(
            Arg::with_name("PATH")
                .help("Sets directory path to find solutions")
                .required(true)
                .index(1),
        );
}