use self::petgraph::algo::DfsSpace;
use crate::info::Info;
use crate::{Consume, ConsumeDisplay};
use ansi_term::Colour::{Green, Red, Yellow, RGB};
use fnv::{FnvHashMap, FnvHashSet};
use prettytable::Table;
use solp::ast::{Conf, Solution};
use solp::msbuild;
use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Display;
use std::path::{Path, PathBuf};

extern crate fnv;
extern crate petgraph;

pub struct Validate {
    show_only_problems: bool,
    debug: bool,
}

impl Validate {
    pub fn new_box(debug: bool, show_only_problems: bool) -> Box<dyn ConsumeDisplay> {
        Box::new(Self {
            show_only_problems,
            debug,
        })
    }
}

impl Consume for Validate {
    fn ok(&mut self, path: &str, solution: &Solution) {
        let projects = new_projects_paths_map(path, solution);

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
            let path = RGB(0xAA, 0xAA, 0xAA).paint(path);
            println!(" {}", path);
        }

        let mut no_problems = true;
        if cycle_detected {
            println!(
                " {}",
                Red.paint("  Solution contains project dependencies cycles")
            );
            println!();
            no_problems = false;
        }

        if !(danglings.is_empty()) {
            println!(" {}", Yellow.paint("  Solution contains dangling project configurations that can be safely removed:"));
            println!();
            Info::print_one_column_table("Project ID", danglings);
            no_problems = false;
        }

        if !(not_found.is_empty()) {
            println!(" {}", Yellow.paint("  Solution contains unexist projects:"));
            println!();
            Info::print_one_column_table("Path", not_found);
            no_problems = false;
        }

        if !(missings.is_empty()) {
            println!(" {}", Yellow.paint("  Solution contains project configurations that are outside solution's configuration|platform list:"));
            println!();

            let mut table = Table::new();

            let fmt = Info::new_format();
            table.set_format(fmt);
            table.set_titles(row![bF=> "Project ID", "Configuration|Platform"]);

            for (id, configs) in missings.iter() {
                for config in configs.iter() {
                    table.add_row(row![*id, format!("{}|{}", config.config, config.platform)]);
                }
            }

            table.printstd();
            println!();

            no_problems = false;
        }

        if !self.show_only_problems && no_problems {
            println!(" {}", Green.paint("  No problems found in solution."));
            println!();
        }
    }

    fn err(&self, path: &str) {
        crate::err(self.debug, path);
    }

    fn is_debug(&self) -> bool {
        self.debug
    }
}

impl Display for Validate {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn make_path(dir: &Path, relative: &str) -> PathBuf {
    let sep = &std::path::MAIN_SEPARATOR.to_string();
    let cleaned = relative.replace("\\", sep);
    PathBuf::from(&dir).join(cleaned)
}

#[cfg(target_os = "windows")]
fn make_path(dir: &Path, relative: &str) -> PathBuf {
    PathBuf::from(&dir).join(relative)
}

fn new_projects_paths_map(path: &str, solution: &Solution) -> FnvHashMap<String, PathBuf> {
    let dir = Path::new(path).parent().unwrap_or_else(|| Path::new(""));

    solution
        .projects
        .iter()
        .filter(|p| !msbuild::is_solution_folder(p.type_id))
        .map(|p| (p.id.to_uppercase(), make_path(dir, p.path)))
        .collect()
}

fn search_not_found(projects: &FnvHashMap<String, PathBuf>) -> BTreeSet<&str> {
    projects
        .iter()
        .filter(|(_, path)| path.canonicalize().is_err())
        .filter_map(|(_, pb)| pb.as_path().to_str())
        .collect()
}

fn search_dangling_configs<'a>(
    solution: &'a Solution,
    projects: &FnvHashMap<String, PathBuf>,
) -> BTreeSet<&'a str> {
    solution
        .project_configs
        .iter()
        .filter(|pc| !projects.contains_key(&pc.project_id.to_uppercase()))
        .map(|pc| pc.project_id)
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

            if !diff.is_empty() {
                return Some((pc.project_id, diff));
            }
            None
        })
        .collect()
}
