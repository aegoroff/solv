use miette::{IntoDiagnostic, WrapErr};
use std::{fs::File, io::Read, path::Path};

use serde::Deserialize;

/// Shows whether id specified is ID of
/// solution folder type project
#[must_use]
pub fn is_solution_folder(id: &str) -> bool {
    id == ID_SOLUTION_FOLDER
}

/// Shows whether id specified is ID of
/// Website type project
#[must_use]
pub fn is_web_site_project(id: &str) -> bool {
    id == ID_WEB_SITE_PROJECT
}

/// Describes project by id.
/// Returns human-readable description
/// or id itself if it's not match any
#[must_use]
pub fn describe_project(id: &str) -> &str {
    PROJECT_TYPES.get(id).unwrap_or(&id)
}

/// `MSBuild` project structure
#[derive(Debug, Deserialize)]
pub struct Project {
    /// MSBuild SDK if applicable
    #[serde(rename = "Sdk", default)]
    pub sdk: Option<String>,

    /// MSBuild project item groups
    #[serde(rename = "ItemGroup", default)]
    pub item_group: Option<Vec<ItemGroup>>,

    /// MSBuild project import groups
    #[serde(rename = "ImportGroup", default)]
    pub import_group: Option<Vec<ImportGroup>>,

    /// MSBuild project imports
    #[serde(rename = "Import")]
    pub imports: Option<Vec<Import>>,
}

/// Represents a group of items within an `MSBuild` project.
///
/// This struct contains references to other projects, packages,
/// and conditions that are part of the item group.
#[derive(Debug, Deserialize)]
pub struct ItemGroup {
    #[serde(rename = "ProjectReference", default)]
    pub project_reference: Option<Vec<ProjectReference>>,
    #[serde(rename = "PackageReference", default)]
    pub package_reference: Option<Vec<PackageReference>>,
    #[serde(rename = "Condition", default)]
    pub condition: Option<String>,
}

/// Represents a group of imported files.
///
/// This field contains a list of `Import` objects, which represent individual
/// imports. The structure and behavior of these imports are defined elsewhere.
#[derive(Debug, Deserialize)]
pub struct ImportGroup {
    #[serde(rename = "Import", default)]
    pub imports: Option<Vec<Import>>,
}

/// Represents a project reference in an MSBuild project.
///
/// This structure contains the `Include` element, which specifies the path to the referenced project.
#[derive(Debug, Deserialize)]
pub struct ProjectReference {
    #[serde(rename = "Include", default)]
    pub include: String,
}

/// A Package Reference represents a dependency on an external package.
///
/// This structure contains the name and version of the referenced package.
#[derive(Debug, Deserialize)]
pub struct PackageReference {
    #[serde(rename = "Include", default)]
    pub name: String,
    #[serde(rename = "Version", default)]
    pub version: String,
}

/// Represents the configuration of packages used by a project.
///
/// This structure is a collection of individual package configurations, each
/// containing information such as the package name and version.
#[derive(Debug, Deserialize)]
pub struct PackagesConfig {
    #[serde(rename = "package", default)]
    pub packages: Vec<Package>,
}

/// Represents a package in the project.
///
/// This struct represents a single package in the project's `Packages.config` file.
/// It contains information about the package, such as its name and version.
#[derive(Debug, Deserialize)]
pub struct Package {
    #[serde(rename = "id", default)]
    pub name: String,
    pub version: String,
}

///
/// Represents an import in the MSBuild project.
///
/// Attributes:
///
/// * `project`: The path to the imported project.
/// * `sdk`: The SDK version used by the imported project (optional).
/// * `condition`: A condition that must be met for the import to take effect (optional).
/// * `label`: An optional label for the import.
#[derive(Debug, Deserialize)]
pub struct Import {
    #[serde(rename = "Project", default)]
    pub project: String,
    #[serde(rename = "Sdk", default)]
    pub sdk: Option<String>,
    #[serde(rename = "Condition", default)]
    pub condition: Option<String>,
    #[serde(rename = "Label", default)]
    pub label: Option<String>,
}

const ID_SOLUTION_FOLDER: &str = "{2150E333-8FDC-42A3-9474-1A3956D46DE8}";
const ID_WEB_SITE_PROJECT: &str = "{E24C65DC-7377-472B-9ABA-BC803B73C61A}";

