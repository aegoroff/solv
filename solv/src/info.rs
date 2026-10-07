use comfy_table::{Attribute, Cell, CellAlignment, ContentArrangement};
use crossterm::style::Stylize;
use num_format::{Locale, ToFormattedString};
use solp::Consume;
use solp::api::Solution;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Display;

use crate::error::Collector;
use crate::{calculate_percent, ux};
pub struct Info {
    total_projects: BTreeMap<String, i32>,
    projects_in_solutions: BTreeMap<String, i32>,
    solutions: i32,
    errors: Collector,
}

/// Summary of a solution
struct SolutionInfo {
    path: String,
    format: String,
    product: String,
    /// (name, version) pairs
    versions: Vec<(String, String)>,
    /// Project type description to number of projects of this type
    projects_by_type: BTreeMap<String, i32>,
    configurations: BTreeSet<String>,
    platforms: BTreeSet<String>,
}

impl SolutionInfo {
    fn new(solution: &Solution) -> Self {
        let mut projects_by_type: BTreeMap<String, i32> = BTreeMap::new();
        for prj in solution.iterate_projects() {
            *projects_by_type
                .entry(prj.type_description.to_owned())
                .or_insert(0) += 1;
        }
        Self {
            path: solution.path.to_owned(),
            format: solution.format.to_owned(),
            product: solution.product.to_owned(),
            versions: solution
                .versions
                .iter()
                .map(|v| (v.name.to_owned(), v.version.to_owned()))
                .collect(),
            projects_by_type,
            configurations: solution
                .configurations
                .iter()
                .map(|c| c.configuration.to_owned())
                .collect(),
            platforms: solution
                .configurations
                .iter()
                .map(|c| c.platform.to_owned())
                .collect(),
        }
    }
}

impl Display for SolutionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut solution_table = ux::create_solution_table(&self.path);
        solution_table.set_content_arrangement(ContentArrangement::Disabled);

        let mut table = ux::new_table();

        table.add_row([
            Cell::new("Format"),
            Cell::new(&self.format).add_attribute(Attribute::Bold),
        ]);
        if !self.product.is_empty() {
            table.add_row([
                Cell::new("Product"),
                Cell::new(&self.product).add_attribute(Attribute::Bold),
            ]);
        }

        for (name, version) in &self.versions {
            table.add_row([
                Cell::new(name),
                Cell::new(version).add_attribute(Attribute::Bold),
            ]);
        }
        solution_table.add_row([Cell::new(table)]);

        let mut table = ux::new_table();
        table.set_header([
            Cell::new("Project type").add_attribute(Attribute::Bold),
            Cell::new("Count").add_attribute(Attribute::Bold),
        ]);

        for (key, value) in &self.projects_by_type {
            table.add_row([
                Cell::new(key),
                Cell::new(*value).add_attribute(Attribute::Italic),
            ]);
        }

        solution_table.add_row([Cell::new(table)]);

        if let Some(t) =
            ux::create_one_column_table("Configuration", None, self.configurations.iter())
        {
            solution_table.add_row([Cell::new(t)]);
        }
        if let Some(t) = ux::create_one_column_table("Platform", None, self.platforms.iter()) {
            solution_table.add_row([Cell::new(t)]);
        }
        writeln!(f, "{solution_table}")
    }
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

    /// Counts solution projects in totals and returns the solution report
    fn report(&mut self, solution: &Solution) -> SolutionInfo {
        self.solutions += 1;
        let info = SolutionInfo::new(solution);
        for (key, value) in &info.projects_by_type {
            *self.total_projects.entry(key.clone()).or_insert(0) += *value;
            *self.projects_in_solutions.entry(key.clone()).or_insert(0) += 1;
        }
        info
    }
}

impl Default for Info {
    fn default() -> Self {
        Self::new()
    }
}

impl Consume for Info {
    fn ok(&mut self, solution: &Solution) {
        let report = self.report(solution);
        print!("{report}");
    }

    fn err(&mut self, path: &str) {
        self.errors.add_path(path);
    }
}

impl Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, " {}", "Statistic:".dark_red().bold())?;

        let mut table = ux::new_table();
        table.set_header([
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
            table.add_row([
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
        table.add_row([
            Cell::new("Total solutions"),
            Cell::new(self.solutions.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
        ]);
        table.add_row([
            Cell::new("Total projects"),
            Cell::new(projects.to_formatted_string(&Locale::en)).add_attribute(Attribute::Italic),
        ]);
        writeln!(f, "{table}")?;

        write!(f, "{}", self.errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slnx_info_counts_projects_by_type_without_folders() {
        // Arrange
        let slnx = r#"<Solution>
  <Folder Name="/src/">
    <Project Path="src/App/App.csproj" />
    <Project Path="src/Native/Native.vcxproj" />
  </Folder>
  <Folder Name="/Solution Items/">
    <File Path="README.md" />
  </Folder>
  <Project Path="src/Lib/Lib.csproj" />
</Solution>"#;
        let solution = solp::parse_str(slnx).unwrap();
        let mut info = Info::new();

        // Act
        let report = info.report(&solution);

        // Assert
        assert!(report.to_string().contains("Debug"));
        assert!(info.to_string().contains("Total solutions"));
        assert_eq!(1, info.solutions);
        assert_eq!(
            vec![("C#", 2), ("C++", 1)],
            info.total_projects
                .iter()
                .map(|(k, v)| (k.as_str(), *v))
                .collect::<Vec<_>>()
        );
    }
}
