use std::collections::BTreeSet;

use miette::Result;

use crate::api::{
    Project, ProjectConfiguration, Solution, SolutionConfiguration, Tag, Version,
};
use crate::msbuild;

use super::{Configurations, Folder, Project as RawProject, RawSolution};

const ID_SOLUTION_FOLDER: &str = "{2150E333-8FDC-42A3-9474-1A3956D46DE8}";

const DEFAULT_BUILD_TYPES: &[&str] = &["Debug", "Release"];
const DEFAULT_PLATFORMS: &[&str] = &["Any CPU"];

/// Converts a deserialized `.slnx` document into the shared public [`Solution`] model.
pub fn to_api<'a>(raw: RawSolution, contents: &'a str, path: &'a str) -> Result<Solution<'a>> {
    let format = match raw.version.as_deref() {
        Some(version) => borrow_in(contents, version)?,
        None => "slnx",
    };
    let product = match raw.description.as_deref() {
        Some(description) => borrow_in(contents, description)?,
        None => "",
    };

    let configurations = solution_configurations(contents, raw.configurations.as_ref())?;
    let mut projects = Vec::new();

    for folder in raw.folders {
        projects.push(folder_to_project(contents, &folder)?);
        for project in folder.projects {
            projects.push(raw_project_to_api(contents, &project)?);
        }
    }

    for project in raw.projects {
        projects.push(raw_project_to_api(contents, &project)?);
    }

    Ok(Solution {
        path: borrow_in(contents, path).unwrap_or(path),
        format,
        product,
        versions: Vec::<Version<'_>>::new(),
        projects,
        configurations,
        dangling_project_configurations: None,
        duplicate_solution_configurations: None,
        duplicate_project_configurations: None,
    })
}

fn solution_configurations<'a>(
    contents: &'a str,
    configs: Option<&Configurations>,
) -> Result<BTreeSet<SolutionConfiguration<'a>>> {
    let build_types: Vec<&str> = match configs {
        Some(configs) if !configs.build_types.is_empty() => configs
            .build_types
            .iter()
            .map(|build_type| borrow_in(contents, &build_type.name))
            .collect::<Result<Vec<_>>>()?,
        _ => DEFAULT_BUILD_TYPES.to_vec(),
    };

    let platforms: Vec<&str> = match configs {
        Some(configs) if !configs.platforms.is_empty() => configs
            .platforms
            .iter()
            .map(|platform| borrow_in(contents, &platform.name))
            .collect::<Result<Vec<_>>>()?,
        _ => DEFAULT_PLATFORMS.to_vec(),
    };

    Ok(build_types
        .into_iter()
        .flat_map(|configuration| {
            platforms
                .iter()
                .copied()
                .map(move |platform| SolutionConfiguration {
                    configuration,
                    platform,
                })
        })
        .collect())
}

fn folder_to_project<'a>(contents: &'a str, folder: &Folder) -> Result<Project<'a>> {
    let items = if folder.files.is_empty() {
        None
    } else {
        Some(
            folder
                .files
                .iter()
                .map(|file| borrow_in(contents, &file.path))
                .collect::<Result<Vec<_>>>()?,
        )
    };

    let name = borrow_in(contents, &folder.name)?;
    Ok(Project {
        type_id: ID_SOLUTION_FOLDER,
        type_description: msbuild::describe_project(ID_SOLUTION_FOLDER),
        id: name,
        name: folder_display_name(name),
        path_or_uri: name,
        configurations: None,
        items,
        depends_from: None,
    })
}

fn folder_display_name(name: &str) -> &str {
    name.trim_matches('/')
}

fn raw_project_to_api<'a>(contents: &'a str, project: &RawProject) -> Result<Project<'a>> {
    let path = borrow_in(contents, &project.path)?;
    let type_id = type_id_for_project(path, project.project_type.as_deref());
    let depends_from = if project.build_dependencies.is_empty() {
        None
    } else {
        Some(
            project
                .build_dependencies
                .iter()
                .map(|dep| borrow_in(contents, &dep.project))
                .collect::<Result<Vec<_>>>()?,
        )
    };

    Ok(Project {
        type_id,
        type_description: msbuild::describe_project(type_id),
        id: path,
        name: match project.display_name.as_deref() {
            Some(display_name) => borrow_in(contents, display_name)?,
            None => project_name(path),
        },
        path_or_uri: path,
        configurations: Some(default_project_configurations()),
        items: None,
        depends_from,
    })
}

