use crate::ast::{Solution, Configuration};
use crate::msbuild;
use crate::Consume;
use ansi_term::Colour::{Green, Red, Yellow, RGB};
use prettytable::format;
use prettytable::format::TableFormat;
use prettytable::Table;
use std::collections::{BTreeMap, BTreeSet, HashSet};

extern crate ansi_term;

pub struct Info {
    debug: bool,
}

pub struct Validate {
    show_only_problems: bool,
    debug: bool,
}

impl Info {
    pub fn new_box(debug: bool) -> Box<dyn Consume> {
        Box::new(Self { debug })
    }

    fn new_format() -> TableFormat {
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

    fn print_one_column_table(head: &str, set: BTreeSet<&str>) {
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

    fn err(debug: bool, path: &str) {
        if debug {
            return;
        }
        let path = Red.paint(path);
        eprintln!("Error parsing {} solution", path);
    }
}

impl Validate {
    pub fn new_box(debug: bool, show_only_problems: bool) -> Box<dyn Consume> {
        Box::new(Self {
            debug,
            show_only_problems,
        })
    }
}

impl Consume for Info {
    fn ok(&self, path: &str, solution: &Solution) {
        let mut projects_by_type: BTreeMap<&str, i32> = BTreeMap::new();
        for prj in &solution.projects {
            if prj.type_id == msbuild::ID_SOLUTION_FOLDER {
                continue;
            }
            *projects_by_type.entry(prj.type_descr).or_insert(0) += 1;
        }

        let path = RGB(0xAA, 0xAA, 0xAA).paint(path);
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

        let fmt = Info::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> "Project type", "Count"]);

        for (key, value) in projects_by_type.iter() {
            table.add_row(row![*key, bFg->*value]);
        }

        table.printstd();
        println!();

        let configurations = solution
            .configurations
            .iter()
            .map(|c| c.configuration)
            .collect::<BTreeSet<&str>>();

        let platforms = solution
            .configurations
            .iter()
            .map(|c| c.platform)
            .collect::<BTreeSet<&str>>();

        Info::print_one_column_table("Configuration", configurations);
        Info::print_one_column_table("Platform", platforms);
    }

    fn err(&self, path: &str) {
        Info::err(self.debug, path);
    }

    fn is_debug(&self) -> bool {
        self.debug
    }
}

impl Consume for Validate {
    fn ok(&self, path: &str, solution: &Solution) {
        let projects = solution
            .projects
            .iter()
            .map(|c| c.id.to_uppercase())
            .collect::<HashSet<String>>();

        let dangling_configurations = solution
            .project_configurations
            .iter()
            .filter(|pc| !projects.contains(&pc.project_id.to_uppercase()))
            .map(|pc| pc.project_id)
            .collect::<BTreeSet<&str>>();

        let solution_platforms = solution
            .configurations
            .iter()
            .map(|c| c.platform)
            .collect::<HashSet<&str>>();

        let solution_configurations = solution
            .configurations
            .iter()
            .map(|c| c.configuration)
            .collect::<HashSet<&str>>();

        let problem_project_configurations = solution
            .project_configurations
            .iter()
            .filter_map(|pc| {
                let missing = pc.configurations.iter().filter(|c| {
                    !solution_platforms.contains(c.platform)
                        || !solution_configurations.contains(c.configuration)
                }).collect::<Vec<&Configuration>>();
                if !missing.is_empty() {
                    return Some((pc.project_id, missing))
                }
                None
            })
            .collect::<Vec<(&str, Vec<&Configuration>)>>();

        let path = RGB(0xAA, 0xAA, 0xAA).paint(path);

        let mut no_problems = true;
        if !(dangling_configurations.is_empty()) {
            println!(" {}", path);
            println!(" {}", Yellow.paint("  Solution contains dangling project configurations that can be safely removed:"));
            println!();
            Info::print_one_column_table("Project ID", dangling_configurations);
            no_problems = false;
        }

        if !(problem_project_configurations.is_empty()) {
            println!(" {}", path);
            println!(" {}", Yellow.paint("  Solution contains project configurations that are outside solution's configuration|platform list:"));
            println!();

            let mut table = Table::new();

            let fmt = Info::new_format();
            table.set_format(fmt);
            table.set_titles(row![bF=> "Project ID", "Configuration|Platform"]);

            for (id, configs) in problem_project_configurations.iter() {
                for config in configs.iter() {
                    table.add_row(row![*id, format!("{}|{}", config.configuration, config.platform)]);
                }
            }

            table.printstd();
            println!();

            no_problems = false;
        }

        if !self.show_only_problems && no_problems {
            println!(" {}", path);
            println!(" {}", Green.paint("  No problems found in solution."));
            println!();
        }
    }

    fn err(&self, path: &str) {
        Info::err(self.debug, path);
    }

    fn is_debug(&self) -> bool {
        self.debug
    }
}
