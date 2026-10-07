use std::collections::BTreeSet;

use miette::Result;

use crate::api::{ProjectConfiguration, Tag};

use super::types::{MISSING_PLATFORM, Resolver, TypeRef, built_in_type_id};
use super::{ConfigurationRule, Configurations, Project as RawProject, ProjectType};

const DEFAULT_BUILD_TYPES: &[&str] = &["Debug", "Release"];
const DEFAULT_PLATFORMS: &[&str] = &["Any CPU"];

#[derive(Debug, Default)]
pub struct SolutionConfigNames<'a> {
    pub build_types: Vec<&'a str>,
    pub platforms: Vec<&'a str>,
}

#[derive(Debug, Default)]
pub struct EffectiveRules<'a> {
    pub build_types: Vec<ConfigurationRuleBorrowed<'a>>,
    pub platforms: Vec<ConfigurationRuleBorrowed<'a>>,
    pub builds: Vec<ConfigurationRuleBorrowed<'a>>,
    pub deploys: Vec<ConfigurationRuleBorrowed<'a>>,
}

impl<'a> EffectiveRules<'a> {
    /// Appends `BuildType`, `Platform`, `Build` and `Deploy` rules (in this order)
    fn append(&mut self, contents: &'a str, sources: [&[ConfigurationRule]; 4]) -> Result<()> {
        let [build_types, platforms, builds, deploys] = sources;
        for (target, source) in [
            (&mut self.build_types, build_types),
            (&mut self.platforms, platforms),
            (&mut self.builds, builds),
            (&mut self.deploys, deploys),
        ] {
            for rule in source {
                target.push(borrow_rule(contents, rule)?);
            }
        }
        Ok(())
    }
}

/// Project type id and configuration rules that apply to a project
#[derive(Debug)]
pub struct ProjectSetup<'a> {
    pub type_id: &'a str,
    pub rules: EffectiveRules<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigurationRuleBorrowed<'a> {
    pub solution: Option<&'a str>,
    pub project: Option<&'a str>,
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
    let mut configurations = BTreeSet::new();
    for solution_configuration in &names.build_types {
        for solution_platform in &names.platforms {
            let configuration = map_build_type(
                solution_configuration,
                solution_platform,
                &rules.build_types,
            );
            let project_platform =
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
                platform: solution_platform,
                project_platform,
                tags,
            });
        }
    }

    configurations
}

/// Resolves project type and collects configuration rules from the most general to the most specific:
/// project type rules (including `BasedOn` chain), solution wide rules and project own rules.
pub fn project_setup<'a>(
    contents: &'a str,
    configs: Option<&Configurations>,
    project: &RawProject,
) -> Result<ProjectSetup<'a>> {
    let resolver = Resolver::new(configs);
    let path = super::borrow_in(contents, &project.path)?;
    let (_, extension) = super::split_file_name(path);
    let explicit_type = project.project_type.as_deref();
    let resolved = resolver.resolve(explicit_type, extension);

    let mut rules = EffectiveRules::default();
    if let Some(project_type) = resolved {
        for project_type in resolver.chain(project_type) {
            append_type_rules(contents, project_type, &mut rules)?;
        }
    }
    for project_type in resolver.solution_defaults() {
        append_custom_type_rules(contents, project_type, &mut rules)?;
    }
    rules.append(
        contents,
        [
            &project.build_types,
            &project.platforms,
            &project.builds,
            &project.deploys,
        ],
    )?;

    // Unknown type is described by Type attribute or by extension like unknown GUID in .sln
    let type_id = match resolved.and_then(|project_type| resolver.type_id(project_type)) {
        Some(id) => match built_in_type_id(id) {
            Some(id) => id,
            None => super::borrow_in(contents, id)?,
        },
        None => match explicit_type {
            Some(explicit_type) => super::borrow_in(contents, explicit_type)?,
            None => extension.unwrap_or_default(),
        },
    };

    Ok(ProjectSetup { type_id, rules })
}

