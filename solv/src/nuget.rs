use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Display},
};

use comfy_table::{Attribute, Cell, Color};
use crossterm::style::Stylize;
use itertools::Itertools;
use solp::{
    api::Solution,
    cpm::{PackageResolver, ProjectPackage},
    msbuild::{Package, PackagesConfig},
    project_files::{self, ProjectFile},
};

use crate::{Consume, error::Collector, ux};

pub struct Nuget {
    show_only_mismatched: bool,
    errors: Collector,
    mismatches_found: bool,
}

impl Nuget {
    #[must_use]
    pub fn new(show_only_mismatched: bool) -> Self {
        Self {
            show_only_mismatched,
            errors: Collector::new(),
            mismatches_found: false,
        }
    }

    /// Whether any solution references different versions of the same package
    #[must_use]
    pub fn mismatches_found(&self) -> bool {
        self.mismatches_found
    }

    /// Collects solution packages and returns the report to show if any
    fn report(&mut self, solution: &Solution) -> Option<SolutionPackages> {
        let (packages, packages_configs) = load_packages(solution);
        let versions = package_versions(&packages, &packages_configs);

        if versions.is_empty() {
            return None;
        }

        let report = SolutionPackages::new(solution.path, versions, self.show_only_mismatched);
        let has_mismatches = report.has_mismatches();
        self.mismatches_found |= has_mismatches;
        if self.show_only_mismatched && !has_mismatches {
            return None;
        }
        Some(report)
    }
}

/// Packages of a solution sorted by name
struct SolutionPackages {
    path: String,
    packages: Vec<PackageVersions>,
}

/// Versions of a package found in solution projects
#[derive(Debug, PartialEq, Eq)]
struct PackageVersions {
    /// Package name as it's met first (NuGet package ids are case insensitive)
    name: String,
    /// Versions grouped by condition (sorted, no condition first)
    groups: Vec<ConditionalVersions>,
}

/// Distinct sorted package versions referenced under the same condition
#[derive(Debug, PartialEq, Eq)]
struct ConditionalVersions {
    condition: Option<String>,
    versions: Vec<String>,
}

impl PackageVersions {
    /// Whether different versions of the package are referenced under the same condition
    fn has_mismatches(&self) -> bool {
        self.groups.iter().any(|group| group.versions.len() > 1)
    }
}

impl SolutionPackages {
    fn new(path: &str, packages: Vec<PackageVersions>, show_only_mismatched: bool) -> Self {
        let packages = packages
            .into_iter()
            .filter(|package| !show_only_mismatched || package.has_mismatches())
            .collect();
        Self {
            path: path.to_owned(),
            packages,
        }
    }

    fn has_mismatches(&self) -> bool {
        self.packages.iter().any(PackageVersions::has_mismatches)
    }
}

impl Display for SolutionPackages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = ux::new_table();
        table.set_header([
            Cell::new("Package").add_attribute(Attribute::Bold),
            Cell::new("Version(s)").add_attribute(Attribute::Bold),
        ]);
        for package in &self.packages {
            for group in &package.groups {
                let comma_separated = group.versions.join(", ");
                let line = match &group.condition {
                    Some(c) => format!("{comma_separated} if {c}"),
                    None => comma_separated,
                };
                let mut line = Cell::new(line).add_attribute(Attribute::Italic);
                if group.versions.len() > 1 {
                    line = line.fg(Color::Red);
                }
                table.add_row([Cell::new(&package.name), line]);
            }
        }
        ux::write_solution_path(f, &self.path)?;
        writeln!(f, "{table}")?;
        writeln!(f)
    }
}

impl Consume for Nuget {
    fn ok(&mut self, solution: &Solution) {
        if let Some(report) = self.report(solution) {
            print!("{report}");
        }
    }

    fn err(&mut self, path: &str) {
        self.errors.add_path(path);
    }
}

