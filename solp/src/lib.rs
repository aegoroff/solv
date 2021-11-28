use std::fs;

use crate::ast::Solution;
use jwalk::{Parallelism, WalkDir};
use std::option::Option::Some;

pub mod ast;
mod lex;
pub mod msbuild;
mod parser;

#[macro_use]
extern crate lalrpop_util;
extern crate jwalk;
extern crate nom;
extern crate petgraph;

#[cfg(test)] // <-- not needed in integration tests
#[macro_use]
extern crate spectral;

#[cfg(test)] // <-- not needed in integration tests
extern crate rstest;

lalrpop_mod!(
    #[allow(clippy::all)]
    #[allow(unused)]
    pub solp
);

/// Consume provides parsed solution consumer
pub trait Consume {
    /// Called in case of success parsing
    fn ok(&mut self, path: &str, solution: &Solution);
    /// Called on error
    fn err(&self, path: &str);
    /// Whether to use debug mode (usually just print AST into console)
    fn is_debug(&self) -> bool;
}

/// parse parses single solution file specified by path.
pub fn parse(path: &str, consumer: &mut dyn Consume) {
    match fs::read_to_string(path) {
        Ok(contents) => {
            if let Some(solution) = parser::parse_str(&contents, consumer.is_debug()) {
                consumer.ok(path, &solution);
            } else {
                consumer.err(path);
            }
        }
        Err(e) => eprintln!("{} - {}", path, e),
    }
}

/// scan parses directory specified by path. recursively
/// it finds all files with sln extension and parses them.
/// returns the number of scanned solutions
pub fn scan(path: &str, extension: &str, consumer: &mut dyn Consume) -> usize {
    let parallelism = Parallelism::RayonNewPool(num_cpus::get_physical());

    let iter = WalkDir::new(path)
        .skip_hidden(false)
        .follow_links(false)
        .parallelism(parallelism);

    let ext = extension.trim_start_matches('.');

    iter.into_iter()
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .filter(|f| f.file_type().is_file())
        .map(|f| f.path())
        .filter(|p| {
            return match p.extension() {
                Some(s) => s == ext,
                None => false,
            };
        })
        .map(|f| f.to_str().unwrap_or("").to_string())
        .inspect(|fp| parse(fp, consumer))
        .count()
}
