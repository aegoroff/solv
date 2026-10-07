//! Central Package Management support.
//!
//! Evaluates `Directory.Build.props`, `Directory.Packages.props` and `Directory.Build.targets`
//! files (including files imported by them) to get centrally managed package versions and global
//! package references. `PackageReference` items defined in these files are collected
//! too because MSBuild adds them to every project below. This is a simplified MSBuild evaluation:
//! * properties may reference other properties (`$(Name)`) and `$(MSBuildThisFileDirectory)`
//! * `[MSBuild]::GetPathOfFileAbove` and `[MSBuild]::GetDirectoryNameOfFileAbove`
//!   property functions are supported (useful for imports)
//! * imported files are evaluated before the importing file content so the importing
//!   file wins
//! * conditions are ignored, imports of missing files are skipped

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::{Component, MAIN_SEPARATOR, Path, PathBuf},
};

use miette::{IntoDiagnostic, WrapErr};
use serde::Deserialize;

use crate::msbuild::{Import, ImportGroup, PackageReference};

/// Central Package Management file name
pub const PACKAGES_PROPS: &str = "Directory.Packages.props";
/// MSBuild file that is imported before `Directory.Packages.props`
pub const BUILD_PROPS: &str = "Directory.Build.props";
/// MSBuild file that is imported at the end of the project
pub const BUILD_TARGETS: &str = "Directory.Build.targets";
/// Files implicitly imported into every project below them in MSBuild evaluation order
pub const IMPLICIT_IMPORTS: [&str; 3] = [BUILD_PROPS, PACKAGES_PROPS, BUILD_TARGETS];

const MAX_IMPORT_DEPTH: usize = 32;
const MAX_EXPAND_DEPTH: usize = 10;
const MSBUILD_FUNCTION_PREFIX: &str = "[MSBuild]::";

#[derive(Debug, Deserialize)]
struct PropsFile {
    #[serde(rename = "PropertyGroup", default)]
    property_group: Vec<HashMap<String, PropertyValue>>,
    #[serde(rename = "ItemGroup", default)]
    item_group: Vec<PropsItemGroup>,
    #[serde(rename = "Import", default)]
    imports: Vec<Import>,
    #[serde(rename = "ImportGroup", default)]
    import_group: Vec<ImportGroup>,
}

/// MSBuild property value i.e. text of the property element.
#[derive(Debug, Default, Deserialize)]
struct PropertyValue {
    #[serde(rename = "#text", default)]
    value: String,
}

#[derive(Debug, Deserialize)]
struct PropsItemGroup {
    #[serde(rename = "PackageVersion", default)]
    package_version: Vec<PackageItem>,
    #[serde(rename = "GlobalPackageReference", default)]
    global_package_reference: Vec<PackageItem>,
    #[serde(rename = "PackageReference", default)]
    package_reference: Vec<PackageItem>,
}

#[derive(Debug, Deserialize)]
struct PackageItem {
    #[serde(rename = "@Include", default)]
    include: String,
    #[serde(rename = "@Update", default)]
    update: String,
    #[serde(rename = "@Version", default)]
    version: String,
    #[serde(rename = "@VersionOverride", default)]
    version_override: Option<String>,
}

impl PackageItem {
    fn name(&self) -> &str {
        if self.include.is_empty() {
            &self.update
        } else {
            &self.include
        }
    }
}

impl PropsFile {
    fn from_reader<R: Read>(reader: R) -> miette::Result<PropsFile> {
        let config = serde_xml_rs::SerdeXml::new().overlapping_sequences(true);
        let mut de = serde_xml_rs::Deserializer::from_config(config, reader);
        PropsFile::deserialize(&mut de)
            .into_diagnostic()
            .wrap_err("Failed to deserialize MSBuild props file")
    }
}

/// Packages defined in `Directory.Build.props` and `Directory.Packages.props`
#[derive(Debug, Default)]
pub struct CentralPackages {
    /// lowercased package name to version map
    versions: HashMap<String, String>,
    global: Vec<PackageReference>,
    references: Vec<PackageReference>,
}

impl CentralPackages {
    /// Evaluates files in the order specified (see [`find_implicit_imports`]).
    /// Unreadable files are skipped.
    #[must_use]
    pub fn load<P: AsRef<Path>>(files: &[P]) -> Self {
        let mut evaluator = Evaluator::default();
        for path in files {
            evaluator.evaluate(path.as_ref(), 0);
        }
        evaluator.into_packages()
    }

