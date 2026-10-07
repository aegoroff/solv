use std::collections::{BTreeSet, HashMap};

use miette::Result;

use crate::api::{Project, Solution, SolutionConfiguration, Version};
use crate::msbuild;

use super::config::{
    SolutionConfigNames, project_configurations, project_setup, solution_build_types,
    solution_platforms,
};
use super::{Folder, Project as RawProject, Properties, SlnxSolution, borrow_in, split_file_name};

const ID_SOLUTION_FOLDER: &str = "{2150E333-8FDC-42A3-9474-1A3956D46DE8}";

/// Name of solution properties group with Visual Studio specific properties
const VISUAL_STUDIO_PROPERTIES: &str = "Visual Studio";
/// Version names like they're named in .sln
const VISUAL_STUDIO_VERSION: &str = "VisualStudioVersion";
const MINIMUM_VISUAL_STUDIO_VERSION: &str = "MinimumVisualStudioVersion";

/// Converts a deserialized `.slnx` document into the shared public [`Solution`] model.
/// Solution path is empty like in `.sln` parsing. File path is set by [`crate::parse_file`].
pub fn to_api<'a>(slnx: SlnxSolution, contents: &'a str) -> Result<Solution<'a>> {
    let format = match slnx.version.as_deref() {
        Some(version) => borrow_in(contents, version)?,
        None => "slnx",
    };
    let vs = VisualStudioProperties::new(contents, &slnx.properties)?;
    // OpenWith is the same as the first comment (# Visual Studio Version 17) in .sln
    let product = match (vs.open_with, slnx.description.as_deref()) {
        (Some(open_with), _) => open_with,
        (None, Some(description)) => borrow_in(contents, description)?,
        (None, None) => "",
    };
    let versions = [
        (VISUAL_STUDIO_VERSION, vs.version),
        (MINIMUM_VISUAL_STUDIO_VERSION, vs.minimum_version),
    ]
    .into_iter()
    .filter_map(|(name, version)| version.map(|version| Version { name, version }))
    .collect();

    let config_names = SolutionConfigNames {
        build_types: solution_build_types(contents, slnx.configurations.as_ref())?,
        platforms: solution_platforms(contents, slnx.configurations.as_ref())?,
    };
    let configurations: BTreeSet<SolutionConfiguration<'a>> = config_names
        .build_types
        .iter()
        .flat_map(|&configuration| {
            config_names
                .platforms
                .iter()
                .map(move |&platform| SolutionConfiguration {
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
    // The first project wins if several projects have the same path
    let mut ids_by_path = HashMap::with_capacity(raw_projects.len());
    for ((_, project), id) in raw_projects.iter().zip(&ids) {
        ids_by_path
            .entry(normalize_project_path(&project.path))
            .or_insert(*id);
    }

    let mut projects = std::mem::take(&mut folders.projects);
    for ((folder, project), id) in raw_projects.iter().zip(&ids) {
        let path = borrow_in(contents, &project.path)?;
        let setup = project_setup(contents, slnx.configurations.as_ref(), project)?;
        let configurations = project_configurations(&config_names, &setup.rules);
        let depends_from = if project.build_dependencies.is_empty() {
            None
        } else {
            Some(
                project
                    .build_dependencies
                    .iter()
                    .map(|dep| dependency_id(contents, &dep.project, &ids_by_path))
                    .collect::<Result<Vec<_>>>()?,
            )
        };
        projects.push(Project {
            type_id: setup.type_id,
            type_description: msbuild::describe_project(setup.type_id),
            id,
            name: match project.display_name.as_deref() {
                Some(display_name) => borrow_in(contents, display_name)?,
                None => split_file_name(path).0,
            },
            path_or_uri: path,
            configurations: (!configurations.is_empty()).then_some(configurations),
            items: None,
            depends_from,
            parent: folder.and_then(|folder| folders.declared_id(&folder.name)),
        });
    }

    Ok(Solution {
        path: "",
        format,
        product,
        versions,
        projects,
        configurations,
        dangling_project_configurations: None,
        duplicate_solution_configurations: None,
        duplicate_project_configurations: None,
    })
}

/// Visual Studio specific solution properties i.e.
/// `<Properties Name="Visual Studio"><Property Name="OpenWith" Value="Visual Studio Version 17" /></Properties>`
#[derive(Debug, Default)]
struct VisualStudioProperties<'a> {
    open_with: Option<&'a str>,
    version: Option<&'a str>,
    minimum_version: Option<&'a str>,
}

impl<'a> VisualStudioProperties<'a> {
    /// Property and group names are case-insensitive. The last value wins like in Visual Studio.
    fn new(contents: &'a str, properties: &[Properties]) -> Result<Self> {
        let mut result = Self::default();
        let vs_properties = properties
            .iter()
            .filter(|group| group.name.eq_ignore_ascii_case(VISUAL_STUDIO_PROPERTIES))
            .flat_map(|group| &group.properties);
        for property in vs_properties {
            let target = if property.name.eq_ignore_ascii_case("OpenWith") {
                &mut result.open_with
            } else if property.name.eq_ignore_ascii_case("Version") {
                &mut result.version
            } else if property.name.eq_ignore_ascii_case("MinimumVersion") {
                &mut result.minimum_version
            } else {
                continue;
            };
            *target = match property.value.as_deref() {
                Some(value) if !value.trim().is_empty() => Some(borrow_in(contents, value)?),
                _ => None,
            };
        }
        Ok(result)
    }
}

/// Solution folders (including implicit parents of nested folders) converted into projects
struct Folders<'a> {
    projects: Vec<Project<'a>>,
    /// Declared folder paths and ids in source
    declared: Vec<(&'a str, &'a str)>,
}

impl<'a> Folders<'a> {
    fn new(contents: &'a str, folders: &[Folder]) -> Result<Self> {
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
            declared,
        })
    }

    fn add_declared(&mut self, contents: &'a str, folder: &Folder) -> Result<()> {
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
    ids_by_path: &HashMap<String, &'a str>,
) -> Result<&'a str> {
    match ids_by_path.get(&normalize_project_path(dependency)) {
        Some(id) => Ok(id),
        None => borrow_in(contents, dependency),
    }
}

