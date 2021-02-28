use std::fs;
use std::path::Path;

use crate::ast::Solution;
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
    fn ok(&self, path: &str, solution: &Solution);
    fn err(&self, path: &str);
    fn is_debug(&self) -> bool;
}

/// parse parses single solution file specified by path.
pub fn parse(path: &str, consumer: &dyn Consume) {
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
pub fn scan(path: &str, extension: &str, consumer: &dyn Consume) -> usize {
    let iter = WalkDir::new(path).skip_hidden(false).follow_links(false);

    iter.into_iter()
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .filter(|f| f.file_type().is_file())
        .filter_map(|f| {
            let ext = f.file_name.to_str().unwrap_or("");
            let ext = get_extension_from_filename(ext)?;
            if ext == extension {
                return Some(f.path().to_str().unwrap_or("").to_string());
            }
            None
        })
        .inspect(|fp| parse(&fp, consumer))
        .count()
}

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension()?.to_str()
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

    #[test]
    fn cut_from_back_until_necessary_chars_more_then_skip_plus_one() {
        // Arrange
        let s = "a.b.c.d";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_eq!("a.b", c);
    }

    #[test]
    fn cut_from_back_until_has_necessary_chars_to_skip() {
        // Arrange
        let s = "a.b.c";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_eq!("a", c);
    }

    #[test]
    fn cut_from_back_until_necessary_chars_to_skip_following_each_other() {
        // Arrange
        let s = "a..b.c";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_eq!("a.", c);
    }

    #[test]
    fn cut_from_back_until_only_necessary_chars() {
        // Arrange
        let s = "...";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_eq!(".", c);
    }

    #[test]
    fn cut_from_back_until_only_necessary_chars_eq_skip_plus_one() {
        // Arrange
        let s = "..";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_eq!("", c);
    }

    #[test]
    fn cut_from_back_until_only_necessary_chars_eq_skip() {
        // Arrange
        let s = ".";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_eq!("", c);
    }

    #[test]
    fn cut_from_back_until_chars_to_skip_not_enough() {
        // Arrange
        let s = "a.b";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_eq!("", c);
    }

    #[test]
    fn cut_from_back_until_chars_to_skip_not_present() {
        // Arrange
        let s = "ab";

        // Act
        let c = cut_from_back_until(s, '.', 1);

        // Assert
        assert_eq!("", c);
    }
}
