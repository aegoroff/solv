use crate::api::{Conf, Project, ProjectConfigs, Solution, Version};
use crate::ast::Node;
use itertools::Itertools;
use std::collections::HashSet;
use std::option::Option::Some;

const UTF8_BOM: &[u8; 3] = b"\xEF\xBB\xBF";

trait Visitor<'a> {
    fn visit(&self, solution: Solution<'a>, node: &Node<'a>) -> Solution<'a>;
}

/// Perses solution from string into Solution model instance
pub fn parse_str(contents: &str) -> Option<Solution> {
    if contents.len() < UTF8_BOM.len() {
        return None;
    }
    let cb = contents.as_bytes();
    // Skip UTF-8 signature if necessary
    let input = if &cb[0..UTF8_BOM.len()] == UTF8_BOM {
        &contents[UTF8_BOM.len()..]
    } else {
        contents
    };

    let parser = crate::solp::SolutionParser::new();
    let lexer = crate::lex::Lexer::new(input);
    match parser.parse(input, lexer) {
        Ok(parsed) => {
            let solution = Solution::default();
            let visitor = SolutionVisitor::new();
            Some(visitor.visit(solution, &parsed))
        }
        Err(e) => {
            if cfg!(debug_assertions) {
                println!("{e:?}");
            }
            None
        }
    }
}

macro_rules! section_content {
    ($s:ident, $n:expr) => {{
        if let Node::Section(begin, content) = $s {
            begin.is_section($n).then_some(content)
        } else {
            None
        }
    }};
}

#[derive(Debug)]
struct SolutionVisitor {}

impl SolutionVisitor {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'a> Visitor<'a> for SolutionVisitor {
    fn visit(&self, solution: Solution<'a>, node: &Node<'a>) -> Solution<'a> {
        let mut s = solution;
        if let Node::Solution(first_line, lines) = node {
            if let Node::FirstLine(ver) = first_line.as_ref() {
                s.format = ver.digit_or_dot();
            }

            s = lines.iter().fold(s, |mut s, line| {
                s = ProjectVisitor::new().visit(s, line);
                s = VersionVisitor::new().visit(s, line);
                s = GlobalVisitor::new().visit(s, line);
                s = CommentVisitor::new().visit(s, line);
                s
            });
        }
        s
    }
}

#[derive(Debug)]
struct ProjectVisitor {}

impl ProjectVisitor {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'a> Visitor<'a> for ProjectVisitor {
    fn visit(&self, mut solution: Solution<'a>, node: &Node<'a>) -> Solution<'a> {
        if let Node::Project(head, sections) = node {
            if let Some(p) = Project::from_begin(head) {
                solution.projects.push(p);
                solution.dependencies.add_node(p.id);
            }

            if let Some(last_project) = solution.projects.last() {
                let edges = sections
                    .iter()
                    .filter_map(|sect| section_content!(sect, "ProjectDependencies"))
                    .flatten()
                    .filter_map(|expr| match expr {
                        Node::SectionContent(left, _) => Some((left.string(), last_project.id)),
                        _ => None,
                    });

                solution.dependencies.extend(edges);
            }
        }
        solution
    }
}

#[derive(Debug)]
struct VersionVisitor {}

impl VersionVisitor {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'a> Visitor<'a> for VersionVisitor {
    fn visit(&self, mut solution: Solution<'a>, node: &Node<'a>) -> Solution<'a> {
        if let Node::Version(name, val) = node {
            let version = Version::from(name, val);
            solution.versions.push(version);
        }
        solution
    }
}

/// Glogal section node visitor
#[derive(Debug)]
struct GlobalVisitor {}

