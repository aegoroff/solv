use std::collections::{BTreeSet, HashMap};

use crossterm::style::{style, Color, Stylize};
use fnv::FnvHashMap;
use itertools::Itertools;
use prettytable::Table;

use crate::{info::Info, Consume, MsbuildProject};

pub struct Nuget {
    show_only_mismatched: bool,
}

impl Nuget {
    #[must_use]
    pub fn new(show_only_mismatched: bool) -> Self {
        Self {
            show_only_mismatched,
        }
    }
}

impl Consume for Nuget {
    fn ok(&mut self, path: &str, solution: &solp::ast::Solution) {
        let projects = crate::new_projects_paths_map(path, solution);

        let nugets = nugets(&projects);
        if nugets.is_empty() {
            return;
        }

        if self.show_only_mismatched
            && nugets
                .iter()
                .filter(|(_, versions)| versions.len() > 1)
                .count()
                == 0
        {
            return;
        }

        let mut table = Table::new();

        let fmt = Info::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> "Package", "Version(s)"]);

        let path = style(path)
            .with(Color::Rgb {
                r: 0xAA,
                g: 0xAA,
                b: 0xAA,
            })
            .bold();
        println!(" {path}");

        for (pkg, versions) in &nugets {
            if self.show_only_mismatched && versions.len() > 1 {
                let versions = versions.iter().join(",");
                table.add_row(row![pkg, iF->versions]);
            }
        }
        table.printstd();
        println!();
    }

    fn err(&self, path: &str) {
        crate::err(path);
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
        .into_grouping_map_by(|c| &c.include)
        .fold(BTreeSet::new(), |mut acc, _key, val| {
            acc.insert(&val.version);
            acc
        })
}