// all project guids from here https://github.com/JamesW75/visual-studio-project-type-guid
// convert command: awk -F '{'  '{print "\"{"$2"\" => \""$1"\","}' ./vs_guids.txt
static PROJECT_TYPES: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "{CC5FD16D-436D-48AD-A40C-5A424C6E3E79}" => "Azure Project",
    "{8BB2217D-0F2D-49D1-97BC-3654ED321F3B}" => "ASP.NET 5",
    "{356CAE8B-CFD3-4221-B0A8-081A261C0C10}" => "ASP.NET Core Empty",
    "{687AD6DE-2DF8-4B75-A007-DEF66CD68131}" => "ASP.NET Core Web API",
    "{E27D8B1D-37A3-4EFC-AFAE-77744ED86BCA}" => "ASP.NET Core Web App",
    "{065C0379-B32B-4E17-B529-0A722277FE2D}" => "ASP.NET Core Web App (Model-View-Controller)",
    "{32F807D6-6071-4239-8605-A9B2205AAD60}" => "ASP.NET Core with Angular",
    "{4C3A4DF3-0AAD-4113-8201-4EEEA5A70EED}" => "ASP.NET Core with React.js",
    "{603C0E0B-DB56-11DC-BE95-000D561079B0}" => "ASP.NET MVC 1",
    "{F85E285D-A4E0-4152-9332-AB1D724D3325}" => "ASP.NET MVC 2",
    "{E53F8FEA-EAE0-44A6-8774-FFD645390401}" => "ASP.NET MVC 3",
    "{E3E379DF-F4C6-4180-9B81-6769533ABE47}" => "ASP.NET MVC 4",
    "{30E03E5A-5F87-4398-9D0D-FEB397AFC92D}" => "Azure Functions",
    "{14B7E1DC-C58C-427C-9728-EED16291B2DA}" => "Azure Resource Group (Blank Template)",
    "{E2FF0EA2-4842-46E0-A434-C62C75BAEC67}" => "Azure Resource Group (Web app)",
    "{BFBC8063-F137-4FC6-AEB4-F96101BA5C8A}" => "Azure WebJob (.NET Framework)",
    "{C8A4CD56-20F4-440B-8375-78386A4431B9}" => "Blazor Server App",
    "{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}" => "C#",
    "{9A19103F-16F7-4668-BE54-9A1E7A4F7556}" => "C# (.Net Core)",
    "{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}" => "C++",
    "{2EFF6E4D-FF75-4ADF-A9BE-74BEC0B0AFF8}" => "Class Library",
    "{008A663C-3F22-40EF-81B0-012B6C27E2FB}" => "Console App",
    "{A9ACE9BB-CECE-4E62-9AA4-C7E7C5BD2124}" => "Database",
    "{4F174C21-8C12-11D0-8340-0000F80270F8}" => "Database (other project types)",
    "{3EA9E505-35AC-4774-B492-AD1749C4943A}" => "Deployment Cab",
    "{06A35CCD-C46D-44D5-987B-CF40FF872267}" => "Deployment Merge Module",
    "{978C614F-708E-4E1A-B201-565925725DBA}" => "Deployment Setup",
    "{AB322303-2255-48EF-A496-5904EB18DA55}" => "Deployment Smart Device Cab",
    "{F135691A-BF7E-435D-8960-F99683D2D49C}" => "Distributed System",
    "{BF6F8E12-879D-49E7-ADF0-5503146B24B8}" => "Dynamics 2012 AX C# in AOT",
    "{F2A71F9B-5D33-465A-A702-920D77279786}" => "F#",
    "{E6FDF86B-F3D1-11D4-8576-0002A516ECE8}" => "J#",
    "{20D4826A-C6FA-45DB-90F4-C717570B9F32}" => "Legacy (2003) Smart Device (C#)",
    "{CB4CE8C6-1BDB-4DC7-A4D3-65A1999772F8}" => "Legacy (2003) Smart Device (VB.NET)",
    "{b69e3092-b931-443c-abe7-7e7b65f2a37f}" => "Micro Framework",
    "{EFBA0AD7-5A72-4C68-AF49-83D382785DCF}" => "Mono for Android or Xamarin.Android",
    "{86F6BF2A-E449-4B3E-813B-9ACC37E5545F}" => "MonoDevelop Addin",
    "{6BC8ED88-2882-458C-8E55-DFD12B67127B}" => "MonoTouch or Xamarin.iOS",
    "{F5B4F3BC-B597-4E2B-B552-EF5D8A32436F}" => "MonoTouch Binding",
    "{786C830F-07A1-408B-BD7F-6EE04809D6DB}" => "Portable Class Library",
    "{66A26720-8FB5-11D2-AA7E-00C04F688DDE}" => "Project Folders",
    "{F5034706-568F-408A-B7B3-4D38C6DB8A32}" => "PowerShell",
    "{888888A0-9F3D-457C-B088-3A5042F75D52}" => "Python",
    "{593B0543-81F6-4436-BA1E-4747859CAAE2}" => "SharePoint (C#)",
    "{EC05E597-79D4-47f3-ADA0-324C4F7C7484}" => "SharePoint (VB.NET)",
    "{F8810EC1-6754-47FC-A15F-DFABD2E3FA90}" => "SharePoint Workflow",
    "{A1591282-1198-4647-A2B1-27E5FF5F6F3B}" => "Silverlight",
    "{4D628B5B-2FBC-4AA6-8C16-197242AEB884}" => "Smart Device (C#)",
    "{68B1623D-7FB9-47D8-8664-7ECEA3297D4F}" => "Smart Device (VB.NET)",
    "{2150E333-8FDC-42A3-9474-1A3956D46DE8}" => "Solution Folder",
    "{159641D6-6404-4A2A-AE62-294DE0FE8301}" => "SSIS",
    "{D183A3D8-5FD8-494B-B014-37F57B35E655}" => "SSIS",
    "{C9674DCB-5085-4A16-B785-4C70DD1589BD}" => "SSIS",
    "{F14B399A-7131-4C87-9E4B-1186C45EF12D}" => "SSRS",
    "{3AC096D0-A1C2-E12C-1390-A8335801FDAB}" => "Test",
    "{A5A43C5B-DE2A-4C0C-9213-0A381AF9435A}" => "Universal Windows Class Library",
    "{F184B08F-C81C-45F6-A57F-5ABD9991F28F}" => "VB.NET",
    "{C252FEB5-A946-4202-B1D4-9916A0590387}" => "Visual Database Tools",
    "{54435603-DBB4-11D2-8724-00A0C9A8B90C}" => "Visual Studio 2015 Installer Project Extension",
    "{A860303F-1F3F-4691-B57E-529FC101A107}" => "Visual Studio Tools for Applications (VSTA)",
    "{BAA0C2D2-18E2-41B9-852F-F413020CAA33}" => "Visual Studio Tools for Office (VSTO)",
    "{04D02946-0DBE-48F9-8383-8B75A5B7BA34}" => "Visual Studio Tools for Office Visual C# Add-in",
    "{BB1F664B-9266-4fd6-B973-E1E44974B511}" => "Visual Studio Tools for Office SharePoint",
    "{349C5851-65DF-11DA-9384-00065B846F21}" => "Web Application",
    "{E24C65DC-7377-472B-9ABA-BC803B73C61A}" => "Web Site",
    "{3D9AD99F-2412-4246-B90B-4EAA41C64699}" => "Windows Communication Foundation (WCF)",
    "{76F1466A-8B6D-4E39-A767-685A06062A39}" => "Windows Phone 8/8.1 Blank/Hub/Webview App",
    "{C089C8C0-30E0-4E22-80C0-CE093F111A43}" => "Windows Phone 8/8.1 App (C#)",
    "{DB03555F-0C8B-43BE-9FF9-57896B3C5E56}" => "Windows Phone 8/8.1 App (VB.NET)",
    "{60DC8134-EBA5-43B8-BCC9-BB4BC16C2548}" => "Windows Presentation Foundation (WPF)",
    "{BC8A1FFA-BEE3-4634-8014-F334798102B3}" => "Windows Store (Metro) Apps & Components",
    "{14822709-B5A1-4724-98CA-57A101D1B079}" => "Workflow (C#)",
    "{D59BE175-2ED0-4C54-BE3D-CDAA9F3214C8}" => "Workflow (VB.NET)",
    "{32F31D43-81CC-4C15-9DE6-3FC5453562B6}" => "Workflow Foundation",
    "{2AA76AF3-4D9E-4AF0-B243-EB9BCDFB143B}" => "Workflow Foundation (Alternate)",
    "{6D335F3A-9D43-41b4-9D22-F6F17C4BE596}" => "XNA (Windows)",
    "{2DF5C3F4-5A5F-47a9-8E94-23B4456F55E2}" => "XNA (XBox)",
    "{D399B71A-8929-442a-A9AC-8BEC78BB2433}" => "XNA (Zune)",
    "{930C7802-8A8C-48F9-8165-68863BCCD9DD}" => "WiX (Windows Installer XML)",
    "{778DAE3C-4631-46EA-AA77-85C1314464D9}" => "VB.NET",
    "{D954291E-2A0B-460D-934E-DC6B0785DB48}" => "Windows Store App Universal",
    "{EAF909A5-FA59-4C3D-9431-0FCC20D5BCF9}" => "Intel C++",
    "{7CF6DF6D-3B04-46F8-A40B-537D21BCA0B4}" => "Sandcastle Documentation",
    "{A33008B1-5DAC-44D5-9060-242E3B6E38F2}" => "#SharpDevelop",
    "{CFEE4113-1246-4D54-95CB-156813CB8593}" => "WiX (Windows Installer XML)",
    "{C1CDDADD-2546-481F-9697-4EA41081F2FC}" => "Office/SharePoint App",
    "{8DB26A54-E6C6-494F-9B32-ACBB256CD3A5}" => "Platform Toolset v120",
    "{C2CAFE0E-DCE1-4D03-BBF6-18283CF86E48}" => "Platform Toolset v141",
    "{581633EB-B896-402F-8E60-36F3DA191C85}" => "LightSwitch Project",
    "{8BB0C5E8-0616-4F60-8E55-A43933E57E9C}" => "LightSwitch",
    "{DA98106F-DEFA-4A62-8804-0BD2F166A45D}" => "LightSwitch",
    "{82B43B9B-A64C-4715-B499-D71E9CA2BD60}" => "Extensibility",
    "{9092AA53-FB77-4645-B42D-1CCCA6BD08BD}" => "Node.js",
    "{E53339B2-1760-4266-BCC7-CA923CBCF16C}" => "Docker Application",
    "{00D1A9C2-B5F0-4AF3-8072-F6C62B433612}" => "SQL Server Database",
    "{262852C6-CD72-467D-83FE-5EEB1973A190}" => "JScript",
    "{B69E3092-B931-443C-ABE7-7E7b65f2A37F}" => "Micro Framework",
    "{EC05E597-79D4-47F3-ADA0-324C4F7C7484}" => "SharePoint (VB.NET)",
    "{C7167F0D-BC9F-4E6E-AFE1-012C56B48DB5}" => "Windows Application Packaging Project (MSIX)",
    "{D399B71A-8929-442A-A9AC-8BEC78BB2433}" => "XNA (Zune)",
};