/// Paths are compared ignoring case and separator kind
fn normalize_project_path(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
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

    #[test]
    fn dependency_on_duplicate_path_resolves_to_first_project() {
        // Arrange
        let slnx = r#"<Solution>
  <Project Path="App/App.csproj">
    <BuildDependency Project="LIB\Lib.csproj" />
  </Project>
  <Project Path="Lib/Lib.csproj" Id="11111111-0000-0000-0000-000000000000" />
  <Project Path="Lib/Lib.csproj" Id="22222222-0000-0000-0000-000000000000" />
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        assert_eq!(
            find(&solution, "App/App.csproj")
                .depends_from
                .as_ref()
                .unwrap(),
            &["11111111-0000-0000-0000-000000000000"]
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
    fn visual_studio_properties_map_to_product_and_versions() {
        // Arrange
        let slnx = r#"<Solution Description="Ignored description">
  <Properties Name="Visual Studio">
    <Property Name="OpenWith" Value="Visual Studio Version 17" />
    <Property Name="Version" Value="17.10.35013.160" />
    <Property Name="MinimumVersion" Value="10.0.40219.1" />
  </Properties>
  <Project Path="App/App.csproj" />
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        assert_eq!(solution.product, "Visual Studio Version 17");
        let versions = solution
            .versions
            .iter()
            .map(|version| (version.name, version.version))
            .collect::<Vec<_>>();
        assert_eq!(
            versions,
            vec![
                ("VisualStudioVersion", "17.10.35013.160"),
                ("MinimumVisualStudioVersion", "10.0.40219.1"),
            ]
        );
    }

    #[test]
    fn visual_studio_properties_are_case_insensitive_and_last_wins() {
        // Arrange
        let slnx = r#"<Solution>
  <Properties Name="visual studio">
    <Property Name="openwith" Value="Visual Studio Version 16" />
  </Properties>
  <Properties Name="Other">
    <Property Name="Version" Value="1.0" />
  </Properties>
  <Properties Name="VISUAL STUDIO">
    <Property Name="OpenWith" Value="Visual Studio Version 17" />
  </Properties>
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        assert_eq!(solution.product, "Visual Studio Version 17");
        assert!(solution.versions.is_empty());
    }

    #[test_case(r#"<Solution Description="Description" />"#, "Description" ; "description fallback")]
    #[test_case(r#"<Solution><Properties Name="Visual Studio"><Property Name="OpenWith" /></Properties></Solution>"#, "" ; "empty value")]
    #[test_case("<Solution />", "" ; "no properties")]
    fn product_without_open_with(slnx: &str, expected: &str) {
        // Arrange

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        assert_eq!(solution.product, expected);
        assert!(solution.versions.is_empty());
    }

    #[test]
    fn project_and_folder_properties_do_not_affect_parsing() {
        // Arrange
        let slnx = r#"<Solution>
  <Folder Name="/src/">
    <Properties Name="FolderBag">
      <Property Name="A" Value="1" />
    </Properties>
    <Project Path="src/Web/Web.csproj">
      <Properties Name="Visual Studio" Scope="PostLoad">
        <Property Name="OpenWith" Value="Should not be product" />
      </Properties>
      <BuildDependency Project="src/Lib/Lib.csproj" />
    </Project>
  </Folder>
  <Project Path="src/Lib/Lib.csproj" />
</Solution>"#;

        // Act
        let solution = super::super::parse_str(slnx).unwrap();

        // Assert
        assert_eq!(solution.product, "");
        assert_eq!(solution.projects.len(), 3);
        assert_eq!(
            find(&solution, "src/Web/Web.csproj")
                .depends_from
                .as_ref()
                .unwrap(),
            &["src/Lib/Lib.csproj"]
        );
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
