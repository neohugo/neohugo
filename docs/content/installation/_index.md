---
title: Installation
description: Install fugo from a release archive on Linux, macOS or Windows, or build it from source with Cargo.
weight: 20
---

fugo is a single binary. It has no runtime dependencies: Sass, JavaScript bundling, image
processing and syntax highlighting are built in.

## Release archives

Download the archive for your platform from the
[releases page](https://github.com/getfugo/fugo/releases):

| Platform | Archive |
|---|---|
| Linux, x86-64 (glibc 2.35 or later) | `fugo_<version>_linux-amd64.tar.gz` |
| Linux, ARM64 (glibc 2.35 or later) | `fugo_<version>_linux-arm64.tar.gz` |
| macOS, Apple silicon | `fugo_<version>_darwin-arm64.tar.gz` |
| macOS, Intel | `fugo_<version>_darwin-amd64.tar.gz` |
| Windows, x86-64 | `fugo_<version>_windows-amd64.zip` |

Each archive holds the `fugo` binary (`fugo.exe` on Windows) and its licence files. Check the
download against the checksums published with the release, then put the binary on your `PATH`.

### Linux

```sh
VERSION=0.149.0   # the release you want
curl -LO https://github.com/getfugo/fugo/releases/download/v$VERSION/fugo_${VERSION}_linux-amd64.tar.gz
curl -LO https://github.com/getfugo/fugo/releases/download/v$VERSION/fugo_${VERSION}_checksums.txt
sha256sum -c fugo_${VERSION}_checksums.txt --ignore-missing
tar -xzf fugo_${VERSION}_linux-amd64.tar.gz fugo
sudo install fugo /usr/local/bin/
```

### macOS

```sh
VERSION=0.149.0
curl -LO https://github.com/getfugo/fugo/releases/download/v$VERSION/fugo_${VERSION}_darwin-arm64.tar.gz
curl -LO https://github.com/getfugo/fugo/releases/download/v$VERSION/fugo_${VERSION}_checksums.txt
shasum -a 256 -c fugo_${VERSION}_checksums.txt --ignore-missing
tar -xzf fugo_${VERSION}_darwin-arm64.tar.gz fugo
sudo install fugo /usr/local/bin/
```

The binary is not notarized. If macOS refuses to open it, remove the quarantine attribute of
the downloaded file: `xattr -d com.apple.quarantine fugo`.

### Windows

Extract `fugo.exe` from the `.zip` into a directory such as `C:\Tools\fugo`, and add that
directory to your `Path` (Settings › System › About › Advanced system settings › Environment
Variables). In PowerShell:

```powershell
Get-FileHash .\fugo_0.149.0_windows-amd64.zip -Algorithm SHA256
```

Compare the hash with the line for the `.zip` in the checksums file.

### Check the installation

```sh
fugo version
```

```text
fugo v0.149.0 linux/amd64 BuildDate=…
```

## Build from source

You need Rust 1.96 or later ([rustup](https://rustup.rs/)) and a C compiler (libwebp and a few
other libraries are compiled from C):

```sh
git clone https://github.com/getfugo/fugo.git
cd fugo
cargo build --release --locked -p ssg-cli
```

The binary is `target/release/fugo`. See [Development](/contribute/development/) for the
workspace and its tests.

## Optional tools

Most sites need nothing else. Three asset pipelines run Node.js tools, looked up when a site
uses them:

| Pipeline | Tool | Install |
|---|---|---|
| [`postcss`](/asset-pipelines/postcss/) | `postcss` (`postcss-cli`) | `npm install -D postcss postcss-cli` |
| [`tailwind`](/asset-pipelines/tailwind-css/) | `tailwindcss` (`@tailwindcss/cli`) | `npm install -D tailwindcss @tailwindcss/cli` |
| [`babel`](/asset-pipelines/babel/) | `babel` (`@babel/cli`) | `npm install -D @babel/core @babel/cli` |

fugo looks for each in the project's `node_modules/.bin`, then in `FUGO_NODE_MODULES`, then on
your `PATH`; `FUGO_POSTCSS_BIN`, `FUGO_TAILWINDCSS_BIN` and `FUGO_BABEL_BIN` name a binary
directly.
