solp
====
A library for parsing Microsoft Visual Studio solution file

Licensed under MIT


### Documentation

https://docs.rs/solp


### Usage

Run `cargo add solp` to automatically add this crate as a dependency
in your `Cargo.toml` file.


### Example

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
          "tags": [
            "Build"
          ]
        },
        {
          "configuration": "Release",
          "solution_configuration": "Release",
          "platform": "Any CPU",
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
          "tags": [
            "Build"
          ]
        },
        {
          "configuration": "Release",
          "solution_configuration": "Release",
          "platform": "Any CPU",
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

### Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.70.0`.

The current policy is that the minimum Rust version required to use this crate
can be increased in minor version updates. For example, if `crate 1.0` requires
Rust 1.20.0, then `crate 1.0.z` for all values of `z` will also require Rust
1.20.0 or newer. However, `crate 1.y` for `y > 0` may require a newer minimum
version of Rust.

In general, this crate will be conservative with respect to the minimum
supported version of Rust.
