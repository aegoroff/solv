use crate::{ux, Consume};
use crossterm::style::Stylize;
use fnv::{FnvHashMap, FnvHashSet};
use petgraph::algo::DfsSpace;
use prettytable::Table;
use solp::ast::{Conf, Solution};
use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Display;
use std::path::PathBuf;

pub struct Validate {
    show_only_problems: bool,
}

impl Validate {
    #[must_use]
    pub fn new(show_only_problems: bool) -> Self {
        Self { show_only_problems }
    }
}

#[must_use]
fn make_projects_paths(path: &str, solution: &Solution) -> FnvHashMap<String, PathBuf> {
    let dir = crate::parent_of(path);
    solution
        .iterate_projects()
        .map(|p| (p.id.to_uppercase(), crate::make_path(dir, p.path)))
        .collect()
}

impl Consume for Validate {
    fn ok(&mut self, path: &str, solution: &Solution) {
        let projects = make_projects_paths(path, solution);

        let not_found = search_not_found(&projects);

        let danglings = search_dangling_configs(solution, &projects);

        let missings = search_missing(solution);

        let mut space = DfsSpace::new(&solution.dependencies);
        let cycle_detected =
            petgraph::algo::toposort(&solution.dependencies, Some(&mut space)).is_err();

        if !danglings.is_empty()
            || !not_found.is_empty()
            || !missings.is_empty()
            || cycle_detected
            || !self.show_only_problems
        {
            ux::print_solution_path(path);
        }

        let mut no_problems = true;
        if cycle_detected {
            println!(
                " {}",
                "  Solution contains project dependencies cycles"
                    .dark_red()
                    .bold()
            );
            println!();
            no_problems = false;
        }

        if !(danglings.is_empty()) {
            println!(
                " {}",
                "  Solution contains dangling project configurations that can be safely removed:"
                    .dark_yellow()
                    .bold()
            );
            println!();
            ux::print_one_column_table("Project ID", danglings.into_iter());
            no_problems = false;
        }

        if !(not_found.is_empty()) {
            println!(
                " {}",
                "  Solution contains unexist projects:".dark_yellow().bold()
            );
            println!();
            ux::print_one_column_table("Path", not_found.into_iter());
            no_problems = false;
        }

        if !(missings.is_empty()) {
            println!(" {}", "  Solution contains project configurations that are outside solution's configuration|platform list:".dark_yellow().bold());
            println!();

            let mut table = Table::new();

            let fmt = ux::new_format();
            table.set_format(fmt);
            table.set_titles(row![bF=> "Project ID", "Configuration|Platform"]);

            for (id, configs) in &missings {
                for config in configs.iter() {
                    table.add_row(row![*id, format!("{}|{}", config.config, config.platform)]);
                }
            }

            table.printstd();
            println!();

            no_problems = false;
        }

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

fn search_not_found(projects: &FnvHashMap<String, PathBuf>) -> BTreeSet<&str> {
    projects
        .iter()
        .filter_map(|(_, p)| {
            if p.canonicalize().is_ok() {
                None
            } else {
                p.as_path().to_str()
            }
        })
        .collect()
}

fn search_dangling_configs<'a>(
    solution: &'a Solution,
    projects: &FnvHashMap<String, PathBuf>,
) -> BTreeSet<&'a str> {
    solution
        .project_configs
        .iter()
        .filter_map(|pc| {
            if projects.contains_key(&pc.project_id.to_uppercase()) {
                None
            } else {
                Some(pc.project_id)
            }
        })
        .collect()
}

fn search_missing<'a>(solution: &'a Solution<'a>) -> Vec<(&'a str, Vec<&'a Conf>)> {
    let solution_platforms_configs = solution
        .solution_configs
        .iter()
        .collect::<FnvHashSet<&Conf>>();

    solution
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
        .collect()
}
