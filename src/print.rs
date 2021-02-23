use crate::ast::Solution;
use crate::msbuild;
use crate::Consume;
use ansi_term::Colour::{Red, RGB};
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

    fn print_one_column_table(head: &str, set: BTreeSet<&str>) {
        if set.is_empty() {
            return;
        }
        let mut table = Table::new();

        let fmt = Print::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> head]);

        for item in set.iter() {
            table.add_row(row![*item]);
        }

        table.printstd();
        println!();
    }
}

impl Consume for Print {
    fn ok(&self, solution: &Solution) {
        let mut projects_by_type: BTreeMap<&str, i32> = BTreeMap::new();
        for prj in &solution.projects {
            if prj.type_id == msbuild::ID_SOLUTION_FOLDER {
                continue;
            }
            *projects_by_type.entry(prj.type_descr).or_insert(0) += 1;
        }

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

        let configurations =
            BTreeSet::from_iter(solution.configurations.iter().map(|c| c.configuration));

        let platforms = BTreeSet::from_iter(solution.configurations.iter().map(|c| c.platform));

        Print::print_one_column_table("Configuration", configurations);
        Print::print_one_column_table("Platform", platforms);
    }

    fn err(&self) {
        let path = Red.paint(&self.path);
        eprintln!("Error parsing {} solution", path);
    }
}
