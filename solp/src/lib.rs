#![warn(unused_extern_crates)]
#![allow(clippy::missing_errors_doc)]

use std::fs;

use api::Solution;
use jwalk::{Parallelism, WalkDir};
use std::option::Option::Some;

pub mod api;
mod ast;
mod lex;
pub mod msbuild;
mod parser;

#[macro_use]
extern crate lalrpop_util;

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
    fn ok(&mut self, solution: &Solution);
    /// Called on error
    fn err(&self, path: &str);
}

/// `parse_file` parses single solution file specified by path.
pub fn parse_file(path: &str, consumer: &mut dyn Consume) {
    match fs::read_to_string(path) {
        Ok(contents) => match parse_str(&contents) {
            Some(mut solution) => {
                solution.path = path;
                consumer.ok(&solution);
            }
            None => consumer.err(path),
        },
        Err(e) => eprintln!("{path} - {e}"),
    }
}

/// `parse_str` parses solution content in memory
#[must_use]
pub fn parse_str(contents: &str) -> Option<Solution> {
    let parsed = parser::parse_str(contents)?;
    Some(Solution::from(&parsed))
}

/// `parse_dir` parses only directory specified by path.
/// it finds all files with extension specified and parses them.
/// returns the number of scanned solutions
pub fn parse_dir(path: &str, extension: &str, consumer: &mut dyn Consume) -> usize {
    let iter = create_dir_iterator(path).max_depth(1);
    parse_dir_or_tree(iter, extension, consumer)
}

/// `parse_dir_tree` parses directory specified by path. recursively
/// it finds all files with extension specified and parses them.
/// returns the number of scanned solutions
pub fn parse_dir_tree(path: &str, extension: &str, consumer: &mut dyn Consume) -> usize {
    let parallelism = Parallelism::RayonNewPool(num_cpus::get_physical());
    let iter = create_dir_iterator(path).parallelism(parallelism);
    parse_dir_or_tree(iter, extension, consumer)
}

fn create_dir_iterator(path: &str) -> WalkDir {
    let root = decorate_path(path);
    WalkDir::new(root).skip_hidden(false).follow_links(false)
}

fn parse_dir_or_tree(iter: WalkDir, extension: &str, consumer: &mut dyn Consume) -> usize {
    let ext = extension.trim_start_matches('.');

    iter.into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|f| f.file_type().is_file())
        .map(|f| f.path())
        .filter(|p| p.extension().map(|s| s == ext).unwrap_or_default())
        .map(|f| f.to_str().unwrap_or("").to_string())
        .inspect(|fp| parse_file(fp, consumer))
        .count()
}

/// On Windows trailing back slash (\) to be added if volume and colon passed (like c:).
/// It needed paths look to be more pleasant
#[cfg(target_os = "windows")]
fn decorate_path(path: &str) -> String {
    if path.len() == 2 && path.ends_with(':') {
        format!("{path}\\")
    } else {
        path.to_owned()
    }
}

/// On Unix just passthrough as is
#[cfg(not(target_os = "windows"))]
fn decorate_path(path: &str) -> String {
    path.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[cfg(not(target_os = "windows"))]
    #[rstest]
    #[case("", "")]
    #[case("/", "/")]
    #[case("/home", "/home")]
    #[case("d:", "d:")]
    #[trace]
    fn decorate_path_tests(#[case] raw_path: &str, #[case] expected: &str) {
        // Arrange

        // Act
        let actual = decorate_path(raw_path);

        // Assert
        assert_eq!(actual, expected);
    }

    #[cfg(target_os = "windows")]
    #[rstest]
    #[case("", "")]
    #[case("/", "/")]
    #[case("d:", "d:\\")]
    #[case("dd:", "dd:")]
    #[trace]
    fn decorate_path_tests(#[case] raw_path: &str, #[case] expected: &str) {
        // Arrange

        // Act
        let actual = decorate_path(raw_path);

        // Assert
        assert_eq!(actual, expected);
    }
}
