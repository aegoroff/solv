use std::collections::{BTreeSet, HashMap};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::msbuild;

/// Represents Visual Studio solution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution<'a> {
    /// Full path to solution file
    pub path: &'a str,
    /// Solution file format detected by content
    pub kind: SolutionKind,
    /// Solution format
    pub format: &'a str,
    /// Solution product like Visual Studio 15 etc
    pub product: &'a str,
    /// Solution versions got from lines starts from # char at the beginning of solution file
    pub versions: Vec<Version<'a>>,
    /// Solution's projects
    pub projects: Vec<Project<'a>>,
    /// All solution's configuration/platform pairs
    pub configurations: BTreeSet<SolutionConfiguration<'a>>,
    /// Dangling (projects with such ids not exist in the solution file) projects configurations inside solution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dangling_project_configurations: Option<Vec<String>>,
    /// Duplicate solution configuration/platform pairs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_solution_configurations: Option<Vec<SolutionConfiguration<'a>>>,
    /// Duplicate project configuration mappings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_project_configurations: Option<Vec<DuplicateProjectConfiguration<'a>>>,
}

/// Solution file format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SolutionKind {
    /// Classic text `.sln` format
    #[default]
    Sln,
    /// XML `.slnx` format
    Slnx,
}

/// Represents [`Solution`] version. NOTE: [`Solution`] may have several versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version<'a> {
    pub name: &'a str,
    pub version: &'a str,
}

/// Represent project inside [`Solution`]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Project<'a> {
    pub type_id: &'a str,
    pub type_description: &'a str,
    pub id: &'a str,
    pub name: &'a str,
    pub path_or_uri: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configurations: Option<BTreeSet<ProjectConfiguration<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<&'a str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_from: Option<Vec<&'a str>>,
    /// Id of the solution folder that contains the project (or folder) if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<&'a str>,
}

/// Represents solution configuration/platform pair
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SolutionConfiguration<'a> {
    /// Solution's configuration name
    pub configuration: &'a str,
    /// Platform i.e. Any CPU, Win32, x86 etc.
    pub platform: &'a str,
}

/// Represents project configuration/platform pair
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectConfiguration<'a> {
    /// Project configuration
    pub configuration: &'a str,
    /// Solution's configuration this project config belongs to
    pub solution_configuration: &'a str,
    /// Solution's platform this project config belongs to i.e. Any CPU, Win32, x86 etc.
    pub platform: &'a str,
    /// Project platform the solution's platform is mapped to i.e. Any CPU, Win32, x64 etc.
    pub project_platform: &'a str,
    /// Configuration tag
    pub tags: Vec<Tag>,
}

/// Represents project configuration tag
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tag {
    /// Defines project configuration buildable
    #[default]
    Build,
    /// Defines project configuration deployable
    Deploy,
}

/// Duplicate project configuration mapping inside a solution file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DuplicateProjectConfiguration<'a> {
    pub project_id: &'a str,
    pub solution_configuration: &'a str,
    pub platform: &'a str,
    pub project_configuration: &'a str,
    pub tag: ConfigurationMappingTag,
}

/// Tag of a duplicate project configuration mapping
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigurationMappingTag {
    ActiveCfg,
    Build,
    Deploy,
}

impl fmt::Display for ConfigurationMappingTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActiveCfg => write!(f, "ActiveCfg"),
            Self::Build => write!(f, "Build"),
            Self::Deploy => write!(f, "Deploy"),
        }
    }
}

impl<'a> Solution<'a> {
    /// Iterates all but solution folder projects inside [`Solution`]
    pub fn iterate_projects(&'a self) -> impl Iterator<Item = &'a Project<'a>> {
        self.projects
            .iter()
            .filter(|p| !msbuild::is_solution_folder(p.type_id))
    }

    /// Iterates all but solution folder and website projects
    pub fn iterate_projects_without_web_sites(&'a self) -> impl Iterator<Item = &'a Project<'a>> {
        self.iterate_projects()
            .filter(|p| !msbuild::is_web_site_project(p.type_id))
    }

    /// Makes project references (`parent` and `depends_from`) use declared project ids
    /// so both solution formats fill them the same way. Ids are compared ignoring case
    /// (the first declared project wins). Unknown references are kept as is.
    pub(crate) fn resolve_references(&mut self) {
        let ids: HashMap<String, &'a str> = self
            .projects
            .iter()
            .rev()
            .map(|p| (p.id.to_uppercase(), p.id))
            .collect();
        let resolve = |reference: &'a str| {
            ids.get(&reference.to_uppercase())
                .copied()
                .unwrap_or(reference)
        };
        for project in &mut self.projects {
            project.parent = project.parent.map(resolve);
            for dependency in project.depends_from.iter_mut().flatten() {
                *dependency = resolve(dependency);
            }
        }
    }
}
