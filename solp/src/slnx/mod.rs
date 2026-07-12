//! SLNX XML solution format support.
#![expect(
    dead_code,
    reason = "SLNX schema types include fields reserved for properties support in later phases"
)]

mod config;
mod convert;

use miette::{IntoDiagnostic, WrapErr};
use serde::Deserialize;

use crate::api::Solution;

/// Visual Studio XML solution file extension
pub const SLNX_SOLUTION_EXT: &str = "slnx";

/// Root element of Solution
#[derive(Debug, Deserialize)]
#[serde(rename = "Solution")]
pub struct SlnxSolution {
    #[serde(rename = "@Description", default)]
    pub description: Option<String>,
    #[serde(rename = "@Version", default)]
    pub version: Option<String>,

    #[serde(rename = "Configurations", default)]
    pub configurations: Option<Configurations>,

    #[serde(rename = "Project", default)]
    pub projects: Vec<Project>,

    #[serde(rename = "Folder", default)]
    pub folders: Vec<Folder>,

    #[serde(rename = "Properties", default)]
    pub properties: Vec<Properties>,
}

/// Configurations: build types, platforms, and project types
#[derive(Debug, Deserialize)]
pub struct Configurations {
    #[serde(rename = "BuildType", default)]
    pub build_types: Vec<BuildTypeConfig>,
    #[serde(rename = "Platform", default)]
    pub platforms: Vec<PlatformConfig>,
    #[serde(rename = "ProjectType", default)]
    pub project_types: Vec<ProjectType>,
}

/// Build type in the Configurations section
#[derive(Debug, Deserialize)]
pub struct BuildTypeConfig {
    #[serde(rename = "@Name")]
    pub name: String,
}

/// Platform in the Configurations section
#[derive(Debug, Deserialize)]
pub struct PlatformConfig {
    #[serde(rename = "@Name")]
    pub name: String,
}

/// Project type with configuration rules
#[derive(Debug, Deserialize)]
pub struct ProjectType {
    #[serde(rename = "BuildType", default)]
    pub build_types: Vec<ConfigurationRule>,
    #[serde(rename = "Platform", default)]
    pub platforms: Vec<ConfigurationRulePlatform>,
    #[serde(rename = "Build", default)]
    pub builds: Vec<ConfigurationRule>,
    #[serde(rename = "Deploy", default)]
    pub deploys: Vec<ConfigurationRule>,

    #[serde(rename = "@TypeId", default)]
    pub type_id: Option<String>,
    #[serde(rename = "@Name", default)]
    pub name: Option<String>,
    #[serde(rename = "@Extension", default)]
    pub extension: Option<String>,
    #[serde(rename = "@BasedOn", default)]
    pub based_on: Option<String>,
    #[serde(rename = "@IsBuildable", default)]
    pub is_buildable: Option<bool>,
    #[serde(rename = "@SupportsPlatform", default)]
    pub supports_platform: Option<bool>,
}

/// Folder containing files, projects, and properties
#[derive(Debug, Deserialize)]
pub struct Folder {
    #[serde(rename = "File", default)]
    pub files: Vec<FileRef>,
    #[serde(rename = "Project", default)]
    pub projects: Vec<Project>,
    #[serde(rename = "Properties", default)]
    pub properties: Vec<Properties>,

    #[serde(rename = "@Name")]
    pub name: String,
}

/// File reference in a folder
#[derive(Debug, Deserialize)]
pub struct FileRef {
    #[serde(rename = "@Path")]
    pub path: String,
}

/// Project in the solution
#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(rename = "BuildDependency", default)]
    pub build_dependencies: Vec<BuildDependency>,

    #[serde(rename = "BuildType", default)]
    pub build_types: Vec<ConfigurationRule>,
    #[serde(rename = "Platform", default)]
    pub platforms: Vec<ConfigurationRulePlatform>,
    #[serde(rename = "Build", default)]
    pub builds: Vec<ConfigurationRule>,
    #[serde(rename = "Deploy", default)]
    pub deploys: Vec<ConfigurationRule>,

    #[serde(rename = "Properties", default)]
    pub properties: Vec<Properties>,

    #[serde(rename = "@Path")]
    pub path: String,
    #[serde(rename = "@Type", default)]
    pub project_type: Option<String>,
    #[serde(rename = "@DisplayName", default)]
    pub display_name: Option<String>,
}

/// Build dependency (reference to another project)
#[derive(Debug, Deserialize)]
pub struct BuildDependency {
    #[serde(rename = "@Project")]
    pub project: String,
}

/// Common configuration rule (BuildType, Build, Deploy)
#[derive(Debug, Deserialize)]
pub struct ConfigurationRule {
    #[serde(rename = "@Solution", default)]
    pub solution: Option<String>,
    #[serde(rename = "@Project", default)]
    pub project: Option<String>,
}

/// Configuration rule for Platform (Project is required)
#[derive(Debug, Deserialize)]
pub struct ConfigurationRulePlatform {
    #[serde(rename = "@Solution", default)]
    pub solution: Option<String>,
    #[serde(rename = "@Project")]
    pub project: String,
}

/// Properties group (PropertiesGroup)
#[derive(Debug, Deserialize)]
pub struct Properties {
    #[serde(rename = "Property", default)]
    pub properties: Vec<Property>,

    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Scope", default)]
    pub scope: Option<String>,
}

/// Individual property
#[derive(Debug, Deserialize)]
pub struct Property {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Value", default)]
    pub value: Option<String>,
}

