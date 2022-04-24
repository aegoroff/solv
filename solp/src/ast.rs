use crate::msbuild;
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, take_until};
use nom::character::complete;
use nom::character::complete::char;
use nom::combinator::recognize;
use nom::error::ParseError;
use nom::error::VerboseError;
use nom::sequence::pair;
use nom::{combinator, sequence, IResult};
use petgraph::prelude::*;

#[derive(Debug)]
pub enum Expr<'input> {
    Comment(&'input str),
    DigitOrDot(&'input str),
    Guid(&'input str),
    Identifier(&'input str),
    Str(&'input str),
    Version(Box<Expr<'input>>, Box<Expr<'input>>),
    FirstLine(Box<Expr<'input>>),
    Global(Vec<Expr<'input>>),
    Project(Box<Expr<'input>>, Vec<Expr<'input>>),
    ProjectBegin(
        Box<Expr<'input>>,
        Box<Expr<'input>>,
        Box<Expr<'input>>,
        Box<Expr<'input>>,
    ),
    Section(Box<Expr<'input>>, Vec<Expr<'input>>),
    SectionBegin(Vec<Expr<'input>>, Box<Expr<'input>>),
    SectionContent(Box<Expr<'input>>, Box<Expr<'input>>),
    SectionKey(Box<Expr<'input>>),
    SectionValue(Box<Expr<'input>>),
}

/// Generates simple &str getters from Expr variants
macro_rules! impl_str_getters {
    ($(($name:ident, $variant:ident)),*) => {
        $(
            pub fn $name(&self) -> &'input str {
                if let Expr::$variant(s) = self {
                    return *s;
                }
                ""
            }
        )*
    };
}

impl<'input> Expr<'input> {
    impl_str_getters!(
        (identifier, Identifier),
        (digit_or_dot, DigitOrDot),
        (string, Str),
        (guid, Guid)
    );

    pub fn is_section(&self, name: &str) -> bool {
        if let Expr::SectionBegin(names, _) = self {
            return names.iter().any(|n| n.identifier() == name);
        }

        false
    }
}

#[derive(Debug, Clone)]
pub struct Solution<'input> {
    pub format: &'input str,
    pub product: &'input str,
    pub projects: Vec<Project<'input>>,
    pub versions: Vec<Version<'input>>,
    pub solution_configs: Vec<Conf<'input>>,
    pub project_configs: Vec<ProjectConfigs<'input>>,
    pub dependencies: DiGraphMap<&'input str, i32>,
}

#[derive(Debug, Copy, Clone)]
pub struct Version<'input> {
    pub name: &'input str,
    pub ver: &'input str,
}

#[derive(Debug, Clone)]
pub struct ProjectConfigs<'input> {
    pub project_id: &'input str,
    pub configs: Vec<Conf<'input>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Conf<'input> {
    pub config: &'input str,
    pub platform: &'input str,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Project<'input> {
    pub type_id: &'input str,
    pub type_descr: &'input str,
    pub id: &'input str,
    pub name: &'input str,
    pub path: &'input str,
}

impl<'input> Default for Solution<'input> {
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

impl<'input> Project<'input> {
    pub fn new(id: &'input str, type_id: &'input str) -> Self {
        let type_descr = msbuild::describe_project(type_id);

        Self {
            type_id,
            type_descr,
            id,
            ..Default::default()
        }
    }

    pub fn from_begin(head: &Expr<'input>) -> Option<Self> {
        if let Expr::ProjectBegin(project_type, name, path, id) = head {
            let prj = Project::from(project_type, name, path, id);
            Some(prj)
        } else {
            None
        }
    }

    pub fn from(
        project_type: &Expr<'input>,
        name: &Expr<'input>,
        path: &Expr<'input>,
        id: &Expr<'input>,
    ) -> Self {
        let type_id = project_type.guid();
        let pid = id.guid();

        let mut prj = Project::new(pid, type_id);
        prj.name = name.string();
        prj.path = path.string();

        prj
    }
}

impl<'input> Version<'input> {
    pub fn new(name: &'input str, ver: &'input str) -> Self {
        Self { name, ver }
    }

    pub fn from(name: &Expr<'input>, val: &Expr<'input>) -> Self {
        let n = name.identifier();
        let v = val.digit_or_dot();
        Version::new(n, v)
    }
}

impl<'input> From<&'input str> for Conf<'input> {
    fn from(s: &'input str) -> Self {
        let r = pipe_terminated::<VerboseError<&str>>(s);
        if let Ok((platform, config)) = r {
            Self { config, platform }
        } else {
            Default::default()
        }
    }
}

impl<'input> Conf<'input> {
    pub fn new(configuration: &'input str, platform: &'input str) -> Self {
        Self {
            config: configuration,
            platform,
        }
    }

    pub fn from_expr(expr: &Expr<'input>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            let conf = Conf::from(left.string());
            Some(conf)
        } else {
            None
        }
    }
}

#[derive(Default, PartialEq, Debug)]
struct ProjectConfig<'input> {
    id: &'input str,
    configuration: &'input str,
    platform: &'input str,
}

impl<'input> ProjectConfigs<'input> {
    pub fn from_id_and_configs(project_id: &'input str, configs: Vec<Conf<'input>>) -> Self {
        let mut configurations = Vec::new();
        configurations.extend(configs);
        Self {
            project_id,
            configs: configurations,
        }
    }

    pub fn from_section_content_key(expr: &Expr<'input>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            ProjectConfigs::from_project_configuration_platform(left.string())
        } else {
            None
        }
    }

    pub fn from_section_content(expr: &Expr<'input>) -> Option<Self> {
        if let Expr::SectionContent(left, right) = expr {
            ProjectConfigs::from_project_configuration(left.string(), right.string())
        } else {
            None
        }
    }

    fn from_project_configuration_platform(k: &'input str) -> Option<Self> {
        let r = ProjectConfigs::parse_project_configuration_platform::<VerboseError<&str>>(k);
        Self::new(r)
    }

    fn from_project_configuration(k: &'input str, v: &'input str) -> Option<Self> {
        let r = ProjectConfigs::parse_project_configuration::<VerboseError<&str>>(k, v);
        Self::new(r)
    }

    fn new(
        r: IResult<&'input str, ProjectConfig<'input>, VerboseError<&'input str>>,
    ) -> Option<Self> {
        if let Ok((_, project_config)) = r {
            let mut configs = Vec::new();

            let config = Conf::new(project_config.configuration, project_config.platform);
            configs.push(config);
            Some(Self {
                project_id: project_config.id,
                configs,
            })
        } else {
            None
        }
    }

    fn parse_project_configuration_platform<'a, E>(
        key: &'a str,
    ) -> IResult<&'a str, ProjectConfig<'a>, E>
    where
        E: ParseError<&'a str> + std::fmt::Debug,
    {
        let parser =
            sequence::separated_pair(guid, char('.'), pair(pipe_terminated, tag_terminated));

        combinator::map(parser, |(project_id, (config, platform))| ProjectConfig {
            id: project_id,
            configuration: config,
            platform,
        })(key)
    }

    fn parse_project_configuration<'a, E>(
        key: &'a str,
        value: &'a str,
    ) -> IResult<&'a str, ProjectConfig<'a>, E>
    where
        E: ParseError<&'a str> + std::fmt::Debug,
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
    use rstest::*;

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
