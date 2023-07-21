#![warn(unused_extern_crates)]
#![allow(clippy::missing_errors_doc)]
pub mod error;
pub mod info;
pub mod nuget;
pub mod ux;
pub mod validate;

use solp::Consume;
use std::path::{Path, PathBuf};
use url::Url;

#[macro_use]
extern crate prettytable;

#[must_use]
pub fn parent_of(path: &str) -> &Path {
    Path::new(path).parent().unwrap_or_else(|| Path::new(""))
}

#[must_use]
pub fn try_make_local_path(dir: &Path, relative: &str) -> Option<PathBuf> {
    // We dont need Uri so if parsed successfully throw it away
    if Url::parse(relative).is_ok() {
        None
    } else {
        Some(make_path(dir, relative))
    }
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

#[must_use]
pub fn calculate_percent(value: i32, total: i32) -> f64 {
    if total == 0 {
        0_f64
    } else {
        (f64::from(value) / f64::from(total)) * 100_f64
    }
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

    #[rstest]
    #[case("/base", "x", Some(PathBuf::from("/base/x")))]
    #[case("/base", "http://localhost/a.csproj", None)]
    #[trace]
    fn try_make_local_path_tests(
        #[case] base: &str,
        #[case] path: &str,
        #[case] expected: Option<PathBuf>,
    ) {
        // Arrange
        let d = Path::new(base);

        // Act
        let actual = try_make_local_path(d, path);

        // Assert
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case(1, 100, 1.0)]
    #[case(0, 100, 0.0)]
    #[case(100, 100, 100.0)]
    #[case(50, 100, 50.0)]
    #[case(20, 100, 20.0)]
    #[trace]
    fn calculate_percent_tests(#[case] value: i32, #[case] total: i32, #[case] expected: f64) {
        // Arrange

        // Act
        let actual = calculate_percent(value, total);

        // Assert
        assert_eq!(actual, expected);
    }
}
