use self::petgraph::algo::DfsSpace;
use crate::info::Info;
use crate::{Consume, ConsumeDisplay};
use crossterm::style::{style, Color, Stylize};
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
            let path = style(path).with(Color::Rgb {
                r: 0xAA,
                g: 0xAA,
                b: 0xAA,
            });
            println!(" {}", path);
        }

        let mut no_problems = true;
        if cycle_detected {
            println!(
                " {}",
                "  Solution contains project dependencies cycles".red()
            );
            println!();
            no_problems = false;
        }

        if !(danglings.is_empty()) {
            println!(
                " {}",
                "  Solution contains dangling project configurations that can be safely removed:"
                    .yellow()
            );
            println!();
            Info::print_one_column_table("Project ID", danglings);
            no_problems = false;
        }

        if !(not_found.is_empty()) {
            println!(" {}", "  Solution contains unexist projects:".yellow());
            println!();
            Info::print_one_column_table("Path", not_found);
            no_problems = false;
        }

        if !(missings.is_empty()) {
            println!(" {}", "  Solution contains project configurations that are outside solution's configuration|platform list:".yellow());
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
            println!(" {}", "  No problems found in solution.".green());
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
    relative
        .split(r"\")
        .fold(PathBuf::from(&dir), |pb, s| pb.join(s))
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

            if diff.is_empty() {
                None
            } else {
                Some((pc.project_id, diff))
            }
        })
        .collect()
}

#[cfg(test)]
#[cfg(not(target_os = "windows"))]
mod tests {
    use super::*;
    use rstest::*;
    use spectral::prelude::*;

    #[rstest]
    #[case("/base", "x", "/base/x")]
    #[case("/base", r"x\y", "/base/x/y")]
    #[trace]
    fn make_path_tests(#[case] base: &str, #[case] path: &str, #[case] expected: &str) {
        // Arrange
        let d = Path::new(base);

        // Act
        let actual = make_path(&d, path);

        // Assert
        assert_that!(actual.to_str().unwrap()).is_equal_to(expected);
    }
}