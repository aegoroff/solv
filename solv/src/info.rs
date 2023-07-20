use crossterm::style::Stylize;
use num_format::{Locale, ToFormattedString};
use prettytable::{format, Table};
use solp::ast::Solution;
use solp::{msbuild, Consume};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Display;

use crate::error::Collector;
use crate::ux;
pub struct Info {
    total_projects: BTreeMap<String, i32>,
    projects_in_solutions: BTreeMap<String, i32>,
    solutions: i32,
    errors: Collector,
}

impl Info {
    #[must_use]
    pub fn new() -> Self {
        Self {
            total_projects: BTreeMap::new(),
            projects_in_solutions: BTreeMap::new(),
            solutions: 0,
            errors: Collector::new(),
        }
    }
}

impl Default for Info {
    fn default() -> Self {
        Self::new()
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

        ux::print_solution_path(path);

        let mut table = Table::new();

        let fmt = format::FormatBuilder::new()
            .column_separator(' ')
            .borders(' ')
            .indent(0)
            .padding(1, 0)
            .build();
        table.set_format(fmt);

        table.add_row(row!["Format", bF->solution.format]);
        if !solution.product.is_empty() {
            table.add_row(row!["Product", bF->solution.product]);
        }

        for version in &solution.versions {
            table.add_row(row![version.name, bF->version.ver]);
        }
        table.printstd();

        println!();

        let mut table = Table::new();

        let fmt = ux::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> "Project type", "Count"]);

        for (key, value) in &projects_by_type {
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

        ux::print_one_column_table("Configuration", configurations.into_iter());
        ux::print_one_column_table("Platform", platforms.into_iter());
    }

    fn err(&mut self, path: &str) {
        self.errors.add_path(path);
    }
}

impl Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", " Totals:".dark_red().bold())?;
        writeln!(f)?;

        let mut table = Table::new();

        let fmt = ux::new_format();
        table.set_format(fmt);
        table
            .set_titles(row![bF->"Project type", bF->"Count", cbF->"%", bF->"Solutions", cbF->"%"]);

        let projects = self.total_projects.iter().fold(0, |total, p| total + *p.1);

        for (key, value) in &self.total_projects {
            let proj_percent = (f64::from(*value) / f64::from(projects)) * 100_f64;
            let in_sols = self.projects_in_solutions.get(key).unwrap();
            let sol_percent = (f64::from(*in_sols) / f64::from(self.solutions)) * 100_f64;
            table.add_row(row![
                key,
                i->*value.to_formatted_string(&Locale::en),
                i->format!("{proj_percent:.2}%"),
                ir->*in_sols.to_formatted_string(&Locale::en),
                i->format!("{sol_percent:.2}%")
            ]);
        }
        table.printstd();

        writeln!(f)?;

        let mut table = Table::new();
        let fmt = ux::new_format();
        table.set_format(fmt);
        table.add_row(row![
            "Total solutions",
            i->self.solutions.to_formatted_string(&Locale::en),
        ]);
        table.add_row(row![
            "Total projects",
            i->projects.to_formatted_string(&Locale::en),
        ]);
        table.printstd();
        writeln!(f)?;

        write!(f, "{}", self.errors)
    }
}
