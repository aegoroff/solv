use std::{
    collections::{BTreeSet, HashMap},
    fmt::{self, Display},
    path::PathBuf,
};

use comfy_table::{Attribute, Cell, Color};
use crossterm::style::Stylize;
use itertools::Itertools;
use solp::{
    cpm::{self, CentralPackages},
    msbuild::{ItemGroup, Package, PackageReference, PackagesConfig},
};

use crate::{Consume, MsbuildProject, error::Collector, ux};

pub struct Nuget {
    show_only_mismatched: bool,
    pub mismatches_found: bool,
    errors: Collector,
}

impl Nuget {
    #[must_use]
    pub fn new(show_only_mismatched: bool) -> Self {
        Self {
            show_only_mismatched,
            mismatches_found: false,
            errors: Collector::new(),
        }
    }
}

/// Package versions found in solution projects. Key is lowercased package name because
/// NuGet package ids are case insensitive. Value is the package name as it's met first
/// and (condition, version) pairs. Condition is optional.
type Nugets<'a> = HashMap<String, (&'a str, Versions<'a>)>;
type Versions<'a> = BTreeSet<(Option<&'a str>, &'a str)>;

fn has_mismatches(versions: &Versions) -> bool {
    versions
        .iter()
        .into_group_map_by(|x| x.0)
        .iter()
        .any(|(_, v)| v.len() > 1)
}