impl Display for Nuget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mismatches_found && !self.show_only_mismatched {
            writeln!(
                f,
                " {}",
                "Solutions with nuget packages inconsistency found"
                    .dark_red()
                    .bold()
            )?;
            writeln!(f)?;
        }
        if self.errors.count() > 0 {
            write!(f, "{}", self.errors)
        } else {
            Ok(())
        }
    }
}

/// Sorted distinct (condition, version) pairs of a package
type ConditionVersionPairs<'a> = BTreeSet<(Option<&'a str>, &'a str)>;

/// Collects versions of project packages and `packages.config` packages
/// sorted by package name ignoring case
fn package_versions(
    packages: &[ProjectPackage],
    packages_configs: &[Package],
) -> Vec<PackageVersions> {
    let from_projects = packages
        .iter()
        .map(|p| (p.condition.as_deref(), p.name.as_str(), p.version.as_str()));
    let from_packages_configs = packages_configs
        .iter()
        .map(|p| (None, p.name.as_str(), p.version.as_str()));

    // key is lowercased package name, value is the name as it's met first and its versions
    let mut by_name: BTreeMap<String, (&str, ConditionVersionPairs)> = BTreeMap::new();
    for (condition, name, version) in from_projects.chain(from_packages_configs) {
        // `Update` items or items without name don't define packages
        if name.is_empty() {
            continue;
        }
        by_name
            .entry(name.to_lowercase())
            .or_insert_with(|| (name, BTreeSet::new()))
            .1
            .insert((condition, version));
    }
    by_name
        .into_values()
        .map(|(name, versions)| {
            let mut groups: Vec<ConditionalVersions> = vec![];
            for (condition, version) in versions {
                match groups.last_mut() {
                    Some(group) if group.condition.as_deref() == condition => {
                        group.versions.push(version.to_owned());
                    }
                    _ => groups.push(ConditionalVersions {
                        condition: condition.map(str::to_owned),
                        versions: vec![version.to_owned()],
                    }),
                }
            }
            PackageVersions {
                name: name.to_owned(),
                groups,
            }
        })
        .collect()
}

/// Loads packages of all solution projects found on disk and `packages.config` packages near
/// them with normalized versions. Not parsable projects are skipped.
fn load_packages(solution: &Solution) -> (Vec<ProjectPackage>, Vec<Package>) {
    let mut resolver = PackageResolver::default();
    let mut packages = vec![];
    let mut packages_configs = vec![];
    for file in project_files::found_files(solution) {
        let Ok(project_packages) = resolver.packages(&file) else {
            continue;
        };
        packages.extend(normalize_versions(project_packages));
        packages_configs.extend(packages_config(&file));
    }
    (packages, packages_configs)
}

fn normalize_versions(mut packages: Vec<ProjectPackage>) -> Vec<ProjectPackage> {
    for pack in &mut packages {
        normalize_version_in_place(&mut pack.version);
    }
    packages
}

fn normalize_version_in_place(version: &mut String) {
    if let Some(normalized) = normalize_version(version) {
        *version = normalized;
    }
}

/// Normalizes version like NuGet does so equal versions are displayed the same way:
/// * exact version range `[1.6.2]` becomes `1.6.2`
/// * `1.6` becomes `1.6.0`, `1.6.2.0` becomes `1.6.2`, `01.6.2` becomes `1.6.2`
/// * build metadata (`+abc`) is removed, prerelease label is kept
///
/// Other ranges (e.g. `[1.0,2.0)`), floating versions (`1.*`) and anything that
/// isn't a version are kept as is (only trimmed). Returns `None` if nothing changed.
fn normalize_version(version: &str) -> Option<String> {
    let trimmed = version.trim();
    let exact = trimmed
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .filter(|v| !v.contains(','))
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let candidate = exact.unwrap_or(trimmed);
    let normalized = normalize_semver(candidate)
        .or_else(|| exact.map(str::to_owned))
        .unwrap_or_else(|| trimmed.to_owned());
    (normalized != version).then_some(normalized)
}

