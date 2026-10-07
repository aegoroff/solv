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
    let mut folders = Folders::new(contents, &slnx.folders)?;
    for folder in &slnx.folders {
        folders.add_declared(contents, folder)?;
    }

    // Projects inside folders go first like they are declared in the file
    let raw_projects = slnx
        .folders
        .iter()
        .flat_map(|folder| {
            folder
                .projects
                .iter()
                .map(move |project| (Some(folder), project))
        })
        .chain(slnx.projects.iter().map(|project| (None, project)))
        .collect::<Vec<_>>();

    let ids = raw_projects
        .iter()
        .map(|(_, project)| project_id(contents, project))
        .collect::<Result<Vec<_>>>()?;

    let mut projects = folders.projects;
    for ((folder, project), id) in raw_projects.iter().zip(&ids) {
        let parent = match folder {
            Some(folder) => folders.ids.get(folder.name.as_str()).copied(),
            None => None,
        };
        let depends_from = if project.build_dependencies.is_empty() {
            None
        } else {
            Some(
                project
                    .build_dependencies
                    .iter()
                    .map(|dep| dependency_id(contents, &dep.project, &raw_projects, &ids))
                    .collect::<Result<Vec<_>>>()?,
            )
        };
        let mut project = raw_project_to_api(
            contents,
            slnx.configurations.as_ref(),
            &config_names,
            project,
        )?;
        project.id = id;
        project.parent = parent;
        project.depends_from = depends_from;
        projects.push(project);
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

/// Solution folders (including implicit parents of nested folders) converted into projects
struct Folders<'a, 's> {
    projects: Vec<Project<'a>>,
    /// Folder path (`Name` attribute) to folder id
    ids: std::collections::HashMap<&'s str, &'a str>,
    /// Declared folder paths in source
    declared: Vec<(&'a str, &'a str)>,
}

impl<'a, 's> Folders<'a, 's> {
    fn new(contents: &'a str, folders: &'s [Folder]) -> Result<Self> {
        let declared = folders
            .iter()
            .map(|folder| {
                let path = borrow_in(contents, &folder.name)?;
                let id = match folder.id.as_deref() {
                    Some(id) => borrow_in(contents, id)?,
                    None => path,
                };
                Ok((path, id))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            projects: Vec::new(),
            ids: std::collections::HashMap::new(),
            declared,
        })
    }

    fn add_declared(&mut self, contents: &'a str, folder: &'s Folder) -> Result<()> {
        let path = borrow_in(contents, &folder.name)?;
        let id = self.declared_id(path).unwrap_or(path);
        let parent = self.ensure_parents(path);
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
        self.ids.insert(folder.name.as_str(), id);
        self.projects.push(folder_project(path, id, parent, items));
        Ok(())
    }

    fn declared_id(&self, path: &str) -> Option<&'a str> {
        self.declared
            .iter()
            .find(|(declared, _)| same_folder(declared, path))
            .map(|(_, id)| *id)
    }

    /// Returns id of the parent folder creating missing (not declared) parents
    fn ensure_parents(&mut self, path: &'a str) -> Option<&'a str> {
        let parent_path = parent_folder(path)?;
        if let Some(id) = self.declared_id(parent_path) {
            return Some(id);
        }
        let already_created = self
            .projects
            .iter()
            .any(|project| same_folder(project.path_or_uri, parent_path));
        if !already_created {
            let grand_parent = self.ensure_parents(parent_path);
            self.projects
                .push(folder_project(parent_path, parent_path, grand_parent, None));
        }
        Some(parent_path)
    }
}

fn folder_project<'a>(
    path: &'a str,
    id: &'a str,
    parent: Option<&'a str>,
    items: Option<Vec<&'a str>>,
) -> Project<'a> {
    Project {
        type_id: ID_SOLUTION_FOLDER,
        type_description: msbuild::describe_project(ID_SOLUTION_FOLDER),
        id,
        name: folder_name(path),
        path_or_uri: path,
        configurations: None,
        items,
        depends_from: None,
        parent,
    }
}

fn same_folder(left: &str, right: &str) -> bool {
    left.trim_matches('/')
        .eq_ignore_ascii_case(right.trim_matches('/'))
}

/// Parent folder path i.e. `/src/` for `/src/Native/`. Root has no parent.
fn parent_folder(path: &str) -> Option<&str> {
    let trimmed = path.trim_end_matches('/');
    let separator = trimmed.rfind('/')?;
    let parent = &path[..=separator];
    if parent.trim_matches('/').is_empty() {
        None
    } else {
        Some(parent)
    }
}

/// Folder name is the last segment of the folder path i.e. `Native` for `/src/Native/`
fn folder_name(path: &str) -> &str {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
}

fn project_id<'a>(contents: &'a str, project: &RawProject) -> Result<&'a str> {
    match project.id.as_deref() {
        Some(id) => borrow_in(contents, id),
        None => borrow_in(contents, &project.path),
    }
}

