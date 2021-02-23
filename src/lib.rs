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
    pub solt
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
    for entry in WalkDir::new(path).skip_hidden(false).follow_links(false) {
        if let Ok(e) = entry {
            if !e.file_type().is_file() {
                continue;
            }
            let file_name = e.file_name.to_str().unwrap();
            if let Some(ext) = get_extension_from_filename(file_name) {
                if ext == "sln" {
                    let full_path = e.path();
                    let full_path = full_path.to_str().unwrap();
                    let prn = Print::new(full_path);
                    parse(full_path, prn, debug);
                }
            }
        }
    }
}

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension()?.to_str()
}
