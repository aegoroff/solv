use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    fmt::{self, Display},
    path::PathBuf,
};

use comfy_table::{Attribute, Cell, Color};
use crossterm::style::Stylize;
use itertools::Itertools;
use solp::{
    api::Solution,
    msbuild::{self, PackagesConfig, Project},
};

use crate::{Consume, error::Collector, ux};

pub struct Nuget {
    show_only_mismatched: bool,
    pub mismatches_found: bool,
    errors: RefCell<Collector>,
}

struct MsbuildProject {
    pub project: Option<msbuild::Project>,
    pub path: PathBuf,
}

impl Nuget {
    #[must_use]
    pub fn new(show_only_mismatched: bool) -> Self {
        Self {
            show_only_mismatched,
            mismatches_found: false,
            errors: RefCell::new(Collector::new()),
        }
    }
}

fn collect_msbuild_projects(solution: &Solution) -> Vec<MsbuildProject> {
    let dir = crate::parent_of(solution.path);

    solution
        .iterate_projects_without_web_sites()
        .filter_map(|p| crate::try_make_local_path(dir, p.path_or_uri))
        .filter_map(|path| match Project::from_path(&path) {
            Ok(project) => Some(MsbuildProject {
                path,
                project: Some(project),
            }),
            Err(e) => {
                if cfg!(debug_assertions) {
                    let p = path.to_str().unwrap_or_default();
                    println!("{p}: {e:?}");
                }
                None
            }
        })
        .collect()
}

fn has_mismatches(versions: &BTreeSet<(Option<&String>, &String)>) -> bool {
    versions
        .iter()
        .into_group_map_by(|x| x.0)
        .iter()
        .any(|(_, v)| v.len() > 1)
}

