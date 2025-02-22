use crate::msbuild;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{is_not, tag, take_until},
    character::complete::{self, char},
    combinator::{self, recognize},
    error::{Error, ParseError},
    sequence::{self, pair},
};

const ACTIVE_CFG_TAG: &str = ".ActiveCfg";
const BUILD_TAG: &str = ".Build.0";
const DEPLOY_TAG: &str = ".Deploy.0";

/// Represents AST node type
#[derive(Debug)]
pub enum Node<'a> {
    Comment(&'a str),
    Version(&'a str, &'a str),
    FirstLine(&'a str),
    Global(Vec<Node<'a>>),
    Project(Box<Node<'a>>, Vec<Node<'a>>),
    ProjectBegin(&'a str, &'a str, &'a str, &'a str),
    Section(Box<Node<'a>>, Vec<Node<'a>>),
    SectionBegin(Vec<&'a str>, &'a str),
    SectionContent(&'a str, &'a str),
    Solution(Box<Node<'a>>, Vec<Node<'a>>),
}

/// Visual Studio solution file (.sln) model
#[derive(Debug, Clone, Default)]
pub struct Sol<'a> {
    /// Path to solution file. Maybe empty string
    /// because solution can be parsed using memory data.
    pub path: &'a str,
    pub format: &'a str,
    pub product: &'a str,
    pub projects: Vec<Prj<'a>>,
    pub versions: Vec<Ver<'a>>,
    pub solution_configs: Vec<Conf<'a>>,
    pub project_configs: Vec<PrjConfAggregate<'a>>,
}

/// Solution version descriptor
#[derive(Debug, Copy, Clone)]
pub struct Ver<'a> {
    pub name: &'a str,
    pub ver: &'a str,
}

/// Project configurations aggregator
#[derive(Debug, Clone)]
pub struct PrjConfAggregate<'a> {
    pub project_id: &'a str,
    pub configs: Vec<PrjConf<'a>>,
}

/// Configuration and platform pair
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Conf<'a> {
    pub config: &'a str,
    pub platform: &'a str,
}

/// Project model
#[derive(Debug, Clone, Default)]
pub struct Prj<'a> {
    pub type_id: &'a str,
    pub type_descr: &'a str,
    pub id: &'a str,
    pub name: &'a str,
    pub path_or_uri: &'a str,
    pub items: Vec<&'a str>,
    pub depends_from: Vec<&'a str>,
}

impl<'a> Prj<'a> {
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
    pub fn from_begin(head: &Node<'a>) -> Option<Self> {
        if let Node::ProjectBegin(project_type, name, path_or_uri, id) = head {
            let prj = Prj::from(project_type, name, path_or_uri, id);
            Some(prj)
        } else {
            None
        }
    }

    #[must_use]
    pub fn from(project_type: &'a str, name: &'a str, path_or_uri: &'a str, id: &'a str) -> Self {
        let mut prj = Prj::new(id, project_type);
        prj.name = name;
        prj.path_or_uri = path_or_uri;

        prj
    }
}

impl<'a> Ver<'a> {
    #[must_use]
    pub fn new(name: &'a str, ver: &'a str) -> Self {
        Self { name, ver }
    }

    #[must_use]
    pub fn from(name: &'a str, val: &'a str) -> Self {
        Ver::new(name, val)
    }
}

impl<'a> From<&'a str> for Conf<'a> {
    fn from(s: &'a str) -> Self {
        pipe_terminated::<Error<&str>>(s)
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
    pub fn from_node(node: &Node<'a>) -> Option<Self> {
        if let Node::SectionContent(left, _) = node {
            let conf = Conf::from(*left);
            Some(conf)
        } else {
            None
        }
    }
}

#[derive(Default, PartialEq, Debug, Clone)]
pub struct PrjConf<'a> {
    pub id: &'a str,
    pub solution_config: &'a str,
    pub project_config: &'a str,
    pub platform: &'a str,
    pub tag: ProjectConfigTag,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProjectConfigTag {
    #[default]
    ActiveCfg,
    Build,
    Deploy,
}

impl<'a> PrjConfAggregate<'a> {
    #[must_use]
    pub fn from_id_and_configs(project_id: &'a str, configs: Vec<PrjConf<'a>>) -> Self {
        Self {
            project_id,
            configs,
        }
    }

