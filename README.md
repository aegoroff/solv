[![crates.io](https://img.shields.io/crates/v/solv.svg)](https://crates.io/crates/solv)
[![downloads](https://img.shields.io/crates/d/solv.svg)](https://crates.io/crates/solv)
[![CI](https://github.com/aegoroff/solv/actions/workflows/ci.yml/badge.svg)](https://github.com/aegoroff/solv/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/aegoroff/solv/branch/master/graph/badge.svg?token=8BzaWjWe0x)](https://codecov.io/gh/aegoroff/solv)

# solv
Microsoft Visual Studio **SOL**ution **V**alidation console tool and parsing library written in Rust

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

**manually**:

Download the pre-compiled binaries from the [releases](https://github.com/aegoroff/solv/releases) and
copy to the desired location.