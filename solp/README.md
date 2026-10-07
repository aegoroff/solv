solp
====
A library for parsing Microsoft Visual Studio solution files. Both formats are supported:

- classic text solution files (`.sln`)
- XML solution files (`.slnx`)

The format is detected by file content, so `parse_str` and `parse_file` accept both.
Both formats are parsed into the same `solp::api::Solution` model.
UTF-8 BOM-prefixed files are supported.

Licensed under MIT


### Documentation

https://docs.rs/solp


### Usage

Run `cargo add solp` to automatically add this crate as a dependency
in your `Cargo.toml` file.


### Example: `.sln`

```rust
use solp::parse_str;

let solution = r#"Microsoft Visual Studio Solution File, Format Version 12.00
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "Project", "Project\Project.csproj", "{93ED4C31-2F29-49DB-88C3-AEA9AF1CA52D}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "Project.Test", "Project.Test\Project.Test.csproj", "{D5BBB06B-B46F-4342-A262-C569D4D2967C}"
EndProject
Global
	GlobalSection(SolutionConfigurationPlatforms) = preSolution
		Debug|Any CPU = Debug|Any CPU
		Release|Any CPU = Release|Any CPU
	EndGlobalSection
	GlobalSection(ProjectConfigurationPlatforms) = postSolution
		{93ED4C31-2F29-49DB-88C3-AEA9AF1CA52D}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{93ED4C31-2F29-49DB-88C3-AEA9AF1CA52D}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{93ED4C31-2F29-49DB-88C3-AEA9AF1CA52D}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{93ED4C31-2F29-49DB-88C3-AEA9AF1CA52D}.Release|Any CPU.Build.0 = Release|Any CPU
		{D5BBB06B-B46F-4342-A262-C569D4D2967C}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{D5BBB06B-B46F-4342-A262-C569D4D2967C}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{D5BBB06B-B46F-4342-A262-C569D4D2967C}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{D5BBB06B-B46F-4342-A262-C569D4D2967C}.Release|Any CPU.Build.0 = Release|Any CPU
	EndGlobalSection
EndGlobal"#;

let result = parse_str(solution);

assert!(result.is_ok());
```
Will parse solution into structure that may be represented by this json
```json
{
  "path": "",
  "format": "12.00",
  "product": "",
  "versions": [],
  "projects": [
    {
      "type_id": "{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}",
      "type_description": "C#",
      "id": "{93ED4C31-2F29-49DB-88C3-AEA9AF1CA52D}",
      "name": "Project",
      "path_or_uri": "Project\\Project.csproj",
      "configurations": [
        {
          "configuration": "Debug",
          "solution_configuration": "Debug",
          "platform": "Any CPU",
          "project_platform": "Any CPU",
          "tags": [
            "Build"
          ]
        },
        {
          "configuration": "Release",
          "solution_configuration": "Release",
          "platform": "Any CPU",
          "project_platform": "Any CPU",
          "tags": [
            "Build"
          ]
        }
      ]
    },
    {
      "type_id": "{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}",
      "type_description": "C#",
      "id": "{D5BBB06B-B46F-4342-A262-C569D4D2967C}",
      "name": "Project.Test",
      "path_or_uri": "Project.Test\\Project.Test.csproj",
      "configurations": [
        {
          "configuration": "Debug",
          "solution_configuration": "Debug",
          "platform": "Any CPU",
          "project_platform": "Any CPU",
          "tags": [
            "Build"
          ]
        },
        {
          "configuration": "Release",
          "solution_configuration": "Release",
          "platform": "Any CPU",
          "project_platform": "Any CPU",
          "tags": [
            "Build"
          ]
        }
      ]
    }
  ],
  "configurations": [
    {
      "configuration": "Debug",
      "platform": "Any CPU"
    },
    {
      "configuration": "Release",
      "platform": "Any CPU"
    }
  ]
}
```

### Example: `.slnx`