    #[must_use]
    pub fn handle_project_config_platform(node: &Node<'a>) -> Option<Self> {
        if let Node::SectionContent(left, right) = node {
            PrjConfAggregate::from_project_configuration_platform(left, right)
        } else {
            None
        }
    }

    #[must_use]
    pub fn handle_project_config(node: &Node<'a>) -> Option<Self> {
        if let Node::SectionContent(left, right) = node {
            PrjConfAggregate::from_project_configuration(left, right)
        } else {
            None
        }
    }

    fn from_project_configuration_platform(k: &'a str, v: &'a str) -> Option<Self> {
        let r = PrjConfAggregate::parse_project_configuration_platform::<Error<&str>>(k, v);
        Self::new(r)
    }

    fn from_project_configuration(k: &'a str, v: &'a str) -> Option<Self> {
        let r = PrjConfAggregate::parse_project_configuration::<Error<&str>>(k, v);
        Self::new(r)
    }

    fn new(r: IResult<&'a str, PrjConf<'a>, Error<&'a str>>) -> Option<Self> {
        r.ok().map(|(_, pc)| Self {
            project_id: pc.id,
            configs: vec![pc],
        })
    }

    // Configuration, platform parsing made by using nom crate that implement parser combinators
    // method. See more about idea https://en.wikipedia.org/wiki/Parser_combinator

    fn parse_project_configuration_platform<'b, E>(
        key: &'b str,
        value: &'b str,
    ) -> IResult<&'b str, PrjConf<'b>, E>
    where
        E: ParseError<&'b str> + std::fmt::Debug,
    {
        let parser =
            sequence::separated_pair(guid, char('.'), pair(pipe_terminated, tag_terminated));

        let project_conf = Conf::from(value);

        combinator::map(parser, |(project_id, (solution_config, platform))| {
            PrjConf {
                id: project_id,
                solution_config,
                project_config: project_conf.config,
                platform,
                tag: define_tag(key),
            }
        })
        .parse(key)
    }

    fn parse_project_configuration<'b, E>(
        key: &'b str,
        value: &'b str,
    ) -> IResult<&'b str, PrjConf<'b>, E>
    where
        E: ParseError<&'b str> + std::fmt::Debug,
    {
        let parser = sequence::separated_pair(guid, char('.'), tag_terminated);

        let project_conf = Conf::from(value);

        combinator::map(parser, |(project_id, solution_config)| PrjConf {
            id: project_id,
            solution_config,
            project_config: project_conf.config,
            platform: project_conf.platform,
            tag: define_tag(key),
        })
        .parse(key)
    }
}

fn define_tag(key: &str) -> ProjectConfigTag {
    if key.ends_with(BUILD_TAG) {
        ProjectConfigTag::Build
    } else if key.ends_with(DEPLOY_TAG) {
        ProjectConfigTag::Deploy
    } else {
        ProjectConfigTag::ActiveCfg
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
    ))
    .parse(input)
}

fn tag_terminated<'a, E>(input: &'a str) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str> + std::fmt::Debug,
{
    sequence::terminated(
        alt((
            take_until(ACTIVE_CFG_TAG),
            take_until(BUILD_TAG),
            take_until(DEPLOY_TAG),
        )),
        alt((tag(ACTIVE_CFG_TAG), tag(BUILD_TAG), tag(DEPLOY_TAG))),
    )
    .parse(input)
}

fn pipe_terminated<'a, E>(input: &'a str) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str> + std::fmt::Debug,
{
    sequence::terminated(is_not("|"), char('|')).parse(input)
}

