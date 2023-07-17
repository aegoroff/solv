use std::{
    collections::{BTreeSet, HashMap},
    fmt::{self, Display},
};

use crossterm::style::Stylize;
use fnv::FnvHashMap;
use itertools::Itertools;
use prettytable::Table;
use solp::msbuild::PackagesConfig;

use crate::{ux, Consume, MsbuildProject};

pub struct Nuget {
    show_only_mismatched: bool,
    pub mismatches_found: bool,
}

impl Nuget {
    #[must_use]
    pub fn new(show_only_mismatched: bool) -> Self {
        Self {
            show_only_mismatched,
            mismatches_found: false,
        }
    }
}

fn has_mismatches(versions: &BTreeSet<(Option<&String>, &String)>) -> bool {
    versions
        .iter()
        .into_group_map_by(|x| x.0)
        .iter()
        .any(|(_, v)| v.len() > 1)
}

impl Consume for Nuget {
    fn ok(&mut self, path: &str, solution: &solp::ast::Solution) {
        let projects = crate::new_projects_paths_map(path, solution);

        let mut nugets = nugets(&projects);
        let nugets_from_packages_config = nugets_from_projects_configs(&projects);
        if nugets.is_empty() && nugets_from_packages_config.is_empty() {
            return;
        }
        for (k, v) in &nugets_from_packages_config {
            let versions = nugets.entry(k).or_insert(BTreeSet::new());
            for ver in v {
                versions.insert((None, ver));
            }
        }

        let mut table = Table::new();

        let fmt = ux::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> "Package", "Version(s)"]);

        let mut mismatch = false;
        nugets
            .iter()
            .filter(|(_, versions)| !self.show_only_mismatched || has_mismatches(versions))
            .sorted_by(|(a, _), (b, _)| Ord::cmp(&a.to_lowercase(), &b.to_lowercase()))
            .for_each(|(pkg, versions)| {
                versions
                    .iter()
                    .into_group_map_by(|x| x.0)
                    .iter()
                    .sorted_by_key(|x| x.0)
                    .for_each(|(c, v)| {
                        mismatch = v.len() > 1;
                        let comma_separated = v.iter().map(|(_, v)| v).join(", ");
                        let line = if c.is_some() {
                            format!("{comma_separated} if {}", c.as_ref().unwrap())
                        } else {
                            comma_separated
                        };
                        if mismatch {
                            table.add_row(row![pkg, iFr->line]);
                        } else {
                            table.add_row(row![pkg, iF->line]);
                        }
                    });
            });
        self.mismatches_found |= mismatch;

        if self.show_only_mismatched && !mismatch {
            return;
        }

        ux::print_solution_path(path);
        table.printstd();
        println!();
    }

    fn err(&self, path: &str) {
        crate::err(path);
    }
}

impl Display for Nuget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mismatches_found && !self.show_only_mismatched {
            writeln!(
                f,
                "{}",
                " Solutions with nuget packages inconsistenty found"
                    .dark_red()
                    .bold()
            )?;
            writeln!(f)?;
        }

        Ok(())
    }
}

fn nugets(
    projects: &FnvHashMap<String, MsbuildProject>,
) -> HashMap<&String, BTreeSet<(Option<&String>, &String)>> {
    projects
        .iter()
        .filter_map(|(_, p)| p.project.as_ref())
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

fn nugets_from_projects_configs(
    projects: &FnvHashMap<String, MsbuildProject>,
) -> HashMap<String, BTreeSet<String>> {
    projects
        .iter()
        .filter_map(|(_, mp)| {
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
        let mut projects = FnvHashMap::<String, MsbuildProject>::default();
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
        projects.insert("1".to_owned(), create_msbuild_project(packs1, None));
        projects.insert("2".to_owned(), create_msbuild_project(packs2, None));

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
        let mut projects = FnvHashMap::<String, MsbuildProject>::default();
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
        projects.insert("1".to_owned(), create_msbuild_project(packs1, None));
        projects.insert("2".to_owned(), create_msbuild_project(packs2, None));

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
        let mut projects = FnvHashMap::<String, MsbuildProject>::default();
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
        projects.insert("1".to_owned(), create_msbuild_project(packs1, None));
        projects.insert("2".to_owned(), create_msbuild_project(packs2, None));

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
        let mut projects = FnvHashMap::<String, MsbuildProject>::default();
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
        projects.insert("1".to_owned(), create_msbuild_project(packs1, None));
        projects.insert(
            "2".to_owned(),
            create_msbuild_project(packs2, Some("1".to_owned())),
        );

        // act
        let actual = nugets(&projects);

        // assert
        assert_eq!(3, actual.len());
        let has_mismatches = actual.iter().any(|(_, v)| has_mismatches(v));
        assert!(!has_mismatches);
        let different_vers_key = "a".to_owned();
        assert!(actual.get(&different_vers_key).is_some());
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
            }),
            path: PathBuf::new(),
        }
    }
}
