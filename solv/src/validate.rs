use crate::error::Collector;
use crate::{calculate_percent, ux, Consume};
use comfy_table::{Attribute, Cell};
use crossterm::style::Stylize;
use fnv::FnvHashSet;
use num_format::{Locale, ToFormattedString};
use petgraph::algo::DfsSpace;
use solp::ast::{Conf, Solution};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Display;
use std::path::PathBuf;

trait Validator {
    /// does validation
    fn validate(&mut self, statistic: &mut Statistic);
    /// will return true if validation succeeded false otherwise
    fn validation_result(&self) -> bool;
    /// prints validation results if any
    fn print_results(&self);
}

pub struct Validate {
    show_only_problems: bool,
    errors: RefCell<Collector>,
    statistic: RefCell<Statistic>,
}

#[derive(Default)]
struct Statistic {
    cycles: u64,
    dangings: u64,
    not_found: u64,
    missings: u64,
    parsed: u64,
    not_parsed: u64,
    total: u64,
}

impl Display for Statistic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", " Statistic:".dark_red().bold())?;

        let mut table = ux::new_table();

        table.set_header(vec![
            Cell::new("Category").add_attribute(Attribute::Bold),
            Cell::new("# Solutions").add_attribute(Attribute::Bold),
            Cell::new("%").add_attribute(Attribute::Bold),
        ]);

        let cycles_percent = calculate_percent(self.cycles as i32, self.total as i32);
        let missings_percent = calculate_percent(self.missings as i32, self.total as i32);
        let dangings_percent = calculate_percent(self.dangings as i32, self.total as i32);
        let not_found_percent = calculate_percent(self.not_found as i32, self.total as i32);
        let parsed_percent = calculate_percent(self.parsed as i32, self.total as i32);
        let not_parsed_percent = calculate_percent(self.not_parsed as i32, self.total as i32);
        let total_percent = calculate_percent(self.total as i32, self.total as i32);

        table.add_row(vec![
            Cell::new("Successfully parsed"),
            Cell::new(self.parsed.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{parsed_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row(vec![
            Cell::new("Contain dependencies cycles"),
            Cell::new(self.cycles.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{cycles_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row(vec![
            Cell::new("Contain project configurations outside solution's list"),
            Cell::new(self.missings.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{missings_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row(vec![
            Cell::new("Contain dangling project configurations"),
            Cell::new(self.dangings.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{dangings_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row(vec![
            Cell::new("Contain projects that not exists"),
            Cell::new(self.not_found.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{not_found_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row(vec![
            Cell::new("Not parsed"),
            Cell::new(self.not_parsed.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{not_parsed_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row(vec!["", "", ""]);
        table.add_row(vec![
            Cell::new("Total"),
            Cell::new(self.total.to_formatted_string(&Locale::en)).add_attribute(Attribute::Italic),
            Cell::new(format!("{total_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        writeln!(f, "{table}")
    }
}

impl Validate {
    #[must_use]
    pub fn new(show_only_problems: bool) -> Self {
        Self {
            show_only_problems,
            errors: RefCell::new(Collector::new()),
            statistic: RefCell::new(Statistic::default()),
        }
    }
}

impl Consume for Validate {
    fn ok(&mut self, path: &str, solution: &Solution) {
        let mut validators: Vec<Box<dyn Validator>> = vec![
            Box::new(Cycles::new(solution)),
            Box::new(Danglings::new(solution)),
            Box::new(NotFouund::new(path, solution)),
            Box::new(Missings::new(solution)),
        ];

        let valid_solution = validators.iter_mut().fold(true, |mut res, validator| {
            validator.validate(&mut self.statistic.borrow_mut());
            res &= validator.validation_result();
            res
        });

        if !self.show_only_problems || !valid_solution {
            ux::print_solution_path(path);
        }
        for v in &validators {
            if !v.validation_result() {
                v.print_results();
            }
        }

        if !self.show_only_problems && valid_solution {
            println!(
                " {}",
                "  No problems found in solution.".dark_green().bold()
            );
            println!();
        }
        self.statistic.borrow_mut().total += 1;
    }

    fn err(&self, path: &str) {
        self.errors.borrow_mut().add_path(path);
    }
}

impl Display for Validate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut statistic = self.statistic.borrow_mut();
        statistic.not_parsed = self.errors.borrow().count();
        statistic.parsed = statistic.total;
        statistic.total += statistic.not_parsed;
        write!(f, "{statistic}")?;
        write!(f, "{}", self.errors.borrow())
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
    fn validate(&mut self, statistic: &mut Statistic) {
        let dir = crate::parent_of(self.path);
        self.bad_paths = self
            .solution
            .iterate_projects_without_web_sites()
            .filter_map(|p| crate::try_make_local_path(dir, p.path_or_uri))
            .filter_map(|full_path| {
                if full_path.canonicalize().is_ok() {
                    None
                } else {
                    Some(full_path)
                }
            })
            .collect();
        if !self.validation_result() {
            statistic.not_found += 1;
        }
    }

    fn print_results(&self) {
        let items: Vec<&str> = self
            .bad_paths
            .iter()
            .filter_map(|p| p.as_path().to_str())
            .collect();
        ux::print_one_column_table(
            "Unexist project path",
            Some(comfy_table::Color::DarkYellow),
            items.into_iter(),
        );
    }

    fn validation_result(&self) -> bool {
        self.bad_paths.is_empty()
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
    fn validate(&mut self, statistic: &mut Statistic) {
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
        if !self.validation_result() {
            statistic.dangings += 1;
        }
    }

    fn print_results(&self) {
        ux::print_one_column_table(
            "Dangling project configurations that can be safely removed",
            Some(comfy_table::Color::DarkYellow),
            self.danglings.iter().map(std::string::String::as_str),
        );
    }

    fn validation_result(&self) -> bool {
        self.danglings.is_empty()
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
    fn validate(&mut self, statistic: &mut Statistic) {
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
        if !self.validation_result() {
            statistic.missings += 1;
        }
    }

    fn print_results(&self) {
        println!("  {}", "Solution contains project configurations that are outside solution's configuration|platform list:".dark_yellow().bold());

        let mut table = ux::new_table();
        table.set_header(vec![
            Cell::new("Project ID").add_attribute(Attribute::Bold),
            Cell::new("Configuration|Platform").add_attribute(Attribute::Bold),
        ]);

        for (id, configs) in &self.missings {
            for config in configs.iter() {
                table.add_row(vec![
                    Cell::new(*id),
                    Cell::new(format!("{}|{}", config.config, config.platform)),
                ]);
            }
        }

        println!("{table}");
    }

    fn validation_result(&self) -> bool {
        self.missings.is_empty()
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
    fn validate(&mut self, statistic: &mut Statistic) {
        let mut space = DfsSpace::new(&self.solution.dependencies);
        self.cycles_detected =
            petgraph::algo::toposort(&self.solution.dependencies, Some(&mut space)).is_err();
        if !self.validation_result() {
            statistic.cycles += 1;
        }
    }

    fn print_results(&self) {
        println!(
            " {}",
            "  Solution contains project dependencies cycles"
                .dark_red()
                .bold()
        );
    }

    fn validation_result(&self) -> bool {
        !self.cycles_detected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_test_correct_solution() {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Validate::new(false);

        // Act
        validator.ok("", &solution);

        // Assert
    }

    #[test]
    fn integration_test_solution_with_danglings() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_DANGLINGS).unwrap();
        let mut validator = Validate::new(false);

        // Act
        validator.ok("", &solution);

        // Assert
    }

    #[test]
    fn integration_test_solution_with_missings() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_MISSING_PROJECT_CONFIGS).unwrap();
        let mut validator = Validate::new(false);

        // Act
        validator.ok("", &solution);

        // Assert
    }

    #[test]
    fn integration_test_solution_with_cycles() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_CYCLES).unwrap();
        let mut validator = Validate::new(false);

        // Act
        validator.ok("", &solution);

        // Assert
    }

    #[test]
    fn dangling_validation_correct() {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Danglings::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(validator.validation_result());
    }

    #[test]
    fn cycles_validation_correct() {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Cycles::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(validator.validation_result());
        assert_eq!(0, statistic.cycles);
    }

    #[test]
    fn cycles_validation_incorrect() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_CYCLES).unwrap();
        let mut validator = Cycles::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(!validator.validation_result());
        assert_eq!(1, statistic.cycles);
    }

    #[test]
    fn missing_validation_correct() {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Missings::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(validator.validation_result());
        assert_eq!(0, statistic.missings);
    }

    #[test]
    fn missing_validation_incorrect() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_MISSING_PROJECT_CONFIGS).unwrap();
        let mut validator = Missings::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(!validator.validation_result());
        assert_eq!(1, statistic.missings);
    }

    #[test]
    fn dangling_validation_incorrect() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_DANGLINGS).unwrap();
        let mut validator = Danglings::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(!validator.validation_result());
        assert_eq!(1, statistic.dangings);
    }

    #[test]
    fn print_statistic_test() {
        // Arrange
        let s = Statistic::default();

        // Act
        println!("{s}");

        // Assert
    }

    const CORRECT_SOLUTION: &str = r###"
Microsoft Visual Studio Solution File, Format Version 8.00
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest", "gtest.vcproj", "{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest_main", "gtest_main.vcproj", "{3AF54C8A-10BF-4332-9147-F68ED9862032}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest_unittest", "gtest_unittest.vcproj", "{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest_prod_test", "gtest_prod_test.vcproj", "{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Global
	GlobalSection(SolutionConfiguration) = preSolution
		Debug = Debug
		Release = Release
	EndGlobalSection
	GlobalSection(ProjectConfiguration) = postSolution
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Debug.ActiveCfg = Debug|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Debug.Build.0 = Debug|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Release.ActiveCfg = Release|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Release.Build.0 = Release|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Debug.ActiveCfg = Debug|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Debug.Build.0 = Debug|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Release.ActiveCfg = Release|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Release.Build.0 = Release|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Debug.ActiveCfg = Debug|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Debug.Build.0 = Debug|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Release.ActiveCfg = Release|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Release.Build.0 = Release|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Debug.ActiveCfg = Debug|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Debug.Build.0 = Debug|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Release.ActiveCfg = Release|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Release.Build.0 = Release|Win32
	EndGlobalSection
	GlobalSection(ExtensibilityGlobals) = postSolution
	EndGlobalSection
	GlobalSection(ExtensibilityAddIns) = postSolution
	EndGlobalSection
EndGlobal
"###;

    const SOLUTION_WITH_MISSING_PROJECT_CONFIGS: &str = r#"
Microsoft Visual Studio Solution File, Format Version 11.00
# Visual Studio 2010
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "a", "a\a.csproj", "{78965571-A6C2-4161-95B1-813B46610EA7}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "b", "b\b.csproj", "{D9523F4D-6CB7-4431-85F6-8122F55EB144}"
EndProject
Global
	GlobalSection(SolutionConfigurationPlatforms) = preSolution
		Debug|Any CPU = Debug|Any CPU
		Release|Any CPU = Release|Any CPU
	EndGlobalSection
	GlobalSection(ProjectConfigurationPlatforms) = postSolution
		{78965571-A6C2-4161-95B1-813B46610EA7}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{78965571-A6C2-4161-95B1-813B46610EA7}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{78965571-A6C2-4161-95B1-813B46610EA7}.Debug|x86.ActiveCfg = Debug|Any CPU
		{78965571-A6C2-4161-95B1-813B46610EA7}.Debug|x86.Build.0 = Debug|Any CPU
		{78965571-A6C2-4161-95B1-813B46610EA7}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{78965571-A6C2-4161-95B1-813B46610EA7}.Release|Any CPU.Build.0 = Release|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Debug|x86.ActiveCfg = Debug|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Debug|x86.Build.0 = Debug|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Release|Any CPU.Build.0 = Release|Any CPU
	EndGlobalSection
	GlobalSection(SolutionProperties) = preSolution
		HideSolutionNode = FALSE
	EndGlobalSection
EndGlobal
"#;

    const SOLUTION_WITH_DANGLINGS: &str = r###"
Microsoft Visual Studio Solution File, Format Version 8.00
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest", "gtest.vcproj", "{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest_main", "gtest_main.vcproj", "{3AF54C8A-10BF-4332-9147-F68ED9862032}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest_unittest", "gtest_unittest.vcproj", "{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Global
	GlobalSection(SolutionConfiguration) = preSolution
		Debug = Debug
		Release = Release
	EndGlobalSection
	GlobalSection(ProjectConfiguration) = postSolution
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Debug.ActiveCfg = Debug|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Debug.Build.0 = Debug|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Release.ActiveCfg = Release|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Release.Build.0 = Release|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Debug.ActiveCfg = Debug|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Debug.Build.0 = Debug|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Release.ActiveCfg = Release|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Release.Build.0 = Release|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Debug.ActiveCfg = Debug|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Debug.Build.0 = Debug|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Release.ActiveCfg = Release|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Release.Build.0 = Release|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Debug.ActiveCfg = Debug|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Debug.Build.0 = Debug|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Release.ActiveCfg = Release|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Release.Build.0 = Release|Win32
	EndGlobalSection
	GlobalSection(ExtensibilityGlobals) = postSolution
	EndGlobalSection
	GlobalSection(ExtensibilityAddIns) = postSolution
	EndGlobalSection
EndGlobal
"###;

    const SOLUTION_WITH_CYCLES: &str = r#"
Microsoft Visual Studio Solution File, Format Version 12.00
# Visual Studio 15
VisualStudioVersion = 15.0.26403.0
MinimumVisualStudioVersion = 10.0.40219.1
Project("{930C7802-8A8C-48F9-8165-68863BCCD9DD}") = "logviewer.install", "logviewer.install\logviewer.install.wixproj", "{27060CA7-FB29-42BC-BA66-7FC80D498354}"
	ProjectSection(ProjectDependencies) = postProject
		{405827CB-84E1-46F3-82C9-D889892645AC} = {405827CB-84E1-46F3-82C9-D889892645AC}
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D} = {CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}
	EndProjectSection
EndProject
Project("{930C7802-8A8C-48F9-8165-68863BCCD9DD}") = "logviewer.install.bootstrap", "logviewer.install.bootstrap\logviewer.install.bootstrap.wixproj", "{1C0ED62B-D506-4E72-BBC2-A50D3926466E}"
	ProjectSection(ProjectDependencies) = postProject
		{27060CA7-FB29-42BC-BA66-7FC80D498354} = {27060CA7-FB29-42BC-BA66-7FC80D498354}
	EndProjectSection
EndProject
Project("{2150E333-8FDC-42A3-9474-1A3956D46DE8}") = "solution items", "solution items", "{3B960F8F-AD5D-45E7-92C0-05B65E200AC4}"
	ProjectSection(SolutionItems) = preProject
		.editorconfig = .editorconfig
		appveyor.yml = appveyor.yml
		logviewer.xml = logviewer.xml
		WiX.msbuild = WiX.msbuild
	EndProjectSection
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.tests", "logviewer.tests\logviewer.tests.csproj", "{939DD379-CDC8-47EF-8D37-0E5E71D99D30}"
	ProjectSection(ProjectDependencies) = postProject
		{383C08FC-9CAC-42E5-9B02-471561479A74} = {383C08FC-9CAC-42E5-9B02-471561479A74}
	EndProjectSection
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.logic", "logviewer.logic\logviewer.logic.csproj", "{383C08FC-9CAC-42E5-9B02-471561479A74}"
	ProjectSection(ProjectDependencies) = postProject
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30} = {939DD379-CDC8-47EF-8D37-0E5E71D99D30}
	EndProjectSection
EndProject
Project("{2150E333-8FDC-42A3-9474-1A3956D46DE8}") = ".nuget", ".nuget", "{B720ED85-58CF-4840-B1AE-55B0049212CC}"
	ProjectSection(SolutionItems) = preProject
		.nuget\NuGet.Config = .nuget\NuGet.Config
	EndProjectSection
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.engine", "logviewer.engine\logviewer.engine.csproj", "{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.install.mca", "logviewer.install.mca\logviewer.install.mca.csproj", "{405827CB-84E1-46F3-82C9-D889892645AC}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.ui", "logviewer.ui\logviewer.ui.csproj", "{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.bench", "logviewer.bench\logviewer.bench.csproj", "{75E0C034-44C8-461B-A677-9A19566FE393}"
EndProject
Global
	GlobalSection(SolutionConfigurationPlatforms) = preSolution
		Debug|Any CPU = Debug|Any CPU
		Debug|Mixed Platforms = Debug|Mixed Platforms
		Debug|x86 = Debug|x86
		Release|Any CPU = Release|Any CPU
		Release|Mixed Platforms = Release|Mixed Platforms
		Release|x86 = Release|x86
	EndGlobalSection
	GlobalSection(ProjectConfigurationPlatforms) = postSolution
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Any CPU.ActiveCfg = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Any CPU.Build.0 = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Mixed Platforms.ActiveCfg = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Mixed Platforms.Build.0 = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|x86.ActiveCfg = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|x86.Build.0 = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Any CPU.ActiveCfg = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Any CPU.Build.0 = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Mixed Platforms.ActiveCfg = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Mixed Platforms.Build.0 = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|x86.ActiveCfg = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|x86.Build.0 = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Any CPU.ActiveCfg = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Any CPU.Build.0 = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Mixed Platforms.ActiveCfg = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Mixed Platforms.Build.0 = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|x86.ActiveCfg = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|x86.Build.0 = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Any CPU.ActiveCfg = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Any CPU.Build.0 = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Mixed Platforms.ActiveCfg = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Mixed Platforms.Build.0 = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|x86.ActiveCfg = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|x86.Build.0 = Release|x86
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|x86.ActiveCfg = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Any CPU.Build.0 = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|x86.ActiveCfg = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|x86.ActiveCfg = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Any CPU.Build.0 = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|x86.ActiveCfg = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|x86.ActiveCfg = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Any CPU.Build.0 = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|x86.ActiveCfg = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|x86.ActiveCfg = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Any CPU.Build.0 = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|x86.ActiveCfg = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|x86.ActiveCfg = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Any CPU.Build.0 = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|x86.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|x86.ActiveCfg = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|x86.Build.0 = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Any CPU.Build.0 = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|x86.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|x86.Build.0 = Release|Any CPU
	EndGlobalSection
	GlobalSection(SolutionProperties) = preSolution
		HideSolutionNode = FALSE
	EndGlobalSection
EndGlobal
"#;
}
