//! Project files i.e. MSBuild project files on disk referenced by solution projects.

use std::borrow::Cow;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::api::{Project, Solution, SolutionKind};
use crate::msbuild;
use crate::slnx::unescape_xml;

/// Location of the project file referenced by a solution project
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectLocation {
    /// Project file doesn't exist. Contains the path relative to solution directory.
    Missing(PathBuf),
    /// Project file exists
    Found(ProjectFile),
}

/// Existing MSBuild project file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFile {
    path: PathBuf,
}

impl ProjectFile {
    #[cfg(test)]
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Canonical path to the project file
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reads and parses the project file
    pub fn load(&self) -> miette::Result<msbuild::Project> {
        msbuild::Project::from_path(&self.path)
    }

    /// Project file that `ProjectReference` item of this project points to.
    /// `include` is the item path relative to the project directory.
    /// Returns `None` if the file doesn't exist.
    #[must_use]
    pub fn reference(&self, include: &str) -> Option<ProjectFile> {
        let dir = self.path.parent()?;
        let path = canonicalize_ignoring_case(&make_path(dir, include))?;
        Some(ProjectFile { path })
    }
}

/// Locates project files of all solution projects but solution folders, web sites and projects
/// referenced by URI. Project paths are resolved relative to the solution file directory.
pub fn locate<'a>(
    solution: &'a Solution<'a>,
) -> impl Iterator<Item = (&'a Project<'a>, ProjectLocation)> + 'a {
    let dir = Path::new(solution.path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    solution
        .iterate_projects_without_web_sites()
        .filter(|p| !p.path_or_uri.contains("://"))
        .map(move |p| {
            let relative = match solution.kind {
                SolutionKind::Slnx => unescape_xml(p.path_or_uri),
                SolutionKind::Sln => Cow::Borrowed(p.path_or_uri),
            };
            let path = make_path(dir, &relative);
            let location = match canonicalize_ignoring_case(&path) {
                Some(path) => ProjectLocation::Found(ProjectFile { path }),
                None => ProjectLocation::Missing(path),
            };
            (p, location)
        })
}

/// Canonical path of the existing file or directory. Solutions and projects are usually
/// written on Windows where file names are case-insensitive, so if the path doesn't
/// exist as is, its components are matched ignoring case (exact match is preferred).
#[must_use]
fn canonicalize_ignoring_case(path: &Path) -> Option<PathBuf> {
    path.canonicalize()
        .ok()
        .or_else(|| find_ignoring_case(path)?.canonicalize().ok())
}

fn find_ignoring_case(path: &Path) -> Option<PathBuf> {
    let mut found = PathBuf::new();
    for component in path.components() {
        let Component::Normal(name) = component else {
            found.push(component);
            continue;
        };
        let exact = found.join(name);
        if exact.exists() {
            found = exact;
            continue;
        }
        let name = name.to_str()?.to_lowercase();
        let dir = if found.as_os_str().is_empty() {
            Path::new(".")
        } else {
            &found
        };
        let entry = fs::read_dir(dir)
            .ok()?
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|entry_name| entry_name.to_lowercase() == name)
            })?;
        found.push(entry.file_name());
    }
    Some(found)
}

#[cfg(not(target_os = "windows"))]
fn make_path(dir: &Path, relative: &str) -> PathBuf {
    // Converts all possible Windows paths into Unix ones
    relative
        .split('\\')
        .fold(dir.to_path_buf(), |pb, s| pb.join(s))
}

