use anyhow::Result;
use std::{fs::File, io::Read, path::Path};

use serde::{Deserialize, Serialize};
use serde_xml_rs::from_reader;

#[must_use]
pub fn is_solution_folder(id: &str) -> bool {
    id == ID_SOLUTION_FOLDER
}

/// Describes project by id.
/// Returns human readable description
/// or id itself if it's not match any
#[must_use]
pub fn describe_project(id: &str) -> &str {
    PROJECT_TYPES.get(id).unwrap_or(&id)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    #[serde(rename = "Sdk", default)]
    pub sdk: Option<String>,

    #[serde(rename = "ItemGroup", default)]
    pub item_group: Option<Vec<ItemGroup>>,

    #[serde(rename = "Import", default)]
    pub imports: Option<Vec<Import>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemGroup {
    #[serde(rename = "ProjectReference", default)]
    pub project_reference: Option<Vec<ProjectReference>>,
    #[serde(rename = "PackageReference", default)]
    pub package_reference: Option<Vec<PackageReference>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectReference {
    #[serde(rename = "Include", default)]
    pub include: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageReference {
    #[serde(rename = "Include", default)]
    pub include: String,
    #[serde(rename = "Version", default)]
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
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

static PROJECT_TYPES: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "{CC5FD16D-436D-48AD-A40C-5A424C6E3E79}" => "Azure Project",
    "{8BB2217D-0F2D-49D1-97BC-3654ED321F3B}" => "ASP.NET 5",
    "{603C0E0B-DB56-11DC-BE95-000D561079B0}" => "ASP.NET MVC 1",
    "{F85E285D-A4E0-4152-9332-AB1D724D3325}" => "ASP.NET MVC 2",
    "{E53F8FEA-EAE0-44A6-8774-FFD645390401}" => "ASP.NET MVC 3",
    "{E3E379DF-F4C6-4180-9B81-6769533ABE47}" => "ASP.NET MVC 4",
    "{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}" => "C#",
    "{9A19103F-16F7-4668-BE54-9A1E7A4F7556}" => "C#",
    "{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}" => "C++",
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
    "{6BC8ED88-2882-458C-8E55-DFD12B67127B}" => "MonoTouch or Xamarin.iOS",
    "{F5B4F3BC-B597-4E2B-B552-EF5D8A32436F}" => "MonoTouch Binding",
    "{786C830F-07A1-408B-BD7F-6EE04809D6DB}" => "Portable Class Library",
    "{66A26720-8FB5-11D2-AA7E-00C04F688DDE}" => "Project Folders",
    "{593B0543-81F6-4436-BA1E-4747859CAAE2}" => "SharePoint (C#)",
    "{EC05E597-79D4-47f3-ADA0-324C4F7C7484}" => "SharePoint (VB.NET)",
    "{F8810EC1-6754-47FC-A15F-DFABD2E3FA90}" => "SharePoint Workflow",
    "{A1591282-1198-4647-A2B1-27E5FF5F6F3B}" => "Silverlight",
    "{4D628B5B-2FBC-4AA6-8C16-197242AEB884}" => "Smart Device (C#)",
    "{68B1623D-7FB9-47D8-8664-7ECEA3297D4F}" => "Smart Device (VB.NET)",
    "{2150E333-8FDC-42A3-9474-1A3956D46DE8}" => "Solution Folder",
    "{3AC096D0-A1C2-E12C-1390-A8335801FDAB}" => "Test",
    "{A5A43C5B-DE2A-4C0C-9213-0A381AF9435A}" => "Universal Windows Class Library",
    "{F184B08F-C81C-45F6-A57F-5ABD9991F28F}" => "VB.NET",
    "{C252FEB5-A946-4202-B1D4-9916A0590387}" => "Visual Database Tools",
    "{54435603-DBB4-11D2-8724-00A0C9A8B90C}" => "Visual Studio 2015 Installer Project Extension",
    "{A860303F-1F3F-4691-B57E-529FC101A107}" => "Visual Studio Tools for Applications (VSTA)",
    "{BAA0C2D2-18E2-41B9-852F-F413020CAA33}" => "Visual Studio Tools for Office (VSTO)",
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
    "{581633EB-B896-402F-8E60-36F3DA191C85}" => "LightSwitch Project",
    "{8BB0C5E8-0616-4F60-8E55-A43933E57E9C}" => "LightSwitch",
    "{82B43B9B-A64C-4715-B499-D71E9CA2BD60}" => "Extensibility",
    "{9092AA53-FB77-4645-B42D-1CCCA6BD08BD}" => "Node.js",
    "{E53339B2-1760-4266-BCC7-CA923CBCF16C}" => "Docker Application",
    "{00D1A9C2-B5F0-4AF3-8072-F6C62B433612}" => "SQL Server Database",
};

pub fn read_project<P: AsRef<Path>>(path: P) -> Result<Project> {
    let file = File::open(path)?;
    read_project_from_reader(file)
}

pub fn read_project_from_reader<R: Read>(reader: R) -> Result<Project> {
    let project: Project = from_reader(reader)?;
    Ok(project)
}

impl Project {
    pub fn is_sdk_project(&self) -> bool {
        self.sdk.is_some()
            || self
                .imports
                .iter()
                .any(|i| i.iter().any(|elt| elt.sdk.is_some()))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn read_project_from_reader_test() {
        // Arrange
        let rdr = Cursor::new(REAL_SDK_PROJECT);

        // Act
        let p = read_project_from_reader(rdr).unwrap();

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
    fn sdk_project_default_project() {
        // Arrange
        let p = Project {
            sdk: None,
            item_group: None,
            imports: None,
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
}
