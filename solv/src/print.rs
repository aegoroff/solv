use self::petgraph::algo::DfsSpace;
use crate::{Consume, ConsumeDisplay};
use ansi_term::Colour::{Green, Red, Yellow, RGB};
use fnv::{FnvHashMap, FnvHashSet};
use prettytable::format;
use prettytable::format::TableFormat;
use prettytable::Table;
use solp::ast::{Conf, Solution};
use solp::msbuild;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Display;
use std::path::{Path, PathBuf};

extern crate ansi_term;
extern crate fnv;
extern crate petgraph;

pub struct Info {
    debug: bool,
    total_projects: BTreeMap<String, i32>,
    projects_in_solutions: BTreeMap<String, i32>,
    solutions: i32,
}

pub struct Validate {
    show_only_problems: bool,
    debug: bool,
}

pub trait ConsumePrintable: Consume + Display {}

impl Info {
    pub fn new_box(debug: bool) -> Box<dyn ConsumeDisplay> {
        Box::new(Self {
            debug,
            total_projects: BTreeMap::new(),
            projects_in_solutions: BTreeMap::new(),
            solutions: 0,
        })
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
    pub fn new_box(debug: bool, show_only_problems: bool) -> Box<dyn ConsumeDisplay> {
        Box::new(Self {
            debug,
            show_only_problems,
        })
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
        Info::err(self.debug, path);
    }

    fn is_debug(&self) -> bool {
        self.debug
    }
}

impl Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", Red.bold().paint(" Totals:"))?;
        writeln!(f, "")?;

        let mut table = Table::new();

        let fmt = Info::new_format();
        table.set_format(fmt);
        table
            .set_titles(row![bF->"Project type", bF->"Count", cbF->"%", bF->"Solutions", cbF->"%"]);

        let projects = self.total_projects.iter().fold(0, |total, p| total + *p.1);

        for (key, value) in self.total_projects.iter() {
            let p = (*value as f64 / projects as f64) * 100 as f64;
            let in_sols = self.projects_in_solutions.get(key).unwrap();
            let ps = (*in_sols as f64 / self.solutions as f64) * 100 as f64;
            table.add_row(row![
                key,
                *value,
                format!("{:.2}%", p),
                r->*in_sols,
                format!("{:.2}%", ps)
            ]);
        }
        table.printstd();
        writeln!(f, "")
    }
}

impl Consume for Validate {
    fn ok(&mut self, path: &str, solution: &Solution) {
        let projects = new_projects_map(path, solution);

        let not_found = Validate::search_not_found(&projects);

        let danglings = Validate::search_dangling_configs(solution, &projects);

        let missings = Validate::search_missing(solution);

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
        Info::err(self.debug, path);
    }

    fn is_debug(&self) -> bool {
        self.debug
    }
}

impl Display for Validate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}

#[cfg(not(target_os = "windows"))]
fn make_path(dir: &Path, relative: &str) -> PathBuf {
    let sep = &std::path::MAIN_SEPARATOR.to_string();
    let cleaned = relative.replace("\\", &sep);
    PathBuf::from(&dir).join(cleaned)
}

#[cfg(target_os = "windows")]
fn make_path(dir: &Path, relative: &str) -> PathBuf {
    PathBuf::from(&dir).join(relative)
}

fn new_projects_map(path: &str, solution: &Solution) -> FnvHashMap<String, PathBuf> {
    let dir = Path::new(path).parent().unwrap_or_else(|| Path::new(""));

    solution
        .projects
        .iter()
        .filter(|p| !msbuild::is_solution_folder(p.type_id))
        .map(|p| (p.id.to_uppercase(), make_path(dir, p.path)))
        .collect()
}

impl Validate {
    fn search_not_found(projects: &FnvHashMap<String, PathBuf>) -> BTreeSet<&str> {
        projects
            .iter()
            .filter(|(_, path)| path.canonicalize().is_err())
            .map(|(_, pb)| pb.as_path().to_str().unwrap_or(""))
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
}
