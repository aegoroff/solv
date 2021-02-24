use std::fs;
use std::path::Path;

use crate::ast::Solution;
use crate::print::Print;
use jwalk::WalkDir;
use std::option::Option::Some;

mod ast;
mod lex;
mod msbuild;
mod parser;
pub mod print;

#[macro_use]
extern crate lalrpop_util;
extern crate jwalk;
#[macro_use]
extern crate prettytable;

lalrpop_mod!(
    #[allow(clippy::all)]
    #[allow(unused)]
    pub solv
);

/// Consume provides parsed solution consumer
pub trait Consume {
    fn ok(&self, solution: &Solution);
    fn err(&self);
}

/// parse parses single solution file specified by path.
pub fn parse<C: Consume>(path: &str, consumer: C, debug: bool) {
    let contents = fs::read_to_string(path).expect("Something went wrong reading the file");
    if let Some(solution) = parser::parse_str(&contents, debug) {
        consumer.ok(&solution);
    } else {
        if !debug {
            consumer.err();
        }
    }
}

/// scan parses directory specified by path. recursively
/// it finds all files with sln extension and parses them.
pub fn scan(path: &str, debug: bool) {
    let iter = WalkDir::new(path).skip_hidden(false).follow_links(false);

    let it = iter
        .into_iter()
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .filter(|f| f.file_type().is_file())
        .filter_map(|f| {
            let ext = f.file_name.to_str().unwrap_or("");
            let ext = get_extension_from_filename(ext)?;
            if ext == "sln" {
                return Some(f.path().to_str().unwrap_or("").to_string());
            }
            None
        });

    for full_path in it {
        let prn = Print::new(&full_path);
        parse(&full_path, prn, debug);
    }
}

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension()?.to_str()
}