```rust
use solp::parse_str;

let solution = r#"<Solution>
  <Properties Name="Visual Studio">
    <Property Name="OpenWith" Value="Visual Studio Version 17" />
  </Properties>
  <Folder Name="/tests/">
    <Project Path="Project.Test/Project.Test.csproj">
      <BuildDependency Project="Project/Project.csproj" />
      <Build Solution="Release|*" Project="false" />
    </Project>
  </Folder>
  <Project Path="Project/Project.csproj" />
</Solution>"#;

let result = parse_str(solution);

assert!(result.is_ok());
```
Will parse solution into structure that may be represented by this json
```json
{
  "path": "",
  "format": "slnx",
  "product": "Visual Studio Version 17",
  "versions": [],
  "projects": [
    {
      "type_id": "{2150E333-8FDC-42A3-9474-1A3956D46DE8}",
      "type_description": "Solution Folder",
      "id": "/tests/",
      "name": "tests",
      "path_or_uri": "/tests/"
    },
    {
      "type_id": "{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}",
      "type_description": "C#",
      "id": "Project.Test/Project.Test.csproj",
      "name": "Project.Test",
      "path_or_uri": "Project.Test/Project.Test.csproj",
      "configurations": [
        {
          "configuration": "Debug",
          "solution_configuration": "Debug",
          "platform": "Any CPU",
          "project_platform": "Any CPU",
          "tags": [
            "Build"
          ]
        },
        {
          "configuration": "Release",
          "solution_configuration": "Release",
          "platform": "Any CPU",
          "project_platform": "Any CPU",
          "tags": []
        }
      ],
      "depends_from": [
        "Project/Project.csproj"
      ],
      "parent": "/tests/"
    },
    {
      "type_id": "{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}",
      "type_description": "C#",
      "id": "Project/Project.csproj",
      "name": "Project",
      "path_or_uri": "Project/Project.csproj",
      "configurations": [
        {
          "configuration": "Debug",
          "solution_configuration": "Debug",
          "platform": "Any CPU",
          "project_platform": "Any CPU",
          "tags": [
            "Build"
          ]
        },
        {
          "configuration": "Release",
          "solution_configuration": "Release",
          "platform": "Any CPU",
          "project_platform": "Any CPU",
          "tags": [
            "Build"
          ]
        }
      ]
    }
  ],
  "configurations": [
    {
      "configuration": "Debug",
      "platform": "Any CPU"
    },
    {
      "configuration": "Release",
      "platform": "Any CPU"
    }
  ]
}
```

`.slnx` specifics:

- Project id is the `Id` attribute if present, otherwise the project path. Folder id is the `Id`
  attribute or the folder path (e.g. `/tests/`).
- Project type, configurations and platforms are calculated like Visual Studio does: by project
  type (`Type` attribute or file extension), built-in project type rules, solution `ProjectType`
  elements and `BuildType`, `Platform`, `Build` and `Deploy` rules.
- `platform` is the solution platform and `project_platform` is the platform the project is built for.
- `parent` is the id of the solution folder that contains the project or folder.
- `parent` and `depends_from` reference projects by their declared ids in both formats
  (`.sln` GUIDs are matched ignoring case).
- `OpenWith`, `Version` and `MinimumVersion` properties of `<Properties Name="Visual Studio">`
  become `product`, `VisualStudioVersion` and `MinimumVisualStudioVersion` like in `.sln`.
- Values are borrowed from the source so values with XML entities are kept escaped
  (e.g. `R&amp;D/App.csproj`). `Solution::kind` tells the detected format.
  Use `solp::project_files::locate` to get project files on disk with unescaped paths.

### Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.88.0`.

The current policy is that the minimum Rust version required to use this crate
can be increased in minor version updates. For example, if `crate 1.0` requires
Rust 1.20.0, then `crate 1.0.z` for all values of `z` will also require Rust
1.20.0 or newer. However, `crate 1.y` for `y > 0` may require a newer minimum
version of Rust.

In general, this crate will be conservative with respect to the minimum
supported version of Rust.