fn project_name(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
}

fn default_project_configurations<'a>() -> BTreeSet<ProjectConfiguration<'a>> {
    DEFAULT_BUILD_TYPES
        .iter()
        .flat_map(|configuration| {
            DEFAULT_PLATFORMS.iter().map(move |platform| ProjectConfiguration {
                configuration,
                solution_configuration: configuration,
                platform,
                tags: vec![Tag::Build],
            })
        })
        .collect()
}

fn type_id_for_project(path: &str, explicit_type: Option<&str>) -> &'static str {
    if let Some(type_name) = explicit_type {
        return type_id_from_type_name(type_name);
    }

    let extension = path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "csproj" => "{9A19103F-16F7-4668-BE54-9A1E7A4F7556}",
        "vcxproj" => "{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}",
        "vbproj" => "{F184B08F-C81C-45F6-A57F-5ABD9991F28F}",
        "fsproj" => "{F2A71F9B-5D33-465A-A702-920D77279786}",
        "sqlproj" => "{00D1A9C2-B5F0-4AF3-8072-F6EAC31C12DA}",
        "njsproj" => "{262852C6-CD72-467D-83FE-D5B9760FE919}",
        _ => "{9A19103F-16F7-4668-BE54-9A1E7A4F7556}",
    }
}

fn type_id_from_type_name(type_name: &str) -> &'static str {
    match type_name.to_ascii_lowercase().as_str() {
        "c#" | "csharp" | "csproj" => "{9A19103F-16F7-4668-BE54-9A1E7A4F7556}",
        "c++" | "cpp" | "vcxproj" => "{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}",
        "vb" | "vbnet" | "vbproj" => "{F184B08F-C81C-45F6-A57F-5ABD9991F28F}",
        "f#" | "fsharp" | "fsproj" => "{F2A71F9B-5D33-465A-A702-920D77279786}",
        _ => "{9A19103F-16F7-4668-BE54-9A1E7A4F7556}",
    }
}

fn borrow_in<'a>(contents: &'a str, value: &str) -> Result<&'a str> {
    if value.is_empty() {
        return Ok(&contents[0..0]);
    }

    contents
        .find(value)
        .map(|start| &contents[start..start + value.len()])
        .ok_or_else(|| miette::miette!("XML value not found in source: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("src/App/App.csproj", None, "{9A19103F-16F7-4668-BE54-9A1E7A4F7556}" ; "csproj extension")]
    #[test_case("native/app.vcxproj", None, "{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}" ; "vcxproj extension")]
    #[test_case("lib/Library.fsproj", None, "{F2A71F9B-5D33-465A-A702-920D77279786}" ; "fsproj extension")]
    #[test_case("legacy/Old.csproj", Some("C#"), "{9A19103F-16F7-4668-BE54-9A1E7A4F7556}" ; "explicit type")]
    fn type_id_for_project_maps_known_types(path: &str, explicit: Option<&str>, expected: &str) {
        // Arrange

        // Act
        let actual = type_id_for_project(path, explicit);

        // Assert
        assert_eq!(actual, expected);
    }

    #[test]
    fn default_configurations_include_debug_and_release() {
        // Arrange

        // Act
        let configs = default_project_configurations();

        // Assert
        assert_eq!(configs.len(), 2);
        assert!(configs.iter().all(|cfg| cfg.tags == vec![Tag::Build]));
    }

    #[test]
    fn borrow_in_finds_value_in_source() {
        // Arrange
        let source = r#"<Project Path="src/App/App.csproj" />"#;

        // Act
        let actual = borrow_in(source, "src/App/App.csproj").unwrap();

        // Assert
        assert_eq!(actual, "src/App/App.csproj");
    }
}
