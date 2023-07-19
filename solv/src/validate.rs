use crate::{ux, Consume};
use crossterm::style::Stylize;
use fnv::FnvHashSet;
use petgraph::algo::DfsSpace;
use prettytable::Table;
use solp::ast::{Conf, Solution};
use solp::msbuild;
use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Display;
use std::path::PathBuf;

trait Validator {
    fn validate(&mut self) -> bool;
    fn results(&self);
}

pub struct Validate {
    show_only_problems: bool,
}

impl Validate {
    #[must_use]
    pub fn new(show_only_problems: bool) -> Self {
        Self { show_only_problems }
    }
}

impl Consume for Validate {
    fn ok(&mut self, path: &str, solution: &Solution) {
        let mut not_found_validator = NotFouund::new(path, solution);
        let mut danglings_validator = Danglings::new(solution);
        let mut missings_validator = Missings::new(solution);
        let mut cycles_validator = Cycles::new(solution);

        let not_found_result = not_found_validator.validate();
        let danglings_result = danglings_validator.validate();
        let missings_result = missings_validator.validate();
        let cycles_result = cycles_validator.validate();
        let no_problems = danglings_result && not_found_result && missings_result && cycles_result;

        if !no_problems || !self.show_only_problems {
            ux::print_solution_path(path);
        }
        cycles_validator.results();
        danglings_validator.results();
        not_found_validator.results();
        missings_validator.results();

        if !self.show_only_problems && no_problems {
            println!(
                " {}",
                "  No problems found in solution.".dark_green().bold()
            );
            println!();
        }
    }

    fn err(&self, path: &str) {
        crate::err(path);
    }
}

impl Display for Validate {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

struct NotFouund<'a> {
    path: &'a str,
    solution: &'a Solution<'a>,
    bad_paths: BTreeSet<PathBuf>,
}

impl<'a> NotFouund<'a> {
    pub fn new(path: &'a str, solution: &'a Solution<'a>) -> Self {
        Self {
            path,
            solution,
            bad_paths: BTreeSet::new(),
        }
    }
}

impl<'a> Validator for NotFouund<'a> {
    fn validate(&mut self) -> bool {
        let dir = crate::parent_of(self.path);
        self.bad_paths = self
            .solution
            .iterate_projects()
            .filter(|p| !msbuild::is_web_site_project(p.type_id))
            .filter_map(|p| {
                let full_path = crate::make_path(dir, p.path);
                if full_path.canonicalize().is_ok() {
                    None
                } else {
                    Some(full_path)
                }
            })
            .collect();
        self.bad_paths.is_empty()
    }

    fn results(&self) {
        if self.bad_paths.is_empty() {
            return;
        }
        println!(
            " {}",
            "  Solution contains unexist projects:".dark_yellow().bold()
        );
        println!();
        let items: Vec<&str> = self
            .bad_paths
            .iter()
            .filter_map(|p| p.as_path().to_str())
            .collect();
        ux::print_one_column_table("Path", items.into_iter());
    }
}

struct Danglings<'a> {
    solution: &'a Solution<'a>,
    danglings: BTreeSet<String>,
}

impl<'a> Danglings<'a> {
    pub fn new(solution: &'a Solution<'a>) -> Self {
        Self {
            solution,
            danglings: BTreeSet::new(),
        }
    }
}

impl<'a> Validator for Danglings<'a> {
    fn validate(&mut self) -> bool {
        let project_ids: FnvHashSet<String> = self
            .solution
            .iterate_projects()
            .map(|p| p.id.to_uppercase())
            .collect();

        self.danglings = self
            .solution
            .project_configs
            .iter()
            .map(|p| p.project_id.to_uppercase())
            .collect::<FnvHashSet<String>>()
            .difference(&project_ids)
            .cloned()
            .collect();
        self.danglings.is_empty()
    }

    fn results(&self) {
        if self.danglings.is_empty() {
            return;
        }
        println!(
            " {}",
            "  Solution contains dangling project configurations that can be safely removed:"
                .dark_yellow()
                .bold()
        );
        println!();
        ux::print_one_column_table(
            "Project ID",
            self.danglings.iter().map(std::string::String::as_str),
        );
    }
}

struct Missings<'a> {
    solution: &'a Solution<'a>,
    missings: Vec<(&'a str, Vec<&'a Conf<'a>>)>,
}

impl<'a> Missings<'a> {
    pub fn new(solution: &'a Solution<'a>) -> Self {
        Self {
            solution,
            missings: vec![],
        }
    }
}

impl<'a> Validator for Missings<'a> {
    fn validate(&mut self) -> bool {
        let solution_platforms_configs = self
            .solution
            .solution_configs
            .iter()
            .collect::<FnvHashSet<&Conf>>();

        self.missings = self
            .solution
            .project_configs
            .iter()
            .filter_map(|pc| {
                let diff = pc
                    .configs
                    .iter()
                    .collect::<FnvHashSet<&Conf>>()
                    .difference(&solution_platforms_configs)
                    .copied()
                    .collect::<Vec<&Conf>>();

                if diff.is_empty() {
                    None
                } else {
                    Some((pc.project_id, diff))
                }
            })
            .collect();
        self.missings.is_empty()
    }

    fn results(&self) {
        if self.missings.is_empty() {
            return;
        }

        println!(" {}", "  Solution contains project configurations that are outside solution's configuration|platform list:".dark_yellow().bold());
        println!();

        let mut table = Table::new();

        let fmt = ux::new_format();
        table.set_format(fmt);
        table.set_titles(row![bF=> "Project ID", "Configuration|Platform"]);

        for (id, configs) in &self.missings {
            for config in configs.iter() {
                table.add_row(row![*id, format!("{}|{}", config.config, config.platform)]);
            }
        }

        table.printstd();
        println!();
    }
}

struct Cycles<'a> {
    solution: &'a Solution<'a>,
    cycles_detected: bool,
}

impl<'a> Cycles<'a> {
    pub fn new(solution: &'a Solution<'a>) -> Self {
        Self {
            solution,
            cycles_detected: false,
        }
    }
}

impl<'a> Validator for Cycles<'a> {
    fn validate(&mut self) -> bool {
        let mut space = DfsSpace::new(&self.solution.dependencies);
        self.cycles_detected =
            petgraph::algo::toposort(&self.solution.dependencies, Some(&mut space)).is_err();
        !self.cycles_detected
    }

    fn results(&self) {
        if !self.cycles_detected {
            return;
        }
        println!(
            " {}",
            "  Solution contains project dependencies cycles"
                .dark_red()
                .bold()
        );
        println!();
    }
}
