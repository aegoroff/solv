use std::collections::BTreeSet;

use miette::Result;

use crate::api::{ProjectConfiguration, Tag};

use super::{
    ConfigurationRule, ConfigurationRulePlatform, Configurations, Project as RawProject,
    ProjectType,
};

const DEFAULT_BUILD_TYPES: &[&str] = &["Debug", "Release"];
const DEFAULT_PLATFORMS: &[&str] = &["Any CPU"];

#[derive(Debug, Default)]
pub struct SolutionConfigNames<'a> {
    pub build_types: Vec<&'a str>,
    pub platforms: Vec<&'a str>,
}

#[derive(Debug)]
pub struct EffectiveRules<'a> {
    pub build_types: Vec<ConfigurationRuleBorrowed<'a>>,
    pub platforms: Vec<ConfigurationRulePlatformBorrowed<'a>>,
    pub builds: Vec<ConfigurationRuleBorrowed<'a>>,
    pub deploys: Vec<ConfigurationRuleBorrowed<'a>>,
    pub is_buildable: bool,
}

impl<'a> Default for EffectiveRules<'a> {
    fn default() -> Self {
        Self {
            build_types: Vec::new(),
            platforms: Vec::new(),
            builds: Vec::new(),
            deploys: Vec::new(),
            is_buildable: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigurationRuleBorrowed<'a> {
    pub solution: Option<&'a str>,
    pub project: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigurationRulePlatformBorrowed<'a> {
    pub solution: Option<&'a str>,
    pub project: &'a str,
}

pub fn solution_build_types<'a>(
    contents: &'a str,
    configs: Option<&Configurations>,
) -> Result<Vec<&'a str>> {
    match configs {
        Some(configs) if !configs.build_types.is_empty() => configs
            .build_types
            .iter()
            .map(|build_type| super::borrow_in(contents, &build_type.name))
            .collect(),
        _ => Ok(DEFAULT_BUILD_TYPES.to_vec()),
    }
}

pub fn solution_platforms<'a>(
    contents: &'a str,
    configs: Option<&Configurations>,
) -> Result<Vec<&'a str>> {
    match configs {
        Some(configs) if !configs.platforms.is_empty() => configs
            .platforms
            .iter()
            .map(|platform| super::borrow_in(contents, &platform.name))
            .collect(),
        _ => Ok(DEFAULT_PLATFORMS.to_vec()),
    }
}

pub fn project_configurations<'a>(
    names: &SolutionConfigNames<'a>,
    rules: &EffectiveRules<'a>,
) -> BTreeSet<ProjectConfiguration<'a>> {
    if !rules.is_buildable {
        return BTreeSet::new();
    }

    let mut configurations = BTreeSet::new();
    for solution_configuration in &names.build_types {
        for solution_platform in &names.platforms {
            let configuration = map_build_type(
                solution_configuration,
                solution_platform,
                &rules.build_types,
            );
            let platform =
                map_platform(solution_configuration, solution_platform, &rules.platforms);
            let mut tags = Vec::new();

            if flag_value(
                solution_configuration,
                solution_platform,
                &rules.builds,
                true,
            ) {
                tags.push(Tag::Build);
            }
            if flag_value(
                solution_configuration,
                solution_platform,
                &rules.deploys,
                false,
            ) {
                tags.push(Tag::Deploy);
            }

            // Configuration without tags is the analogue of ActiveCfg only mapping in .sln
            configurations.insert(ProjectConfiguration {
                configuration,
                solution_configuration,
                platform,
                tags,
            });
        }
    }

    configurations
}

pub fn effective_rules<'a>(
    contents: &'a str,
    configs: Option<&Configurations>,
    project: &RawProject,
) -> Result<EffectiveRules<'a>> {
    let project_type = configs.and_then(|configs| find_project_type(configs, project));
    let mut rules = EffectiveRules {
        is_buildable: project_type
            .and_then(|project_type| project_type.is_buildable)
            .unwrap_or(true),
        ..Default::default()
    };

    if let Some(project_type) = project_type {
        append_type_rules(contents, project_type, &mut rules)?;
    }
    append_project_rules(contents, project, &mut rules)?;

    Ok(rules)
}

fn find_project_type<'a>(
    configs: &'a Configurations,
    project: &RawProject,
) -> Option<&'a ProjectType> {
    if let Some(type_name) = project.project_type.as_deref()
        && let Some(project_type) = configs
            .project_types
            .iter()
            .find(|project_type| project_type.name.as_deref() == Some(type_name))
    {
        return Some(project_type);
    }

    let extension = project
        .path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();

    configs.project_types.iter().find(|project_type| {
        project_type
            .extension
            .as_deref()
            .is_some_and(|configured| configured.eq_ignore_ascii_case(&extension))
    })
}