impl GlobalVisitor {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'a> Visitor<'a> for GlobalVisitor {
    fn visit(&self, mut solution: Solution<'a>, node: &Node<'a>) -> Solution<'a> {
        if let Node::Global(sections) = node {
            let configs_and_platforms = sections
                .iter()
                .filter_map(|sect| section_content!(sect, "SolutionConfigurationPlatforms"))
                .flatten()
                .filter_map(Conf::from_node);

            solution.solution_configs.extend(configs_and_platforms);

            let project_config_platform_grp = sections
                .iter()
                .filter_map(|sect| section_content!(sect, "ProjectConfigurationPlatforms"))
                .flatten()
                .filter_map(ProjectConfigs::from_section_content_key)
                .group_by(|x| x.project_id);

            let project_configs_platforms =
                project_config_platform_grp
                    .into_iter()
                    .map(|(pid, project_configs)| {
                        let c = project_configs.flat_map(|c| c.configs).collect();
                        ProjectConfigs::from_id_and_configs(pid, c)
                    });
            solution.project_configs.extend(project_configs_platforms);

            let project_configs = sections
                .iter()
                .filter_map(|sect| section_content!(sect, "ProjectConfiguration"))
                .flatten()
                .filter_map(ProjectConfigs::from_section_content)
                .group_by(|x| x.project_id)
                .into_iter()
                .map(|(pid, project_configs)| {
                    let c = project_configs.flat_map(|c| c.configs).collect();
                    ProjectConfigs::from_id_and_configs(pid, c)
                })
                .collect_vec();

            let solution_configurations = sections
                .iter()
                .filter_map(|sect| section_content!(sect, "SolutionConfiguration"))
                .flatten()
                .filter_map(|expr| match expr {
                    Node::SectionContent(_, right) => Some(right.string()),
                    _ => None,
                })
                .collect::<HashSet<&str>>();

            let from_project_configurations = project_configs
                .iter()
                .flat_map(|pc| pc.configs.iter())
                .filter(|c| solution_configurations.contains(c.config));
            solution
                .solution_configs
                .extend(from_project_configurations);

            solution.project_configs.extend(project_configs);
        }
        solution
    }
}

#[derive(Debug)]
struct CommentVisitor {}

impl CommentVisitor {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'a> Visitor<'a> for CommentVisitor {
    fn visit(&self, mut solution: Solution<'a>, node: &Node<'a>) -> Solution<'a> {
        if let Node::Comment(s) = node {
            // Only comment text without sharp sign and spacess
            let skip: &[_] = &['#', ' ', '\t'];
            solution.product = s.trim_start_matches(skip);
        }
        solution
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::Lexer;
    use petgraph::dot::{Config, Dot};
    use proptest::prelude::*;
    use rstest::rstest;

    #[rstest]
    #[case("")]
    #[case("123243")]
    #[case("ZZ(1Z\t22\"2")]
    #[case("Z\u{1}\u{365}\u{b}\n\u{0}\u{0}")]
    #[case("\rXZZ,\rM2Section(\r    =2     =2")]
    #[case("ZZ\t)X\t)X,\t0#  溾\n\t\t)E(Z)E#溾")]
    #[case("A\n\t = \n0")]
    #[case("\rYXZZ,\rM2)Section()\r\r))ZZ,\u{1}\t)X9Z)Z\u{fa970}Tz\u{1}\u{fa970}`\u{1}\u{fa970}Tz\u{1}\u{ea970}=\u{1}\u{11}\u{0}MZG\u{0}\u{1}\u{11}\u{0}\u{1}\u{fa970}Tz\u{1}\u{fa970}`\u{1}\u{fa970}Tz\u{1}\u{fa970}\non()\r)YA,\rM1\rKg\u{17}Y)\u{6}")]
    #[case("\nMicrosoft Visual Studio Solution File, Format Version 12.00\n# Visual Studio 2013\nVisualStudioVersion = 12.0.31101.0\nMinimumVisualStudioVersion = 10.0.40219.1\nProject(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Grok\", \"Grok\\Grok.csproj\", \"{EC6D1E9B-2DA0-4225-9109-E9CF1C924116}\"\nEndProject\nGlobal\n\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\t\tDebug|Any CPU = Debug|Any CPU\n\t\tRelease|Any CPU = Release|Any CPU\n\tEnnGlobalSectionease|Any CPU = Release|Any CPU\\n\\tEnnGlobalSection\\n\\tGlobalSection(ProjectConfigurationPlatforms) = postSolution\\n\\t\\t{EC6D1E9B-2DA0-4225-9109-E9CF1C924116}.Debug|Any CPU.ActiveCfg = Debug|Ady CPU\\n\\t\\t{EC6D1E9B-2DA0-4225-9109-E9CF1C924116}.Debug|Any CPU.")]
    #[case("\nMicrosoft Visual Studio Solution File, Format Version 12.00\n# Visual Studio 2013\nVisualStudioVersion = 12.0.31101.0\nMinimumVisualStudioVersion = 10.0.40219.1\nProject(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Grok\", \"Grok\\Grok.csproj\", \"{EC6D1E9B-2DA0-4225-9109-E9CF1C924116}\"\nEndProject\nGlobal\n\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\t\tDebug|Any CPU = Deb")]
    #[trace]
    fn parse_str_crashes_found_by_fuzz(#[case] content: &str) {
        // Act
        let result = parse_str(content);

        // Assert
        assert!(result.is_none());
    }

