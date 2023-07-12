[![crates.io](https://img.shields.io/crates/v/solv.svg)](https://crates.io/crates/solv)
[![downloads](https://img.shields.io/crates/d/solv.svg)](https://crates.io/crates/solv)
[![CI](https://github.com/aegoroff/solv/actions/workflows/ci.yml/badge.svg)](https://github.com/aegoroff/solv/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/aegoroff/solv/branch/master/graph/badge.svg?token=8BzaWjWe0x)](https://codecov.io/gh/aegoroff/solv)
[![](https://tokei.rs/b1/github/aegoroff/solv?category=code)](https://github.com/XAMPPRocky/tokei)

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