pub(crate) fn borrow_in<'a>(contents: &'a str, value: &str) -> miette::Result<&'a str> {
    if value.is_empty() {
        return Ok(&contents[0..0]);
    }

    contents
        .find(value)
        .map(|start| &contents[start..start + value.len()])
        .ok_or_else(|| miette::miette!("XML value not found in source: {value}"))
}

/// Returns `true` when the content looks like an XML `.slnx` solution file.
#[must_use]
pub fn is_slnx(contents: &str) -> bool {
    let trimmed = contents.trim_start();
    trimmed.starts_with('<') && !trimmed.starts_with("Microsoft Visual Studio")
}

/// Parses `.slnx` XML content and converts it into the public [`Solution`] API type.
pub fn parse_str(contents: &str) -> miette::Result<Solution<'_>> {
    let raw = deserialize_xml(contents)?;
    convert::to_api(raw, contents, "")
}

fn deserialize_xml(contents: &str) -> miette::Result<SlnxSolution> {
    let config = serde_xml_rs::SerdeXml::new().overlapping_sequences(true);
    let mut de = serde_xml_rs::Deserializer::from_config(config, contents.as_bytes());
    SlnxSolution::deserialize(&mut de)
        .into_diagnostic()
        .wrap_err("Failed to deserialize .slnx solution file")
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    const MINIMAL_SLNX: &str = r#"<Solution>
  <Project Path="src/App/App.csproj" />
</Solution>"#;

    const SLNX_WITH_FOLDER: &str = r#"<Solution Description="Test solution" Version="1.0">
  <Folder Name="/Solution Items/">
    <File Path="Directory.Build.props" />
  </Folder>
  <Project Path="src/App/App.csproj" DisplayName="My Application" />
</Solution>"#;

    const SLNX_WITH_EXPLICIT_CONFIGURATIONS: &str = r#"<Solution>
  <Configurations>
    <BuildType Name="Debug" />
    <BuildType Name="Release" />
    <Platform Name="x64" />
  </Configurations>
  <Project Path="src/App/App.csproj" />
</Solution>"#;

    #[test]
    fn deserialize_minimal_slnx() {
        // Arrange

        // Act
        let raw = deserialize_xml(MINIMAL_SLNX).unwrap();

        // Assert
        assert_eq!(raw.projects.len(), 1);
        assert_eq!(raw.projects[0].path, "src/App/App.csproj");
        assert!(raw.folders.is_empty());
    }

    #[test]
    fn deserialize_slnx_with_folder() {
        // Arrange

        // Act
        let raw = deserialize_xml(SLNX_WITH_FOLDER).unwrap();

        // Assert
        assert_eq!(raw.description.as_deref(), Some("Test solution"));
        assert_eq!(raw.version.as_deref(), Some("1.0"));
        assert_eq!(raw.folders.len(), 1);
        assert_eq!(raw.folders[0].name, "/Solution Items/");
        assert_eq!(raw.folders[0].files[0].path, "Directory.Build.props");
        assert_eq!(
            raw.projects[0].display_name.as_deref(),
            Some("My Application")
        );
    }

    #[test]
    fn parse_str_minimal_slnx() {
        // Arrange

        // Act
        let solution = parse_str(MINIMAL_SLNX).unwrap();

        // Assert
        assert_eq!(solution.projects.len(), 1);
        assert_eq!(solution.projects[0].path_or_uri, "src/App/App.csproj");
        assert_eq!(solution.projects[0].name, "App.csproj");
        assert_eq!(solution.configurations.len(), 2);
    }

    #[test]
    fn parse_str_slnx_with_solution_folder() {
        // Arrange

        // Act
        let solution = parse_str(SLNX_WITH_FOLDER).unwrap();

        // Assert
        assert_eq!(solution.projects.len(), 2);
        assert_eq!(solution.product, "Test solution");
        assert_eq!(solution.format, "1.0");

        let folder = solution
            .projects
            .iter()
            .find(|p| crate::msbuild::is_solution_folder(p.type_id))
            .expect("solution folder project");
        assert_eq!(folder.items.as_ref().unwrap().len(), 1);
        assert_eq!(folder.items.as_ref().unwrap()[0], "Directory.Build.props");
    }

    #[test_case("<Solution></Solution>" ; "empty solution")]
    #[test_case(MINIMAL_SLNX ; "minimal project")]
    #[test_case(SLNX_WITH_FOLDER ; "folder and project")]
    fn is_slnx_detects_xml(content: &str) {
        // Arrange

        // Act
        let actual = is_slnx(content);

        // Assert
        assert!(actual);
    }

    #[test]
    fn is_slnx_rejects_legacy_sln() {
        // Arrange
        let content = "Microsoft Visual Studio Solution File, Format Version 12.00\n";

        // Act
        let actual = is_slnx(content);

        // Assert
        assert!(!actual);
    }

    #[test]
    fn lib_parse_str_routes_slnx() {
        // Arrange

        // Act
        let solution = crate::parse_str(MINIMAL_SLNX).unwrap();

        // Assert
        assert_eq!(solution.projects.len(), 1);
        assert_eq!(solution.format, "slnx");
    }

    #[test]
    fn parse_str_explicit_configurations_use_declared_platform() {
        // Arrange

        // Act
        let solution = parse_str(SLNX_WITH_EXPLICIT_CONFIGURATIONS).unwrap();

        // Assert
        assert_eq!(solution.configurations.len(), 2);
        assert!(
            solution
                .configurations
                .iter()
                .all(|configuration| configuration.platform == "x64")
        );
    }
}
