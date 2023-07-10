#![warn(unused_extern_crates)]
#![allow(clippy::missing_errors_doc)]
pub mod info;
pub mod nuget;
pub mod validate;
pub mod ux;

use std::path::{Path, PathBuf};

use crossterm::style::Stylize;
use fnv::FnvHashMap;
use solp::{
    ast::Solution,
    msbuild::{self, Project},
    Consume,
};

#[macro_use]
extern crate prettytable;

pub struct MsbuildProject {
    pub project: Option<msbuild::Project>,
    pub path: PathBuf,
}

fn err(path: &str) {
    eprintln!("Error parsing {} solution", path.red());
}

#[must_use]
pub fn new_projects_paths_map(
    path: &str,
    solution: &Solution,
) -> FnvHashMap<String, MsbuildProject> {
    let dir = Path::new(path).parent().unwrap_or_else(|| Path::new(""));

    solution
        .projects
        .iter()
        .filter_map(|p| {
            if msbuild::is_solution_folder(p.type_id) {
                None
            } else {
                let project_path = make_path(dir, p.path);
                let project = Project::from_path(&project_path).ok();
                Some((
                    p.id.to_uppercase(),
                    MsbuildProject {
                        path: project_path,
                        project,
                    },
                ))
            }
        })
        .collect()
}

#[cfg(not(target_os = "windows"))]
fn make_path(dir: &Path, relative: &str) -> PathBuf {
    // Converts all possible Windows paths into Unix ones
    relative
        .split('\\')
        .fold(PathBuf::from(&dir), |pb, s| pb.join(s))
}

#[cfg(target_os = "windows")]
fn make_path(dir: &Path, relative: &str) -> PathBuf {
    PathBuf::from(&dir).join(relative)
}

#[cfg(test)]
#[cfg(not(target_os = "windows"))]
mod tests {
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