    proptest! {
        #[test]
        fn parse_arbitrary_str(s in "\\PC*") {
            // Act
            let result = parse_str(&s);

            // Assert
            assert!(result.is_none());
        }
    }

    #[test]
    fn parse_str_real_solution() {
        // Act
        let result = parse_str(REAL_SOLUTION);

        // Assert
        assert!(result.is_some());
        let solution = result.unwrap();
        assert_eq!(solution.projects.len(), 10);
        assert_eq!(solution.iterate_projects().count(), 8); // solution folders excluded
        assert_eq!(solution.dependencies.node_count(), 10);
        assert_eq!(solution.dependencies.edge_count(), 4);
        assert_eq!(solution.format, "12.00");
        assert_eq!(solution.product, "Visual Studio 15");
        println!(
            "{:?}",
            Dot::with_config(&solution.dependencies, &[Config::EdgeNoLabel])
        );
    }

    #[test]
    fn parse_str_no_line_break() {
        // Arrange
        let mut binary = Vec::new();
        binary.extend_from_slice(UTF8_BOM);
        binary.extend_from_slice(REAL_SOLUTION.as_bytes());
        let sln = String::from_utf8(binary).unwrap();

        // Act
        let result = parse_str(&sln);

        // Assert
        assert!(result.is_some());
    }

    #[test]
    fn parse_str_start_from_utf_8_signature() {
        // Arrange
        let sln = REAL_SOLUTION.trim_end();

        // Act
        let result = parse_str(sln);

        // Assert
        assert!(result.is_some());
    }

    #[test]
    fn parser_version8_solution() {
        // Arrange

        // Act
        let sln = parse_str(VERSION8_SOLUTION);

        // Assert
        assert!(sln.is_some());
    }

    #[test]
    fn lex_version8_solution() {
        let lexer = Lexer::new(VERSION8_SOLUTION);
        for tok in lexer {
            println!("{tok:#?}");
        }
    }

    #[test]
    fn parse_str_apr_generated_solution() {
        // Arrange

        // Act
        let sln = parse_str(APR_SOLUTION);

        // Assert
        assert!(sln.is_some());
    }

    #[test]
    fn parse_str_apr_generated_solution_with_leading_whitespaces() {
        // Arrange
        let solution = format!("   \t{APR_SOLUTION}");

        // Act
        let sln = parse_str(&solution);

        // Assert
        assert!(sln.is_some());
    }

    #[test]
    fn lex_apr_generated_solution() {
        let lexer = Lexer::new(APR_SOLUTION);
        for tok in lexer {
            println!("{tok:#?}");
        }
    }

    const REAL_SOLUTION: &str = r#"
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

    const VERSION8_SOLUTION: &str = r#"
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

