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
use jwalk::{Parallelism, WalkDir};
use miette::{Context, IntoDiagnostic};

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

/// Parses a solution file at the specified path and notifies the consumer of the result.
///
/// This function reads the content of the file at the given path and attempts to parse it
/// as a Microsoft Visual Studio solution file. If the file is successfully read and parsed,
/// the consumer's `ok` method is called with the parsed `Solution`. If any errors occur during
/// reading or parsing, the consumer's `err` method is called with the path of the file, and an
/// error is returned.
///
/// # Parameters
///
/// - `path`: A string slice that holds the path to the solution file.
/// - `consumer`: A mutable reference to an object that implements the `Consume` trait. This consumer
///   will be notified of the result of the parse operation.
///
/// # Returns
///
/// A `Result` which is `Ok(())` if the file was successfully read and parsed, or an error if any
/// issues occurred during reading or parsing.
///
/// # Errors
///
/// This function will return an error if the file cannot be read or if the content cannot be parsed
/// as a valid solution file. In both cases, the consumer's `err` method will be called with the path
/// of the file.
///
/// # Example
///
/// ```rust
/// use solp::parse_file;
/// use solp::api::Solution;
/// use solp::Consume;
///
/// struct Consumer;
///
/// impl Consume for Consumer {
///   fn ok(&mut self, solution: &Solution) {
///      // ...
///   }
///
///   fn err(&self, path: &str) {
///      // ...
///   }
/// }
///
/// let path = "path/to/solution.sln";
/// let mut consumer = Consumer{};
/// match parse_file(path, &mut consumer) {
///     Ok(()) => println!("Successfully parsed the solution file."),
///     Err(e) => eprintln!("Failed to parse the solution file: {:?}", e),
/// }
/// ```
pub fn parse_file(path: &str, consumer: &mut dyn Consume) -> miette::Result<()> {
    let contents = fs::read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| {
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

/// Parses a solution file content from a string slice and returns a [`Solution`] object.
///
/// This function takes the content of a solution file as a string slice, attempts to parse it,
/// and returns a `Solution` object representing the parsed content. If parsing fails, an error
/// is returned.
///
/// # Parameters
///
/// - `contents`: A string slice that holds the content of the solution file to be parsed.
///
/// # Returns
///
/// A `Result` containing a [`Solution`] object if parsing is successful, or an error if parsing fails.
///
/// # Errors
///
/// This function will return an error if the content cannot be parsed as a valid solution file.
///
/// # Example
///
/// ```rust
/// use solp::parse_str;
///
/// let solution_content = r#"
/// Microsoft Visual Studio Solution File, Format Version 12.00
/// # Visual Studio 16
/// VisualStudioVersion = 16.0.28701.123
/// MinimumVisualStudioVersion = 10.0.40219.1
/// Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "MyProject", "MyProject.csproj", "{A61CD222-0F3B-47B6-9F7F-25D658368EEC}"
/// EndProject
/// Global
///     GlobalSection(SolutionConfigurationPlatforms) = preSolution
///         Debug|Any CPU = Debug|Any CPU
///         Release|Any CPU = Release|Any CPU
///     EndGlobalSection
///     GlobalSection(ProjectConfigurationPlatforms) = postSolution
///         {A61CD222-0F3B-47B6-9F7F-25D658368EEC}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
///         {A61CD222-0F3B-47B6-9F7F-25D658368EEC}.Debug|Any CPU.Build.0 = Debug|Any CPU
///         {A61CD222-0F3B-47B6-9F7F-25D658368EEC}.Release|Any CPU.ActiveCfg = Release|Any CPU
///         {A61CD222-0F3B-47B6-9F7F-25D658368EEC}.Release|Any CPU.Build.0 = Release|Any CPU
///     EndGlobalSection
/// EndGlobal
/// "#;
///
/// parse_str(solution_content);
/// // This will return a Result containing a Solution object if parsing is successful.
/// ```
///
/// # Remarks
///
/// This function uses the `parser::parse_str` function to perform the actual parsing and then
/// constructs a [`Solution`] object from the parsed data.
pub fn parse_str(contents: &str) -> miette::Result<Solution> {
    let parsed = parser::parse_str(contents)?;
    Ok(Solution::from(&parsed))
}

/// `parse_dir` parses only directory specified by path.
/// it finds all files with extension specified and parses them.
/// returns the number of scanned solutions
///
/// ## Remarks
/// Any errors occurred during parsing of found files will be ignored (so parsing won't stopped)
/// but error paths will be added into error files list (using err function of [`Consume`] trait)
pub fn parse_dir(
    path: &str,
    extension: &str,
    consumer: &mut dyn Consume,
    show_errors: bool,
) -> usize {
    let iter = create_dir_iterator(path).max_depth(1);
    parse_dir_or_tree(iter, extension, consumer, show_errors)
}

/// `parse_dir_tree` parses directory specified by path. recursively
/// it finds all files with extension specified and parses them.
/// returns the number of scanned solutions
///
/// ## Remarks
/// Any errors occurred during parsing of found files will be ignored (so parsing won't stopped)
/// but error paths will be added into error files list (using err function of [`Consume`] trait)
pub fn parse_dir_tree(
    path: &str,
    extension: &str,
    consumer: &mut dyn Consume,
    show_errors: bool,
) -> usize {
    let parallelism = Parallelism::RayonNewPool(num_cpus::get_physical());
    let iter = create_dir_iterator(path).parallelism(parallelism);
    parse_dir_or_tree(iter, extension, consumer, show_errors)
}

fn create_dir_iterator(path: &str) -> WalkDir {
    let root = decorate_path(path);
    WalkDir::new(root).skip_hidden(false).follow_links(false)
}

/// Parses the directory or directory tree and processes files with the specified extension.
///
/// This function takes an iterator over directory entries (`WalkDir`), a file extension to filter by,
/// and a consumer that implements the `Consume` trait. It filters the directory entries to only include
/// files with the specified extension, attempts to parse each file, and counts how many files were
/// successfully parsed.
///
/// # Parameters
///
/// - `iter`: An iterator over directory entries (`WalkDir`). This can be configured to either walk a
///   single directory or recursively walk a directory tree.
/// - `extension`: The file extension to filter by. Files must have this extension to be processed.
/// - `consumer`: A mutable reference to an object that implements the `Consume` trait. This consumer
///   will be notified of successful and failed parse attempts.
/// - `show_errors`: Whether to show parsing errors during scan.
///
/// # Returns
///
/// The number of files that were successfully parsed.
///
/// # Remarks
///
/// Any errors that occur during the parsing of files will be ignored, but the paths of the files that
/// caused errors will be added to the error files list using the `err` function of the `Consume` trait.
fn parse_dir_or_tree(
    iter: WalkDir,
    extension: &str,
    consumer: &mut dyn Consume,
    show_errors: bool,
) -> usize {
    let ext = extension.trim_start_matches('.');

    iter.into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|f| f.file_type().is_file())
        .map(|f| f.path())
        .filter(|p| p.extension().is_some_and(|s| s == ext))
        .filter_map(|fp| {
            if let Some(p) = fp.to_str() {
                if let Err(e) = parse_file(p, consumer) {
                    if show_errors {
                        println!("{e:?}");
                    }
                    None
                } else {
                    Some(())
                }
            } else {
                None
            }
        })
        .count()
}

/// On Windows trailing backslash (\) to be added if volume and colon passed (like c:).
/// It needed paths look to be more pleasant
#[cfg(target_os = "windows")]
fn decorate_path(path: &str) -> String {
    if path.len() == 2 && path.ends_with(':') {
        format!("{path}\\")
    } else {
        path.to_owned()
    }
}

/// On Unix just pass-through as is
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