#[cfg(target_os = "windows")]
fn make_path(dir: &Path, relative: &str) -> PathBuf {
    dir.join(relative)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    const PROJECT_TYPE: &str = "{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}";

    fn sln(project_path: &str) -> String {
        format!(
            "\u{FEFF}
Microsoft Visual Studio Solution File, Format Version 12.00
Project(\"{PROJECT_TYPE}\") = \"App\", \"{project_path}\", \"{{11111111-1111-1111-1111-111111111111}}\"
EndProject
"
        )
    }

    fn slnx(project_path: &str) -> String {
        format!("<Solution>\n  <Project Path=\"{project_path}\" />\n</Solution>\n")
    }

    fn missing_paths(solution: &Solution) -> Vec<PathBuf> {
        locate(solution)
            .map(|(_, location)| match location {
                ProjectLocation::Missing(path) => path,
                ProjectLocation::Found(file) => panic!("unexpected found {file:?}"),
            })
            .collect()
    }

    #[cfg(not(target_os = "windows"))]
    #[test_case(r"x\App.csproj", "/base/x/App.csproj" ; "windows separators")]
    #[test_case("x/App.csproj", "/base/x/App.csproj" ; "unix separators")]
    #[test_case("R&amp;D/App.csproj", "/base/R&amp;D/App.csproj" ; "entities kept")]
    fn locate_sln_missing(project_path: &str, expected: &str) {
        // Arrange
        let contents = sln(project_path);
        let mut solution = crate::parse_str(&contents).unwrap();
        solution.path = "/base/a.sln";

        // Act
        let actual = missing_paths(&solution);

        // Assert
        assert_eq!(actual, vec![PathBuf::from(expected)]);
    }

    #[cfg(not(target_os = "windows"))]
    #[test_case("/base/a.slnx" ; "slnx extension")]
    #[test_case("/base/a.sln" ; "slnx content with other extension")]
    fn locate_slnx_unescapes_path(solution_path: &str) {
        // Arrange
        let contents = slnx("R&amp;D/App.csproj");
        let mut solution = crate::parse_str(&contents).unwrap();
        solution.path = solution_path;

        // Act
        let actual = missing_paths(&solution);

        // Assert
        assert_eq!(actual, vec![PathBuf::from("/base/R&D/App.csproj")]);
    }

    #[test]
    fn locate_skips_uri() {
        // Arrange
        let contents = sln("http://localhost/App.csproj");
        let mut solution = crate::parse_str(&contents).unwrap();
        solution.path = "/base/a.sln";

        // Act
        let actual = locate(&solution).count();

        // Assert
        assert_eq!(actual, 0);
    }

    #[test]
    fn locate_found_loads_project() {
        // Arrange
        let dir = std::env::temp_dir().join(format!("solp_project_files_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("App")).unwrap();
        fs::write(
            dir.join("App").join("App.csproj"),
            r#"<Project Sdk="Microsoft.NET.Sdk"></Project>"#,
        )
        .unwrap();
        let solution_path = dir.join("a.slnx");
        let solution_path = solution_path.to_str().unwrap();
        let contents = slnx("App/App.csproj");
        let mut solution = crate::parse_str(&contents).unwrap();
        solution.path = solution_path;

        // Act
        let located: Vec<_> = locate(&solution).collect();

        // Assert
        let expected = dir.join("App").join("App.csproj").canonicalize().unwrap();
        let result = match &located[..] {
            [(_, ProjectLocation::Found(file))] => Some((file.path().to_path_buf(), file.load())),
            _ => None,
        };
        let _ = fs::remove_dir_all(&dir);
        let (path, project) = result.unwrap();
        assert_eq!(path, expected);
        assert!(project.unwrap().is_sdk_project());
    }

    #[test]
    fn locate_found_unparsable_project() {
        // Arrange
        let dir =
            std::env::temp_dir().join(format!("solp_project_files_bad_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("App.csproj"), "not xml").unwrap();
        let solution_path = dir.join("a.slnx");
        let solution_path = solution_path.to_str().unwrap();
        let contents = slnx("App.csproj");
        let mut solution = crate::parse_str(&contents).unwrap();
        solution.path = solution_path;

        // Act
        let loaded: Vec<_> = locate(&solution)
            .map(|(_, location)| match location {
                ProjectLocation::Found(file) => file.load().is_ok(),
                ProjectLocation::Missing(_) => true,
            })
            .collect();

        // Assert
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(loaded, vec![false]);
    }

    #[test]
    fn locate_found_ignoring_case() {
        // Arrange
        let dir =
            std::env::temp_dir().join(format!("solp_project_files_case_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Src").join("App")).unwrap();
        fs::create_dir_all(dir.join("Tools")).unwrap();
        fs::write(dir.join("Src").join("App").join("App.csproj"), "").unwrap();
        let solution_path = dir.join("Tools").join("a.sln");
        let solution_path = solution_path.to_str().unwrap();
        let contents = sln(r"..\src\app\APP.csproj");
        let mut solution = crate::parse_str(&contents).unwrap();
        solution.path = solution_path;

        // Act
        let located: Vec<_> = locate(&solution).map(|(_, location)| location).collect();

        // Assert
        let expected = dir
            .join("Src")
            .join("App")
            .join("App.csproj")
            .canonicalize();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(
            located,
            vec![ProjectLocation::Found(ProjectFile::new(expected.unwrap()))]
        );
    }

    #[test_case(r"..\lib\LIB.csproj", true ; "relative windows path ignoring case")]
    #[test_case("../Lib/Lib.csproj", true ; "relative unix path")]
    #[test_case(r"..\Lib\Missing.csproj", false ; "missing file")]
    fn reference_is_resolved_relative_to_project(include: &str, expected_found: bool) {
        // Arrange
        static CASE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let case = CASE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "solp_project_files_reference_{}_{case}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("App")).unwrap();
        fs::create_dir_all(dir.join("Lib")).unwrap();
        fs::write(dir.join("App").join("App.csproj"), "").unwrap();
        fs::write(dir.join("Lib").join("Lib.csproj"), "").unwrap();
        let file = ProjectFile::new(dir.join("App").join("App.csproj").canonicalize().unwrap());

        // Act
        let actual = file.reference(include);

        // Assert
        let expected = dir.join("Lib").join("Lib.csproj").canonicalize().unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(actual, expected_found.then(|| ProjectFile::new(expected)));
    }

    #[test]
    fn canonicalize_ignoring_case_missing() {
        // Arrange
        let dir = std::env::temp_dir().join(format!(
            "solp_project_files_case_missing_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("App")).unwrap();
        let path = dir.join("app").join("App.csproj");

        // Act
        let actual = canonicalize_ignoring_case(&path);

        // Assert
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(actual, None);
    }
}