    const APR_SOLUTION: &str = r#"Microsoft Visual Studio Solution File, Format Version 12.00
# Visual Studio 2013
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "ALL_BUILD", "ALL_BUILD.vcxproj", "{BBF8893C-A160-3C70-B90B-535F5E3312C9}"
	ProjectSection(ProjectDependencies) = postProject
		{B26E4563-5F01-3488-9242-EAB29C8F9513} = {B26E4563-5F01-3488-9242-EAB29C8F9513}
		{68964C8B-1971-3532-88C5-533804C9BFDB} = {68964C8B-1971-3532-88C5-533804C9BFDB}
		{A359F328-78FA-3DD7-ADC4-FA4319B010F4} = {A359F328-78FA-3DD7-ADC4-FA4319B010F4}
		{1276D7BA-8FF1-38C1-A6B9-6068D5E5B722} = {1276D7BA-8FF1-38C1-A6B9-6068D5E5B722}
		{BBD76E2D-0604-3335-B756-A1D4A90FF9E0} = {BBD76E2D-0604-3335-B756-A1D4A90FF9E0}
		{64126389-3479-392A-8F9A-16A536FB7502} = {64126389-3479-392A-8F9A-16A536FB7502}
		{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC} = {E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "INSTALL", "INSTALL.vcxproj", "{E8CF42A2-27E7-378D-A954-E757587CCCB5}"
	ProjectSection(ProjectDependencies) = postProject
		{BBF8893C-A160-3C70-B90B-535F5E3312C9} = {BBF8893C-A160-3C70-B90B-535F5E3312C9}
		{B26E4563-5F01-3488-9242-EAB29C8F9513} = {B26E4563-5F01-3488-9242-EAB29C8F9513}
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "ZERO_CHECK", "ZERO_CHECK.vcxproj", "{B26E4563-5F01-3488-9242-EAB29C8F9513}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "apr-1", "apr-1.vcxproj", "{68964C8B-1971-3532-88C5-533804C9BFDB}"
	ProjectSection(ProjectDependencies) = postProject
		{B26E4563-5F01-3488-9242-EAB29C8F9513} = {B26E4563-5F01-3488-9242-EAB29C8F9513}
		{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC} = {E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "aprapp-1", "aprapp-1.vcxproj", "{A359F328-78FA-3DD7-ADC4-FA4319B010F4}"
	ProjectSection(ProjectDependencies) = postProject
		{B26E4563-5F01-3488-9242-EAB29C8F9513} = {B26E4563-5F01-3488-9242-EAB29C8F9513}
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gen_test_char", "gen_test_char.vcxproj", "{1276D7BA-8FF1-38C1-A6B9-6068D5E5B722}"
	ProjectSection(ProjectDependencies) = postProject
		{B26E4563-5F01-3488-9242-EAB29C8F9513} = {B26E4563-5F01-3488-9242-EAB29C8F9513}
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "libapr-1", "libapr-1.vcxproj", "{BBD76E2D-0604-3335-B756-A1D4A90FF9E0}"
	ProjectSection(ProjectDependencies) = postProject
		{B26E4563-5F01-3488-9242-EAB29C8F9513} = {B26E4563-5F01-3488-9242-EAB29C8F9513}
		{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC} = {E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "libaprapp-1", "libaprapp-1.vcxproj", "{64126389-3479-392A-8F9A-16A536FB7502}"
	ProjectSection(ProjectDependencies) = postProject
		{B26E4563-5F01-3488-9242-EAB29C8F9513} = {B26E4563-5F01-3488-9242-EAB29C8F9513}
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "test_char_header", "test_char_header.vcxproj", "{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}"
	ProjectSection(ProjectDependencies) = postProject
		{B26E4563-5F01-3488-9242-EAB29C8F9513} = {B26E4563-5F01-3488-9242-EAB29C8F9513}
		{1276D7BA-8FF1-38C1-A6B9-6068D5E5B722} = {1276D7BA-8FF1-38C1-A6B9-6068D5E5B722}
	EndProjectSection
EndProject
Global
	GlobalSection(SolutionConfigurationPlatforms) = preSolution
		Debug|Win32 = Debug|Win32
		Release|Win32 = Release|Win32
		MinSizeRel|Win32 = MinSizeRel|Win32
		RelWithDebInfo|Win32 = RelWithDebInfo|Win32
	EndGlobalSection
	GlobalSection(ProjectConfigurationPlatforms) = postSolution
		{BBF8893C-A160-3C70-B90B-535F5E3312C9}.Debug|Win32.ActiveCfg = Debug|Win32
		{BBF8893C-A160-3C70-B90B-535F5E3312C9}.Debug|Win32.Build.0 = Debug|Win32
		{BBF8893C-A160-3C70-B90B-535F5E3312C9}.Release|Win32.ActiveCfg = Release|Win32
		{BBF8893C-A160-3C70-B90B-535F5E3312C9}.Release|Win32.Build.0 = Release|Win32
		{BBF8893C-A160-3C70-B90B-535F5E3312C9}.MinSizeRel|Win32.ActiveCfg = MinSizeRel|Win32
		{BBF8893C-A160-3C70-B90B-535F5E3312C9}.MinSizeRel|Win32.Build.0 = MinSizeRel|Win32
		{BBF8893C-A160-3C70-B90B-535F5E3312C9}.RelWithDebInfo|Win32.ActiveCfg = RelWithDebInfo|Win32
		{BBF8893C-A160-3C70-B90B-535F5E3312C9}.RelWithDebInfo|Win32.Build.0 = RelWithDebInfo|Win32
		{E8CF42A2-27E7-378D-A954-E757587CCCB5}.Debug|Win32.ActiveCfg = Debug|Win32
		{E8CF42A2-27E7-378D-A954-E757587CCCB5}.Release|Win32.ActiveCfg = Release|Win32
		{E8CF42A2-27E7-378D-A954-E757587CCCB5}.MinSizeRel|Win32.ActiveCfg = MinSizeRel|Win32
		{E8CF42A2-27E7-378D-A954-E757587CCCB5}.RelWithDebInfo|Win32.ActiveCfg = RelWithDebInfo|Win32
		{B26E4563-5F01-3488-9242-EAB29C8F9513}.Debug|Win32.ActiveCfg = Debug|Win32
		{B26E4563-5F01-3488-9242-EAB29C8F9513}.Debug|Win32.Build.0 = Debug|Win32
		{B26E4563-5F01-3488-9242-EAB29C8F9513}.Release|Win32.ActiveCfg = Release|Win32
		{B26E4563-5F01-3488-9242-EAB29C8F9513}.Release|Win32.Build.0 = Release|Win32
		{B26E4563-5F01-3488-9242-EAB29C8F9513}.MinSizeRel|Win32.ActiveCfg = MinSizeRel|Win32
		{B26E4563-5F01-3488-9242-EAB29C8F9513}.MinSizeRel|Win32.Build.0 = MinSizeRel|Win32
		{B26E4563-5F01-3488-9242-EAB29C8F9513}.RelWithDebInfo|Win32.ActiveCfg = RelWithDebInfo|Win32
		{B26E4563-5F01-3488-9242-EAB29C8F9513}.RelWithDebInfo|Win32.Build.0 = RelWithDebInfo|Win32
		{68964C8B-1971-3532-88C5-533804C9BFDB}.Debug|Win32.ActiveCfg = Debug|Win32
		{68964C8B-1971-3532-88C5-533804C9BFDB}.Debug|Win32.Build.0 = Debug|Win32
		{68964C8B-1971-3532-88C5-533804C9BFDB}.Release|Win32.ActiveCfg = Release|Win32
		{68964C8B-1971-3532-88C5-533804C9BFDB}.Release|Win32.Build.0 = Release|Win32
		{68964C8B-1971-3532-88C5-533804C9BFDB}.MinSizeRel|Win32.ActiveCfg = MinSizeRel|Win32
		{68964C8B-1971-3532-88C5-533804C9BFDB}.MinSizeRel|Win32.Build.0 = MinSizeRel|Win32
		{68964C8B-1971-3532-88C5-533804C9BFDB}.RelWithDebInfo|Win32.ActiveCfg = RelWithDebInfo|Win32
		{68964C8B-1971-3532-88C5-533804C9BFDB}.RelWithDebInfo|Win32.Build.0 = RelWithDebInfo|Win32
		{A359F328-78FA-3DD7-ADC4-FA4319B010F4}.Debug|Win32.ActiveCfg = Debug|Win32
		{A359F328-78FA-3DD7-ADC4-FA4319B010F4}.Debug|Win32.Build.0 = Debug|Win32
		{A359F328-78FA-3DD7-ADC4-FA4319B010F4}.Release|Win32.ActiveCfg = Release|Win32
		{A359F328-78FA-3DD7-ADC4-FA4319B010F4}.Release|Win32.Build.0 = Release|Win32
		{A359F328-78FA-3DD7-ADC4-FA4319B010F4}.MinSizeRel|Win32.ActiveCfg = MinSizeRel|Win32
		{A359F328-78FA-3DD7-ADC4-FA4319B010F4}.MinSizeRel|Win32.Build.0 = MinSizeRel|Win32
		{A359F328-78FA-3DD7-ADC4-FA4319B010F4}.RelWithDebInfo|Win32.ActiveCfg = RelWithDebInfo|Win32
		{A359F328-78FA-3DD7-ADC4-FA4319B010F4}.RelWithDebInfo|Win32.Build.0 = RelWithDebInfo|Win32
		{1276D7BA-8FF1-38C1-A6B9-6068D5E5B722}.Debug|Win32.ActiveCfg = Debug|Win32
		{1276D7BA-8FF1-38C1-A6B9-6068D5E5B722}.Debug|Win32.Build.0 = Debug|Win32
		{1276D7BA-8FF1-38C1-A6B9-6068D5E5B722}.Release|Win32.ActiveCfg = Release|Win32
		{1276D7BA-8FF1-38C1-A6B9-6068D5E5B722}.Release|Win32.Build.0 = Release|Win32
		{1276D7BA-8FF1-38C1-A6B9-6068D5E5B722}.MinSizeRel|Win32.ActiveCfg = MinSizeRel|Win32
		{1276D7BA-8FF1-38C1-A6B9-6068D5E5B722}.MinSizeRel|Win32.Build.0 = MinSizeRel|Win32
		{1276D7BA-8FF1-38C1-A6B9-6068D5E5B722}.RelWithDebInfo|Win32.ActiveCfg = RelWithDebInfo|Win32
		{1276D7BA-8FF1-38C1-A6B9-6068D5E5B722}.RelWithDebInfo|Win32.Build.0 = RelWithDebInfo|Win32
		{BBD76E2D-0604-3335-B756-A1D4A90FF9E0}.Debug|Win32.ActiveCfg = Debug|Win32
		{BBD76E2D-0604-3335-B756-A1D4A90FF9E0}.Debug|Win32.Build.0 = Debug|Win32
		{BBD76E2D-0604-3335-B756-A1D4A90FF9E0}.Release|Win32.ActiveCfg = Release|Win32
		{BBD76E2D-0604-3335-B756-A1D4A90FF9E0}.Release|Win32.Build.0 = Release|Win32
		{BBD76E2D-0604-3335-B756-A1D4A90FF9E0}.MinSizeRel|Win32.ActiveCfg = MinSizeRel|Win32
		{BBD76E2D-0604-3335-B756-A1D4A90FF9E0}.MinSizeRel|Win32.Build.0 = MinSizeRel|Win32
		{BBD76E2D-0604-3335-B756-A1D4A90FF9E0}.RelWithDebInfo|Win32.ActiveCfg = RelWithDebInfo|Win32
		{BBD76E2D-0604-3335-B756-A1D4A90FF9E0}.RelWithDebInfo|Win32.Build.0 = RelWithDebInfo|Win32
		{64126389-3479-392A-8F9A-16A536FB7502}.Debug|Win32.ActiveCfg = Debug|Win32
		{64126389-3479-392A-8F9A-16A536FB7502}.Debug|Win32.Build.0 = Debug|Win32
		{64126389-3479-392A-8F9A-16A536FB7502}.Release|Win32.ActiveCfg = Release|Win32
		{64126389-3479-392A-8F9A-16A536FB7502}.Release|Win32.Build.0 = Release|Win32
		{64126389-3479-392A-8F9A-16A536FB7502}.MinSizeRel|Win32.ActiveCfg = MinSizeRel|Win32
		{64126389-3479-392A-8F9A-16A536FB7502}.MinSizeRel|Win32.Build.0 = MinSizeRel|Win32
		{64126389-3479-392A-8F9A-16A536FB7502}.RelWithDebInfo|Win32.ActiveCfg = RelWithDebInfo|Win32
		{64126389-3479-392A-8F9A-16A536FB7502}.RelWithDebInfo|Win32.Build.0 = RelWithDebInfo|Win32
		{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}.Debug|Win32.ActiveCfg = Debug|Win32
		{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}.Debug|Win32.Build.0 = Debug|Win32
		{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}.Release|Win32.ActiveCfg = Release|Win32
		{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}.Release|Win32.Build.0 = Release|Win32
		{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}.MinSizeRel|Win32.ActiveCfg = MinSizeRel|Win32
		{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}.MinSizeRel|Win32.Build.0 = MinSizeRel|Win32
		{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}.RelWithDebInfo|Win32.ActiveCfg = RelWithDebInfo|Win32
		{E66A19F4-86EC-35C1-B2CF-985D6AC8E7DC}.RelWithDebInfo|Win32.Build.0 = RelWithDebInfo|Win32
	EndGlobalSection
	GlobalSection(ExtensibilityGlobals) = postSolution
		SolutionGuid = {A13EFA7E-93E5-3AA8-85BA-838151D3EF23}
	EndGlobalSection
	GlobalSection(ExtensibilityAddIns) = postSolution
	EndGlobalSection
EndGlobal
"#;
}
