use crate::msbuild;
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, take_until};
use nom::character::complete;
use nom::character::complete::char;
use nom::combinator::{opt, recognize};
use nom::error::ParseError;
use nom::error::VerboseError;
use nom::sequence::tuple;
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
            return Some(prj);
        }
        None
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
        let r = configuration::<VerboseError<&str>>(s);
        match r {
            Ok((platform, config)) => Self { config, platform },
            _ => Default::default(),
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
            return Some(conf);
        }

        None
    }
}

impl<'input> From<&'input str> for ProjectConfigs<'input> {
    fn from(s: &'input str) -> Self {
        let (_, project_config) =
            ProjectConfigs::parse::<VerboseError<&str>>(s).unwrap_or_default();

        let mut configs = Vec::new();
        let config = Conf::new(project_config.configuration, project_config.platform);
        configs.push(config);
        Self {
            project_id: project_config.id,
            configs,
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

    pub fn new(expr: &Expr<'input>) -> Option<Self> {
        if let Expr::SectionContent(left, _) = expr {
            let conf = ProjectConfigs::from(left.string());
            return Some(conf);
        }

        None
    }

    fn parse<'a, E>(input: &'a str) -> IResult<&'a str, ProjectConfig<'a>, E>
    where
        E: ParseError<&'a str> + std::fmt::Debug,
    {
        let parser =
            sequence::separated_pair(guid, char('.'), tuple((opt(configuration), platform)));

        combinator::map(parser, |(project_id, (config, platform))| match config {
            None => ProjectConfig {
                id: project_id,
                configuration: platform,
                platform: "",
            },
            Some(c) => ProjectConfig {
                id: project_id,
                configuration: c,
                platform,
            },
        })(input)
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

fn platform<'a, E>(input: &'a str) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str> + std::fmt::Debug,
{
    sequence::terminated(
        alt((take_until(".ActiveCfg"), take_until(".Build.0"), take_until(".Deploy.0"))),
        alt((tag(".ActiveCfg"), tag(".Build.0"), tag(".Deploy.0"))),
    )(input)
}

fn configuration<'a, E>(input: &'a str) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str> + std::fmt::Debug,
{
    sequence::terminated(is_not("|"), char('|'))(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use spectral::prelude::*;

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
        assert_that!(c).is_equal_to(expected);
    }

    #[test]
    fn from_project_configurations_correct() {
        // Arrange
        let s = "{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Any CPU.ActiveCfg";

        // Act
        let c = ProjectConfigs::from(s);

        // Assert
        assert_that!(c.project_id).is_equal_to("{27060CA7-FB29-42BC-BA66-7FC80D498354}");
        assert_that!(c.configs).has_length(1);
        assert_that!(c.configs[0].config).is_equal_to("Debug");
        assert_that!(c.configs[0].platform).is_equal_to("Any CPU");
    }

    #[test]
    fn from_project_configurations_config_with_dot() {
        // Arrange
        let s = "{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug .NET 4.0|Any CPU.ActiveCfg";

        // Act
        let c = ProjectConfigs::from(s);

        // Assert
        assert_that!(c.project_id).is_equal_to("{27060CA7-FB29-42BC-BA66-7FC80D498354}");
        assert_that!(c.configs).has_length(1);
        assert_that!(c.configs[0].config).is_equal_to("Debug .NET 4.0");
        assert_that!(c.configs[0].platform).is_equal_to("Any CPU");
    }

    #[test]
    fn from_project_configurations_platform_with_dot_active() {
        // Arrange
        let s = "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.ActiveCfg";

        // Act
        let c = ProjectConfigs::from(s);

        // Assert
        assert_that!(c.project_id).is_equal_to("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}");
        assert_that!(c.configs).has_length(1);
        assert_that!(c.configs[0].config).is_equal_to("Release");
        assert_that!(c.configs[0].platform).is_equal_to(".NET");
    }

    #[test]
    fn from_project_configurations_without_platform() {
        // Arrange
        let s = "{5228E9CE-A216-422F-A5E6-58E95E2DD71D}.DLL Debug.ActiveCfg";

        // Act
        let c = ProjectConfigs::from(s);

        // Assert
        assert_that!(c.project_id).is_equal_to("{5228E9CE-A216-422F-A5E6-58E95E2DD71D}");
        assert_that!(c.configs).has_length(1);
        assert_that!(c.configs[0].config).is_equal_to("DLL Debug");
        assert_that!(c.configs[0].platform).is_equal_to("");
    }

    #[test]
    fn guid_test() {
        // Arrange
        let s = "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.Build.0";

        // Act
        let result = guid::<VerboseError<&str>>(s);

        // Assert
        assert_that!(result).is_equal_to(Ok((
            ".Release|.NET.Build.0",
            "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}",
        )));
    }

    #[rstest]
    #[case(".NET.Build.0", ".NET")]
    #[case(".NET.ActiveCfg", ".NET")]
    #[trace]
    fn platform_tests(#[case] i: &str, #[case] expected: &str) {
        // Arrange

        // Act
        let result = platform::<VerboseError<&str>>(i);

        // Assert
        assert_that!(result).is_equal_to(Ok(("", expected)));
    }

    #[rstest]
    #[case("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.Build.0", ProjectConfig { id: "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}", configuration: "Release", platform: ".NET" })]
    #[case("{60BB14A5-0871-4656-BC38-4F0958230F9A}.Debug|ARM.Deploy.0", ProjectConfig { id: "{60BB14A5-0871-4656-BC38-4F0958230F9A}", configuration: "Debug", platform: "ARM" })]
    #[case("{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}.Release|.NET.ActiveCfg", ProjectConfig { id: "{7C2EF610-BCA0-4D1F-898A-DE9908E4970C}", configuration: "Release", platform: ".NET" })]
    #[case("{5228E9CE-A216-422F-A5E6-58E95E2DD71D}.DLL Debug.ActiveCfg", ProjectConfig { id: "{5228E9CE-A216-422F-A5E6-58E95E2DD71D}", configuration: "DLL Debug", platform: "" })]
    #[trace]
    fn project_configs_parse_tests(#[case] i: &str, #[case] expected: ProjectConfig) {
        // Arrange

        // Act
        let result = ProjectConfigs::parse::<VerboseError<&str>>(i);

        // Assert
        assert_that!(result).is_equal_to(Ok(("", expected)));
    }
}
