use std::{
    collections::{BTreeSet, HashMap},
    fmt::{self, Display},
};

use crossterm::style::Stylize;
use fnv::FnvHashMap;
use itertools::{any, Itertools};
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
                versions.insert(ver);
            }
        }

        if self.show_only_mismatched && !any(&nugets, |(_, versions)| versions.len() > 1) {
            return;
        }

        let mut table = Table::new();

        let fmt = ux::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> "Package", "Version(s)"]);

        ux::print_solution_path(path);

        nugets
            .iter()
            .filter(|(_, versions)| !self.show_only_mismatched || versions.len() > 1)
            .sorted_by(|(a, _), (b, _)| Ord::cmp(&a.to_lowercase(), &b.to_lowercase()))
            .for_each(|(pkg, versions)| {
                let mismatch = versions.len() > 1;
                let versions = versions.iter().join(", ");
                self.mismatches_found |= mismatch;
                if mismatch {
                    table.add_row(row![pkg, iFr->versions]);
                } else {
                    table.add_row(row![pkg, iF->versions]);
                }
            });
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

fn nugets(projects: &FnvHashMap<String, MsbuildProject>) -> HashMap<&String, BTreeSet<&String>> {
    projects
        .iter()
        .filter_map(|(_, p)| p.project.as_ref())
        .filter_map(|p| p.item_group.as_ref())
        .flatten()
        .filter_map(|ig| ig.package_reference.as_ref())
        .flatten()
        .into_grouping_map_by(|c| &c.name)
        .fold(BTreeSet::new(), |mut acc, _key, val| {
            acc.insert(&val.version);
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
