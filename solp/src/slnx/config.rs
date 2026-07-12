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
            let configuration = map_build_type(solution_configuration, &rules.build_types);
            let platform = map_platform(solution_platform, &rules.platforms);
            let mut tags = Vec::new();

            if should_build(solution_configuration, &rules.builds) {
                tags.push(Tag::Build);
            }
            if should_deploy(solution_configuration, &rules.deploys) {
                tags.push(Tag::Deploy);
            }

            if tags.is_empty() {
                continue;
            }

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

fn find_project_type<'a>(configs: &'a Configurations, project: &RawProject) -> Option<&'a ProjectType> {
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

fn map_build_type<'a>(solution_configuration: &'a str, rules: &[ConfigurationRuleBorrowed<'a>]) -> &'a str {
    rules
        .iter()
        .find(|rule| rule_matches_solution(rule.solution, solution_configuration))
        .and_then(|rule| rule.project)
        .unwrap_or(solution_configuration)
}

fn map_platform<'a>(solution_platform: &'a str, rules: &[ConfigurationRulePlatformBorrowed<'a>]) -> &'a str {
    rules
        .iter()
        .find(|rule| rule_matches_solution(rule.solution, solution_platform))
        .map(|rule| rule.project)
        .unwrap_or(solution_platform)
}

fn should_build(solution_configuration: &str, rules: &[ConfigurationRuleBorrowed<'_>]) -> bool {
    if rules.is_empty() {
        return true;
    }

    rules
        .iter()
        .any(|rule| rule_matches_solution(rule.solution, solution_configuration))
}

fn should_deploy(solution_configuration: &str, rules: &[ConfigurationRuleBorrowed<'_>]) -> bool {
    rules
        .iter()
        .any(|rule| rule_matches_solution(rule.solution, solution_configuration))
}

fn rule_matches_solution(rule_solution: Option<&str>, solution_configuration: &str) -> bool {
    rule_solution.is_none_or(|value| value == solution_configuration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    const SLNX_DEBUG_ONLY_BUILD: &str = r#"<Solution>
  <Project Path="tests/Tests.csproj">
    <Build Solution="Debug" />
  </Project>
</Solution>"#;

    const SLNX_PLATFORM_MAPPING: &str = r#"<Solution>
  <Configurations>
    <ProjectType Extension="csproj">
      <Platform Solution="Any CPU" Project="x64" />
    </ProjectType>
  </Configurations>
  <Project Path="src/App/App.csproj" />
</Solution>"#;

    #[test]
    fn default_rules_build_in_debug_and_release() {
        // Arrange
        let names = SolutionConfigNames {
            build_types: DEFAULT_BUILD_TYPES.to_vec(),
            platforms: DEFAULT_PLATFORMS.to_vec(),
        };
        let rules = EffectiveRules::default();

        // Act
        let configurations = project_configurations(&names, &rules);

        // Assert
        assert_eq!(configurations.len(), 2);
        assert!(configurations
            .iter()
            .all(|configuration| configuration.tags == vec![Tag::Build]));
    }

    #[test]
    fn debug_only_build_rule_limits_configurations() {
        // Arrange

        // Act
        let solution = super::super::parse_str(SLNX_DEBUG_ONLY_BUILD).unwrap();

        // Assert
        let configurations = solution.projects[0]
            .configurations
            .as_ref()
            .expect("project configurations");
        assert_eq!(configurations.len(), 1);
        let configuration = configurations.iter().next().unwrap();
        assert_eq!(configuration.solution_configuration, "Debug");
        assert_eq!(configuration.tags, vec![Tag::Build]);
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
        assert!(configurations
            .iter()
            .all(|configuration| configuration.platform == "x64"));
    }

    #[test]
    fn debug_only_build_rule_limits_configurations_unit() {
        // Arrange
        let names = SolutionConfigNames {
            build_types: DEFAULT_BUILD_TYPES.to_vec(),
            platforms: DEFAULT_PLATFORMS.to_vec(),
        };
        let rules = EffectiveRules {
            builds: vec![ConfigurationRuleBorrowed {
                solution: Some("Debug"),
                project: None,
            }],
            ..Default::default()
        };

        // Act
        let configurations = project_configurations(&names, &rules);

        // Assert
        assert_eq!(configurations.len(), 1);
    }

    #[test]
    fn project_type_platform_mapping_applies_unit() {
        // Arrange
        let names = SolutionConfigNames {
            build_types: DEFAULT_BUILD_TYPES.to_vec(),
            platforms: DEFAULT_PLATFORMS.to_vec(),
        };
        let rules = EffectiveRules {
            platforms: vec![ConfigurationRulePlatformBorrowed {
                solution: Some("Any CPU"),
                project: "x64",
            }],
            ..Default::default()
        };

        // Act
        let configurations = project_configurations(&names, &rules);

        // Assert
        assert!(configurations
            .iter()
            .all(|configuration| configuration.platform == "x64"));
    }

    #[test_case("Debug", None, true ; "missing solution matches all")]
    #[test_case("Debug", Some("Debug"), true ; "matching solution")]
    #[test_case("Release", Some("Debug"), false ; "non matching solution")]
    fn rule_matches_solution_cases(
        solution_configuration: &str,
        rule_solution: Option<&str>,
        expected: bool,
    ) {
        // Arrange

        // Act
        let actual = rule_matches_solution(rule_solution, solution_configuration);

        // Assert
        assert_eq!(actual, expected);
    }
}
