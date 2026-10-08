//! Conversion of parsed `.sln` into the shared public [`Solution`] model.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::api::{
    ConfigurationMappingTag, DuplicateProjectConfiguration, Project, ProjectConfiguration,
    Solution, SolutionConfiguration, SolutionKind, Tag, Version,
};
use crate::ast::{ProjectConfigTag, Sol};
use crate::msbuild;

/// Converts parsed `.sln` into [`Solution`]. Project references are resolved
/// by [`Solution::resolve_references`] later.
pub(crate) fn to_api<'a>(solution: &Sol<'a>) -> Solution<'a> {
    Solution {
        path: solution.path,
        kind: SolutionKind::Sln,
        format: solution.format,
        product: solution.product,
        versions: versions(solution),
        projects: projects(solution),
        configurations: configurations(solution),
        dangling_project_configurations: danglings(solution),
        duplicate_solution_configurations: duplicate_solution_configurations(solution),
        duplicate_project_configurations: duplicate_project_configurations(solution),
    }
}

fn versions<'a>(solution: &Sol<'a>) -> Vec<Version<'a>> {
    solution
        .versions
        .iter()
        .map(|v| Version {
            name: v.name,
            version: v.ver,
        })
        .collect()
}

fn configurations<'a>(solution: &Sol<'a>) -> BTreeSet<SolutionConfiguration<'a>> {
    solution
        .solution_configs
        .iter()
        .map(|c| SolutionConfiguration {
            configuration: c.config,
            platform: c.platform,
        })
        .collect()
}

fn projects<'a>(solution: &Sol<'a>) -> Vec<Project<'a>> {
    // Configurations section may use different GUID case
    let mut project_configs: HashMap<String, BTreeSet<ProjectConfiguration>> = HashMap::new();
    for c in &solution.project_configs {
        let mut configs: HashMap<_, ProjectConfiguration> = HashMap::new();
        for pc in &c.configs {
            let key = (
                pc.project_config,
                pc.solution_config,
                pc.platform,
                pc.project_platform,
            );
            let config = configs.entry(key).or_insert_with(|| ProjectConfiguration {
                configuration: pc.project_config,
                solution_configuration: pc.solution_config,
                platform: pc.platform,
                project_platform: pc.project_platform,
                tags: vec![],
            });
            match pc.tag {
                ProjectConfigTag::ActiveCfg => {}
                ProjectConfigTag::Build => config.tags.push(Tag::Build),
                ProjectConfigTag::Deploy => config.tags.push(Tag::Deploy),
            }
        }
        project_configs
            .entry(c.project_id.to_uppercase())
            .or_default()
            .extend(configs.into_values());
    }
    // NestedProjects section may use different GUID case
    let parents = solution
        .nested_projects
        .iter()
        .map(|(child, parent)| (child.to_uppercase(), *parent))
        .collect::<HashMap<String, &str>>();
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
                configurations: project_configs.get(&p.id.to_uppercase()).cloned(),
                items,
                depends_from,
                parent: parents.get(&p.id.to_uppercase()).copied(),
            }
        })
        .collect()
}

fn danglings(solution: &Sol<'_>) -> Option<Vec<String>> {
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

fn duplicate_solution_configurations<'a>(
    solution: &Sol<'a>,
) -> Option<Vec<SolutionConfiguration<'a>>> {
    let mut seen = HashSet::new();
    let mut duplicates = BTreeSet::new();
    for config in &solution.solution_configuration_platform_entries {
        let item = SolutionConfiguration {
            configuration: config.config,
            platform: config.platform,
        };
        if !seen.insert((config.config, config.platform)) {
            duplicates.insert(item);
        }
    }

    if duplicates.is_empty() {
        None
    } else {
        Some(duplicates.into_iter().collect())
    }
}

fn duplicate_project_configurations<'a>(
    solution: &Sol<'a>,
) -> Option<Vec<DuplicateProjectConfiguration<'a>>> {
    let mut seen = HashSet::new();
    let mut duplicates = BTreeSet::new();
    for config in &solution.project_configuration_entries {
        let key = (
            config.id.to_ascii_uppercase(),
            config.solution_config,
            config.platform,
            config.project_config,
            config.tag.clone(),
        );
        if !seen.insert(key) {
            duplicates.insert(DuplicateProjectConfiguration {
                project_id: config.id,
                solution_configuration: config.solution_config,
                platform: config.platform,
                project_configuration: config.project_config,
                tag: project_config_tag_name(&config.tag),
            });
        }
    }

    if duplicates.is_empty() {
        None
    } else {
        Some(duplicates.into_iter().collect())
    }
}

fn project_config_tag_name(tag: &ProjectConfigTag) -> ConfigurationMappingTag {
    match tag {
        ProjectConfigTag::ActiveCfg => ConfigurationMappingTag::ActiveCfg,
        ProjectConfigTag::Build => ConfigurationMappingTag::Build,
        ProjectConfigTag::Deploy => ConfigurationMappingTag::Deploy,
    }
}
