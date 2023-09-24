use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    fmt::{self, Display},
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use solp::Consume;

use crate::error::Collector;

pub struct Json {
    serialized: Vec<String>,
    errors: RefCell<Collector>,
    pretty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution<'a> {
    pub path: &'a str,
    pub format: &'a str,
    pub product: &'a str,
    pub versions: Vec<Version<'a>>,
    pub projects: Vec<Project<'a>>,
    pub configurations: Vec<Configuration<'a>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version<'a> {
    pub name: &'a str,
    pub version: &'a str,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Project<'a> {
    pub type_id: &'a str,
    pub type_description: &'a str,
    pub id: &'a str,
    pub name: &'a str,
    pub path_or_uri: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configurations: Option<BTreeSet<Configuration<'a>>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Configuration<'a> {
    pub configuration: &'a str,
    pub platform: &'a str,
}

impl<'a> Solution<'a> {
    pub fn from(solution: &'a solp::api::Solution) -> Self {
        let versions = solution
            .versions
            .iter()
            .map(|v| Version {
                name: v.name,
                version: v.ver,
            })
            .collect();
        let project_configs = solution
            .project_configs
            .iter()
            .map(|c| {
                (
                    c.project_id,
                    c.configs
                        .iter()
                        .map(|pc| Configuration {
                            configuration: pc.config,
                            platform: pc.platform,
                        })
                        .collect(),
                )
            })
            .collect::<HashMap<&str, BTreeSet<Configuration>>>();
        let projects = solution
            .projects
            .iter()
            .map(|p| Project {
                type_id: p.type_id,
                type_description: p.type_descr,
                id: p.id,
                name: p.name,
                path_or_uri: p.path_or_uri,
                configurations: project_configs.get(p.id).cloned(),
            })
            .collect();
        let configurations = solution
            .solution_configs
            .iter()
            .map(|c| Configuration {
                configuration: c.config,
                platform: c.platform,
            })
            .collect_vec();
        Self {
            path: solution.path,
            format: solution.format,
            product: solution.product,
            versions,
            projects,
            configurations,
        }
    }
}

impl Json {
    #[must_use]
    pub fn new(pretty: bool) -> Self {
        Self {
            serialized: vec![],
            errors: RefCell::new(Collector::new()),
            pretty,
        }
    }
}

impl Consume for Json {
    fn ok(&mut self, solution: &solp::api::Solution) {
        let conveter = if self.pretty {
            serde_json::to_string_pretty
        } else {
            serde_json::to_string
        };
        let sol = Solution::from(solution);
        if let Ok(s) = conveter(&sol) {
            self.serialized.push(s);
        }
    }

    fn err(&self, path: &str) {
        self.errors.borrow_mut().add_path(path);
    }
}

impl Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let many_solutions = self.serialized.len() > 1;
        if many_solutions {
            write!(f, "[")?;
        }
        for (ix, s) in self.serialized.iter().enumerate() {
            write!(f, "{s}")?;
            if ix < self.serialized.len() - 1 {
                write!(f, ",")?;
            }
        }
        if many_solutions {
            write!(f, "]")?;
        }
        writeln!(f)?;
        if self.errors.borrow().count() > 0 {
            write!(f, "{}", self.errors.borrow())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(true)]
    #[case(false)]
    #[trace]
    fn output_tests(#[case] pretty: bool) {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Json::new(pretty);

        // Act
        validator.ok(&solution);

        // Assert
        let s = format!("{validator}");
        let deserialized = serde_json::from_str::<Solution>(&s);
        assert!(deserialized.is_ok());
        assert_eq!(4, deserialized.unwrap().projects.len())
    }

    const CORRECT_SOLUTION: &str = r#"
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
"#;
}
