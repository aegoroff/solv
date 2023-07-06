use std::collections::{BTreeSet, HashMap};

use crossterm::style::{style, Color, Stylize};
use fnv::FnvHashMap;
use itertools::Itertools;
use prettytable::Table;

use crate::{info::Info, Consume, MsbuildProject};

pub struct Nuget {}

impl Nuget {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Nuget {
    fn default() -> Self {
        Self::new()
    }
}

impl Consume for Nuget {
    fn ok(&mut self, path: &str, solution: &solp::ast::Solution) {
        let projects = crate::new_projects_paths_map(path, solution);

        let nugets = nugets(&projects);
        if nugets.is_empty() {
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

        nugets.iter().for_each(|(pkg, versions)| {
            let versions = versions.iter().join(",");
            table.add_row(row![pkg, iF->versions]);
        });
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
