use crate::msbuild;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_until},
    character::complete::{self, char},
    combinator::{self, recognize},
    error::{ParseError, VerboseError},
    sequence::{self, pair},
    IResult,
};
use petgraph::prelude::*;

#[derive(Debug)]
pub enum Expr<'a> {
    Comment(&'a str),
    DigitOrDot(&'a str),
    Guid(&'a str),
    Identifier(&'a str),
    Str(&'a str),
    Version(Box<Expr<'a>>, Box<Expr<'a>>),
    FirstLine(Box<Expr<'a>>),
    Global(Vec<Expr<'a>>),
    Project(Box<Expr<'a>>, Vec<Expr<'a>>),
    ProjectBegin(Box<Expr<'a>>, Box<Expr<'a>>, Box<Expr<'a>>, Box<Expr<'a>>),
    Section(Box<Expr<'a>>, Vec<Expr<'a>>),
    SectionBegin(Vec<Expr<'a>>, Box<Expr<'a>>),
    SectionContent(Box<Expr<'a>>, Box<Expr<'a>>),
    SectionKey(Box<Expr<'a>>),
    SectionValue(Box<Expr<'a>>),
}

/// Generates simple &str getters from Expr variants
macro_rules! impl_str_getters {
    ($(($name:ident, $variant:ident)),*) => {
        $(
            #[must_use] pub fn $name(&self) -> &'a str {
                if let Expr::$variant(s) = self {
                    return *s;
                }
                ""
            }
        )*
    };
}

impl<'a> Expr<'a> {
    impl_str_getters!(
        (identifier, Identifier),
        (digit_or_dot, DigitOrDot),
        (string, Str),
        (guid, Guid)
    );

    #[must_use]
    pub fn is_section(&self, name: &str) -> bool {
        if let Expr::SectionBegin(names, _) = self {
            names.iter().any(|n| n.identifier() == name)
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct Solution<'a> {
    pub format: &'a str,
    pub product: &'a str,
    pub projects: Vec<Project<'a>>,
    pub versions: Vec<Version<'a>>,
    pub solution_configs: Vec<Conf<'a>>,
    pub project_configs: Vec<ProjectConfigs<'a>>,
    pub dependencies: DiGraphMap<&'a str, ()>,
}

#[derive(Debug, Copy, Clone)]
pub struct Version<'a> {
    pub name: &'a str,
    pub ver: &'a str,
}

#[derive(Debug, Clone)]
pub struct ProjectConfigs<'a> {
    pub project_id: &'a str,
    pub configs: Vec<Conf<'a>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Conf<'a> {
    pub config: &'a str,
    pub platform: &'a str,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Project<'a> {
    pub type_id: &'a str,
    pub type_descr: &'a str,
    pub id: &'a str,
    pub name: &'a str,
    pub path_or_uri: &'a str,
}

impl<'a> Solution<'a> {
    pub fn iterate_projects(&'a self) -> impl Iterator<Item = &'a Project<'a>> {
        self.projects
            .iter()
            .filter(|p| !msbuild::is_solution_folder(p.type_id))
    }

    pub fn iterate_projects_without_web_sites(&'a self) -> impl Iterator<Item = &'a Project<'a>> {
        self.iterate_projects()
            .filter(|p| !msbuild::is_web_site_project(p.type_id))
    }
}

impl<'a> Default for Solution<'a> {
    fn default() -> Self {
        Self {
            format: "",
            product: "",
            projects: Vec::new(),
            versions: Vec::new(),
            solution_configs: Vec::new(),
            project_configs: Vec::new(),
            dependencies: DiGraphMap::new(),
        }
    }
}

impl<'a> Project<'a> {
    #[must_use]
    pub fn new(id: &'a str, type_id: &'a str) -> Self {
        let type_descr = msbuild::describe_project(type_id);

        Self {
            type_id,
            type_descr,
            id,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn from_begin(head: &Expr<'a>) -> Option<Self> {
        if let Expr::ProjectBegin(project_type, name, path_or_uri, id) = head {
            let prj = Project::from(project_type, name, path_or_uri, id);
            Some(prj)
        } else {
            None
        }
    }

    #[must_use]
    pub fn from(
        project_type: &Expr<'a>,
        name: &Expr<'a>,
        path_or_uri: &Expr<'a>,
        id: &Expr<'a>,
    ) -> Self {
        let type_id = project_type.guid();
        let pid = id.guid();

        let mut prj = Project::new(pid, type_id);
        prj.name = name.string();
        prj.path_or_uri = path_or_uri.string();

        prj
    }
}

impl<'a> Version<'a> {
    #[must_use]
    pub fn new(name: &'a str, ver: &'a str) -> Self {
        Self { name, ver }
    }

    #[must_use]
    pub fn from(name: &Expr<'a>, val: &Expr<'a>) -> Self {
        let n = name.identifier();
        let v = val.digit_or_dot();
        Version::new(n, v)
    }
}

impl<'a> From<&'a str> for Conf<'a> {
    fn from(s: &'a str) -> Self {
        pipe_terminated::<VerboseError<&str>>(s)
            .map(|(platform, config)| Self { config, platform })
            .unwrap_or_default()
    }
}

impl<'a> Conf<'a> {
    #[must_use]
    pub fn new(configuration: &'a str, platform: &'a str) -> Self {
        Self {
            config: configuration,
            platform,
        }
    }

    #[must_use]
    pub fn from_expr(expr: &Expr<'a>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            let conf = Conf::from(left.string());
            Some(conf)
        } else {
            None
        }
    }
}

#[derive(Default, PartialEq, Debug)]
struct ProjectConfig<'a> {
    id: &'a str,
    configuration: &'a str,
    platform: &'a str,
}

impl<'a> ProjectConfigs<'a> {
    #[must_use]
    pub fn from_id_and_configs(project_id: &'a str, configs: Vec<Conf<'a>>) -> Self {
        let mut configurations = Vec::new();
        configurations.extend(configs);
        Self {
            project_id,
            configs: configurations,
        }
    }

    #[must_use]
    pub fn from_section_content_key(expr: &Expr<'a>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            ProjectConfigs::from_project_configuration_platform(left.string())
        } else {
            None
        }
    }

    #[must_use]
    pub fn from_section_content(expr: &Expr<'a>) -> Option<Self> {
        if let Expr::SectionContent(left, right) = expr {
            ProjectConfigs::from_project_configuration(left.string(), right.string())
        } else {
            None
        }
    }

    fn from_project_configuration_platform(k: &'a str) -> Option<Self> {
        let r = ProjectConfigs::parse_project_configuration_platform::<VerboseError<&str>>(k);
        Self::new(r)
    }

    fn from_project_configuration(k: &'a str, v: &'a str) -> Option<Self> {
        let r = ProjectConfigs::parse_project_configuration::<VerboseError<&str>>(k, v);
        Self::new(r)
    }

    fn new(r: IResult<&'a str, ProjectConfig<'a>, VerboseError<&'a str>>) -> Option<Self> {
        r.ok().map(|(_, pc)| Self {
            project_id: pc.id,
            configs: vec![Conf::new(pc.configuration, pc.platform)],
        })
    }

    fn parse_project_configuration_platform<'b, E>(
        key: &'b str,
    ) -> IResult<&'b str, ProjectConfig<'b>, E>
    where
        E: ParseError<&'b str> + std::fmt::Debug,
    {
        let parser =
            sequence::separated_pair(guid, char('.'), pair(pipe_terminated, tag_terminated));

        combinator::map(parser, |(project_id, (config, platform))| ProjectConfig {
            id: project_id,
            configuration: config,
            platform,
        })(key)
    }

    fn parse_project_configuration<'b, E>(
        key: &'b str,
        value: &'b str,
    ) -> IResult<&'b str, ProjectConfig<'b>, E>
    where
        E: ParseError<&'b str> + std::fmt::Debug,
    {
        let parser = sequence::separated_pair(guid, char('.'), tag_terminated);

        let conf = Conf::from(value);

        combinator::map(parser, |(project_id, config)| ProjectConfig {
            id: project_id,
            configuration: config,
            platform: conf.platform,
        })(key)
    }
}

fn guid<'a, E>(input: &'a str) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str> + std::fmt::Debug,
{
    recognize(sequence::delimited(
        complete::char('{'),
        is_not("{}"),
        complete::char('}'),
    ))(input)
}

fn tag_terminated<'a, E>(input: &'a str) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str> + std::fmt::Debug,
{
    const ACTIVE_CFG_TAG: &str = ".ActiveCfg";
    const BUILD_TAG: &str = ".Build.0";
    const DEPLOY_TAG: &str = ".Deploy.0";
    sequence::terminated(
        alt((
            take_until(ACTIVE_CFG_TAG),
            take_until(BUILD_TAG),
            take_until(DEPLOY_TAG),
        )),
        alt((tag(ACTIVE_CFG_TAG), tag(BUILD_TAG), tag(DEPLOY_TAG))),
    )(input)
}

fn pipe_terminated<'a, E>(input: &'a str) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str> + std::fmt::Debug,
{
    sequence::terminated(is_not("|"), char('|'))(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("Release|Any CPU", Conf { config: "Release", platform: "Any CPU" })]
    #[case("", Conf { config: "", platform: "" })]
    #[case("Release Any CPU", Conf { config: "", platform: "" })]
    #[case("Release|Any CPU|test", Conf { config: "Release", platform: "Any CPU|test" })]
    #[trace]
    fn from_configuration_tests(#[case] i: &str, #[case] expected: Conf) {
        // Arrange

        // Act
        let c = Conf::from(i);

        // Assert
        assert_eq!(c, expected);
    }

    #[test]
    fn from_project_configurations_correct() {
        // Arrange
        let s = "{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Any CPU.ActiveCfg";

        // Act
        let c = ProjectConfigs::from_project_configuration_platform(s);

        // Assert
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(c.project_id, "{27060CA7-FB29-42BC-BA66-7FC80D498354}");
        assert_eq!(c.configs.len(), 1);
        assert_eq!(c.configs[0].config, "Debug");
        assert_eq!(c.configs[0].platform, "Any CPU");
    }

    #[test]
    fn from_project_configurations_config_with_dot() {
        // Arrange
        let s = "{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug .NET 4.0|Any CPU.ActiveCfg";

        // Act
        let c = ProjectConfigs::from_project_configuration_platform(s);

        // Assert
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(c.project_id, "{27060CA7-FB29-42BC-BA66-7FC80D498354}");
        assert_eq!(c.configs.len(), 1);
        assert_eq!(c.configs[0].config, "Debug .NET 4.0");
        assert_eq!(c.configs[0].platform, "Any CPU");
    }

    #[test]
    fn from_project_configurations_platform_with_dot_active() {
        // Arrange
        let s = "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.ActiveCfg";

        // Act
        let c = ProjectConfigs::from_project_configuration_platform(s);

        // Assert
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(c.project_id, "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}");
        assert_eq!(c.configs.len(), 1);
        assert_eq!(c.configs[0].config, "Release");
        assert_eq!(c.configs[0].platform, ".NET");
    }

    #[test]
    fn from_project_configurations_without_platform() {
        // Arrange
        let s = "{5228E9CE-A216-422F-A5E6-58E95E2DD71D}.DLL Debug.ActiveCfg";

        // Act
        let c = ProjectConfigs::from_project_configuration_platform(s);

        // Assert
        assert!(c.is_none());
    }

    #[test]
    fn guid_test() {
        // Arrange
        let s = "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.Build.0";

        // Act
        let result = guid::<VerboseError<&str>>(s);

        // Assert
        assert_eq!(
            result,
            Ok((
                ".Release|.NET.Build.0",
                "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}",
            ))
        );
    }

    #[rstest]
    #[case(".NET.Build.0", ".NET")]
    #[case(".NET.ActiveCfg", ".NET")]
    #[trace]
    fn tag_terminated_tests(#[case] i: &str, #[case] expected: &str) {
        // Arrange

        // Act
        let result = tag_terminated::<VerboseError<&str>>(i);

        // Assert
        assert_eq!(result, Ok(("", expected)));
    }

    #[rstest]
    #[case("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.Build.0", ProjectConfig { id: "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}", configuration: "Release", platform: ".NET" })]
    #[case("{60BB14A5-0871-4656-BC38-4F0958230F9A}.Debug|ARM.Deploy.0", ProjectConfig { id: "{60BB14A5-0871-4656-BC38-4F0958230F9A}", configuration: "Debug", platform: "ARM" })]
    #[case("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.ActiveCfg", ProjectConfig { id: "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}", configuration: "Release", platform: ".NET" })]
    #[trace]
    fn project_configs_parse_project_configuration_platform_tests(
        #[case] i: &str,
        #[case] expected: ProjectConfig,
    ) {
        // Arrange

        // Act
        let result = ProjectConfigs::parse_project_configuration_platform::<VerboseError<&str>>(i);

        // Assert
        assert_eq!(result, Ok(("", expected)));
    }

    #[rstest]
    #[case("{5228E9CE-A216-422F-A5E6-58E95E2DD71D}.DLL Debug.ActiveCfg", "Release|x64", ProjectConfig { id: "{5228E9CE-A216-422F-A5E6-58E95E2DD71D}", configuration: "DLL Debug", platform: "x64" })]
    #[trace]
    fn project_configs_parse_project_configuration_tests(
        #[case] k: &str,
        #[case] v: &str,
        #[case] expected: ProjectConfig,
    ) {
        // Arrange

        // Act
        let result = ProjectConfigs::parse_project_configuration::<VerboseError<&str>>(k, v);

        // Assert
        assert_eq!(result, Ok(("", expected)));
    }
}
