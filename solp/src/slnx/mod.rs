//! SLNX XML solution format support.
//!
//! Only elements and attributes used by conversion are described. Other ones
//! (e.g. project and folder level `Properties`) are skipped by deserializer.

mod config;
mod convert;
mod types;

use std::borrow::Cow;

use serde::Deserialize;

use crate::api::Solution;
use crate::parser::strip_utf8_bom;

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
    pub platforms: Vec<ConfigurationRule>,
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

/// Folder containing files and projects
#[derive(Debug, Deserialize)]
pub struct Folder {
    #[serde(rename = "File", default)]
    pub files: Vec<FileRef>,
    #[serde(rename = "Project", default)]
    pub projects: Vec<Project>,

    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Id", default)]
    pub id: Option<String>,
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
    pub platforms: Vec<ConfigurationRule>,
    #[serde(rename = "Build", default)]
    pub builds: Vec<ConfigurationRule>,
    #[serde(rename = "Deploy", default)]
    pub deploys: Vec<ConfigurationRule>,

    #[serde(rename = "@Path")]
    pub path: String,
    #[serde(rename = "@Type", default)]
    pub project_type: Option<String>,
    #[serde(rename = "@DisplayName", default)]
    pub display_name: Option<String>,
    #[serde(rename = "@Id", default)]
    pub id: Option<String>,
}

/// Build dependency (reference to another project)
#[derive(Debug, Deserialize)]
pub struct BuildDependency {
    #[serde(rename = "@Project")]
    pub project: String,
}

/// Configuration rule (BuildType, Platform, Build, Deploy)
#[derive(Debug, Deserialize)]
pub struct ConfigurationRule {
    #[serde(rename = "@Solution", default)]
    pub solution: Option<String>,
    #[serde(rename = "@Project", default)]
    pub project: Option<String>,
}

/// Properties group (PropertiesGroup)
#[derive(Debug, Deserialize)]
pub struct Properties {
    #[serde(rename = "Property", default)]
    pub properties: Vec<Property>,

    #[serde(rename = "@Name")]
    pub name: String,
}

/// Individual property
#[derive(Debug, Deserialize)]
pub struct Property {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Value", default)]
    pub value: Option<String>,
}

/// Borrows `value` (already unescaped by XML deserializer) from the source `contents`.
///
/// If value contains characters that were escaped in source (e.g. `&amp;`) or whitespace
/// normalized by XML parser (tabs and line breaks) the value isn't present in source. In this case
/// raw attribute value is returned because borrowed [`Solution`] cannot hold newly allocated strings.
pub(crate) fn borrow_in<'a>(contents: &'a str, value: &str) -> miette::Result<&'a str> {
    if value.is_empty() {
        return Ok(&contents[0..0]);
    }

    contents
        .find(value)
        .map(|start| &contents[start..start + value.len()])
        .or_else(|| find_escaped_attribute(contents, value))
        .ok_or_else(|| miette::miette!("XML value not found in source: {value}"))
}

/// Finds raw attribute value that contains entity references or whitespace to be normalized
/// and is equal to `value` after unescaping
fn find_escaped_attribute<'a>(contents: &'a str, value: &str) -> Option<&'a str> {
    contents.match_indices('=').find_map(|(eq, _)| {
        let rest = &contents[eq + 1..];
        let quoted = rest.trim_start();
        let quote = quoted.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let start = eq + 1 + (rest.len() - quoted.len()) + 1;
        let end = start + contents[start..].find(quote)?;
        let raw = &contents[start..end];
        (raw.contains(['&', '\t', '\n', '\r']) && unescape(raw).as_deref() == Some(value))
            .then_some(raw)
    })
}

/// Unescapes raw `.slnx` attribute value kept by [`Solution`] when it contains XML entities
/// (e.g. `R&amp;D/App.csproj` project path). Value without entities or malformed one is returned as is.
#[must_use]
pub(crate) fn unescape_xml(raw: &str) -> Cow<'_, str> {
    if !raw.contains('&') {
        return Cow::Borrowed(raw);
    }
    unescape(raw).map_or(Cow::Borrowed(raw), Cow::Owned)
}

/// Unescapes predefined XML entities and character references and normalizes literal tabs and
/// line breaks to spaces like XML parser does for attribute values. Returns `None` on malformed input.
fn unescape(raw: &str) -> Option<String> {
    let mut result = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(amp) = rest.find('&') {
        push_normalized(&mut result, &rest[..amp]);
        let semicolon = rest[amp..].find(';')? + amp;
        let entity = &rest[amp + 1..semicolon];
        let c = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = if let Some(hex) = entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                {
                    u32::from_str_radix(hex, 16).ok()?
                } else {
                    entity.strip_prefix('#')?.parse().ok()?
                };
                char::from_u32(code)?
            }
        };
        result.push(c);
        rest = &rest[semicolon + 1..];
    }
    push_normalized(&mut result, rest);
    Some(result)
}