fn append_type_rules<'a>(
    contents: &'a str,
    project_type: &ProjectType,
    rules: &mut EffectiveRules<'a>,
) -> Result<()> {
    if let Some(is_buildable) = project_type.is_buildable {
        rules.is_buildable = is_buildable;
    }

    for rule in &project_type.build_types {
        rules.build_types.push(borrow_rule(contents, rule)?);
    }
    for rule in &project_type.platforms {
        rules.platforms.push(borrow_platform_rule(contents, rule)?);
    }
    for rule in &project_type.builds {
        rules.builds.push(borrow_rule(contents, rule)?);
    }
    for rule in &project_type.deploys {
        rules.deploys.push(borrow_rule(contents, rule)?);
    }

    Ok(())
}

fn append_project_rules<'a>(
    contents: &'a str,
    project: &RawProject,
    rules: &mut EffectiveRules<'a>,
) -> Result<()> {
    for rule in &project.build_types {
        rules.build_types.push(borrow_rule(contents, rule)?);
    }
    for rule in &project.platforms {
        rules.platforms.push(borrow_platform_rule(contents, rule)?);
    }
    for rule in &project.builds {
        rules.builds.push(borrow_rule(contents, rule)?);
    }
    for rule in &project.deploys {
        rules.deploys.push(borrow_rule(contents, rule)?);
    }

    Ok(())
}

fn borrow_rule<'a>(
    contents: &'a str,
    rule: &ConfigurationRule,
) -> Result<ConfigurationRuleBorrowed<'a>> {
    Ok(ConfigurationRuleBorrowed {
        solution: match rule.solution.as_deref() {
            Some(value) => Some(super::borrow_in(contents, value)?),
            None => None,
        },
        project: match rule.project.as_deref() {
            Some(value) => Some(super::borrow_in(contents, value)?),
            None => None,
        },
    })
}

fn borrow_platform_rule<'a>(
    contents: &'a str,
    rule: &ConfigurationRulePlatform,
) -> Result<ConfigurationRulePlatformBorrowed<'a>> {
    Ok(ConfigurationRulePlatformBorrowed {
        solution: match rule.solution.as_deref() {
            Some(value) => Some(super::borrow_in(contents, value)?),
            None => None,
        },
        project: super::borrow_in(contents, &rule.project)?,
    })
}

fn map_build_type<'a>(
    solution_build_type: &'a str,
    solution_platform: &str,
    rules: &[ConfigurationRuleBorrowed<'a>],
) -> &'a str {
    rules
        .iter()
        .rev()
        .filter(|rule| {
            rule_matches_solution(
                rule.solution,
                Dimension::BuildType,
                solution_build_type,
                solution_platform,
            )
        })
        .find_map(|rule| rule.project)
        .unwrap_or(solution_build_type)
}

fn map_platform<'a>(
    solution_build_type: &str,
    solution_platform: &'a str,
    rules: &[ConfigurationRulePlatformBorrowed<'a>],
) -> &'a str {
    rules
        .iter()
        .rev()
        .find(|rule| {
            rule_matches_solution(
                rule.solution,
                Dimension::Platform,
                solution_build_type,
                solution_platform,
            )
        })
        .map_or(solution_platform, |rule| rule.project)
}

/// Returns the value of the last matching `Build`/`Deploy` rule or `default` when no rule matches.
fn flag_value(
    solution_build_type: &str,
    solution_platform: &str,
    rules: &[ConfigurationRuleBorrowed<'_>],
    default: bool,
) -> bool {
    rules
        .iter()
        .rev()
        .find(|rule| {
            rule_matches_solution(
                rule.solution,
                Dimension::Flag,
                solution_build_type,
                solution_platform,
            )
        })
        .map_or(default, |rule| {
            rule.project
                .is_none_or(|value| !value.trim().eq_ignore_ascii_case("false"))
        })
}

/// Rule dimension. Defines how a bare `Solution` value (without `|`) is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dimension {
    BuildType,
    Platform,
    Flag,
}

/// Matches rule's `Solution` attribute against solution configuration.
///
/// The attribute has `BuildType|Platform` form where any part may be `*` (or empty) meaning all values.
/// A bare value without `|` is accepted leniently: it is treated as a platform for `Platform` rules
/// and as a build type for others.
fn rule_matches_solution(
    rule_solution: Option<&str>,
    dimension: Dimension,
    solution_build_type: &str,
    solution_platform: &str,
) -> bool {
    let Some(rule_solution) = rule_solution else {
        return true;
    };
    let (build_type, platform) = match rule_solution.split_once('|') {
        Some((build_type, platform)) => (build_type, platform),
        None if dimension == Dimension::Platform => ("*", rule_solution),
        None => (rule_solution, "*"),
    };
    part_matches(build_type, solution_build_type) && part_matches(platform, solution_platform)
}

