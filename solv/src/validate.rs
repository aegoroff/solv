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
    /// does validation
    fn validate(&mut self);
    /// will return true if validation succeeded false otherwise
    fn validation_result(&self) -> bool;
    /// prints validation results if any
    fn print_results(&self);
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
        let mut validators: Vec<Box<dyn Validator>> = vec![
            Box::new(Cycles::new(solution)),
            Box::new(Danglings::new(solution)),
            Box::new(NotFouund::new(path, solution)),
            Box::new(Missings::new(solution)),
        ];

        let valid_solution = validators.iter_mut().fold(true, |mut res, validator| {
            validator.validate();
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
    fn validate(&mut self) {
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
    }

    fn print_results(&self) {
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
    fn validate(&mut self) {
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
    }

    fn print_results(&self) {
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
    fn validate(&mut self) {
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
    }

    fn print_results(&self) {
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
    fn validate(&mut self) {
        let mut space = DfsSpace::new(&self.solution.dependencies);
        self.cycles_detected =
            petgraph::algo::toposort(&self.solution.dependencies, Some(&mut space)).is_err();
    }

    fn print_results(&self) {
        println!(
            " {}",
            "  Solution contains project dependencies cycles"
                .dark_red()
                .bold()
        );
        println!();
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
    fn dangling_validation_correct() {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Danglings::new(&solution);

        // Act
        validator.validate();

        // Assert
        assert!(validator.validation_result())
    }

    #[test]
    fn cycles_validation_correct() {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Cycles::new(&solution);

        // Act
        validator.validate();

        // Assert
        assert!(validator.validation_result())
    }

    #[test]
    fn missing_validation_correct() {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Missings::new(&solution);

        // Act
        validator.validate();

        // Assert
        assert!(validator.validation_result())
    }

    #[test]
    fn missing_validation_incorrect() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_MISSING_PROJECT_CONFIGS).unwrap();
        let mut validator = Missings::new(&solution);

        // Act
        validator.validate();

        // Assert
        assert!(!validator.validation_result())
    }

    #[test]
    fn dangling_validation_incorrect() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_DANGLINGS).unwrap();
        let mut validator = Danglings::new(&solution);

        // Act
        validator.validate();

        // Assert
        assert!(!validator.validation_result())
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
}