/// Pushes attribute text replacing each tab, line feed, carriage return and `\r\n` pair by a space
fn push_normalized(result: &mut String, text: &str) {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                chars.next_if_eq(&'\n');
                result.push(' ');
            }
            '\t' | '\n' => result.push(' '),
            c => result.push(c),
        }
    }
}

/// Splits file name of the path into name without extension (Visual Studio default project name)
/// and extension e.g. `App.Tests` and `csproj` for `src\App\App.Tests.csproj`
fn split_file_name(path: &str) -> (&str, Option<&str>) {
    let file_name = path
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path);
    match file_name.rsplit_once('.') {
        // Name of dot file (e.g. `.hidden`) is the whole file name
        Some((stem, extension)) => (
            if stem.is_empty() { file_name } else { stem },
            Some(extension).filter(|extension| !extension.is_empty()),
        ),
        None => (file_name, None),
    }
}

/// Returns `true` when the content looks like an XML `.slnx` solution file.
#[must_use]
pub fn is_slnx(contents: &str) -> bool {
    strip_utf8_bom(contents).0.trim_start().starts_with('<')
}

/// Returns `true` if the first element of XML document is `<Solution>`.
/// XML declaration, processing instructions, comments and DOCTYPE before it are skipped.
fn has_solution_root(contents: &str) -> bool {
    const ROOT: &str = "<Solution";
    let mut rest = strip_utf8_bom(contents).0.trim_start();
    loop {
        let skip_until = if rest.starts_with("<?") {
            "?>"
        } else if rest.starts_with("<!--") {
            "-->"
        } else if rest.starts_with("<!") {
            ">"
        } else {
            break;
        };
        let Some(end) = rest.find(skip_until) else {
            return false;
        };
        rest = rest[end + skip_until.len()..].trim_start();
    }
    rest.strip_prefix(ROOT).is_some_and(|after| {
        after
            .chars()
            .next()
            .is_some_and(|c| c.is_whitespace() || c == '>' || c == '/')
    })
}

/// Parses `.slnx` XML content and converts it into the public [`Solution`] API type.
pub fn parse_str(contents: &str) -> miette::Result<Solution<'_>> {
    if !has_solution_root(contents) {
        return Err(miette::miette!(
            "Failed to parse .slnx solution file: root element must be <Solution>"
        ));
    }
    let raw = deserialize_xml(contents)?;
    convert::to_api(raw, contents)
}

