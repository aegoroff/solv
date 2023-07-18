#![warn(unused_extern_crates)]
#![allow(clippy::missing_errors_doc)]
pub mod info;
pub mod nuget;
pub mod ux;
pub mod validate;

use std::path::{Path, PathBuf};

use crossterm::style::Stylize;
use solp::Consume;

#[macro_use]
extern crate prettytable;

fn err(path: &str) {
    eprintln!("Error parsing {} solution", path.red());
}

#[must_use]
pub fn parent_of(path: &str) -> &Path {
    Path::new(path).parent().unwrap_or_else(|| Path::new(""))
}

#[must_use]
#[cfg(not(target_os = "windows"))]
pub fn make_path(dir: &Path, relative: &str) -> PathBuf {
    // Converts all possible Windows paths into Unix ones
    relative
        .split('\\')
        .fold(PathBuf::from(&dir), |pb, s| pb.join(s))
}

#[must_use]
#[cfg(target_os = "windows")]
fn make_path(dir: &Path, relative: &str) -> PathBuf {
    PathBuf::from(&dir).join(relative)
}

#[cfg(test)]
#[cfg(not(target_os = "windows"))]
pub mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("/base", "x", "/base/x")]
    #[case("/base", r"x\y", "/base/x/y")]
    #[case("/base", "x/y", "/base/x/y")]
    #[trace]
    fn make_path_tests(#[case] base: &str, #[case] path: &str, #[case] expected: &str) {
        // Arrange
        let d = Path::new(base);

        // Act
        let actual = make_path(d, path);

        // Assert
        assert_eq!(actual.to_str().unwrap(), expected);
    }
}