fn part_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.trim();
    pattern.is_empty() || pattern == "*" || pattern.eq_ignore_ascii_case(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    const SLNX_RELEASE_NOT_BUILT: &str = r#"<Solution>
  <Project Path="tests/Tests.csproj">
    <Build Solution="Release|*" Project="false" />
  </Project>
</Solution>"#;

    const SLNX_PLATFORM_MAPPING: &str = r#"<Solution>
  <Configurations>
    <ProjectType Extension="csproj">
      <Platform Solution="*|Any CPU" Project="x64" />
    </ProjectType>
  </Configurations>
  <Project Path="src/App/App.csproj" />
</Solution>"#;

    const SLNX_PROJECT_RULES_OVERRIDE_TYPE_RULES: &str = r#"<Solution>
  <Configurations>
    <ProjectType Extension="vcxproj">
      <Build Project="false" />
    </ProjectType>
  </Configurations>
  <Project Path="native/Native.vcxproj">
    <Build Solution="Debug|*" />
  </Project>
</Solution>"#;

    const SLNX_FULL_RULES: &str = r#"<Solution>
  <Configurations>
    <Platform Name="Any CPU" />
    <Platform Name="x64" />
  </Configurations>
  <Project Path="native/Native.vcxproj">
    <BuildType Solution="Release|*" Project="Debug" />
    <Platform Solution="*|Any CPU" Project="Win32" />
    <Build Solution="Debug|x64" Project="false" />
    <Deploy Solution="Release|x64" />
  </Project>
</Solution>"#;

    fn default_names() -> SolutionConfigNames<'static> {
        SolutionConfigNames {
            build_types: DEFAULT_BUILD_TYPES.to_vec(),
            platforms: DEFAULT_PLATFORMS.to_vec(),
        }
    }

    fn find<'a>(
        configurations: &'a BTreeSet<ProjectConfiguration<'a>>,
        solution_configuration: &str,
        platform: &str,
    ) -> &'a ProjectConfiguration<'a> {
        configurations
            .iter()
            .find(|configuration| {
                configuration.solution_configuration == solution_configuration
                    && configuration.platform == platform
            })
            .expect("project configuration")
    }

    #[test]
    fn default_rules_build_in_debug_and_release() {
        // Arrange
        let names = default_names();
        let rules = EffectiveRules::default();

        // Act
        let configurations = project_configurations(&names, &rules);

        // Assert
        assert_eq!(configurations.len(), 2);
        assert!(
            configurations
                .iter()
                .all(|configuration| configuration.tags == vec![Tag::Build])
        );
    }

    #[test]
    fn not_buildable_project_has_no_configurations() {
        // Arrange
        let names = default_names();
        let rules = EffectiveRules {
            is_buildable: false,
            ..Default::default()
        };

        // Act
        let configurations = project_configurations(&names, &rules);

        // Assert
        assert!(configurations.is_empty());
    }

    #[test]
    fn build_false_rule_excludes_configuration_from_build() {
        // Arrange

        // Act
        let solution = super::super::parse_str(SLNX_RELEASE_NOT_BUILT).unwrap();

        // Assert
        let configurations = solution.projects[0]
            .configurations
            .as_ref()
            .expect("project configurations");
        assert_eq!(configurations.len(), 2);
        assert_eq!(
            find(configurations, "Debug", "Any CPU").tags,
            vec![Tag::Build]
        );
        assert!(find(configurations, "Release", "Any CPU").tags.is_empty());
    }

    #[test]
    fn project_type_platform_mapping_applies_to_matching_extension() {
        // Arrange

        // Act
        let solution = super::super::parse_str(SLNX_PLATFORM_MAPPING).unwrap();

        // Assert
        let configurations = solution.projects[0]
            .configurations
            .as_ref()
            .expect("project configurations");
        assert_eq!(configurations.len(), 2);
        assert!(
            configurations
                .iter()
                .all(|configuration| configuration.platform == "x64")
        );
    }

    #[test]
    fn project_rules_override_project_type_rules() {
        // Arrange

        // Act
        let solution = super::super::parse_str(SLNX_PROJECT_RULES_OVERRIDE_TYPE_RULES).unwrap();

        // Assert
        let configurations = solution.projects[0]
            .configurations
            .as_ref()
            .expect("project configurations");
        assert_eq!(
            find(configurations, "Debug", "Any CPU").tags,
            vec![Tag::Build]
        );
        assert!(find(configurations, "Release", "Any CPU").tags.is_empty());
    }

    // Project configuration doesn't keep solution platform so mapped platform identifies it
    #[test_case("Debug", "Win32", "Debug", vec![Tag::Build] ; "debug any cpu")]
    #[test_case("Debug", "x64", "Debug", vec![] ; "debug x64 not built")]
    #[test_case("Release", "Win32", "Debug", vec![Tag::Build] ; "release any cpu")]
    #[test_case("Release", "x64", "Debug", vec![Tag::Build, Tag::Deploy] ; "release x64 deployed")]
    fn full_rules_are_applied(
        solution_configuration: &str,
        platform: &str,
        expected_configuration: &str,
        expected_tags: Vec<Tag>,
    ) {
        // Arrange
        let solution = super::super::parse_str(SLNX_FULL_RULES).unwrap();
        let configurations = solution.projects[0]
            .configurations
            .as_ref()
            .expect("project configurations");

        // Act
        let actual = find(configurations, solution_configuration, platform);

        // Assert
        assert_eq!(configurations.len(), 4);
        assert_eq!(actual.configuration, expected_configuration);
        assert_eq!(actual.tags, expected_tags);
    }

    #[test]
    fn last_matching_build_type_rule_wins() {
        // Arrange
        let rules = [
            ConfigurationRuleBorrowed {
                solution: Some("*|*"),
                project: Some("First"),
            },
            ConfigurationRuleBorrowed {
                solution: Some("Debug|*"),
                project: Some("Second"),
            },
        ];

        // Act
        let actual = map_build_type("Debug", "Any CPU", &rules);

        // Assert
        assert_eq!(actual, "Second");
    }

    #[test_case(None, true, true ; "no rules uses default true")]
    #[test_case(None, false, false ; "no rules uses default false")]
    #[test_case(Some(None), false, true ; "missing project value means true")]
    #[test_case(Some(Some("false")), true, false ; "false value")]
    #[test_case(Some(Some("False")), true, false ; "false value ignores case")]
    #[test_case(Some(Some("true")), false, true ; "true value")]
    fn flag_value_cases(rule_project: Option<Option<&str>>, default: bool, expected: bool) {
        // Arrange
        let rules = rule_project
            .map(|project| {
                vec![ConfigurationRuleBorrowed {
                    solution: Some("Debug|*"),
                    project,
                }]
            })
            .unwrap_or_default();

        // Act
        let actual = flag_value("Debug", "Any CPU", &rules, default);

        // Assert
        assert_eq!(actual, expected);
    }

    #[test_case(None, Dimension::Flag, "Debug", "x64", true ; "missing solution matches all")]
    #[test_case(Some("Debug|x64"), Dimension::Flag, "Debug", "x64", true ; "full match")]
    #[test_case(Some("Debug|x64"), Dimension::Flag, "Debug", "Any CPU", false ; "platform mismatch")]
    #[test_case(Some("Debug|*"), Dimension::Flag, "Debug", "x64", true ; "any platform")]
    #[test_case(Some("*|x64"), Dimension::Flag, "Release", "x64", true ; "any build type")]
    #[test_case(Some("*|x64"), Dimension::Flag, "Release", "Any CPU", false ; "any build type platform mismatch")]
    #[test_case(Some("*|*"), Dimension::Flag, "Release", "Any CPU", true ; "all wildcards")]
    #[test_case(Some("|x64"), Dimension::Flag, "Release", "x64", true ; "empty build type is wildcard")]
    #[test_case(Some("debug|X64"), Dimension::Flag, "Debug", "x64", true ; "case insensitive")]
    #[test_case(Some("Debug"), Dimension::Flag, "Debug", "x64", true ; "bare value is build type")]
    #[test_case(Some("Debug"), Dimension::BuildType, "Release", "x64", false ; "bare build type mismatch")]
    #[test_case(Some("x64"), Dimension::Platform, "Release", "x64", true ; "bare value is platform for platform rule")]
    #[test_case(Some("x64"), Dimension::Platform, "Release", "Any CPU", false ; "bare platform mismatch")]
    fn rule_matches_solution_cases(
        rule_solution: Option<&str>,
        dimension: Dimension,
        solution_build_type: &str,
        solution_platform: &str,
        expected: bool,
    ) {
        // Arrange

        // Act
        let actual = rule_matches_solution(
            rule_solution,
            dimension,
            solution_build_type,
            solution_platform,
        );

        // Assert
        assert_eq!(actual, expected);
    }
}
