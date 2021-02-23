use crate::ast::Solution;
use crate::msbuild;
use crate::Consume;
use ansi_term::Colour::RGB;
use prettytable::format;
use prettytable::format::TableFormat;
use prettytable::Table;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::iter::FromIterator;

extern crate ansi_term;

pub struct Print {
    path: String,
}

impl Print {
    pub fn new(path: &str) -> Self {
        Self {
            path: String::from(path),
        }
    }

    fn new_format() -> TableFormat {
        let fmt = format::FormatBuilder::new()
            .column_separator(' ')
            .borders(' ')
            .separators(
                &[format::LinePosition::Title],
                format::LineSeparator::new('-', ' ', ' ', ' '),
            )
            .indent(3)
            .padding(0, 0)
            .build();
        fmt
    }
}

impl Consume for Print {
    fn consume(&self, solution: &Solution) {
        let mut projects_by_type: BTreeMap<&str, i32> = BTreeMap::new();
        for prj in &solution.projects {
            if prj.type_id == msbuild::ID_SOLUTION_FOLDER {
                continue;
            }
            *projects_by_type.entry(prj.type_descr).or_insert(0) += 1;
        }

        let configurations =
            BTreeSet::from_iter(solution.configurations.iter().map(|c| c.configuration));

        let platforms = BTreeSet::from_iter(solution.configurations.iter().map(|c| c.platform));

        let path = RGB(0xAA, 0xAA, 0xAA).paint(&self.path);
        println!(" {}", path);

        let mut table = Table::new();

        let fmt = format::FormatBuilder::new()
            .column_separator(' ')
            .borders(' ')
            .indent(0)
            .padding(1, 0)
            .build();
        table.set_format(fmt);

        table.add_row(row!["Format", bF->solution.format]);
        table.add_row(row!["Product", bF->solution.product]);

        for version in &solution.versions {
            table.add_row(row![version.name, bF->version.ver]);
        }
        table.printstd();

        println!();

        let mut table = Table::new();

        let fmt = Print::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> "Project type", "Count"]);

        for (key, value) in projects_by_type.iter() {
            table.add_row(row![*key, bFg->*value]);
        }

        table.printstd();
        println!();

        let mut table = Table::new();

        let fmt = Print::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> "Configuration"]);

        for item in configurations.iter() {
            table.add_row(row![*item]);
        }

        table.printstd();
        println!();

        let mut table = Table::new();

        let fmt = Print::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> "Platform"]);

        for item in platforms.iter() {
            table.add_row(row![*item]);
        }

        table.printstd();
        println!();
    }
}
