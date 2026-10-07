# Glossary

Domain terms used across `solp` and `solv`. Use them in code, docs and reviews.

## Solution
A Visual Studio solution file (`.sln` or `.slnx`) parsed into `solp::api::Solution`.
Its **kind** (`SolutionKind::Sln` / `SolutionKind::Slnx`) is detected by content, never by file extension.

## Solution project
An entry inside a solution (`solp::api::Project`): a project, solution folder or web site.
It only refers to a project file through `path_or_uri`, relative to the solution directory.
_Not the same as_ a project file.

## Project file
The MSBuild project file on disk (`.csproj`, `.vcxproj`, …) referenced by a solution project.
Resolved only by `solp::project_files::locate`, which yields a `ProjectLocation`:
- `Missing(path)`: the file referenced by the solution doesn't exist
- `Found(ProjectFile)`: the file exists; `path()` is canonical and serves as the file identity, `load()` parses it on demand

Solution folders, web sites and projects referenced by URI have no project file.

## Project package
A NuGet package a project file references after MSBuild evaluation (`solp::cpm::ProjectPackage`):
the project's own `PackageReference` items plus the ones inherited from `Directory.Build.props`,
`Directory.Packages.props` and `Directory.Build.targets`, with `Update` items, `VersionOverride`
and central versions applied. Resolved only by `solp::cpm::PackageResolver`.
Its version is not normalized; `solv nuget` normalizes versions before comparing them.
_Not the same as_ a `PackageReference` item, which is raw XML and may be an `Update` item.
