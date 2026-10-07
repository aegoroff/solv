use std::collections::BTreeSet;

use miette::Result;

use crate::api::{Project, Solution, SolutionConfiguration, Version};
use crate::msbuild;

use super::config::{
    SolutionConfigNames, project_configurations, project_setup, solution_build_types,
    solution_platforms,
};
use super::{Configurations, Folder, Project as RawProject, SlnxSolution, borrow_in};

const ID_SOLUTION_FOLDER: &str = "{2150E333-8FDC-42A3-9474-1A3956D46DE8}";

/// Converts a deserialized `.slnx` document into the shared public [`Solution`] model.
pub fn to_api<'a>(slnx: SlnxSolution, contents: &'a str, path: &'a str) -> Result<Solution<'a>> {
    let format = match slnx.version.as_deref() {
        Some(version) => borrow_in(contents, version)?,
        None => "slnx",
    };
    let product = match slnx.description.as_deref() {
        Some(description) => borrow_in(contents, description)?,
        None => "",
    };

    let build_types = solution_build_types(contents, slnx.configurations.as_ref())?;
    let platforms = solution_platforms(contents, slnx.configurations.as_ref())?;
    let config_names = SolutionConfigNames {
        build_types: build_types.clone(),
        platforms: platforms.clone(),
    };
    let configurations: BTreeSet<SolutionConfiguration<'a>> = build_types
        .iter()
        .copied()
        .flat_map(|configuration| {
            platforms
                .iter()
                .copied()
                .map(move |platform| SolutionConfiguration {
                    configuration,
                    platform,
                })
        })
        .collect();
    let mut projects = Vec::new();

    for folder in slnx.folders {
        projects.push(folder_to_project(contents, &folder)?);
        for project in folder.projects {
            projects.push(raw_project_to_api(
                contents,
                slnx.configurations.as_ref(),
                &config_names,
                &project,
            )?);
        }
    }

    for project in slnx.projects {
        projects.push(raw_project_to_api(
            contents,
            slnx.configurations.as_ref(),
            &config_names,
            &project,
        )?);
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

fn raw_project_to_api<'a>(
    contents: &'a str,
    configurations: Option<&Configurations>,
    config_names: &SolutionConfigNames<'a>,
    project: &RawProject,
) -> Result<Project<'a>> {
    let path = borrow_in(contents, &project.path)?;
    let setup = project_setup(contents, configurations, project)?;
    let type_id = setup.type_id;
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
    let project_configurations = project_configurations(config_names, &setup.rules);

    Ok(Project {
        type_id,
        type_description: msbuild::describe_project(type_id),
        id: path,
        name: match project.display_name.as_deref() {
            Some(display_name) => borrow_in(contents, display_name)?,
            None => project_name(path),
        },
        path_or_uri: path,
        configurations: if project_configurations.is_empty() {
            None
        } else {
            Some(project_configurations)
        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    const SLNX_WITH_DEPENDENCIES: &str = r#"<Solution>
  <Project Path="src/App/App.csproj">
    <BuildDependency Project="src/Lib/Lib.csproj" />
  </Project>
  <Project Path="src/Lib/Lib.csproj" />
</Solution>"#;

    #[test]
    fn dependencies_are_preserved() {
        // Arrange

        // Act
        let solution = super::super::parse_str(SLNX_WITH_DEPENDENCIES).unwrap();

        // Assert
        let app = solution
            .projects
            .iter()
            .find(|project| project.path_or_uri == "src/App/App.csproj")
            .expect("app project");
        assert_eq!(app.depends_from.as_ref().unwrap(), &["src/Lib/Lib.csproj"]);
    }

    #[test]
    fn borrow_in_finds_value_in_source() {
        // Arrange
        let source = r#"<Project Path="src/App/App.csproj" />"#;

        // Act
        let actual = super::borrow_in(source, "src/App/App.csproj").unwrap();

        // Assert
        assert_eq!(actual, "src/App/App.csproj");
    }
}