impl Consume for Nuget {
    fn ok(&mut self, solution: &solp::api::Solution) {
        let mut projects = crate::collect_msbuild_projects(solution);
        apply_central_packages(&mut projects);
        let packages_configs = packages_from_packages_configs(&projects);

        let nugets = nugets(&projects, &packages_configs);

        if nugets.is_empty() {
            return;
        }

        let mut table = ux::new_table();

        table.set_header([
            Cell::new("Package").add_attribute(Attribute::Bold),
            Cell::new("Version(s)").add_attribute(Attribute::Bold),
        ]);

        let mut solutions_mismatches = false;
        nugets
            .iter()
            .filter(|(_, (_, versions))| !self.show_only_mismatched || has_mismatches(versions))
            .sorted_unstable_by_key(|(key, _)| *key)
            .for_each(|(_, (pkg, versions))| {
                let grouped = versions.iter().into_group_map_by(|x| x.0);
                let rows = grouped
                    .iter()
                    .sorted_unstable_by_key(|x| x.0)
                    .map(|(c, v)| {
                        let mismatch = v.len() > 1;
                        let comma_separated = v.iter().map(|(_, v)| v).join(", ");
                        let line = match c {
                            Some(c) => format!("{comma_separated} if {c}"),
                            None => comma_separated,
                        };
                        let mut line = Cell::new(line).add_attribute(Attribute::Italic);
                        if mismatch {
                            line = line.fg(Color::Red);
                        }
                        solutions_mismatches |= mismatch;
                        [Cell::new(pkg), line]
                    });
                table.add_rows(rows);
            });

        self.mismatches_found |= solutions_mismatches;

        if self.show_only_mismatched && !solutions_mismatches {
            return;
        }

        ux::print_solution_path(solution.path);
        println!("{table}");
        println!();
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

/// Collects package versions from projects `PackageReference` items and `packages.config` packages
fn nugets<'a>(projects: &'a [MsbuildProject], packages_configs: &'a [Package]) -> Nugets<'a> {
    let from_projects = projects
        .iter()
        .filter_map(|p| p.project.as_ref())
        .filter_map(|p| p.item_group.as_ref())
        .flatten()
        .filter_map(|ig| {
            let condition = ig.condition.as_deref();
            let packs = ig.package_reference.as_ref()?.iter();
            Some(packs.map(move |p| (condition, p.name.as_str(), p.version.as_str())))
        })
        .flatten();
    let from_packages_configs = packages_configs
        .iter()
        .map(|p| (None, p.name.as_str(), p.version.as_str()));

    let mut result = Nugets::new();
    for (condition, name, version) in from_projects.chain(from_packages_configs) {
        // `Update` items or items without name don't define packages
        if name.is_empty() {
            continue;
        }
        result
            .entry(name.to_lowercase())
            .or_insert_with(|| (name, BTreeSet::new()))
            .1
            .insert((condition, version));
    }
    result
}

/// Applies `Directory.Build.props`, `Directory.Packages.props` and `Directory.Build.targets` packages to projects:
/// * `PackageReference` and `GlobalPackageReference` items from these files are added to every project below
/// * `PackageReference Update` items from these files and the project change versions of included packages
///   (props files before project items, `Directory.Build.targets` after them)
/// * `VersionOverride` wins, otherwise if `Version` is not set the version is taken
///   from `PackageVersion` items (Central Package Management)
/// * exact versions are normalized (see [`normalize_version`])
fn apply_central_packages(projects: &mut [MsbuildProject]) {
    // key - Directory.Build.props, Directory.Packages.props, Directory.Build.targets paths
    let mut cache: HashMap<Vec<PathBuf>, CentralPackages> = HashMap::new();
    for mp in projects.iter_mut() {
        let Some(project) = mp.project.as_mut() else {
            continue;
        };
        let central = mp.path.parent().and_then(|dir| {
            let files = cpm::find_implicit_imports(dir);
            if files.is_empty() {
                return None;
            }
            let central = cache
                .entry(files)
                .or_insert_with_key(|files| CentralPackages::load(files));
            Some(&*central)
        });
        if let Some(central) = central {
            let inherited: Vec<_> = central
                .global_references()
                .iter()
                .chain(central.references())
                .cloned()
                .collect();
            if !inherited.is_empty() {
                project.item_group.get_or_insert_default().push(ItemGroup {
                    project_reference: None,
                    package_reference: Some(inherited),
                    condition: None,
                });
            }
        }
        // Update items change already included packages and don't define packages themselves
        let mut own_updates = vec![];
        let groups = project.item_group.iter_mut().flatten();
        for packs in groups.filter_map(|ig| ig.package_reference.as_mut()) {
            let (updates, includes): (Vec<_>, Vec<_>) = std::mem::take(packs)
                .into_iter()
                .partition(|p| p.name.is_empty() && p.update.is_some());
            *packs = includes;
            own_updates.extend(updates);
        }
        let mut packs: Vec<&mut PackageReference> = project
            .item_group
            .iter_mut()
            .flatten()
            .filter_map(|ig| ig.package_reference.as_mut())
            .flatten()
            .collect();
        let updates = central
            .map(CentralPackages::updates_before_project)
            .unwrap_or_default()
            .iter()
            .chain(&own_updates)
            .chain(
                central
                    .map(CentralPackages::updates_after_project)
                    .unwrap_or_default(),
            );
        for update in updates {
            apply_update(&mut packs, update);
        }
        for pack in packs {
            if let Some(version_override) = pack.version_override.take() {
                pack.version = version_override;
            } else if pack.version.is_empty()
                && let Some(version) = central.and_then(|c| c.version(&pack.name))
            {
                version.clone_into(&mut pack.version);
            }
            if let Some(normalized) = normalize_version(&pack.version) {
                pack.version = normalized;
            }
        }
    }
}

/// Applies `PackageReference Update` item version metadata to included packages with names
/// specified (`;` separated list). The last applied update wins like in MSBuild.
fn apply_update(packs: &mut [&mut PackageReference], update: &PackageReference) {
    let names = update
        .update
        .iter()
        .flat_map(|names| names.split(';'))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    let updated = packs.iter_mut().filter(|pack| {
        names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&pack.name))
    });
    for pack in updated {
        if !update.version.is_empty() {
            update.version.clone_into(&mut pack.version);
        }
        if update.version_override.is_some() {
            pack.version_override.clone_from(&update.version_override);
        }
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

/// Packages from `packages.config` files near projects with normalized versions
fn packages_from_packages_configs(projects: &[MsbuildProject]) -> Vec<Package> {
    projects
        .iter()
        .filter_map(|mp| {
            let parent = mp.path.parent()?;
            let packages_config = parent.join("packages.config");
            PackagesConfig::from_path(packages_config).ok()
        })
        .flat_map(|p| p.packages)
        .map(|mut p| {
            if let Some(normalized) = normalize_version(&p.version) {
                p.version = normalized;
            }
            p
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use solp::msbuild::{ItemGroup, PackageReference, Project};
    use test_case::test_case;

    use super::*;

    #[test_case("10.0.1", "13.0.3", true ; "different versions")]
    #[test_case("13.0.3", "13.0.3", false ; "same versions")]
    fn slnx_nuget_mismatches(app_version: &str, lib_version: &str, expected: bool) {
        // Arrange
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solv-slnx-nuget-{uniq}"));
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
        assert_eq!(expected, nuget.mismatches_found);
    }

    #[test_case("", None, "13.0.3" ; "central version")]
    #[test_case("", Some("12.0.1"), "12.0.1" ; "version override")]
    #[test_case("11.0.1", None, "11.0.1" ; "explicit version")]
    fn apply_central_packages_tests(version: &str, version_override: Option<&str>, expected: &str) {
        // Arrange
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solv-cpm-nuget-{uniq}"));
        let project_dir = root.join("src").join("App");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(
            root.join(cpm::BUILD_PROPS),
            r#"<Project>
  <PropertyGroup>
    <Json>13.0.3</Json>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="SonarAnalyzer.CSharp" />
  </ItemGroup>
</Project>"#,
        )
        .unwrap();
        fs::write(
            root.join(cpm::PACKAGES_PROPS),
            r#"<Project>
  <ItemGroup>
    <PackageVersion Include="Newtonsoft.Json" Version="$(Json)" />
    <GlobalPackageReference Include="StyleCop.Analyzers" Version="1.1.118" />
    <PackageVersion Include="SonarAnalyzer.CSharp" Version="10.15.0" />
  </ItemGroup>
</Project>"#,
        )
        .unwrap();
        let mut projects = vec![create_msbuild_project(
            vec![PackageReference {
                name: "newtonsoft.json".to_string(),
                update: None,
                version: version.to_string(),
                version_override: version_override.map(str::to_string),
            }],
            None,
        )];
        projects[0].path = project_dir.join("App.csproj");

        // Act
        apply_central_packages(&mut projects);

        // Assert
        fs::remove_dir_all(&root).unwrap();
        let actual = nugets(&projects, &[]);
        assert_eq!(3, actual.len());
        let json = &actual["newtonsoft.json"].1;
        assert_eq!(1, json.len());
        assert_eq!(expected, json.first().unwrap().1);
        let global = &actual["stylecop.analyzers"].1;
        assert_eq!("1.1.118", global.first().unwrap().1);
        let inherited = &actual["sonaranalyzer.csharp"].1;
        assert_eq!("10.15.0", inherited.first().unwrap().1);
    }

    #[test]
    fn apply_build_props_and_targets_references_without_cpm() {
        // Arrange
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solv-build-props-nuget-{uniq}"));
        let project_dir = root.join("App");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(
            root.join(cpm::BUILD_PROPS),
            r#"<Project>
  <ItemGroup>
    <PackageReference Include="StyleCop.Analyzers" Version="1.1.118" />
  </ItemGroup>
</Project>"#,
        )
        .unwrap();
        fs::write(
            root.join(cpm::BUILD_TARGETS),
            r#"<Project>
  <ItemGroup>
    <PackageReference Include="Roslynator.Analyzers" Version="4.12.0" />
  </ItemGroup>
</Project>"#,
        )
        .unwrap();
        let mut projects = vec![create_msbuild_project(
            vec![PackageReference {
                name: "StyleCop.Analyzers".to_string(),
                update: None,
                version: "1.2.0".to_string(),
                version_override: None,
            }],
            None,
        )];
        projects[0].path = project_dir.join("App.csproj");

        // Act
        apply_central_packages(&mut projects);

        // Assert
        fs::remove_dir_all(&root).unwrap();
        let actual = nugets(&projects, &[]);
        assert!(has_mismatches(&actual["stylecop.analyzers"].1));
        let from_targets = &actual["roslynator.analyzers"].1;
        assert_eq!("4.12.0", from_targets.first().unwrap().1);
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
    fn apply_central_packages_equal_versions_match() {
        // Arrange
        let pack = |version: &str| PackageReference {
            name: "a".to_string(),
            update: None,
            version: version.to_string(),
            version_override: None,
        };
        let mut projects = vec![
            create_msbuild_project(vec![pack("[1.6.2]")], None),
            create_msbuild_project(vec![pack("1.6.2")], None),
            create_msbuild_project(vec![pack("1.6.2.0")], None),
        ];

        // Act
        apply_central_packages(&mut projects);

        // Assert
        let actual = nugets(&projects, &[]);
        let versions = &actual["a"].1;
        assert_eq!(1, versions.len());
        assert_eq!("1.6.2", versions.first().unwrap().1);
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
        let actual = nugets(&projects, &[]);

        // assert
        assert_eq!(4, actual.len());
        let has_mismatches = actual.values().any(|(_, v)| has_mismatches(v));
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
        let actual = nugets(&projects, &[]);

        // assert
        assert_eq!(3, actual.len());
        let has_mismatches = actual.values().any(|(_, v)| has_mismatches(v));
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
        let actual = nugets(&projects, &[]);

        // assert
        assert_eq!(3, actual.len());
        let has_mismatches = actual.values().any(|(_, v)| has_mismatches(v));
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
        let actual = nugets(&projects, &[]);

        // assert
        assert_eq!(3, actual.len());
        let has_mismatches = actual.values().any(|(_, v)| has_mismatches(v));
        assert!(!has_mismatches);
        let different_vers_key = "a".to_owned();
        assert!(actual.contains_key(&different_vers_key));
        assert_eq!(2, actual[&different_vers_key].1.len());
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
        let projects = vec![
            create_msbuild_project(vec![pack(name, "1.0.0")], None),
            create_msbuild_project(vec![pack(other_name, other_version)], None),
        ];

        // Act
        let actual = nugets(&projects, &[]);

        // Assert
        assert_eq!(1, actual.len());
        let (display_name, versions) = &actual["newtonsoft.json"];
        assert_eq!(name, *display_name);
        assert_eq!(expected, has_mismatches(versions));
    }

    #[test_case("12.0.1", true ; "different versions")]
    #[test_case("13.0.3", false ; "same versions")]
    fn nugets_merges_packages_config_with_package_references(config_version: &str, expected: bool) {
        // Arrange
        let projects = vec![create_msbuild_project(
            vec![pack("Newtonsoft.Json", "13.0.3")],
            None,
        )];
        let packages_configs = vec![Package {
            name: "Newtonsoft.Json".to_string(),
            version: config_version.to_string(),
        }];

        // Act
        let actual = nugets(&projects, &packages_configs);

        // Assert
        let versions = &actual["newtonsoft.json"].1;
        assert!(versions.contains(&(None, "13.0.3")));
        assert!(versions.contains(&(None, config_version)));
        assert_eq!(expected, has_mismatches(versions));
    }

    #[test]
    fn nugets_skips_packages_without_name() {
        // Arrange
        let projects = vec![create_msbuild_project(
            vec![pack("", "1.0.0"), pack("a", "1.0.0")],
            None,
        )];

        // Act
        let actual = nugets(&projects, &[]);

        // Assert
        assert_eq!(1, actual.len());
        assert!(actual.contains_key("a"));
    }

    #[test_case(None, "3.0.0" ; "project update")]
    #[test_case(Some(r#"<PackageReference Update="serilog" Version="2.0.0" />"#), "2.0.0" ; "targets update wins")]
    #[test_case(Some(r#"<PackageReference Update="Other;Serilog" VersionOverride="4.0.0" />"#), "4.0.0" ; "targets update list with override")]
    #[test_case(Some(r#"<PackageReference Update="Other" Version="2.0.0" />"#), "3.0.0" ; "targets update of other package")]
    fn apply_central_packages_package_updates(targets_item: Option<&str>, expected: &str) {
        // Arrange
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solv-update-nuget-{uniq}"));
        let project_dir = root.join("App");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(
            root.join(cpm::BUILD_PROPS),
            r#"<Project>
  <ItemGroup>
    <PackageReference Update="Serilog" Version="1.0.0" />
  </ItemGroup>
</Project>"#,
        )
        .unwrap();
        if let Some(item) = targets_item {
            fs::write(
                root.join(cpm::BUILD_TARGETS),
                format!("<Project><ItemGroup>{item}</ItemGroup></Project>"),
            )
            .unwrap();
        }
        let update = PackageReference {
            update: Some("Serilog".to_string()),
            version: "3.0.0".to_string(),
            ..Default::default()
        };
        let mut projects = vec![create_msbuild_project(
            vec![pack("Serilog", ""), update],
            None,
        )];
        projects[0].path = project_dir.join("App.csproj");

        // Act
        apply_central_packages(&mut projects);

        // Assert
        fs::remove_dir_all(&root).unwrap();
        let actual = nugets(&projects, &[]);
        assert_eq!(1, actual.len());
        let versions = &actual["serilog"].1;
        assert_eq!(1, versions.len());
        assert_eq!(expected, versions.first().unwrap().1);
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
    ) -> MsbuildProject {
        MsbuildProject {
            project: Some(Project {
                sdk: Some("5".to_owned()),
                item_group: Some(vec![ItemGroup {
                    project_reference: None,
                    package_reference: Some(packs),
                    condition,
                }]),
                imports: None,
                import_group: None,
            }),
            path: PathBuf::new(),
        }
    }
}
