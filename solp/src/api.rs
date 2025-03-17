use std::collections::{BTreeSet, HashMap, HashSet};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{ast::Sol, msbuild};

/// Represents Visual Studio solution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution<'a> {
    /// Full path to solution file
    pub path: &'a str,
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
    /// Platform i.e. Any CPU, Win32, x86 etc.
    pub platform: &'a str,
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

impl<'a> Solution<'a> {
    /// Creates new [`Solution`] instance from [`ast::Sol`] instance
    #[must_use]
    pub fn from(solution: &Sol<'a>) -> Self {
        Self {
            path: solution.path,
            format: solution.format,
            product: solution.product,
            versions: Self::versions(solution),
            projects: Self::projects(solution),
            configurations: Self::configurations(solution),
            dangling_project_configurations: Self::danglings(solution),
        }
    }

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

    fn versions(solution: &Sol<'a>) -> Vec<Version<'a>> {
        solution
            .versions
            .iter()
            .map(|v| Version {
                name: v.name,
                version: v.ver,
            })
            .collect()
    }

    fn configurations(solution: &Sol<'a>) -> BTreeSet<SolutionConfiguration<'a>> {
        solution
            .solution_configs
            .iter()
            .map(|c| SolutionConfiguration {
                configuration: c.config,
                platform: c.platform,
            })
            .collect()
    }

    fn projects(solution: &Sol<'a>) -> Vec<Project<'a>> {
        let project_configs = solution
            .project_configs
            .iter()
            .map(|c| {
                (
                    c.project_id,
                    c.configs
                        .iter()
                        .into_grouping_map_by(|pc| {
                            (pc.project_config, pc.solution_config, pc.platform)
                        })
                        .fold(
                            ProjectConfiguration::default(),
                            |mut pc, (p, s, plat), val| {
                                pc.configuration = p;
                                pc.solution_configuration = s;
                                pc.platform = plat;
                                match val.tag {
                                    crate::ast::ProjectConfigTag::ActiveCfg => {}
                                    crate::ast::ProjectConfigTag::Build => pc.tags.push(Tag::Build),
                                    crate::ast::ProjectConfigTag::Deploy => {
                                        pc.tags.push(Tag::Deploy);
                                    }
                                };
                                pc
                            },
                        )
                        .into_values()
                        .collect(),
                )
            })
            .collect::<HashMap<&str, BTreeSet<ProjectConfiguration>>>();
        solution
            .projects
            .iter()
            .map(|p| {
                let items = if p.items.is_empty() {
                    None
                } else {
                    Some(p.items.clone())
                };
                let depends_from = if p.depends_from.is_empty() {
                    None
                } else {
                    Some(p.depends_from.clone())
                };
                Project {
                    type_id: p.type_id,
                    type_description: p.type_descr,
                    id: p.id,
                    name: p.name,
                    path_or_uri: p.path_or_uri,
                    configurations: project_configs.get(p.id).cloned(),
                    items,
                    depends_from,
                }
            })
            .collect()
    }

    fn danglings(solution: &Sol<'a>) -> Option<Vec<String>> {
        let project_ids: HashSet<String> = solution
            .projects
            .iter()
            .filter(|p| !msbuild::is_solution_folder(p.type_id))
            .map(|p| p.id.to_uppercase())
            .collect();

        let mut danglings = Vec::with_capacity(solution.project_configs.len());
        for aggr in &solution.project_configs {
            let id = aggr.project_id.to_uppercase();
            if !project_ids.contains(&id) {
                danglings.push(id);
            }
        }

        if danglings.is_empty() {
            None
        } else {
            Some(danglings)
        }
    }
}
