use crate::ast::{Conf, Solution};
use crate::msbuild;
use crate::Consume;
use ansi_term::Colour::{Green, Red, Yellow, RGB};
use prettytable::format;
use prettytable::format::TableFormat;
use prettytable::Table;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

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

impl Consume for Validate {
    fn ok(&self, path: &str, solution: &Solution) {
        let dir = Path::new(path).parent().unwrap_or_else(|| Path::new(""));

        let projects = solution
            .projects
            .iter()
            .filter(|p| !msbuild::is_solution_folder(p.type_id))
            .map(|p| (p.id.to_uppercase(), make_path(dir, p.path)))
            .collect::<HashMap<String, PathBuf>>();

        let not_found: BTreeSet<&str> = projects
            .iter()
            .filter(|(_, path)| path.canonicalize().is_err())
            .map(|(_, pb)| pb.as_path().to_str().unwrap_or(""))
            .collect();

        let dangling_configurations = solution
            .project_configs
            .iter()
            .filter(|pc| !projects.contains_key(&pc.project_id.to_uppercase()))
            .map(|pc| pc.project_id)
            .collect::<BTreeSet<&str>>();

        let solution_platforms_configs =
            solution.solution_configs.iter().collect::<HashSet<&Conf>>();

        let problem_project_configurations = solution
            .project_configs
            .iter()
            .filter_map(|pc| {
                let missing = pc
                    .configs
                    .iter()
                    .filter(|c| !solution_platforms_configs.contains(c))
                    .collect::<Vec<&Conf>>();
                if !missing.is_empty() {
                    return Some((pc.project_id, missing));
                }
                None
            })
            .collect::<Vec<(&str, Vec<&Conf>)>>();

        if !dangling_configurations.is_empty()
            || !not_found.is_empty()
            || !problem_project_configurations.is_empty()
            || !self.show_only_problems
        {
            let path = RGB(0xAA, 0xAA, 0xAA).paint(path);
            println!(" {}", path);
        }

        let mut no_problems = true;
        if !(dangling_configurations.is_empty()) {
            println!(" {}", Yellow.paint("  Solution contains dangling project configurations that can be safely removed:"));
            println!();
            Info::print_one_column_table("Project ID", dangling_configurations);
            no_problems = false;
        }

        if !(not_found.is_empty()) {
            println!(" {}", Yellow.paint("  Solution contains unexist projects:"));
            println!();
            Info::print_one_column_table("Path", not_found);
            no_problems = false;
        }

        if !(problem_project_configurations.is_empty()) {
            println!(" {}", Yellow.paint("  Solution contains project configurations that are outside solution's configuration|platform list:"));
            println!();

            let mut table = Table::new();

            let fmt = Info::new_format();
            table.set_format(fmt);
            table.set_titles(row![bF=> "Project ID", "Configuration|Platform"]);

            for (id, configs) in problem_project_configurations.iter() {
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

#[cfg(not(target_os = "windows"))]
fn make_path(dir: &Path, relative: &str) -> PathBuf {
    let sep = &std::path::MAIN_SEPARATOR.to_string();
    let mut pb = PathBuf::from(&dir);
    let cleaned = relative.replace("\\", &sep);
    let cleaned = cleaned.trim_matches('"');
    pb.push(cleaned);
    pb
}

#[cfg(target_os = "windows")]
fn make_path(dir: &Path, relative: &str) -> PathBuf {
    let mut pb = PathBuf::from(&dir);
    let cleaned = relative.trim_matches('"');
    pb.push(cleaned);
    pb
}