/// Normalizes `major[.minor[.patch[.revision]]][-prerelease][+metadata]`.
/// Returns `None` if the string isn't such a version.
fn normalize_semver(version: &str) -> Option<String> {
    let without_metadata = version.split_once('+').map_or(version, |(v, _)| v);
    let (numbers, prerelease) = match without_metadata.split_once('-') {
        Some((n, p)) if !p.is_empty() => (n, Some(p)),
        Some(_) => return None,
        None => (without_metadata, None),
    };
    let mut parts = numbers
        .split('.')
        .map(|p| {
            p.bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| p.parse::<u64>().ok())
                .flatten()
        })
        .collect::<Option<Vec<u64>>>()?;
    if parts.len() > 4 {
        return None;
    }
    parts.resize(parts.len().max(3), 0);
    if parts.len() == 4 && parts[3] == 0 {
        parts.pop();
    }
    let mut result = parts.iter().join(".");
    if let Some(prerelease) = prerelease {
        result.push('-');
        result.push_str(prerelease);
    }
    Some(result)
}

/// Packages from `packages.config` file near the project with normalized versions
fn packages_config(file: &ProjectFile) -> Vec<Package> {
    let Some(packages_config) = file.path().parent().map(|dir| dir.join("packages.config")) else {
        return vec![];
    };
    let Ok(config) = PackagesConfig::from_path(packages_config) else {
        return vec![];
    };
    config
        .packages
        .into_iter()
        .map(|mut p| {
            normalize_version_in_place(&mut p.version);
            p
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use solp::msbuild::PackageReference;
    use test_case::test_case;

    use super::*;

    #[test_case("10.0.1", "13.0.3", true ; "different versions")]
    #[test_case("13.0.3", "13.0.3", false ; "same versions")]
    fn slnx_nuget_mismatches(app_version: &str, lib_version: &str, expected: bool) {
        // Arrange
        // cases run in parallel and the clock may be too coarse to tell them apart
        static CASE: AtomicUsize = AtomicUsize::new(0);
        let case = CASE.fetch_add(1, Ordering::Relaxed);
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solv-slnx-nuget-{uniq}-{case}"));
        for (name, version) in [("App", app_version), ("Lib", lib_version)] {
            fs::create_dir_all(root.join(name)).unwrap();
            fs::write(
                root.join(name).join(format!("{name}.csproj")),
                format!(
                    r#"<Project Sdk="Microsoft.NET.Sdk">
  <ItemGroup>
    <PackageReference Include="Newtonsoft.Json" Version="{version}" />
  </ItemGroup>
</Project>"#
                ),
            )
            .unwrap();
        }
        let slnx_path = root.join("test.slnx");
        fs::write(
            &slnx_path,
            r#"<Solution>
  <Folder Name="/src/">
    <Project Path="App/App.csproj" />
  </Folder>
  <Project Path="Lib\Lib.csproj" />
</Solution>"#,
        )
        .unwrap();
        let mut nuget = Nuget::new(false);

        // Act
        let result = solp::parse_file(slnx_path.to_str().unwrap(), &mut nuget);

        // Assert
        fs::remove_dir_all(&root).unwrap();
        assert!(result.is_ok());
        assert_eq!(expected, nuget.mismatches_found());
        let actual = nuget.to_string();
        assert_eq!(
            expected,
            actual.contains("Solutions with nuget packages inconsistency found")
        );
    }

    #[test_case("1.6.2", None ; "plain")]
    #[test_case("[1.6.2]", Some("1.6.2") ; "exact")]
    #[test_case(" [ 1.6.2 ] ", Some("1.6.2") ; "exact with spaces")]
    #[test_case(" 1.6.2 ", Some("1.6.2") ; "plain with spaces")]
    #[test_case("1.6.2.0", Some("1.6.2") ; "zero revision")]
    #[test_case("[1.6.2.0]", Some("1.6.2") ; "exact zero revision")]
    #[test_case("1.6.2.1", None ; "non zero revision")]
    #[test_case("1.6", Some("1.6.0") ; "two parts")]
    #[test_case("1", Some("1.0.0") ; "one part")]
    #[test_case("01.06.002", Some("1.6.2") ; "leading zeros")]
    #[test_case("1.6.2-beta.1", None ; "prerelease")]
    #[test_case("1.6.2.0-beta.1", Some("1.6.2-beta.1") ; "prerelease zero revision")]
    #[test_case("1.6.2+sha.abc", Some("1.6.2") ; "metadata")]
    #[test_case("1.6.2-rc+sha.abc", Some("1.6.2-rc") ; "prerelease metadata")]
    #[test_case("1.6.*", None ; "floating")]
    #[test_case("1.6.2.0.0", None ; "too many parts")]
    #[test_case("1.6.2-", None ; "empty prerelease")]
    #[test_case("$(Unknown)", None ; "unknown property")]
    #[test_case("[$(Unknown)]", Some("$(Unknown)") ; "exact unknown property")]
    #[test_case("[1.0,2.0)", None ; "range")]
    #[test_case("[1.0,]", None ; "open range")]
    #[test_case("[]", None ; "empty brackets")]
    #[test_case("", None ; "empty")]
    fn normalize_version_tests(version: &str, expected: Option<&str>) {
        // Act
        let actual = normalize_version(version);

        // Assert
        assert_eq!(expected, actual.as_deref());
    }

    #[test]
    fn normalize_versions_equal_versions_match() {
        // Arrange
        let pack = |version: &str| PackageReference {
            name: "a".to_string(),
            update: None,
            version: version.to_string(),
            version_override: None,
        };
        let packages = [
            create_msbuild_project(vec![pack("[1.6.2]")], None),
            create_msbuild_project(vec![pack("1.6.2")], None),
            create_msbuild_project(vec![pack("1.6.2.0")], None),
        ]
        .concat();

        // Act
        let packages = normalize_versions(packages);

        // Assert
        let actual = package_versions(&packages, &[]);
        let expected = ConditionalVersions {
            condition: None,
            versions: vec!["1.6.2".to_owned()],
        };
        assert_eq!(vec![expected], actual[0].groups);
    }
    #[test]
    fn nugets_no_mismatches() {
        // arrange
        let mut projects = vec![];
        let packs1 = vec![
            PackageReference {
                name: "a".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
            PackageReference {
                name: "b".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
        ];
        let packs2 = vec![
            PackageReference {
                name: "c".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
            PackageReference {
                name: "d".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
        ];
        projects.push(create_msbuild_project(packs1, None));
        projects.push(create_msbuild_project(packs2, None));

        // act
        let packages = projects.concat();
        let actual = package_versions(&packages, &[]);

        // assert
        assert_eq!(4, actual.len());
        let has_mismatches = actual.iter().any(PackageVersions::has_mismatches);
        assert!(!has_mismatches);
    }

    #[test]
    fn nugets_no_mismatches_same_pgk_in_different_projects() {
        // arrange
        let mut projects = vec![];
        let packs1 = vec![
            PackageReference {
                name: "a".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
            PackageReference {
                name: "b".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
        ];
        let packs2 = vec![
            PackageReference {
                name: "c".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
            PackageReference {
                name: "a".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
        ];
        projects.push(create_msbuild_project(packs1, None));
        projects.push(create_msbuild_project(packs2, None));

        // act
        let packages = projects.concat();
        let actual = package_versions(&packages, &[]);

        // assert
        assert_eq!(3, actual.len());
        let has_mismatches = actual.iter().any(PackageVersions::has_mismatches);
        assert!(!has_mismatches);
    }

    #[test]
    fn nugets_has_mismatches() {
        // arrange
        let mut projects = vec![];
        let packs1 = vec![
            PackageReference {
                name: "a".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
            PackageReference {
                name: "b".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
        ];
        let packs2 = vec![
            PackageReference {
                name: "c".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
            PackageReference {
                name: "a".to_string(),
                update: None,
                version: "2.0.0".to_string(),
                version_override: None,
            },
        ];
        projects.push(create_msbuild_project(packs1, None));
        projects.push(create_msbuild_project(packs2, None));

        // act
        let packages = projects.concat();
        let actual = package_versions(&packages, &[]);

        // assert
        assert_eq!(3, actual.len());
        let has_mismatches = actual.iter().any(PackageVersions::has_mismatches);
        assert!(has_mismatches);
    }

    #[test]
    fn nugets_no_mismatches_by_conditions() {
        // arrange
        let mut projects = vec![];
        let packs1 = vec![
            PackageReference {
                name: "a".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
            PackageReference {
                name: "b".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
        ];
        let packs2 = vec![
            PackageReference {
                name: "c".to_string(),
                update: None,
                version: "1.0.0".to_string(),
                version_override: None,
            },
            PackageReference {
                name: "a".to_string(),
                update: None,
                version: "2.0.0".to_string(),
                version_override: None,
            },
        ];
        projects.push(create_msbuild_project(packs1, None));
        projects.push(create_msbuild_project(packs2, Some("1".to_owned())));

        // act
        let packages = projects.concat();
        let actual = package_versions(&packages, &[]);

        // assert
        assert_eq!(3, actual.len());
        let has_mismatches = actual.iter().any(PackageVersions::has_mismatches);
        assert!(!has_mismatches);
        assert_eq!("a", actual[0].name);
        assert_eq!(2, actual[0].groups.len());
    }

    #[test_case("Newtonsoft.Json", "newtonsoft.json", "2.0.0", true ; "different case different versions")]
    #[test_case("Newtonsoft.Json", "NEWTONSOFT.JSON", "1.0.0", false ; "different case same versions")]
    fn nugets_package_names_case_insensitive(
        name: &str,
        other_name: &str,
        other_version: &str,
        expected: bool,
    ) {
        // Arrange
        let projects = [
            create_msbuild_project(vec![pack(name, "1.0.0")], None),
            create_msbuild_project(vec![pack(other_name, other_version)], None),
        ];

        // Act
        let packages = projects.concat();
        let actual = package_versions(&packages, &[]);

        // Assert
        assert_eq!(1, actual.len());
        assert_eq!(name, actual[0].name);
        assert_eq!(expected, actual[0].has_mismatches());
    }

    #[test_case("12.0.1", true ; "different versions")]
    #[test_case("13.0.3", false ; "same versions")]
    fn nugets_merges_packages_config_with_package_references(config_version: &str, expected: bool) {
        // Arrange
        let projects = [create_msbuild_project(
            vec![pack("Newtonsoft.Json", "13.0.3")],
            None,
        )];
        let packages_configs = vec![Package {
            name: "Newtonsoft.Json".to_string(),
            version: config_version.to_string(),
        }];

        // Act
        let packages = projects.concat();
        let actual = package_versions(&packages, &packages_configs);

        // Assert
        let versions = &actual[0].groups[0].versions;
        assert!(versions.iter().any(|v| v == "13.0.3"));
        assert!(versions.iter().any(|v| v == config_version));
        assert_eq!(expected, actual[0].has_mismatches());
    }

    #[test]
    fn nugets_skips_packages_without_name() {
        // Arrange
        let projects = [create_msbuild_project(
            vec![pack("", "1.0.0"), pack("a", "1.0.0")],
            None,
        )];

        // Act
        let packages = projects.concat();
        let actual = package_versions(&packages, &[]);

        // Assert
        assert_eq!(1, actual.len());
        assert_eq!("a", actual[0].name);
    }

    fn pack(name: &str, version: &str) -> PackageReference {
        PackageReference {
            name: name.to_string(),
            version: version.to_string(),
            ..Default::default()
        }
    }

    fn create_msbuild_project(
        packs: Vec<PackageReference>,
        condition: Option<String>,
    ) -> Vec<ProjectPackage> {
        packs
            .into_iter()
            .map(|p| ProjectPackage {
                name: p.name,
                version: p.version,
                condition: condition.clone(),
            })
            .collect()
    }
}
