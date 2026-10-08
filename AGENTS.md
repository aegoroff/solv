# AGENTS.md

This file provides guidance to LLM when working with code in this repository.

## Project Overview

**solv** is a Microsoft Visual Studio solution (`.sln` and `.slnx`) validation console tool and parsing library, written in Rust. The repository is a Cargo workspace composed of two crates:

- **`solp/`** — A library that parses Visual Studio solution files into a structured AST. Classic `.sln` files are parsed with [LALRPOP](https://github.com/lalrpop/lalrpop) grammar (`solp/src/solp.lalrpop`) and custom lexing (`solp/src/lex.rs`); XML `.slnx` files are deserialized with `serde-xml-rs` (`solp/src/slnx/`). Exposes a `Consume` trait and `SolpWalker` for directory traversal.
- **`solv/`** — The CLI binary that consumes `solp`. Built with `clap`. Implements the subcommands: `validate`, `info`, `nuget`, `json`, `completion`, `bugreport`.

The default workspace member is `solv` (see the root `Cargo.toml`).

## Architecture

### `solp` (parsing library)
- `src/solp.lalrpop` — LALRPOP grammar. Compiled at build time by `build.rs` into `solp.rs`.
- `src/lex.rs` — Hand-written lexer feeding LALRPOP.
- `src/parser.rs` — High-level parse functions built on top of the generated parser.
- `src/ast.rs` — Internal AST produced by the grammar.
- `src/api.rs` — Public `Solution`, `Project`, `Configuration`, etc. types exposed to consumers. Also `Solution::resolve_references` — the shared step `parse_str` runs after either format parser: `parent` and `depends_from` become declared project ids (case-insensitive).
- `src/sln/` — `.sln` support: `convert.rs` builds `api::Solution` from `ast::Sol` (configurations grouping, nested projects, danglings, duplicates), symmetric to `slnx/convert.rs`.
- `src/msbuild.rs` — MSBuild-specific helpers (parsing referenced `.csproj`/`.vcxproj` metadata, packages, etc.).
- `src/project_files.rs` — Project files: `locate(&Solution)` resolves every local solution project to its MSBuild project file on disk (`ProjectLocation::Missing` / `Found(ProjectFile)`, canonical path, lazy `load()`); `ProjectFile::reference(include)` resolves a `ProjectReference` path relative to the project. Owns path joining, `\` → `/` conversion, URI skipping and `.slnx` XML unescaping by `Solution::kind`. Paths are matched ignoring case when they don't exist as written (`canonicalize_ignoring_case`, also for `ProjectReference` paths), because solutions come from case-insensitive Windows. Consumers must not resolve project paths themselves.
- `src/cpm.rs` — Central Package Management: simplified evaluation of `Directory.Build.props` + `Directory.Packages.props` + `Directory.Build.targets` (imports, `$(Property)` expansion, `GetPathOfFileAbove`/`GetDirectoryNameOfFileAbove`, `PackageVersion`, `GlobalPackageReference`, `PackageReference` inherited by projects). The only public entry point is `PackageResolver::packages(&ProjectFile) -> Vec<ProjectPackage>`: it applies MSBuild order (inherited references, `Update` items before/after project items, `VersionOverride`, `PackageVersion` fallback) and caches evaluated files. Versions are returned as MSBuild sees them; NuGet version normalization stays in `solv nuget`.
- `src/lib.rs` — Entry point. Defines:
  - `parse_str(&str) -> Result<Solution, ...>` — detects the format by content (`slnx::is_slnx`) and routes to `.sln` or `.slnx` parser
  - `parse_file(path, &mut impl Consume) -> Result<...>`
  - `Consume` trait (`ok(&Solution)` / `err(path)`)
  - `SolpWalker<C: Consume>` for directory walking via `dua-core`. Extension may be a comma-separated list; default is `DEFAULT_SOLUTION_EXTENSIONS` (`sln,slnx`).
- `src/slnx/` — `.slnx` support:
  - `mod.rs` — serde schema of the XML format, format detection, `borrow_in` (borrows deserialized values from source text so `Solution` keeps borrowing input).
  - `convert.rs` — conversion into `api::Solution` (folders hierarchy, ids, dependencies, Visual Studio properties).
  - `config.rs` — configuration rules (`BuildType`, `Platform`, `Build`, `Deploy`) with `BuildType|Platform` patterns; the last matching rule wins.
  - `types.rs` — built-in project types table with implicit rules and project type resolution (`Type`, extension, `ProjectType`, `BasedOn`). Mirrors Microsoft.VisualStudio.SolutionPersistence.
- `fuzz/` — `cargo-fuzz` target (`fuzz_targets/parse.rs`). Only included in the workspace when explicitly enabled (see comment in root `Cargo.toml`).

### `solv` (CLI)
- `src/main.rs` — clap command tree. Each subcommand constructs a `Consume` implementation and passes it to `scan_path` / `scan_stream`.
- `src/validate.rs` — `Validate` consumer: each `Validator` (cycles, danglings, duplicate GUIDs/configurations, not found, missings, orphans, redundant references) returns `Option<Problem>`; problems are collected into per-solution reports and counted in `Statistic`. `ValidateFix` removes redundant references from project files.
- `src/info.rs` — `Info` consumer: collects summary info about solutions (projects, configurations, versions) and totals.
- `src/nuget.rs` — `Nuget` consumer: aggregates NuGet packages referenced by projects in the solution, optionally reporting version mismatches. `mismatches_found()` is used by `--fail`.
- `src/json.rs` — `Json` consumer: serializes the `Solution` to JSON (optionally pretty).
- `src/ux.rs` — Shared terminal table/colour helpers (`comfy-table`, `crossterm`).
- `src/error.rs` — Error types / miette diagnostics used by the CLI.
- `src/lib.rs` — Re-exports to expose consumers for integration tests.

### Key patterns
- **Consumer pattern**: every CLI subcommand is a `Consume` impl. Piping into `SolpWalker` gives free recursion, parallelism, and stdin support. When adding a new subcommand, add a new consumer type with `Display` + `Consume`. Keep analysis separate from printing: a consumer's `report(&Solution)` method analyzes the solution, updates totals and returns a per-solution report that implements `Display`; `Consume::ok` only prints that report right away, so results appear while scanning. The consumer's own `Display` writes totals only; `main.rs` prints it after the scan. Tests assert on returned reports and `to_string()`. Use `ux::write_*` helpers that write to a formatter.
- **Global allocator**: on Linux `solv` uses `mimalloc` as the global allocator (`#[global_allocator]` in `main.rs`).
- **`unsafe_code = "forbid"`** is set in `[workspace.lints.rust]` — do not introduce `unsafe`.
- Dependency versions are pinned with `=x.y.z` throughout. Keep this style when adding dependencies.

## Build, Test, Lint

All commands are run from the workspace root.

```sh
# Build everything (debug)
cargo build --workspace

# Build release (LTO + strip + panic=abort per release profile)
cargo build --workspace --release

# Run the CLI without installing
cargo run -- validate path/to/dir
cargo run -- info path/to/solution.sln
cargo run -- nuget --mismatch --fail path/to/dir
cargo run -- json --pretty path/to/solution.sln

# Tests (workspace-wide)
cargo test --workspace --release

# Lint (CI uses -Dwarnings on --all-features --release)
cargo clippy --workspace --all-features --release -- -D warnings

# Coverage (as run in CI)
cargo llvm-cov --workspace --lcov --output-path lcov.info

# Security audit (CI)
cargo audit
```

### Minimum Rust version
Rust **1.88.0** or newer. Both crates use `edition = "2024"` and the workspace uses `resolver = "3"`.

## Things to watch out for
- `.slnx` behavior follows the reference implementation [Microsoft.VisualStudio.SolutionPersistence](https://github.com/microsoft/vs-solutionpersistence). Check it before changing project type or configuration rules logic.
- `api::ProjectConfiguration::platform` is always the solution platform; the project platform is in `project_platform`. `api::Project::parent` is the containing solution folder id and `depends_from` holds ids of projects the project depends on; both are resolved to declared project ids by `Solution::resolve_references`, so consumers compare ids as is. Both formats must fill these fields the same way: `parse_str_sln_and_slnx_give_same_*` tests in `solp/src/lib.rs` check it.
- Changing `solp/src/solp.lalrpop` regenerates the parser via `build.rs`. After edits, run `cargo build -p solp` and check for LALRPOP conflicts.
- Public API of `solp::api` is re-exported and consumed by `solv`; breaking changes require coordinated updates in both crates.
- `solv/src/main.rs` reads stdin only for subcommands that route through `scan_path_or_stdin` (`info`, `json`). `validate` and `nuget` require a path.
- CI runs on Linux (x64/aarch64 musl), macOS (x64/arm64), and Windows (MSVC). Avoid platform-specific code outside of the existing `cfg(target_os = "linux")` mimalloc block.

## Things to do
- Create tests for a new functionality
- Write tests in AAA pattern
- If tests can be parameterized use `test-case` crate

## Things NOT to do
- Dont add new crates
- Dont use unsafe code
- Dont use multithreading and dont validate problems with it
- Dont search performmance, copy/paste, architecture problems in tests