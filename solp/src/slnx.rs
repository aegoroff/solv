use serde::{Deserialize, Serialize};

/// Root element of Solution
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename = "Solution")]
pub struct Solution {
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
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Configurations {
    #[serde(rename = "BuildType", default)]
    pub build_types: Vec<BuildTypeConfig>,
    #[serde(rename = "Platform", default)]
    pub platforms: Vec<PlatformConfig>,
    #[serde(rename = "ProjectType", default)]
    pub project_types: Vec<ProjectType>,
}

/// Build type in the Configurations section
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct BuildTypeConfig {
    #[serde(rename = "@Name")]
    pub name: String,
}

/// Platform in the Configurations section
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct PlatformConfig {
    #[serde(rename = "@Name")]
    pub name: String,
}

/// Project type with configuration rules
#[derive(Debug, Deserialize, Serialize, PartialEq)]
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
#[derive(Debug, Deserialize, Serialize, PartialEq)]
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
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct FileRef {
    #[serde(rename = "@Path")]
    pub path: String,
}

/// Project in the solution
#[derive(Debug, Deserialize, Serialize, PartialEq)]
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
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct BuildDependency {
    #[serde(rename = "@Project")]
    pub project: String,
}

/// Common configuration rule (BuildType, Build, Deploy)
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ConfigurationRule {
    #[serde(rename = "@Solution", default)]
    pub solution: Option<String>,
    #[serde(rename = "@Project", default)]
    pub project: Option<String>,
}

/// Configuration rule for Platform (Project is required)
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ConfigurationRulePlatform {
    #[serde(rename = "@Solution", default)]
    pub solution: Option<String>,
    #[serde(rename = "@Project")]
    pub project: String,
}

/// Properties group (PropertiesGroup)
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Properties {
    #[serde(rename = "Property", default)]
    pub properties: Vec<Property>,

    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Scope", default)]
    pub scope: Option<String>,
}

/// Individual property
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Property {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Value", default)]
    pub value: Option<String>,
}