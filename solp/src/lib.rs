/*!
A library for parsing Microsoft Visual Studio solution file


## Example: parsing solution from [&str]

```
use solp::parse_str;

const SOLUTION: &str = r#"
Microsoft Visual Studio Solution File, Format Version 12.00
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "bench", "bench\bench.csproj", "{A61CD222-0F3B-47B6-9F7F-25D658368EEC}"
EndProject
Global
    GlobalSection(SolutionConfigurationPlatforms) = preSolution
        Debug|Any CPU = Debug|Any CPU
        Release|Any CPU = Release|Any CPU
    EndGlobalSection
    GlobalSection(ProjectConfigurationPlatforms) = postSolution
        {A61CD222-0F3B-47B6-9F7F-25D658368EEC}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
        {A61CD222-0F3B-47B6-9F7F-25D658368EEC}.Debug|Any CPU.Build.0 = Debug|Any CPU
        {A61CD222-0F3B-47B6-9F7F-25D658368EEC}.Release|Any CPU.ActiveCfg = Release|Any CPU
        {A61CD222-0F3B-47B6-9F7F-25D658368EEC}.Release|Any CPU.Build.0 = Release|Any CPU
    EndGlobalSection
EndGlobal
"#;

let result = parse_str(SOLUTION);
assert!(result.is_ok());
let solution = result.unwrap();
assert_eq!(solution.projects.len(), 1);
assert_eq!(solution.configurations.len(), 2);
assert_eq!(solution.format, "12.00");

```
*/

#![warn(unused_extern_crates)]
#![allow(clippy::missing_errors_doc)]
use std::fs;

use api::Solution;
use color_eyre::{eyre::Context, Result};
use jwalk::{Parallelism, WalkDir};

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
    #[allow(clippy::no_effect_underscore_binding)]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    #[allow(clippy::cloned_instead_of_copied)]
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::match_same_arms)]
    #[allow(clippy::uninlined_format_args)]
    #[allow(clippy::unused_self)]
    #[allow(clippy::needless_raw_string_hashes)]
    solp
);

/// Consume provides parsed [`Solution`] consumer
pub trait Consume {
    /// Called in case of success parsing
    fn ok(&mut self, solution: &Solution);
    /// Called on error
    fn err(&self, path: &str);
}

/// `parse_file` parses single solution file specified by path..
///
/// # Errors
///
/// This function will return an error if file content cannot be read into memory
/// or solution file has invalid syntax.
pub fn parse_file(path: &str, consumer: &mut dyn Consume) -> Result<()> {
    let contents = fs::read_to_string(path).wrap_err_with(|| {
        consumer.err(path);
        format!("Failed to read content from path: {path}")
    })?;
    let mut solution = parse_str(&contents).wrap_err_with(|| {
        consumer.err(path);
        format!("Failed to parse solution from path: {path}")
    })?;

    solution.path = path;
    consumer.ok(&solution);
    Ok(())
}

/// `parse_str` parses solution content from `&str` and returns [`Solution`] in case of success
///
/// # Errors
///
/// This function will return an error if solution file has invalid syntax or corrupted.
pub fn parse_str(contents: &str) -> Result<Solution> {
    let parsed = parser::parse_str(contents)?;
    Ok(Solution::from(&parsed))
}

/// `parse_dir` parses only directory specified by path.
/// it finds all files with extension specified and parses them.
/// returns the number of scanned solutions
///
/// ## Remarks
/// Any errors occured during parsing of found files will be ignored (so parsing won't stopped)
/// but error paths will be added into error files list (using err function of [`Consume`] trait)
pub fn parse_dir(path: &str, extension: &str, consumer: &mut dyn Consume) -> usize {
    let iter = create_dir_iterator(path).max_depth(1);
    parse_dir_or_tree(iter, extension, consumer)
}

/// `parse_dir_tree` parses directory specified by path. recursively
/// it finds all files with extension specified and parses them.
/// returns the number of scanned solutions
///
/// ## Remarks
/// Any errors occured during parsing of found files will be ignored (so parsing won't stopped)
/// but error paths will be added into error files list (using err function of [`Consume`] trait)
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
        .filter(|p| p.extension().is_some_and(|s| s == ext))
        .map(|f| f.to_str().unwrap_or("").to_string())
        .filter_map(|fp| parse_file(&fp, consumer).ok())
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
