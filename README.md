[![crates.io](https://img.shields.io/crates/v/solv.svg)](https://crates.io/crates/solp)
[![downloads](https://img.shields.io/crates/d/solv.svg)](https://crates.io/crates/solv)
[![CI](https://github.com/aegoroff/solv/actions/workflows/ci.yml/badge.svg)](https://github.com/aegoroff/solv/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/aegoroff/solv/branch/master/graph/badge.svg?token=8BzaWjWe0x)](https://codecov.io/gh/aegoroff/solv)
[![](https://tokei.rs/b1/github/aegoroff/solv?category=code)](https://github.com/XAMPPRocky/tokei)
[![Minimum Stable Rust Version](https://img.shields.io/badge/Rust-1.88.0-blue?color=fc8d62&logo=rust)](https://blog.rust-lang.org/2025/06/26/Rust-1.88.0/)

# solv
Microsoft Visual Studio **SOL**ution **V**alidation console tool and parsing library written in Rust.

The repository contains two crates:

- [`solv`](https://crates.io/crates/solv) — the command-line tool that validates and analyzes
  Visual Studio solution (`.sln`) files.
- [`solp`](https://crates.io/crates/solp) — the underlying parsing library that `solv` is built on.
  It can be used independently to parse `.sln` files from your own Rust code.

## Features

- Validate Visual Studio solutions and detect common problems
  (missing projects, duplicates, invalid configurations, dangling project references, etc.).
- Show detailed information about solutions and their projects.
- Inspect NuGet packages referenced by a solution and find version mismatches.
- Convert a solution to JSON for further processing.
- Scan a single `.sln` file, read from standard input, or recursively scan a directory.
- Generate shell auto-completion scripts.

## Install the pre-compiled binary

**homebrew** (only on macOS and Linux for now):

Add my tap (do it once):
```sh
brew tap aegoroff/tap
```
And then install solv:
```sh
brew install solv
```
Update solv if already installed:
```sh
brew upgrade solv
```
**scoop**:

```sh
scoop bucket add aegoroff https://github.com/aegoroff/scoop-bucket.git
scoop install solv
```

**AUR (Arch Linux User Repository)**:

install binary package:
```sh
 yay -S solv-bin
```
or if yay reports that package not found force updating repo info
```sh
yay -Syyu solv-bin
```
install using cargo so builiding on target machine:
```sh
 yay -S solv
```
or if yay reports that package not found force updating repo info
```sh
yay -Syyu solv
```

**manually**:

Download the pre-compiled binaries from the [releases](https://github.com/aegoroff/solv/releases) and
copy to the desired location. RPM and DEB packages are available to install under RedHat or Debian based Linux distros.

**install deb package on Arch Linux**:

1. Install [debtap](https://github.com/helixarch/debtap) from AUR using yay:
```sh
 yay -S debtap
```
2. Create equivalent package using debtap:
```sh
 sudo debtap -u
 debtap solv_x.x.x_amd64.deb
```
3. Install using pacman:
```sh
sudo pacman -U solv-x.x.x-1-x86_64.pkg.tar.zst
```

## Install from source

With a recent Rust toolchain installed you can build and install `solv` from
[crates.io](https://crates.io/crates/solv):

```sh
cargo install solv
```

## Usage

`solv` is a subcommand-based CLI. Run `solv --help` to see a list of subcommands,
or `solv <subcommand> --help` for details on a specific one.

```
solv <SUBCOMMAND> [OPTIONS] [PATH]
```

Available subcommands:

| Subcommand      | Alias | Description                                                                       |
| --------------- | ----- | --------------------------------------------------------------------------------- |
| `validate`      | `va`  | Validates solutions within a directory or a single file                           |
| `validate fix`  |       | Fix redundant project references in project files                                 |
| `info`          | `i`   | Show information about found solutions (projects, configurations, versions, ...) |
| `nuget`         | `nu`  | Show NuGet packages used in solutions and detect version mismatches               |
| `json`          | `j`   | Convert solution(s) into JSON                                                     |
| `completion`    |       | Generate the autocompletion script for the specified shell                        |
| `bugreport`     |       | Collect information about the system and environment for bug reports              |

Common options accepted by `validate`, `validate fix`, `info`, `nuget` and `json`:

| Option                    | Description                                                                |
| ------------------------- | -------------------------------------------------------------------------- |
| `-e, --ext <EXTENSION>`   | Visual Studio solution extension (default: `sln`)                          |
| `-r, --recursively`       | Scan the directory recursively (default: `false`)                          |
| `--showerrors`            | Output solution parsing errors while scanning directories (default: `false`) |
| `-t, --time`              | Show scanning time when scanning a directory (default: `false`)            |

The `PATH` argument can be either a path to a single `.sln` file or to a
directory. For the `info` and `json` subcommands, if `PATH` is omitted the
solution is read from standard input.

### Examples

Validate a single solution file:

```sh
solv validate path/to/MySolution.sln
```

Recursively validate all solutions in a directory, showing only the ones with problems:

```sh
solv validate -r -p path/to/sources
```

Show information about a solution:

```sh
solv info path/to/MySolution.sln
```

Find NuGet package version mismatches and fail (non-zero exit code) if any are found:

```sh
solv nuget -r -m -f path/to/sources
```

Fix redundant project references recursively in a directory:

```sh
solv validate fix -r path/to/sources
```

Convert a solution to pretty-printed JSON:

```sh
solv json -p path/to/MySolution.sln
```

Read a solution from standard input and convert it to JSON:

```sh
cat path/to/MySolution.sln | solv json
```

Generate a shell completion script (example for `bash`):

```sh
solv completion bash > /etc/bash_completion.d/solv
```
