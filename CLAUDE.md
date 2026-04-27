# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**solv** is a Microsoft Visual Studio solution (`.sln`) validation console tool and parsing library, written in Rust. The repository is a Cargo workspace composed of two crates:

- **`solp/`** — A library that parses Visual Studio solution files into a structured AST. Uses [LALRPOP](https://github.com/lalrpop/lalrpop) for grammar generation (`solp/src/solp.lalrpop`) and custom lexing (`solp/src/lex.rs`). Exposes a `Consume` trait and `SolpWalker` for directory traversal.
- **`solv/`** — The CLI binary that consumes `solp`. Built with `clap`. Implements the subcommands: `validate`, `info`, `nuget`, `json`, `completion`, `bugreport`.

The default workspace member is `solv` (see the root `Cargo.toml`).

## Architecture

### `solp` (parsing library)
- `src/solp.lalrpop` — LALRPOP grammar. Compiled at build time by `build.rs` into `solp.rs`.
- `src/lex.rs` — Hand-written lexer feeding LALRPOP.
- `src/parser.rs` — High-level parse functions built on top of the generated parser.
- `src/ast.rs` — Internal AST produced by the grammar.
- `src/api.rs` — Public `Solution`, `Project`, `Configuration`, etc. types exposed to consumers.
- `src/msbuild.rs` — MSBuild-specific helpers (parsing referenced `.csproj`/`.vcxproj` metadata, packages, etc.).
- `src/lib.rs` — Entry point. Defines:
  - `parse_str(&str) -> Result<Solution, ...>`
  - `parse_file(path, &mut impl Consume) -> Result<...>`
  - `Consume` trait (`ok(&Solution)` / `err(path)`)
  - `SolpWalker<C: Consume>` for parallel directory walking via `jwalk`.
- `fuzz/` — `cargo-fuzz` target (`fuzz_targets/parse.rs`). Only included in the workspace when explicitly enabled (see comment in root `Cargo.toml`).

### `solv` (CLI)
- `src/main.rs` — clap command tree. Each subcommand constructs a `Consume` implementation and passes it to `scan_path` / `scan_stream`.
- `src/validate.rs` — `Validate` consumer: detects problems (duplicate configurations, missing platforms, dangling project refs, etc.) and prints a report.
- `src/info.rs` — `Info` consumer: prints summary info about a solution (projects, configurations, versions).
- `src/nuget.rs` — `Nuget` consumer: aggregates NuGet packages referenced by projects in the solution, optionally reporting version mismatches. Returns a `mismatches_found` flag used by `--fail`.
- `src/json.rs` — `Json` consumer: serializes the `Solution` to JSON (optionally pretty).
- `src/ux.rs` — Shared terminal table/colour helpers (`comfy-table`, `crossterm`).
- `src/error.rs` — Error types / miette diagnostics used by the CLI.
- `src/lib.rs` — Re-exports to expose consumers for integration tests.

### Key patterns
- **Consumer pattern**: every CLI subcommand is a `Consume` impl. Piping into `SolpWalker` gives free recursion, parallelism, and stdin support. When adding a new subcommand, add a new consumer type with `Display` + `Consume`.
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
- Changing `solp/src/solp.lalrpop` regenerates the parser via `build.rs`. After edits, run `cargo build -p solp` and check for LALRPOP conflicts.
- Public API of `solp::api` is re-exported and consumed by `solv`; breaking changes require coordinated updates in both crates.
- `solv/src/main.rs` reads stdin only for subcommands that route through `scan_path_or_stdin` (`info`, `json`). `validate` and `nuget` require a path.
- CI runs on Linux (x64/aarch64 musl), macOS (x64/arm64), and Windows (MSVC). Avoid platform-specific code outside of the existing `cfg(target_os = "linux")` mimalloc block.

## Things to do
- Create tests for a new functionality
- Write tests in AAA pattern
- If tests can be parameterized use `test-case` crate