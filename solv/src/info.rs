use crate::ConsumeDisplay;
use crossterm::style::{style, Color, Stylize};
use num_format::{Locale, ToFormattedString};
use prettytable::format::TableFormat;
use prettytable::{format, Table};
use solp::ast::Solution;
use solp::{msbuild, Consume};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Display;

extern crate num_format;

pub struct Info {
    debug: bool,
    total_projects: BTreeMap<String, i32>,
    projects_in_solutions: BTreeMap<String, i32>,
    solutions: i32,
}

impl Info {
    pub fn new_box(debug: bool) -> Box<dyn ConsumeDisplay> {
        Box::new(Self {
            debug,
            total_projects: BTreeMap::new(),
            projects_in_solutions: BTreeMap::new(),
            solutions: 0,
        })
    }

    pub fn new_format() -> TableFormat {
        format::FormatBuilder::new()
            .column_separator(' ')
            .borders(' ')
            .separators(
                &[format::LinePosition::Title],
                format::LineSeparator::new('-', ' ', ' ', ' '),
            )
            .indent(3)
            .padding(0, 0)
            .build()
    }

    pub fn print_one_column_table(head: &str, set: BTreeSet<&str>) {
        if set.is_empty() {
            return;
        }
        let mut table = Table::new();

        let fmt = Info::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> head]);

        for item in set.iter() {
            table.add_row(row![*item]);
        }

        table.printstd();
        println!();
    }
}

impl Consume for Info {
    fn ok(&mut self, path: &str, solution: &Solution) {
        self.solutions += 1;
        let mut projects_by_type: BTreeMap<&str, i32> = BTreeMap::new();
        for prj in &solution.projects {
            if msbuild::is_solution_folder(prj.type_id) {
                continue;
            }
            *projects_by_type.entry(prj.type_descr).or_insert(0) += 1;
        }

        let path = style(path)
            .with(Color::Rgb {
                r: 0xAA,
                g: 0xAA,
                b: 0xAA,
            })
            .bold();
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
        if solution.product != "" {
            table.add_row(row!["Product", bF->solution.product]);
        }

        for version in &solution.versions {
            table.add_row(row![version.name, bF->version.ver]);
        }
        table.printstd();

        println!();

        let mut table = Table::new();

        let fmt = Info::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> "Project type", "Count"]);

        for (key, value) in projects_by_type.iter() {
            *self.total_projects.entry(String::from(*key)).or_insert(0) += *value;
            *self
                .projects_in_solutions
                .entry(String::from(*key))
                .or_insert(0) += 1;
            table.add_row(row![*key, bFg->*value]);
        }

        table.printstd();
        println!();

        let configurations = solution
            .solution_configs
            .iter()
            .map(|c| c.config)
            .collect::<BTreeSet<&str>>();

        let platforms = solution
            .solution_configs
            .iter()
            .map(|c| c.platform)
            .collect::<BTreeSet<&str>>();

        Info::print_one_column_table("Configuration", configurations);
        Info::print_one_column_table("Platform", platforms);
    }

    fn err(&self, path: &str) {
        crate::err(self.debug, path);
    }

    fn is_debug(&self) -> bool {
        self.debug
    }
}

impl Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", " Totals:".dark_red().bold())?;
        writeln!(f)?;

        let mut table = Table::new();

        let fmt = Info::new_format();
        table.set_format(fmt);
        table
            .set_titles(row![bF->"Project type", bF->"Count", cbF->"%", bF->"Solutions", cbF->"%"]);

        let projects = self.total_projects.iter().fold(0, |total, p| total + *p.1);

        for (key, value) in self.total_projects.iter() {
            let proj_percent = (*value as f64 / projects as f64) * 100_f64;
            let in_sols = self.projects_in_solutions.get(key).unwrap();
            let sol_percent = (*in_sols as f64 / self.solutions as f64) * 100_f64;
            table.add_row(row![
                key,
                *value.to_formatted_string(&Locale::en),
                format!("{:.2}%", proj_percent),
                r->*in_sols.to_formatted_string(&Locale::en),
                format!("{:.2}%", sol_percent)
            ]);
        }
        table.printstd();

        writeln!(f)?;

        let mut table = Table::new();
        let fmt = Info::new_format();
        table.set_format(fmt);
        table.add_row(row![
            "Total solutions",
            self.solutions.to_formatted_string(&Locale::en),
        ]);
        table.add_row(row![
            "Total projects",
            projects.to_formatted_string(&Locale::en),
        ]);
        table.printstd();

        writeln!(f)
    }
}