    /// Centrally defined version of the package. Package name is case insensitive.
    #[must_use]
    pub fn version(&self, name: &str) -> Option<&str> {
        self.versions.get(&name.to_lowercase()).map(String::as_str)
    }

    /// Packages referenced by all projects (`GlobalPackageReference` items)
    #[must_use]
    pub fn global_references(&self) -> &[PackageReference] {
        &self.global
    }

    /// `PackageReference` items defined in props files i.e. referenced by every project below.
    /// Version is empty if it should be taken from `PackageVersion` (see [`CentralPackages::version`])
    #[must_use]
    pub fn references(&self) -> &[PackageReference] {
        &self.references
    }
}

/// Finds the nearest [`IMPLICIT_IMPORTS`] files for the project directory in MSBuild evaluation order.
#[must_use]
pub fn find_implicit_imports(project_dir: &Path) -> Vec<PathBuf> {
    IMPLICIT_IMPORTS
        .iter()
        .filter_map(|name| find_file_above(project_dir, name))
        .collect()
}

/// Finds file with the name specified in the directory or any of its parents.
#[must_use]
pub fn find_file_above(dir: &Path, name: &str) -> Option<PathBuf> {
    normalize(dir)
        .ancestors()
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// Not expanded value with directory of the file where it was defined
/// (to expand `$(MSBuildThisFileDirectory)` correctly)
#[derive(Debug)]
struct RawValue {
    value: String,
    dir: PathBuf,
}

#[derive(Debug, Default)]
struct Evaluator {
    /// lowercased property name to its value map
    properties: HashMap<String, RawValue>,
    versions: Vec<(String, RawValue)>,
    global: Vec<(String, RawValue)>,
    /// package name, version, version override
    references: Vec<(String, RawValue, Option<RawValue>)>,
    visited: HashSet<PathBuf>,
}

impl Evaluator {
    fn evaluate(&mut self, path: &Path, depth: usize) {
        let path = normalize(path);
        if depth > MAX_IMPORT_DEPTH || !self.visited.insert(path.clone()) {
            return;
        }
        let Some(props) = File::open(&path)
            .ok()
            .and_then(|f| PropsFile::from_reader(f).ok())
        else {
            return;
        };
        let dir = path.parent().unwrap_or_else(|| Path::new("")).to_path_buf();

        // own properties are set before imports so imports can use them
        // and set again after imports so the importing file wins
        self.set_properties(&props, &dir);
        let imports = props.imports.iter().chain(
            props
                .import_group
                .iter()
                .filter_map(|g| g.imports.as_ref())
                .flatten(),
        );
        for import in imports {
            let project = self.expand(&import.project, &dir, 0);
            let project = project.trim();
            if project.is_empty() {
                continue;
            }
            let import_path = dir.join(to_path(project));
            if import_path.is_file() {
                self.evaluate(&import_path, depth + 1);
            }
        }
        self.set_properties(&props, &dir);

        for group in &props.item_group {
            let versions = group.package_version.iter().map(|p| raw_item(p, &dir));
            self.versions.extend(versions);
            let global = group
                .global_package_reference
                .iter()
                .map(|p| raw_item(p, &dir));
            self.global.extend(global);
            // Update attribute changes existing references so only Include matters here
            let references = group
                .package_reference
                .iter()
                .filter(|p| !p.include.is_empty())
                .map(|p| {
                    let (name, version) = raw_item(p, &dir);
                    let version_override = p.version_override.as_ref().map(|v| RawValue {
                        value: v.clone(),
                        dir: dir.clone(),
                    });
                    (name, version, version_override)
                });
            self.references.extend(references);
        }
    }

    fn set_properties(&mut self, props: &PropsFile, dir: &Path) {
        let properties = props
            .property_group
            .iter()
            .flatten()
            .filter(|(name, _)| !name.starts_with('@'))
            .map(|(name, value)| {
                let raw = RawValue {
                    value: value.value.clone(),
                    dir: dir.to_path_buf(),
                };
                (name.to_lowercase(), raw)
            });
        self.properties.extend(properties);
    }

    fn into_packages(self) -> CentralPackages {
        // items are evaluated after all properties like MSBuild does
        let versions = self
            .versions
            .iter()
            .map(|(name, raw)| (name.to_lowercase(), self.expand_raw(raw)))
            .collect();
        let global = self
            .global
            .iter()
            .map(|(name, raw)| PackageReference {
                name: name.clone(),
                version: self.expand_raw(raw),
                version_override: None,
            })
            .collect();
        let references = self
            .references
            .iter()
            .map(|(name, version, version_override)| PackageReference {
                name: name.clone(),
                version: self.expand_raw(version),
                version_override: version_override.as_ref().map(|v| self.expand_raw(v)),
            })
            .collect();
        CentralPackages {
            versions,
            global,
            references,
        }
    }

    fn expand_raw(&self, raw: &RawValue) -> String {
        self.expand(&raw.value, &raw.dir, 0)
    }

    /// Expands `$(...)` expressions. Unknown properties and unsupported functions are left as is.
    fn expand(&self, value: &str, dir: &Path, depth: usize) -> String {
        if depth > MAX_EXPAND_DEPTH || !value.contains("$(") {
            return value.to_owned();
        }
        let mut result = String::with_capacity(value.len());
        let mut rest = value;
        while let Some(start) = rest.find("$(") {
            result.push_str(&rest[..start]);
            rest = &rest[start..];
            let Some(len) = closing_paren(&rest[2..]) else {
                break;
            };
            let (expression, tail) = rest.split_at(len + 3);
            match self.evaluate_expression(expression[2..len + 2].trim(), dir, depth) {
                Some(v) => result.push_str(&v),
                None => result.push_str(expression),
            }
            rest = tail;
        }
        result.push_str(rest);
        result
    }

    fn evaluate_expression(&self, expression: &str, dir: &Path, depth: usize) -> Option<String> {
        if let Some(prefix) = expression.get(..MSBUILD_FUNCTION_PREFIX.len())
            && prefix.eq_ignore_ascii_case(MSBUILD_FUNCTION_PREFIX)
        {
            return self.call_function(&expression[MSBUILD_FUNCTION_PREFIX.len()..], dir, depth);
        }
        let name = expression.to_lowercase();
        if name == "msbuildthisfiledirectory" {
            return Some(format!("{}{MAIN_SEPARATOR}", dir.to_string_lossy()));
        }
        let raw = self.properties.get(&name)?;
        Some(self.expand(&raw.value, &raw.dir, depth + 1))
    }

    fn call_function(&self, call: &str, dir: &Path, depth: usize) -> Option<String> {
        let open = call.find('(')?;
        let name = call[..open].trim();
        let args = call[open + 1..].trim_end().strip_suffix(')')?;
        let args: Vec<String> = split_args(args)
            .map(|a| self.expand(unquote(a.trim()), dir, depth + 1))
            .collect();
        let found = if name.eq_ignore_ascii_case("GetPathOfFileAbove") {
            // GetPathOfFileAbove(file, startingDirectory = MSBuildThisFileDirectory)
            let file = args.first()?;
            let start = args
                .get(1)
                .map_or_else(|| dir.to_path_buf(), |s| dir.join(to_path(s)));
            find_file_above(&start, file)
        } else if name.eq_ignore_ascii_case("GetDirectoryNameOfFileAbove") {
            // GetDirectoryNameOfFileAbove(startingDirectory, file)
            let start = dir.join(to_path(args.first()?));
            find_file_above(&start, args.get(1)?).and_then(|p| p.parent().map(Path::to_path_buf))
        } else {
            return None;
        };
        // MSBuild returns empty string if nothing found
        Some(
            found
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        )
    }
}

fn raw_item(item: &PackageItem, dir: &Path) -> (String, RawValue) {
    let raw = RawValue {
        value: item.version.clone(),
        dir: dir.to_path_buf(),
    };
    (item.name().to_owned(), raw)
}

/// Index of the closing paren that matches already opened one
fn closing_paren(s: &str) -> Option<usize> {
    let mut level = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => level += 1,
            ')' if level == 0 => return Some(i),
            ')' => level -= 1,
            _ => {}
        }
    }
    None
}

