use comfy_table::{Attribute, Cell, CellAlignment, ContentArrangement};
use crossterm::style::Stylize;
use num_format::{Locale, ToFormattedString};
use solp::ast::Solution;
use solp::{msbuild, Consume};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Display;

use crate::error::Collector;
use crate::{calculate_percent, ux};
pub struct Info {
    total_projects: BTreeMap<String, i32>,
    projects_in_solutions: BTreeMap<String, i32>,
    solutions: i32,
    errors: RefCell<Collector>,
}

impl Info {
    #[must_use]
    pub fn new() -> Self {
        Self {
            total_projects: BTreeMap::new(),
            projects_in_solutions: BTreeMap::new(),
            solutions: 0,
            errors: RefCell::new(Collector::new()),
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

        let mut solution_table = ux::create_solution_table(path);
        solution_table.set_content_arrangement(ContentArrangement::Disabled);

        let mut table = ux::new_table();

        table.add_row(vec![
            Cell::new("Format"),
            Cell::new(solution.format).add_attribute(Attribute::Bold),
        ]);
        if !solution.product.is_empty() {
            table.add_row(vec![
                Cell::new("Product"),
                Cell::new(solution.product).add_attribute(Attribute::Bold),
            ]);
        }

        for version in &solution.versions {
            table.add_row(vec![
                Cell::new(version.name),
                Cell::new(version.ver).add_attribute(Attribute::Bold),
            ]);
        }
        solution_table.add_row(vec![Cell::new(table)]);

        let mut table = ux::new_table();
        table.set_header(vec![
            Cell::new("Project type").add_attribute(Attribute::Bold),
            Cell::new("Count").add_attribute(Attribute::Bold),
        ]);

        for (key, value) in &projects_by_type {
            *self.total_projects.entry(String::from(*key)).or_insert(0) += *value;
            *self
                .projects_in_solutions
                .entry(String::from(*key))
                .or_insert(0) += 1;
            table.add_row(vec![
                Cell::new(*key),
                Cell::new(*value).add_attribute(Attribute::Italic),
            ]);
        }

        solution_table.add_row(vec![Cell::new(table)]);

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

        if let Some(t) =
            ux::create_one_column_table("Configuration", None, configurations.into_iter())
        {
            solution_table.add_row(vec![Cell::new(t)]);
        }
        if let Some(t) = ux::create_one_column_table("Platform", None, platforms.into_iter()) {
            solution_table.add_row(vec![Cell::new(t)]);
        }
        println!("{solution_table}");
    }

    fn err(&self, path: &str) {
        self.errors.borrow_mut().add_path(path);
    }
}

impl Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", " Statistic:".dark_red().bold())?;

        let mut table = ux::new_table();
        table.set_header(vec![
            Cell::new("Project type").add_attribute(Attribute::Bold),
            Cell::new("Count").add_attribute(Attribute::Bold),
            Cell::new("%").add_attribute(Attribute::Bold),
            Cell::new("# Solutions").add_attribute(Attribute::Bold),
            Cell::new("%").add_attribute(Attribute::Bold),
        ]);

        let projects = self.total_projects.iter().fold(0, |total, p| total + *p.1);

        for (key, value) in &self.total_projects {
            let proj_percent = calculate_percent(*value, projects);
            let in_sols = self.projects_in_solutions.get(key).unwrap();
            let sol_percent = calculate_percent(*in_sols, self.solutions);
            table.add_row(vec![
                Cell::new(key),
                Cell::new(value.to_formatted_string(&Locale::en)).add_attribute(Attribute::Italic),
                Cell::new(format!("{proj_percent:.2}%")).add_attribute(Attribute::Italic),
                Cell::new(in_sols.to_formatted_string(&Locale::en))
                    .set_alignment(CellAlignment::Right)
                    .add_attribute(Attribute::Italic),
                Cell::new(format!("{sol_percent:.2}%")).add_attribute(Attribute::Italic),
            ]);
        }
        writeln!(f, "{table}")?;

        let mut table = ux::new_table();
        table.add_row(vec![
            Cell::new("Total solutions"),
            Cell::new(self.solutions.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
        ]);
        table.add_row(vec![
            Cell::new("Total projects"),
            Cell::new(projects.to_formatted_string(&Locale::en)).add_attribute(Attribute::Italic),
        ]);
        writeln!(f, "{table}")?;

        write!(f, "{}", self.errors.borrow())
    }
}