fn append_type_rules<'a>(
    contents: &'a str,
    project_type: TypeRef<'_>,
    rules: &mut EffectiveRules<'a>,
) -> Result<()> {
    match project_type {
        TypeRef::BuiltIn(built_in) => {
            rules.platforms.extend_from_slice(built_in.platform_rules());
            rules.builds.extend_from_slice(built_in.build_rules());
            Ok(())
        }
        TypeRef::Custom(custom) => append_custom_type_rules(contents, custom, rules),
    }
}

fn append_custom_type_rules<'a>(
    contents: &'a str,
    project_type: &ProjectType,
    rules: &mut EffectiveRules<'a>,
) -> Result<()> {
    // Like the reference implementation: not buildable type has only "no build" rule
    // and its own rules are ignored
    if project_type.is_buildable == Some(false) {
        rules.builds.push(ConfigurationRuleBorrowed {
            solution: None,
            project: Some("false"),
        });
        return Ok(());
    }
    if project_type.supports_platform == Some(false) {
        rules.platforms.push(ConfigurationRuleBorrowed {
            solution: None,
            project: Some(MISSING_PLATFORM),
        });
    }

    rules.append(
        contents,
        [
            &project_type.build_types,
            &project_type.platforms,
            &project_type.builds,
            &project_type.deploys,
        ],
    )
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
    rules: &[ConfigurationRuleBorrowed<'a>],
) -> &'a str {
    rules
        .iter()
        .rev()
        .filter(|rule| {
            rule_matches_solution(
                rule.solution,
                Dimension::Platform,
                solution_build_type,
                solution_platform,
            )
        })
        .find_map(|rule| rule.project)
        .unwrap_or(solution_platform)
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
    pattern.is_empty()
        || pattern == "*"
        || canonical_platform(pattern).eq_ignore_ascii_case(canonical_platform(value))
}