fn deserialize_xml(contents: &str) -> miette::Result<SlnxSolution> {
    crate::msbuild::from_xml(
        strip_utf8_bom(contents).0.as_bytes(),
        "Failed to deserialize .slnx solution file",
    )
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
        assert_eq!(solution.projects[0].name, "App");
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
    fn parse_str_slnx_with_bom_and_xml_declaration() {
        // Arrange
        let content = "\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<Solution>\n  <Project Path=\"src/App/App.csproj\" />\n</Solution>";

        // Act
        let solution = crate::parse_str(content).unwrap();

        // Assert
        assert_eq!(solution.format, "slnx");
        assert_eq!(solution.projects.len(), 1);
    }

    #[test_case("\u{feff}<Solution></Solution>" ; "bom")]
    #[test_case("\u{feff}  \n<?xml version=\"1.0\"?><Solution></Solution>" ; "bom whitespace and declaration")]
    fn is_slnx_detects_xml_with_bom(content: &str) {
        // Arrange

        // Act
        let actual = is_slnx(content);

        // Assert
        assert!(actual);
    }

    #[test_case("Microsoft Visual Studio Solution File, Format Version 12.00\n" ; "plain")]
    #[test_case("\u{feff}\r\nMicrosoft Visual Studio Solution File, Format Version 12.00\n" ; "bom")]
    fn is_slnx_rejects_legacy_sln_variants(content: &str) {
        // Arrange

        // Act
        let actual = is_slnx(content);

        // Assert
        assert!(!actual);
    }

    #[test_case("a &amp; b", Some("a & b") ; "amp")]
    #[test_case("&lt;&gt;&quot;&apos;", Some("<>\"'") ; "predefined entities")]
    #[test_case("&#38;&#x26;&#X26;", Some("&&&") ; "character references")]
    #[test_case("plain", Some("plain") ; "no entities")]
    #[test_case("a &amp b", None ; "unterminated entity")]
    #[test_case("&unknown;", None ; "unknown entity")]
    #[test_case("a\tb\nc\r\nd\re", Some("a b c d e") ; "whitespace normalized")]
    #[test_case("&#9;&#10;&#13;", Some("\t\n\r") ; "whitespace character references kept")]
    fn unescape_cases(raw: &str, expected: Option<&str>) {
        // Arrange

        // Act
        let actual = unescape(raw);

        // Assert
        assert_eq!(actual.as_deref(), expected);
    }

    #[test_case("R&amp;D/App.csproj", "R&D/App.csproj" ; "entity")]
    #[test_case("src/App.csproj", "src/App.csproj" ; "without entities")]
    #[test_case("R&amp D/App.csproj", "R&amp D/App.csproj" ; "malformed")]
    fn unescape_xml_cases(raw: &str, expected: &str) {
        // Arrange

        // Act
        let actual = unescape_xml(raw);

        // Assert
        assert_eq!(actual, expected);
    }

    #[test_case(r#"<Folder Name="/R&amp;D/" />"#, "/R&D/", "/R&amp;D/" ; "double quoted")]
    #[test_case("<Folder Name = '/R&#38;D/' />", "/R&D/", "/R&#38;D/" ; "single quoted with spaces")]
    #[test_case(r#"<Project Path="a.csproj" />"#, "a.csproj", "a.csproj" ; "unescaped value")]
    #[test_case("<Project DisplayName=\"My\tApp\" />", "My App", "My\tApp" ; "tab")]
    #[test_case("<Project DisplayName=\"My\r\n  App\" />", "My   App", "My\r\n  App" ; "line break")]
    fn borrow_in_cases(contents: &str, value: &str, expected: &str) {
        // Arrange

        // Act
        let actual = borrow_in(contents, value).unwrap();

        // Assert
        assert_eq!(actual, expected);
    }

    #[test]
    fn borrow_in_missing_value_fails() {
        // Arrange
        let contents = r#"<Project Path="a.csproj" />"#;

        // Act
        let actual = borrow_in(contents, "b.csproj");

        // Assert
        assert!(actual.is_err());
    }

    #[test]
    fn parse_str_slnx_with_escaped_values() {
        // Arrange
        let content = r#"<Solution>
  <Folder Name="/R&amp;D/">
    <File Path="notes &amp; docs.md" />
  </Folder>
  <Project Path="src/R&amp;D/App.csproj">
    <BuildDependency Project="src/R&amp;D/Lib.csproj" />
  </Project>
  <Project Path="src/R&amp;D/Lib.csproj" />
</Solution>"#;

        // Act
        let solution = parse_str(content).unwrap();

        // Assert
        assert_eq!(solution.projects.len(), 3);
        assert_eq!(solution.projects[0].name, "R&amp;D");
        assert_eq!(
            solution.projects[0].items.as_ref().unwrap(),
            &["notes &amp; docs.md"]
        );
        assert_eq!(
            solution.projects[1].depends_from.as_ref().unwrap(),
            &[solution.projects[2].id]
        );
    }

    #[test]
    fn parse_str_slnx_with_multiline_attribute() {
        // Arrange
        let content =
            "<Solution>\n  <Project Path=\"App.csproj\" DisplayName=\"My\n  App\" />\n</Solution>";

        // Act
        let solution = parse_str(content).unwrap();

        // Assert
        assert_eq!(solution.projects[0].name, "My\n  App");
    }

    #[test_case("<Solution />" ; "self closing")]
    #[test_case("<Solution>\n</Solution>" ; "with content")]
    #[test_case("<Solution Description=\"d\"></Solution>" ; "with attribute")]
    #[test_case("\u{feff}<?xml version=\"1.0\"?>\n<!-- comment > with gt -->\n<!DOCTYPE Solution>\n<Solution/>" ; "prolog")]
    fn has_solution_root_accepts(content: &str) {
        // Arrange

        // Act
        let actual = has_solution_root(content);

        // Assert
        assert!(actual);
    }

    #[test_case("<Project Sdk=\"Microsoft.NET.Sdk\"></Project>" ; "csproj")]
    #[test_case("<SolutionX></SolutionX>" ; "similar name")]
    #[test_case("<solution></solution>" ; "different case")]
    #[test_case("<!-- unterminated comment <Solution/>" ; "unterminated comment")]
    #[test_case("<?xml version=\"1.0\"?>" ; "prolog only")]
    fn has_solution_root_rejects(content: &str) {
        // Arrange

        // Act
        let actual = has_solution_root(content);

        // Assert
        assert!(!actual);
    }

    #[test]
    fn parse_str_rejects_non_solution_xml() {
        // Arrange
        let content = r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup /></Project>"#;

        // Act
        let actual = crate::parse_str(content);

        // Assert
        assert!(actual.is_err());
    }

    #[test_case("src/App/App.csproj", "App", Some("csproj") ; "unix path")]
    #[test_case("src\\App\\App.Tests.csproj", "App.Tests", Some("csproj") ; "windows path with dots")]
    #[test_case("App", "App", None ; "without extension")]
    #[test_case("src/.hidden", ".hidden", Some("hidden") ; "dot file")]
    #[test_case("src/App.", "App", None ; "empty extension")]
    fn split_file_name_cases(path: &str, expected_name: &str, expected_extension: Option<&str>) {
        // Arrange

        // Act
        let actual = split_file_name(path);

        // Assert
        assert_eq!(actual, (expected_name, expected_extension));
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
