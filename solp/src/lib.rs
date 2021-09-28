use std::fs;

use crate::ast::Solution;
use jwalk::WalkDir;
use std::option::Option::Some;

pub mod ast;
mod lex;
pub mod msbuild;
mod parser;

#[macro_use]
extern crate lalrpop_util;
extern crate jwalk;
extern crate petgraph;
extern crate spectral;

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
    let iter = WalkDir::new(path).skip_hidden(false).follow_links(false);

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

fn cut_from_back_until(s: &str, ch: char, skip: usize) -> &str {
    let cut = cut_count(s, ch, skip);
    &s[..s.len() - cut]
}

fn cut_count(s: &str, ch: char, skip: usize) -> usize {
    let mut counter = 0;

    let count = s
        .chars()
        .rev()
        .take_while(|c| {
            if *c == ch {
                counter += 1;
            }
            counter <= skip
        })
        .count();

    if count == s.len() {
        s.len()
    } else {
        count + 1 // Last ch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn cut_from_back_until_necessary_chars_more_then_skip_plus_one() {
        // Arrange
        let s = "a.b.c.d";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_that(&c).is_equal_to(&"a.b");
    }

    #[test]
    fn cut_from_back_until_has_necessary_chars_to_skip() {
        // Arrange
        let s = "a.b.c";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_that(&c).is_equal_to(&"a");
    }

    #[test]
    fn cut_from_back_until_necessary_chars_to_skip_following_each_other() {
        // Arrange
        let s = "a..b.c";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_that(&c).is_equal_to(&"a.");
    }

    #[test]
    fn cut_from_back_until_only_necessary_chars() {
        // Arrange
        let s = "...";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_that(&c).is_equal_to(&".");
    }

    #[test]
    fn cut_from_back_until_only_necessary_chars_eq_skip_plus_one() {
        // Arrange
        let s = "..";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_that(&c).is_equal_to(&"");
    }

    #[test]
    fn cut_from_back_until_only_necessary_chars_eq_skip() {
        // Arrange
        let s = ".";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_that(&c).is_equal_to(&"");
    }

    #[test]
    fn cut_from_back_until_chars_to_skip_not_enough() {
        // Arrange
        let s = "a.b";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_that(&c).is_equal_to(&"");
    }

    #[test]
    fn cut_from_back_until_chars_to_skip_not_present() {
        // Arrange
        let s = "ab";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_that(&c).is_equal_to(&"");
    }
}