/// `AnyCPU` and `Any CPU` are the same platform
fn canonical_platform(value: &str) -> &str {
    if value.eq_ignore_ascii_case("AnyCPU") {
        "Any CPU"
    } else {
        value
    }
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

    const SLNX_BUILT_IN_TYPES: &str = r#"<Solution>
  <Configurations>
    <Platform Name="Any CPU" />
    <Platform Name="x86" />
    <Platform Name="x64" />
  </Configurations>
  <Project Path="App/App.csproj" />
  <Project Path="Native/Native.vcxproj" />
  <Project Path="Shared/Shared.vcxitems" />
  <Project Path="Db/Db.sqlproj" />
  <Project Path="Reports/Reports.rptproj" />
  <Project Path="Tool/Tool.pyproj" />
</Solution>"#;

    fn project_configuration<'a>(
        solution: &'a crate::api::Solution<'a>,
        path: &str,
        solution_platform: &str,
    ) -> &'a ProjectConfiguration<'a> {
        let project = solution
            .projects
            .iter()
            .find(|project| project.path_or_uri == path)
            .expect("project");
        find(
            project
                .configurations
                .as_ref()
                .expect("project configurations"),
            "Debug",
            solution_platform,
        )
    }

    #[test_case("App/App.csproj", "x64", "Any CPU", true ; "clr project platform is any cpu")]
    #[test_case("Native/Native.vcxproj", "Any CPU", "x64", true ; "vc any cpu maps to x64")]
    #[test_case("Native/Native.vcxproj", "x86", "Win32", true ; "vc x86 maps to win32")]
    #[test_case("Native/Native.vcxproj", "x64", "x64", true ; "vc x64 stays as is")]
    #[test_case("Shared/Shared.vcxitems", "x64", "x64", false ; "vcxitems is not built")]
    #[test_case("Db/Db.sqlproj", "Any CPU", "Any CPU", false ; "sql project is not built")]
    #[test_case("Reports/Reports.rptproj", "x64", MISSING_PLATFORM, true ; "ssrs has no platforms")]
    #[test_case("Tool/Tool.pyproj", "x86", "x86", true ; "unknown type uses default rules")]
    fn built_in_type_rules_are_applied(
        path: &str,
        solution_platform: &str,
        expected_platform: &str,
        expected_built: bool,
    ) {
        // Arrange
        let solution = super::super::parse_str(SLNX_BUILT_IN_TYPES).unwrap();

        // Act
        let actual = project_configuration(&solution, path, solution_platform);

        // Assert
        assert_eq!(actual.project_platform, expected_platform);
        assert_eq!(actual.tags.contains(&Tag::Build), expected_built);
    }

    #[test_case(r#"<Project Path="Db/Db.sqlproj" />"#, false ; "built in no build type")]
    #[test_case(r#"<Project Path="Db/Db.sqlproj"><Build /></Project>"#, true ; "project rule overrides no build type")]
    #[test_case(r#"<Project Path="A/A.proj" Type="Custom" />"#, false ; "not buildable custom type")]
    #[test_case(r#"<Project Path="A/A.proj" Type="Custom"><Build Solution="Debug|*" /></Project>"#, true ; "project rule overrides not buildable custom type")]
    fn not_buildable_types_can_be_overridden_by_project(project: &str, expected_built: bool) {
        // Arrange
        let slnx = format!(
            r#"<Solution>
  <Configurations>
    <ProjectType Name="Custom" IsBuildable="false" />
  </Configurations>
  {project}
</Solution>"#
        );

        // Act
        let solution = super::super::parse_str(&slnx).unwrap();

        // Assert
        let configurations = solution.projects[0]
            .configurations
            .as_ref()
            .expect("project configurations");
        assert_eq!(
            find(configurations, "Debug", "Any CPU")
                .tags
                .contains(&Tag::Build),
            expected_built
        );
    }

    #[test]
    fn platform_rule_without_project_is_ignored() {
        // Arrange
        let slnx = r#"<Solution>
  <Project Path="Native/Native.vcxproj">
    <Platform Solution="*|Any CPU" />
  </Project>
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        let configurations = solution.projects[0].configurations.as_ref().unwrap();
        assert_eq!(
            find(configurations, "Debug", "Any CPU").project_platform,
            "x64"
        );
    }

    #[test]
    fn not_buildable_type_ignores_own_rules() {
        // Arrange
        let slnx = r#"<Solution>
  <Configurations>
    <ProjectType Name="Custom" IsBuildable="false">
      <Build Solution="Debug|*" />
      <Platform Solution="*|Any CPU" Project="x64" />
    </ProjectType>
  </Configurations>
  <Project Path="A/A.proj" Type="Custom" />
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        let configurations = solution.projects[0].configurations.as_ref().unwrap();
        let debug = find(configurations, "Debug", "Any CPU");
        assert!(debug.tags.is_empty());
        assert_eq!(debug.project_platform, "Any CPU");
    }

    #[test]
    fn based_on_type_rules_apply_before_own_rules() {
        // Arrange
        let slnx = r#"<Solution>
  <Configurations>
    <Platform Name="Any CPU" />
    <Platform Name="x86" />
    <ProjectType Name="Native" Extension="nproj" BasedOn="VC">
      <Platform Solution="*|x86" Project="x86" />
    </ProjectType>
  </Configurations>
  <Project Path="A/A.nproj" />
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        let project = &solution.projects[0];
        let configurations = project.configurations.as_ref().unwrap();
        assert_eq!(project.type_id, "{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}");
        assert_eq!(
            find(configurations, "Debug", "Any CPU").project_platform,
            "x64"
        );
        assert_eq!(find(configurations, "Debug", "x86").project_platform, "x86");
    }

    #[test]
    fn solution_wide_rules_apply_to_all_projects() {
        // Arrange
        let slnx = r#"<Solution>
  <Configurations>
    <ProjectType>
      <Build Solution="Release|*" Project="false" />
    </ProjectType>
  </Configurations>
  <Project Path="A/A.csproj" />
  <Project Path="B/B.vcxproj" />
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        for project in &solution.projects {
            let configurations = project.configurations.as_ref().unwrap();
            assert!(
                find(configurations, "Debug", "Any CPU")
                    .tags
                    .contains(&Tag::Build)
            );
            assert!(find(configurations, "Release", "Any CPU").tags.is_empty());
        }
    }

    #[test_case(r#"<Project Path="A/A.csproj" />"#, "{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}" ; "csproj is classic c sharp")]
    #[test_case(r#"<Project Path="A/A.csproj" Type="Common C#" />"#, "{9A19103F-16F7-4668-BE54-9A1E7A4F7556}" ; "common c sharp by name")]
    #[test_case(r#"<Project Path="A/A.csproj" Type="9a19103f-16f7-4668-be54-9a1e7a4f7556" />"#, "{9A19103F-16F7-4668-BE54-9A1E7A4F7556}" ; "built in guid is normalized")]
    #[test_case(r#"<Project Path="A/A.vbproj" />"#, "{F184B08F-C81C-45F6-A57F-5ABD9991F28F}" ; "vbproj")]
    #[test_case(r#"<Project Path="A/A.njsproj" />"#, "{9092AA53-FB77-4645-B42D-1CCCA6BD08BD}" ; "njsproj")]
    #[test_case(r#"<Project Path="A/A.pyproj" Type="888888A0-9F3D-457C-B088-3A5042F75D52" />"#, "888888A0-9F3D-457C-B088-3A5042F75D52" ; "unknown guid kept as is")]
    #[test_case(r#"<Project Path="A/A.pyproj" Type="Python" />"#, "Python" ; "unknown name kept as is")]
    #[test_case(r#"<Project Path="A/A.pyproj" />"#, "pyproj" ; "unknown extension")]
    #[test_case(r#"<Project Path="A/A.proj" Type="Custom" />"#, "11111111-2222-3333-4444-555555555555" ; "solution defined type id")]
    fn project_type_id_is_resolved(project: &str, expected: &str) {
        // Arrange
        let slnx = format!(
            r#"<Solution>
  <Configurations>
    <ProjectType Name="Custom" TypeId="11111111-2222-3333-4444-555555555555" />
  </Configurations>
  {project}
</Solution>"#
        );

        // Act
        let solution = super::super::parse_str(&slnx).unwrap();

        // Assert
        assert_eq!(solution.projects[0].type_id, expected);
    }

    #[test_case("AnyCPU", "Any CPU", true ; "any cpu without space")]
    #[test_case("anycpu", "Any CPU", true ; "any cpu ignores case")]
    #[test_case("Any CPU", "AnyCPU", true ; "solution any cpu without space")]
    #[test_case("x64", "Any CPU", false ; "different platforms")]
    fn part_matches_canonical_platforms(pattern: &str, value: &str, expected: bool) {
        // Arrange

        // Act
        let actual = part_matches(pattern, value);

        // Assert
        assert_eq!(actual, expected);
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
        assert!(configurations.iter().all(|configuration| {
            configuration.platform == "Any CPU" && configuration.project_platform == "x64"
        }));
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

    #[test_case("Debug", "Any CPU", "Debug", "Win32", vec![Tag::Build] ; "debug any cpu")]
    #[test_case("Debug", "x64", "Debug", "x64", vec![] ; "debug x64 not built")]
    #[test_case("Release", "Any CPU", "Debug", "Win32", vec![Tag::Build] ; "release any cpu")]
    #[test_case("Release", "x64", "Debug", "x64", vec![Tag::Build, Tag::Deploy] ; "release x64 deployed")]
    fn full_rules_are_applied(
        solution_configuration: &str,
        solution_platform: &str,
        expected_configuration: &str,
        expected_project_platform: &str,
        expected_tags: Vec<Tag>,
    ) {
        // Arrange
        let solution = super::super::parse_str(SLNX_FULL_RULES).unwrap();
        let configurations = solution.projects[0]
            .configurations
            .as_ref()
            .expect("project configurations");

        // Act
        let actual = find(configurations, solution_configuration, solution_platform);

        // Assert
        assert_eq!(configurations.len(), 4);
        assert_eq!(actual.configuration, expected_configuration);
        assert_eq!(actual.project_platform, expected_project_platform);
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