impl Consume for Nuget {
    fn ok(&mut self, solution: &solp::api::Solution) {
        let projects = collect_msbuild_projects(solution);

        let mut nugets = nugets(&projects);
        let nugets_from_packages_config = nugets_from_packages_configs(&projects);

        let nugets_from_packages_config = nugets_from_packages_config
            .iter()
            .map(|(k, v)| (k, v.iter().map(|v1| (None, v1)).collect()));

        // merging packages from packages.config if any
        nugets.extend(nugets_from_packages_config);

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
            .filter(|(_, versions)| !self.show_only_mismatched || has_mismatches(versions))
            .sorted_unstable_by(|(a, _), (b, _)| Ord::cmp(&a.to_lowercase(), &b.to_lowercase()))
            .for_each(|(pkg, versions)| {
                let groupped = versions.iter().into_group_map_by(|x| x.0);
                let rows = groupped
                    .iter()
                    .sorted_unstable_by_key(|x| x.0)
                    .map(|(c, v)| {
                        let mismatch = v.len() > 1;
                        let comma_separated = v.iter().map(|(_, v)| v).join(", ");
                        let line = if c.is_some() {
                            format!("{comma_separated} if {}", c.as_ref().unwrap())
                        } else {
                            comma_separated
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

    fn err(&self, path: &str) {
        self.errors.borrow_mut().add_path(path);
    }
}

impl Display for Nuget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mismatches_found && !self.show_only_mismatched {
            writeln!(
                f,
                " {}",
                "Solutions with nuget packages inconsistenty found"
                    .dark_red()
                    .bold()
            )?;
            writeln!(f)?;
        }
        if self.errors.borrow().count() > 0 {
            write!(f, "{}", self.errors.borrow())
        } else {
            Ok(())
        }
    }
}

/// returns hashmap where<br/>
/// key - package name<br/>
/// value - (condition, version) tuples set<br/>
/// condition is optional
fn nugets(projects: &[MsbuildProject]) -> HashMap<&String, BTreeSet<(Option<&String>, &String)>> {
    projects
        .iter()
        .filter_map(|p| p.project.as_ref())
        .filter_map(|p| p.item_group.as_ref())
        .flatten()
        .filter_map(|ig| {
            Some(
                ig.package_reference
                    .as_ref()?
                    .iter()
                    .map(|p| (ig.condition.as_ref(), p)),
            )
        })
        .flatten()
        .into_grouping_map_by(|(_, pack)| &pack.name)
        .fold(BTreeSet::new(), |mut acc, _key, (cond, val)| {
            acc.insert((cond, &val.version));
            acc
        })
}

fn nugets_from_packages_configs(projects: &[MsbuildProject]) -> HashMap<String, BTreeSet<String>> {
    projects
        .iter()
        .filter_map(|mp| {
            let parent = mp.path.parent()?;
            let packages_config = parent.join("packages.config");
            PackagesConfig::from_path(packages_config).ok()
        })
        .flat_map(|p| p.packages)
        .into_grouping_map_by(|p| p.name.clone())
        .fold(BTreeSet::new(), |mut acc, _key, val| {
            acc.insert(val.version);
            acc
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use solp::msbuild::{ItemGroup, PackageReference, Project};

    use super::*;

    #[test]
    fn nugets_no_mismatches() {
        // arramge
        let mut projects = vec![];
        let packs1 = vec![
            PackageReference {
                name: "a".to_string(),
                version: "1.0.0".to_string(),
            },
            PackageReference {
                name: "b".to_string(),
                version: "1.0.0".to_string(),
            },
        ];
        let packs2 = vec![
            PackageReference {
                name: "c".to_string(),
                version: "1.0.0".to_string(),
            },
            PackageReference {
                name: "d".to_string(),
                version: "1.0.0".to_string(),
            },
        ];
        projects.push(create_msbuild_project(packs1, None));
        projects.push(create_msbuild_project(packs2, None));

        // act
        let actual = nugets(&projects);

        // assert
        assert_eq!(4, actual.len());
        let has_mismatches = actual.iter().any(|(_, v)| has_mismatches(v));
        assert!(!has_mismatches);
    }

    #[test]
    fn nugets_no_mismatches_same_pgk_in_different_projects() {
        // arramge
        let mut projects = vec![];
        let packs1 = vec![
            PackageReference {
                name: "a".to_string(),
                version: "1.0.0".to_string(),
            },
            PackageReference {
                name: "b".to_string(),
                version: "1.0.0".to_string(),
            },
        ];
        let packs2 = vec![
            PackageReference {
                name: "c".to_string(),
                version: "1.0.0".to_string(),
            },
            PackageReference {
                name: "a".to_string(),
                version: "1.0.0".to_string(),
            },
        ];
        projects.push(create_msbuild_project(packs1, None));
        projects.push(create_msbuild_project(packs2, None));

        // act
        let actual = nugets(&projects);

        // assert
        assert_eq!(3, actual.len());
        let has_mismatches = actual.iter().any(|(_, v)| has_mismatches(v));
        assert!(!has_mismatches);
    }

    #[test]
    fn nugets_has_mismatches() {
        // arramge
        let mut projects = vec![];
        let packs1 = vec![
            PackageReference {
                name: "a".to_string(),
                version: "1.0.0".to_string(),
            },
            PackageReference {
                name: "b".to_string(),
                version: "1.0.0".to_string(),
            },
        ];
        let packs2 = vec![
            PackageReference {
                name: "c".to_string(),
                version: "1.0.0".to_string(),
            },
            PackageReference {
                name: "a".to_string(),
                version: "2.0.0".to_string(),
            },
        ];
        projects.push(create_msbuild_project(packs1, None));
        projects.push(create_msbuild_project(packs2, None));

        // act
        let actual = nugets(&projects);

        // assert
        assert_eq!(3, actual.len());
        let has_mismatches = actual.iter().any(|(_, v)| has_mismatches(v));
        assert!(has_mismatches);
    }

    #[test]
    fn nugets_no_mismatches_by_conditions() {
        // arramge
        let mut projects = vec![];
        let packs1 = vec![
            PackageReference {
                name: "a".to_string(),
                version: "1.0.0".to_string(),
            },
            PackageReference {
                name: "b".to_string(),
                version: "1.0.0".to_string(),
            },
        ];
        let packs2 = vec![
            PackageReference {
                name: "c".to_string(),
                version: "1.0.0".to_string(),
            },
            PackageReference {
                name: "a".to_string(),
                version: "2.0.0".to_string(),
            },
        ];
        projects.push(create_msbuild_project(packs1, None));
        projects.push(create_msbuild_project(packs2, Some("1".to_owned())));

        // act
        let actual = nugets(&projects);

        // assert
        assert_eq!(3, actual.len());
        let has_mismatches = actual.iter().any(|(_, v)| has_mismatches(v));
        assert!(!has_mismatches);
        let different_vers_key = "a".to_owned();
        assert!(actual.contains_key(&different_vers_key));
        assert_eq!(2, actual.get(&different_vers_key).unwrap().len());
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