/// Splits function arguments by commas that are not inside parens or quotes
fn split_args(args: &str) -> impl Iterator<Item = &str> {
    let mut level = 0usize;
    let mut quoted = false;
    let mut start = 0;
    let mut parts = vec![];
    for (i, c) in args.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            '(' if !quoted => level += 1,
            ')' if !quoted => level = level.saturating_sub(1),
            ',' if !quoted && level == 0 => {
                parts.push(&args[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&args[start..]);
    parts.into_iter().filter(|a| !a.trim().is_empty())
}

fn unquote(s: &str) -> &str {
    s.strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(s)
}

/// MSBuild paths may use Windows separators. Slash is a separator on all platforms
fn to_path(s: &str) -> PathBuf {
    PathBuf::from(s.replace('\\', "/"))
}

/// Lexically removes `.` and `..` path components
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => match result.components().next_back() {
                Some(Component::Normal(_)) => {
                    result.pop();
                }
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => result.push(c),
            },
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use test_case::test_case;

    use super::*;

    /// Creates files in a new temp directory and returns the directory
    fn create_tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solp-cpm-{name}-{uniq}"));
        for (path, content) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        root
    }

    #[test]
    fn load_versions_with_properties() {
        // Arrange
        let root = create_tree(
            "props",
            &[(
                PACKAGES_PROPS,
                "\u{feff}<Project>
  <PropertyGroup>
    <ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally>
    <BssPlatform>[1.6.2]</BssPlatform>
    <DotNet>10.0.12</DotNet>
    <Empty />
  </PropertyGroup>
  <PropertyGroup Condition=\"'$(X)' == ''\">
    <Json Condition=\"'$(Y)' == ''\">13.0.3</Json>
  </PropertyGroup>
  <ItemGroup>
    <PackageVersion Include=\"Microsoft.EntityFrameworkCore\" Version=\"$(dotnet)\" />
    <PackageVersion Include=\"Bss.Platform\" Version=\"$(BssPlatform)\" />
  </ItemGroup>
  <ItemGroup>
  </ItemGroup>
  <ItemGroup Condition=\"'$(X)' == ''\">
    <PackageVersion Include=\"Newtonsoft.Json\" Version=\"$(Json)\" />
    <PackageVersion Include=\"Unknown\" Version=\"$(Unknown)\" />
  </ItemGroup>
</Project>",
            )],
        );

        // Act
        let packages = CentralPackages::load(&[&root.join(PACKAGES_PROPS)]);

        // Assert
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            Some("10.0.12"),
            packages.version("Microsoft.EntityFrameworkCore")
        );
        assert_eq!(Some("[1.6.2]"), packages.version("bss.platform"));
        assert_eq!(Some("13.0.3"), packages.version("Newtonsoft.Json"));
        assert_eq!(Some("$(Unknown)"), packages.version("Unknown"));
        assert_eq!(None, packages.version("Missing"));
        assert!(packages.global_references().is_empty());
    }

    #[test]
    fn load_properties_from_build_props() {
        // Arrange
        let root = create_tree(
            "build",
            &[
                (
                    BUILD_PROPS,
                    r"<Project>
  <PropertyGroup>
    <A>1.0.0</A>
    <B>1.0.0</B>
  </PropertyGroup>
</Project>",
                ),
                (
                    PACKAGES_PROPS,
                    r#"<Project>
  <PropertyGroup>
    <B>2.0.0</B>
  </PropertyGroup>
  <ItemGroup>
    <PackageVersion Include="a" Version="$(A)" />
    <PackageVersion Include="b" Version="$(B)" />
  </ItemGroup>
</Project>"#,
                ),
            ],
        );

        // Act
        let packages =
            CentralPackages::load(&[&root.join(BUILD_PROPS), &root.join(PACKAGES_PROPS)]);

        // Assert
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(Some("1.0.0"), packages.version("a"));
        assert_eq!(Some("2.0.0"), packages.version("b"));
    }

    #[test_case(r#"<Import Project="$([MSBuild]::GetPathOfFileAbove(Directory.Packages.props, $(MSBuildThisFileDirectory)..))" />"# ; "get path of file above")]
    #[test_case(r#"<Import Project="$([MSBuild]::GetPathOfFileAbove('Directory.Packages.props', '$(MSBuildThisFileDirectory)../'))" />"# ; "get path of file above quoted")]
    #[test_case(r#"<Import Project="$([MSBuild]::GetDirectoryNameOfFileAbove($(MSBuildThisFileDirectory).., Directory.Packages.props))\Directory.Packages.props" />"# ; "get directory name of file above")]
    #[test_case(r#"<Import Project="..\Directory.Packages.props" />"# ; "relative path")]
    #[test_case(r#"<ImportGroup><Import Project="$(MSBuildThisFileDirectory)../Directory.Packages.props" /></ImportGroup>"# ; "import group")]
    fn load_nested_props(import: &str) {
        // Arrange
        let nested = format!(
            r#"<Project>
  {import}
  <PropertyGroup>
    <B>3.0.0</B>
  </PropertyGroup>
  <ItemGroup>
    <PackageVersion Update="a" Version="2.0.0" />
    <PackageVersion Include="c" Version="$(A)" />
  </ItemGroup>
</Project>"#
        );
        let root = create_tree(
            "nested",
            &[
                (
                    PACKAGES_PROPS,
                    r#"<Project>
  <PropertyGroup>
    <A>1.0.0</A>
    <B>1.0.0</B>
  </PropertyGroup>
  <ItemGroup>
    <PackageVersion Include="a" Version="$(A)" />
    <PackageVersion Include="b" Version="$(B)" />
    <GlobalPackageReference Include="g" Version="$(A)" />
  </ItemGroup>
</Project>"#,
                ),
                (&format!("tests/{PACKAGES_PROPS}"), &nested),
            ],
        );

        // Act
        let packages = CentralPackages::load(&[&root.join("tests").join(PACKAGES_PROPS)]);

        // Assert
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(Some("2.0.0"), packages.version("a"));
        assert_eq!(Some("3.0.0"), packages.version("b"));
        assert_eq!(Some("1.0.0"), packages.version("c"));
        assert_eq!(1, packages.global_references().len());
        assert_eq!("g", packages.global_references()[0].name);
        assert_eq!("1.0.0", packages.global_references()[0].version);
    }

    #[test]
    fn load_global_package_references() {
        // Arrange
        let root = create_tree(
            "global",
            &[(
                PACKAGES_PROPS,
                r#"<Project>
  <ItemGroup>
    <PackageVersion Include="a" Version="1.0.0" />
    <GlobalPackageReference Include="StyleCop.Analyzers" Version="1.1.118" />
    <PackageVersion Include="b" Version="1.0.0" />
  </ItemGroup>
</Project>"#,
            )],
        );

        // Act
        let packages = CentralPackages::load(&[&root.join(PACKAGES_PROPS)]);

        // Assert
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(Some("1.0.0"), packages.version("a"));
        assert_eq!(Some("1.0.0"), packages.version("b"));
        let global = packages.global_references();
        assert_eq!(1, global.len());
        assert_eq!("StyleCop.Analyzers", global[0].name);
        assert_eq!("1.1.118", global[0].version);
    }

    #[test]
    fn load_package_references() {
        // Arrange
        let root = create_tree(
            "references",
            &[
                (
                    BUILD_PROPS,
                    r#"<Project>
  <Import Project="build/common.props" />
  <PropertyGroup>
    <StyleCop>1.1.118</StyleCop>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="StyleCop.Analyzers" Version="$(StyleCop)" PrivateAssets="all" />
    <PackageReference Include="SonarAnalyzer.CSharp" />
    <PackageReference Include="Roslynator.Analyzers" VersionOverride="$(StyleCop)" />
    <PackageReference Update="Some.Package" Version="9.9.9" />
  </ItemGroup>
</Project>"#,
                ),
                (
                    "build/common.props",
                    r#"<Project>
  <ItemGroup>
    <PackageReference Include="Imported" Version="1.0.0" />
  </ItemGroup>
</Project>"#,
                ),
            ],
        );

        // Act
        let packages = CentralPackages::load(&[&root.join(BUILD_PROPS)]);

        // Assert
        fs::remove_dir_all(&root).unwrap();
        let actual: Vec<_> = packages
            .references()
            .iter()
            .map(|p| {
                (
                    p.name.as_str(),
                    p.version.as_str(),
                    p.version_override.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            vec![
                ("Imported", "1.0.0", None),
                ("StyleCop.Analyzers", "1.1.118", None),
                ("SonarAnalyzer.CSharp", "", None),
                ("Roslynator.Analyzers", "", Some("1.1.118")),
            ],
            actual
        );
    }

    #[test]
    fn load_build_targets() {
        // Arrange
        let root = create_tree(
            "targets",
            &[
                (
                    PACKAGES_PROPS,
                    r#"<Project>
  <PropertyGroup>
    <A>1.0.0</A>
  </PropertyGroup>
  <ItemGroup>
    <PackageVersion Include="a" Version="$(A)" />
  </ItemGroup>
</Project>"#,
                ),
                (
                    BUILD_TARGETS,
                    r#"<Project>
  <PropertyGroup>
    <A>2.0.0</A>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="a" />
  </ItemGroup>
  <Target Name="Custom" BeforeTargets="Build">
    <ItemGroup>
      <PackageReference Include="ignored" Version="1.0.0" />
    </ItemGroup>
    <Message Text="$(A)" />
  </Target>
</Project>"#,
                ),
            ],
        );

        // Act
        let packages = CentralPackages::load(&find_implicit_imports(&root));

        // Assert
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(Some("2.0.0"), packages.version("a"));
        assert_eq!(1, packages.references().len());
        assert_eq!("a", packages.references()[0].name);
    }

    #[test]
    fn find_implicit_imports_order_and_nearest() {
        // Arrange
        let root = create_tree(
            "find",
            &[
                (BUILD_TARGETS, "<Project />"),
                (PACKAGES_PROPS, "<Project />"),
                (BUILD_PROPS, "<Project />"),
                (&format!("src/{BUILD_PROPS}"), "<Project />"),
                ("src/App/App.csproj", "<Project />"),
            ],
        );

        // Act
        let actual = find_implicit_imports(&root.join("src").join("App"));

        // Assert
        let expected = vec![
            root.join("src").join(BUILD_PROPS),
            root.join(PACKAGES_PROPS),
            root.join(BUILD_TARGETS),
        ];
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn load_self_import_not_hangs() {
        // Arrange
        let root = create_tree(
            "cycle",
            &[(
                PACKAGES_PROPS,
                r#"<Project>
  <Import Project="$(MSBuildThisFileDirectory)Directory.Packages.props" />
  <ItemGroup>
    <PackageVersion Include="a" Version="1.0.0" />
  </ItemGroup>
</Project>"#,
            )],
        );

        // Act
        let packages = CentralPackages::load(&[&root.join(PACKAGES_PROPS)]);

        // Assert
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(Some("1.0.0"), packages.version("a"));
    }

    #[test]
    fn load_missing_or_invalid_files() {
        // Arrange
        let root = create_tree("invalid", &[(BUILD_PROPS, "<Project><PropertyGroup>")]);

        // Act
        let packages =
            CentralPackages::load(&[&root.join(BUILD_PROPS), &root.join(PACKAGES_PROPS)]);

        // Assert
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(None, packages.version("a"));
        assert!(packages.global_references().is_empty());
    }

    #[test_case("1.0", "1.0" ; "no properties")]
    #[test_case("$(A)", "1" ; "single")]
    #[test_case("$(a)", "1" ; "case insensitive")]
    #[test_case("[$(A).$( B )]", "[1.2]" ; "several")]
    #[test_case("$(C)", "1.2" ; "nested")]
    #[test_case("$(D)", "$(D)" ; "unknown")]
    #[test_case("$(A", "$(A" ; "unclosed")]
    #[test_case("x$(A)$(A", "x1$(A" ; "unclosed after known")]
    #[test_case("$(E)", "$(E)" ; "recursive")]
    #[test_case("$([System.IO.Path]::Combine(a, b))", "$([System.IO.Path]::Combine(a, b))" ; "unsupported function")]
    #[test_case("$([MSBuild]::Unknown(a))", "$([MSBuild]::Unknown(a))" ; "unsupported msbuild function")]
    #[test_case("$([MSBuild]::GetPathOfFileAbove(not-exist.file))", "" ; "file above not found")]
    fn expand_tests(value: &str, expected: &str) {
        // Arrange
        let mut evaluator = Evaluator::default();
        for (name, value) in [("a", "1"), ("b", "2"), ("c", "$(A).$(B)"), ("e", "$(E)")] {
            let raw = RawValue {
                value: value.to_owned(),
                dir: PathBuf::new(),
            };
            evaluator.properties.insert(name.to_owned(), raw);
        }

        // Act
        let actual = evaluator.expand(value, Path::new("/not-exist-dir"), 0);

        // Assert
        assert_eq!(expected, actual);
    }

    #[test_case("a, b", &["a", " b"] ; "simple")]
    #[test_case("'a, b', c", &["'a, b'", " c"] ; "quoted comma")]
    #[test_case("$(x, y), z", &["$(x, y)", " z"] ; "comma in parens")]
    #[test_case("", &[] ; "empty")]
    fn split_args_tests(args: &str, expected: &[&str]) {
        // Act
        let actual: Vec<&str> = split_args(args).collect();

        // Assert
        assert_eq!(expected, actual.as_slice());
    }

    #[cfg(not(target_os = "windows"))]
    #[test_case("/a/b/../c", "/a/c" ; "parent")]
    #[test_case("/a/./b/", "/a/b" ; "current")]
    #[test_case("/..", "/" ; "parent of root")]
    #[test_case("../../a", "../../a" ; "relative parents")]
    #[test_case("a/../../b", "../b" ; "relative mixed")]
    fn normalize_tests(path: &str, expected: &str) {
        // Act
        let actual = normalize(Path::new(path));

        // Assert
        assert_eq!(Path::new(expected), actual);
    }
}