impl Project {
    pub fn from_path<P: AsRef<Path>>(path: P) -> miette::Result<Project> {
        let file = File::open(path)
            .into_diagnostic()
            .wrap_err("Failed to read project file")?;
        Project::from_reader(file)
    }

    pub fn from_reader<R: Read>(reader: R) -> miette::Result<Project> {
        let mut de =
            serde_xml_rs::Deserializer::new_from_reader(reader).non_contiguous_seq_elements(true);
        let project: Project = Project::deserialize(&mut de)
            .into_diagnostic()
            .wrap_err("Failed to deserialize project file")?;
        Ok(project)
    }

    #[must_use]
    pub fn is_sdk_project(&self) -> bool {
        self.sdk.is_some()
            || self
                .imports
                .iter()
                .any(|i| i.iter().any(|elt| elt.sdk.is_some()))
    }
}

impl PackagesConfig {
    pub fn from_path<P: AsRef<Path>>(path: P) -> miette::Result<PackagesConfig> {
        let file = File::open(path)
            .into_diagnostic()
            .wrap_err("Failed to read packages.config")?;
        PackagesConfig::from_reader(file)
    }

    pub fn from_reader<R: Read>(reader: R) -> miette::Result<PackagesConfig> {
        let mut de =
            serde_xml_rs::Deserializer::new_from_reader(reader).non_contiguous_seq_elements(true);
        let config: PackagesConfig = PackagesConfig::deserialize(&mut de)
            .into_diagnostic()
            .wrap_err("Failed to deserialize packages.config")?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn read_packages_config_from_reader_test() {
        // Arrange
        let rdr = Cursor::new(PACKAGES_CONFIG);

        // Act
        let p = PackagesConfig::from_reader(rdr).unwrap();

        // Assert
        //assert!(p.packages.is_some());
        assert_eq!(3, p.packages.len());
        assert_eq!("YaccLexTools", p.packages[0].name);
        assert_eq!("0.2.2", p.packages[0].version);
    }

    #[test]
    fn read_project_from_reader_test() {
        // Arrange
        let rdr = Cursor::new(REAL_SDK_PROJECT);

        // Act
        let p = Project::from_reader(rdr).unwrap();

        // Assert
        assert!(p.item_group.is_some());
        assert_eq!(1, p.item_group.as_ref().unwrap().len());
        assert_eq!(
            13,
            p.item_group.as_ref().unwrap()[0]
                .package_reference
                .as_ref()
                .unwrap()
                .len()
        );
    }

    #[test]
    fn read_project_with_nugets_and_project_refs_test() {
        // Arrange
        let rdr = Cursor::new(PROJECT_WITH_PKG_AND_REF);

        // Act
        let p = Project::from_reader(rdr).unwrap();

        // Assert
        assert!(p.item_group.is_some());
        assert_eq!(2, p.item_group.as_ref().unwrap().len());
        assert_eq!(
            6,
            p.item_group.as_ref().unwrap()[0]
                .package_reference
                .as_ref()
                .unwrap()
                .len()
        );
        assert_eq!(
            1,
            p.item_group.as_ref().unwrap()[1]
                .project_reference
                .as_ref()
                .unwrap()
                .len()
        );
    }

    #[test]
    fn read_project_real_vcxproj_test() {
        // Arrange
        let rdr = Cursor::new(VCXPROJ);

        // Act
        let p = Project::from_reader(rdr).unwrap();

        // Assert
        assert!(p.item_group.is_some());
        assert!(p.imports.is_some());
        assert_eq!(2, p.item_group.as_ref().unwrap().len());
        assert_eq!(3, p.imports.as_ref().unwrap().len());
    }

    #[test]
    fn sdk_project_default_project() {
        // Arrange
        let p = Project {
            sdk: None,
            item_group: None,
            imports: None,
            import_group: None,
        };

        // Act
        let actual = p.is_sdk_project();

        // Assert
        assert!(!actual);
    }

    #[test]
    fn sdk_project_sdk_elt_set() {
        // Arrange
        let p = Project {
            sdk: Some("1".to_owned()),
            item_group: None,
            imports: None,
            import_group: None,
        };

        // Act
        let actual = p.is_sdk_project();

        // Assert
        assert!(actual);
    }

    #[test]
    fn sdk_project_one_import_with_sdk_elt_set() {
        // Arrange
        let p = Project {
            sdk: None,
            item_group: None,
            imports: Some(vec![Import {
                project: "p1".to_owned(),
                sdk: Some("1".to_owned()),
                condition: None,
                label: None,
            }]),
            import_group: None,
        };

        // Act
        let actual = p.is_sdk_project();

        // Assert
        assert!(actual);
    }

    #[test]
    fn sdk_project_one_import_without_sdk_elt_set() {
        // Arrange
        let p = Project {
            sdk: None,
            item_group: None,
            imports: Some(vec![Import {
                project: "p1".to_owned(),
                sdk: None,
                condition: None,
                label: None,
            }]),
            import_group: None,
        };

        // Act
        let actual = p.is_sdk_project();

        // Assert
        assert!(!actual);
    }

    const REAL_SDK_PROJECT: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
    <PropertyGroup>
      <TargetFramework>net6.0</TargetFramework>
      <ProjectTypeGuids>{3AC096D0-A1C2-E12C-1390-A8335801FDAB};{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}</ProjectTypeGuids>
      <TestProjectType>UnitTest</TestProjectType>
      <AssemblyTitle>_tst.net</AssemblyTitle>
      <Company>Egoroff</Company>
      <Product>_tst.net</Product>
      <Copyright>Copyright © 2009-2022 Alexander Egorov</Copyright>
      <OutputPath>bin\$(Configuration)\</OutputPath>
      <AppendTargetFrameworkToOutputPath>false</AppendTargetFrameworkToOutputPath>
    </PropertyGroup>
    <PropertyGroup Condition=" '$(Configuration)|$(Platform)' == 'Debug|AnyCPU' ">
      <DebugType>portable</DebugType>
    </PropertyGroup>
    <PropertyGroup Condition=" '$(Configuration)|$(Platform)' == 'Release|AnyCPU' ">
      <DebugType>portable</DebugType>
    </PropertyGroup>
    <ItemGroup>
      <PackageReference Include="FluentAssertions" Version="6.7.0" />
      <PackageReference Include="System.Runtime.CompilerServices.Unsafe" Version="6.0.0" />
      <PackageReference Include="System.Threading.Tasks.Extensions" Version="4.5.4" />
      <PackageReference Include="System.ValueTuple" Version="4.5.0" />
      <PackageReference Include="xunit" Version="2.4.1" />
      <PackageReference Include="xunit.abstractions" Version="2.0.3" />
      <PackageReference Include="xunit.analyzers" Version="0.10.0" />
      <PackageReference Include="xunit.assert" Version="2.4.1" />
      <PackageReference Include="xunit.core" Version="2.4.1" />
      <PackageReference Include="xunit.extensibility.core" Version="2.4.1" />
      <PackageReference Include="xunit.extensibility.execution" Version="2.4.1" />
      <PackageReference Include="Microsoft.NET.Test.Sdk" Version="17.2.0" />
      <PackageReference Include="xunit.runner.visualstudio" Version="2.4.5">
        <PrivateAssets>all</PrivateAssets>
        <IncludeAssets>runtime; build; native; contentfiles; analyzers; buildtransitive</IncludeAssets>
      </PackageReference>
    </ItemGroup>
  </Project>"#;

    const PACKAGES_CONFIG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
    <packages>
      <package id="YaccLexTools" version="0.2.2" targetFramework="net45" />
      <package id="Enums.NET" version="4.0.0" targetFramework="net48" />
      <package id="FluentValidation" version="9.5.2" targetFramework="net48" />
    </packages>"#;

    const PROJECT_WITH_PKG_AND_REF: &str = r#"<Project Sdk="Microsoft.NET.Sdk">

    <PropertyGroup>
        <TargetFramework>net5.0</TargetFramework>

        <IsPackable>false</IsPackable>
        <LangVersion>latest</LangVersion>
    </PropertyGroup>

    <ItemGroup>
        <PackageReference Include="FluentAssertions" Version="5.10.3" />
        <PackageReference Include="Microsoft.NET.Test.Sdk" Version="16.8.3" />
        <PackageReference Include="Moq" Version="4.15.2" />
        <PackageReference Include="xunit" Version="2.4.1" />
        <PackageReference Include="xunit.runner.visualstudio" Version="2.4.3">
          <PrivateAssets>all</PrivateAssets>
          <IncludeAssets>runtime; build; native; contentfiles; analyzers; buildtransitive</IncludeAssets>
        </PackageReference>
        <PackageReference Include="coverlet.collector" Version="1.3.0">
          <PrivateAssets>all</PrivateAssets>
          <IncludeAssets>runtime; build; native; contentfiles; analyzers; buildtransitive</IncludeAssets>
        </PackageReference>
    </ItemGroup>

    <ItemGroup>
      <ProjectReference Include="..\Hiring\Hiring.csproj" />
    </ItemGroup>

</Project>
"#;

    const VCXPROJ: &str = r#"<?xml version="1.0" encoding="utf-8"?>
    <Project DefaultTargets="Build" ToolsVersion="14.0" xmlns="http://schemas.microsoft.com/developer/msbuild/2003">
      <ItemGroup Label="ProjectConfigurations">
        <ProjectConfiguration Include="Debug-static|Win32">
          <Configuration>Debug-static</Configuration>
          <Platform>Win32</Platform>
        </ProjectConfiguration>
        <ProjectConfiguration Include="Release|x64">
          <Configuration>Release</Configuration>
          <Platform>x64</Platform>
        </ProjectConfiguration>
      </ItemGroup>
      <ItemGroup>
        <ClCompile Include="..\..\..\..\src\arena.c" />
        <ClCompile Include="..\..\..\..\src\background_thread.c" />
        <ClCompile Include="..\..\..\..\src\base.c" />
        <ClCompile Include="..\..\..\..\src\bin.c" />
      </ItemGroup>
      <PropertyGroup Label="Globals">
        <ProjectGuid>{8D6BB292-9E1C-413D-9F98-4864BDC1514A}</ProjectGuid>
        <Keyword>Win32Proj</Keyword>
        <RootNamespace>jemalloc</RootNamespace>
        <WindowsTargetPlatformVersion>8.1</WindowsTargetPlatformVersion>
      </PropertyGroup>
      <Import Project="$(VCTargetsPath)\Microsoft.Cpp.Default.props" />
      <PropertyGroup Condition="'$(Configuration)|$(Platform)'=='Release|x64'" Label="Configuration">
        <ConfigurationType>DynamicLibrary</ConfigurationType>
        <UseDebugLibraries>false</UseDebugLibraries>
        <PlatformToolset>v140</PlatformToolset>
        <WholeProgramOptimization>true</WholeProgramOptimization>
        <CharacterSet>MultiByte</CharacterSet>
      </PropertyGroup>
      <Import Project="$(VCTargetsPath)\Microsoft.Cpp.props" />
      <ImportGroup Label="ExtensionSettings">
      </ImportGroup>
      <ImportGroup Label="Shared">
      </ImportGroup>
      <ImportGroup Label="PropertySheets" Condition="'$(Configuration)|$(Platform)'=='Release|x64'">
        <Import Project="$(UserRootDir)\Microsoft.Cpp.$(Platform).user.props" Condition="exists('$(UserRootDir)\Microsoft.Cpp.$(Platform).user.props')" Label="LocalAppDataPlatform" />
      </ImportGroup>
      <PropertyGroup Label="UserMacros" />
      <PropertyGroup Condition="'$(Configuration)|$(Platform)'=='Release|x64'">
        <OutDir>$(SolutionDir)$(Platform)\$(Configuration)\</OutDir>
        <IntDir>$(Platform)\$(Configuration)\</IntDir>
      </PropertyGroup>
      <ItemDefinitionGroup Condition="'$(Configuration)|$(Platform)'=='Release|x64'">
        <ClCompile>
          <WarningLevel>Level3</WarningLevel>
          <PrecompiledHeader>
          </PrecompiledHeader>
          <Optimization>MaxSpeed</Optimization>
          <FunctionLevelLinking>true</FunctionLevelLinking>
          <IntrinsicFunctions>true</IntrinsicFunctions>
          <AdditionalIncludeDirectories>..\..\..\..\include;..\..\..\..\include\msvc_compat;%(AdditionalIncludeDirectories)</AdditionalIncludeDirectories>
          <PreprocessorDefinitions>JEMALLOC_NO_PRIVATE_NAMESPACE;_REENTRANT;_WINDLL;DLLEXPORT;NDEBUG;%(PreprocessorDefinitions)</PreprocessorDefinitions>
          <DisableSpecificWarnings>4090;4146;4267;4334</DisableSpecificWarnings>
          <ProgramDataBaseFileName>$(OutputPath)$(TargetName).pdb</ProgramDataBaseFileName>
        </ClCompile>
        <Link>
          <SubSystem>Windows</SubSystem>
          <GenerateDebugInformation>true</GenerateDebugInformation>
          <EnableCOMDATFolding>true</EnableCOMDATFolding>
          <OptimizeReferences>true</OptimizeReferences>
        </Link>
      </ItemDefinitionGroup>
      <Import Project="$(VCTargetsPath)\Microsoft.Cpp.targets" />
      <ImportGroup Label="ExtensionTargets">
      </ImportGroup>
    </Project>"#;
}