/// `BuildDependency` references project by path. Returns id of that project or the path itself
/// if solution has no such project.
fn dependency_id<'a>(
    contents: &'a str,
    dependency: &str,
    projects: &[(Option<&Folder>, &RawProject)],
    ids: &[&'a str],
) -> Result<&'a str> {
    let normalize = |path: &str| path.replace('\\', "/").to_ascii_lowercase();
    let dependency_path = normalize(dependency);
    match projects
        .iter()
        .position(|(_, project)| normalize(&project.path) == dependency_path)
    {
        Some(index) => Ok(ids[index]),
        None => borrow_in(contents, dependency),
    }
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
        depends_from: None,
        parent: None,
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

    fn find<'a>(solution: &'a Solution<'a>, path: &str) -> &'a Project<'a> {
        solution
            .projects
            .iter()
            .find(|project| project.path_or_uri == path)
            .expect("project")
    }

    #[test]
    fn nested_folders_keep_hierarchy() {
        // Arrange
        let slnx = r#"<Solution>
  <Folder Name="/src/" />
  <Folder Name="/src/Native/">
    <Project Path="src/Native/Native.vcxproj" />
  </Folder>
  <Project Path="App/App.csproj" />
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        assert_eq!(solution.projects.len(), 4);
        let src = find(&solution, "/src/");
        let native = find(&solution, "/src/Native/");
        assert_eq!((src.name, src.parent), ("src", None));
        assert_eq!((native.name, native.parent), ("Native", Some("/src/")));
        assert_eq!(
            find(&solution, "src/Native/Native.vcxproj").parent,
            Some("/src/Native/")
        );
        assert_eq!(find(&solution, "App/App.csproj").parent, None);
    }

    #[test]
    fn missing_parent_folders_are_created() {
        // Arrange
        let slnx = r#"<Solution>
  <Folder Name="/a/b/c/">
    <File Path="c.txt" />
  </Folder>
  <Folder Name="/a/x/" />
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        let folders = solution
            .projects
            .iter()
            .map(|project| (project.path_or_uri, project.name, project.parent))
            .collect::<Vec<_>>();
        assert_eq!(
            folders,
            vec![
                ("/a/", "a", None),
                ("/a/b/", "b", Some("/a/")),
                ("/a/b/c/", "c", Some("/a/b/")),
                ("/a/x/", "x", Some("/a/")),
            ]
        );
    }

    #[test]
    fn parent_declared_after_child_is_not_duplicated() {
        // Arrange
        let slnx = r#"<Solution>
  <Folder Name="/src/Native/" />
  <Folder Name="/src/" Id="11111111-1111-1111-1111-111111111111" />
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        assert_eq!(solution.projects.len(), 2);
        assert_eq!(
            find(&solution, "/src/Native/").parent,
            Some("11111111-1111-1111-1111-111111111111")
        );
    }

    #[test]
    fn ids_are_used_for_projects_folders_and_dependencies() {
        // Arrange
        let slnx = r#"<Solution>
  <Folder Name="/src/" Id="aaaaaaaa-0000-0000-0000-000000000000">
    <Project Path="src\App\App.csproj" Id="bbbbbbbb-0000-0000-0000-000000000000">
      <BuildDependency Project="src/Lib/Lib.csproj" />
      <BuildDependency Project="src/Missing/Missing.csproj" />
    </Project>
  </Folder>
  <Project Path="src/Lib/Lib.csproj" Id="cccccccc-0000-0000-0000-000000000000" />
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        let folder = find(&solution, "/src/");
        let app = find(&solution, "src\\App\\App.csproj");
        assert_eq!(folder.id, "aaaaaaaa-0000-0000-0000-000000000000");
        assert_eq!(app.id, "bbbbbbbb-0000-0000-0000-000000000000");
        assert_eq!(app.parent, Some("aaaaaaaa-0000-0000-0000-000000000000"));
        assert_eq!(
            app.depends_from.as_ref().unwrap(),
            &[
                "cccccccc-0000-0000-0000-000000000000",
                "src/Missing/Missing.csproj"
            ]
        );
    }

    #[test_case("/src/Native/", Some("/src/") ; "nested")]
    #[test_case("/src/", None ; "root")]
    #[test_case("/a/b/c/", Some("/a/b/") ; "deep")]
    #[test_case("src/Native/", Some("src/") ; "without leading slash")]
    #[test_case("src", None ; "without slashes")]
    fn parent_folder_cases(path: &str, expected: Option<&str>) {
        // Arrange

        // Act
        let actual = parent_folder(path);

        // Assert
        assert_eq!(actual, expected);
    }

    #[test_case("/src/Native/", "Native" ; "nested")]
    #[test_case("/src/", "src" ; "root")]
    #[test_case("src", "src" ; "without slashes")]
    fn folder_name_cases(path: &str, expected: &str) {
        // Arrange

        // Act
        let actual = folder_name(path);

        // Assert
        assert_eq!(actual, expected);
    }

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