impl Node<'_> {
    #[must_use]
    pub fn is_section(&self, name: &str) -> bool {
        if let Node::SectionBegin(names, _) = self {
            names.iter().any(|n| *n == name)
        } else {
            false
        }
    }
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
        let k = "{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Any CPU.ActiveCfg";
        let v = "Debug|x86";

        // Act
        let c = PrjConfAggregate::from_project_configuration_platform(k, v);

        // Assert
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(c.project_id, "{27060CA7-FB29-42BC-BA66-7FC80D498354}");
        assert_eq!(c.configs.len(), 1);
        assert_eq!(c.configs[0].solution_config, "Debug");
        assert_eq!(c.configs[0].project_config, "Debug");
        assert_eq!(c.configs[0].platform, "Any CPU");
    }

    #[test]
    fn from_project_configurations_config_with_dot() {
        // Arrange
        let k = "{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug .NET 4.0|Any CPU.ActiveCfg";
        let v = "Debug|x86";

        // Act
        let c = PrjConfAggregate::from_project_configuration_platform(k, v);

        // Assert
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(c.project_id, "{27060CA7-FB29-42BC-BA66-7FC80D498354}");
        assert_eq!(c.configs.len(), 1);
        assert_eq!(c.configs[0].solution_config, "Debug .NET 4.0");
        assert_eq!(c.configs[0].project_config, "Debug");
        assert_eq!(c.configs[0].platform, "Any CPU");
    }

    #[test]
    fn from_project_configurations_platform_with_dot_active() {
        // Arrange
        let k = "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.ActiveCfg";
        let v = "Release|x86";

        // Act
        let c = PrjConfAggregate::from_project_configuration_platform(k, v);

        // Assert
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(c.project_id, "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}");
        assert_eq!(c.configs.len(), 1);
        assert_eq!(c.configs[0].solution_config, "Release");
        assert_eq!(c.configs[0].project_config, "Release");
        assert_eq!(c.configs[0].platform, ".NET");
    }

    #[test]
    fn from_project_configurations_without_platform() {
        // Arrange
        let k = "{5228E9CE-A216-422F-A5E6-58E95E2DD71D}.DLL Debug.ActiveCfg";
        let v = "Debug|x86";

        // Act
        let c = PrjConfAggregate::from_project_configuration_platform(k, v);

        // Assert
        assert!(c.is_none());
    }

    #[test]
    fn guid_test() {
        // Arrange
        let s = "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.Build.0";

        // Act
        let result = guid::<Error<&str>>(s);

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
        let result = tag_terminated::<Error<&str>>(i);

        // Assert
        assert_eq!(result, Ok(("", expected)));
    }

    #[rstest]
    #[case("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.Build.0", "Release|.NET", PrjConf { id: "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}", solution_config: "Release", project_config: "Release", platform: ".NET", tag: ProjectConfigTag::Build })]
    #[case("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.SolutionRelease|.NET.Build.0", "ProjectRelease|.NET", PrjConf { id: "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}", solution_config: "SolutionRelease", project_config: "ProjectRelease", platform: ".NET", tag: ProjectConfigTag::Build })]
    #[case("{60BB14A5-0871-4656-BC38-4F0958230F9A}.Debug|ARM.Deploy.0", "Debug|ARM", PrjConf { id: "{60BB14A5-0871-4656-BC38-4F0958230F9A}", solution_config: "Debug", project_config: "Debug", platform: "ARM", tag: ProjectConfigTag::Deploy })]
    #[case("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.ActiveCfg", "Release|.NET", PrjConf { id: "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}", solution_config: "Release", project_config: "Release", platform: ".NET", tag: ProjectConfigTag::ActiveCfg })]
    #[trace]
    fn project_configs_parse_project_configuration_platform_tests(
        #[case] k: &str,
        #[case] v: &str,
        #[case] expected: PrjConf,
    ) {
        // Arrange

        // Act
        let result = PrjConfAggregate::parse_project_configuration_platform::<Error<&str>>(k, v);

        // Assert
        assert_eq!(result, Ok(("", expected)));
    }

    #[rstest]
    #[case("{5228E9CE-A216-422F-A5E6-58E95E2DD71D}.DLL Debug.ActiveCfg", "Debug|x64", PrjConf { id: "{5228E9CE-A216-422F-A5E6-58E95E2DD71D}", solution_config: "DLL Debug", project_config: "Debug", platform: "x64", tag: ProjectConfigTag::ActiveCfg })]
    #[trace]
    fn project_configs_parse_project_configuration_tests(
        #[case] k: &str,
        #[case] v: &str,
        #[case] expected: PrjConf,
    ) {
        // Arrange

        // Act
        let result = PrjConfAggregate::parse_project_configuration::<Error<&str>>(k, v);

        // Assert
        assert_eq!(result, Ok(("", expected)));
    }
}
