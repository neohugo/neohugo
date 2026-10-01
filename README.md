[bep]: https://github.com/bep
[bugs]: https://github.com/gohugoio/hugo/issues?q=is%3Aopen+is%3Aissue+label%3ABug
[contributing]: CONTRIBUTING.md
[create a proposal]: https://github.com/gohugoio/hugo/issues/new?labels=Proposal%2C+NeedsTriage&template=feature_request.md
[documentation repository]: https://github.com/gohugoio/hugoDocs
[documentation]: https://gohugo.io/documentation
[features]: https://gohugo.io/about/features/
[forum]: https://discourse.gohugo.io
[friends]: https://github.com/gohugoio/hugo/graphs/contributors
[hugo modules]: https://gohugo.io/hugo-modules/
[installation]: #installation
[issue queue]: https://github.com/gohugoio/hugo/issues
[releases]: https://github.com/neohugo/neohugo/releases
[requesting help]: https://discourse.gohugo.io/t/requesting-help/9132
[rust]: https://www.rust-lang.org/
[spf13]: https://github.com/spf13
[static site generator]: https://en.wikipedia.org/wiki/Static_site_generator
[support]: https://discourse.gohugo.io
[themes]: https://themes.gohugo.io/
[website]: https://gohugo.io

what is the different between neohugo vs hugo?

[Neohugo vs Hugo](https://github.com/neohugo/neohugo/wiki/Diff-hugo-neohugo)


[Website](https://neohugo.github.io) |
[Forum](https://github.com/neohugo/neohugo/discussions) |
[Documentation](https://neohugo.github.io/getting-started/) |
[Installation Guide](https://neohugo.github.io/getting-started/installing/) |
[Contribution Guide](CONTRIBUTING.md)

[![CI](https://github.com/neohugo/neohugo/actions/workflows/ci.yml/badge.svg)](https://github.com/neohugo/neohugo/actions/workflows/ci.yml)

[Website] | [Installation] | [Documentation] | [Support] | [Contributing] | <a rel="me" href="https://fosstodon.org/@gohugoio">Mastodon</a>
## Overview

Neohugo is a fork of Hugo, a [static site generator] optimized for speed and designed for flexibility, rewritten in [Rust]. With its advanced templating system and fast asset pipelines, neohugo renders a complete site in seconds, often less.

Due to its flexible framework, multilingual support, and powerful taxonomy system, Hugo is widely used to create:

- Corporate, government, nonprofit, education, news, event, and project sites
- Documentation sites
- Image portfolios
- Landing pages
- Business, professional, and personal blogs
- Resumes and CVs

Use Hugo's embedded web server during development to instantly see changes to content, structure, behavior, and presentation. Then deploy the site to your host, or push changes to your Git provider for automated builds and deployment.

Neohugo's fast asset pipelines include:

- Image processing &ndash; Convert, resize, crop, rotate, adjust colors, apply filters, overlay text and images, and extract EXIF data
- JavaScript bundling &ndash; Transpile TypeScript and JSX to JavaScript, bundle, tree shake, minify, create source maps, and perform SRI hashing.
- Sass processing &ndash; Transpile Sass to CSS, bundle, tree shake, minify, create source maps, perform SRI hashing, and integrate with PostCSS
- Tailwind CSS processing &ndash; Compile Tailwind CSS utility classes into standard CSS, bundle, tree shake, optimize, minify, perform SRI hashing, and integrate with PostCSS

Neohugo reads Hugo's project layout and configuration (a `neohugo.toml` wins over a `hugo.toml` next to it). Its templates are Tera 2 with Hugo's v0.146 layout names instead of Go templates; [Upgrading from the Go build](#upgrading-from-the-go-build) says what else changed with v0.149. The known differences from Hugo are listed in [docs/rust-port/HANDOFF.md](docs/rust-port/HANDOFF.md#7-known-deviations-from-hugo).

See the [features] section of the documentation for a comprehensive summary of Hugo's capabilities.

## Sponsors

<p>&nbsp;</p>
<p float="left">
  <a href="https://www.linode.com/?utm_campaign=hugosponsor&utm_medium=banner&utm_source=hugogithub" target="_blank"><img src="https://raw.githubusercontent.com/gohugoio/gohugoioTheme/master/assets/images/sponsors/linode-logo_standard_light_medium.png" width="200" alt="Linode"></a>
&nbsp;&nbsp;&nbsp;
  <a href="https://www.jetbrains.com/go/?utm_source=OSS&utm_medium=referral&utm_campaign=hugo" target="_blank"><img src="https://raw.githubusercontent.com/gohugoio/gohugoioTheme/master/assets/images/sponsors/goland.svg" width="200" alt="The complete IDE crafted for professional Go developers."></a>
</p>

## Installation

Download the archive for your platform from the [releases] page. Releases are tagged `v<version>`; v0.149 and later are the Rust implementation, v0.148.2 and earlier are the former Go implementation. The archives are named as before, `neohugo_<version>_<os>-<arch>.tar.gz` (`.zip` for Windows), and hold the `neohugo` binary, `README.md`, `LICENSE`, `THIRD_PARTY_NOTICES.txt`, `PROVENANCE.md` and `THIRD_PARTY/`:

- `linux-amd64`, `linux-arm64` (glibc 2.35 or later)
- `darwin-amd64`, `darwin-arm64`
- `windows-amd64`

Check a download with `sha256sum -c neohugo_<version>_checksums.txt --ignore-missing`, then put `neohugo` on your `PATH`:

```text
neohugo version
neohugo -s <site>                # build into the publish directory
neohugo server -s <site>         # development server with live reload
```

The commands and flags (Hugo's, in kebab-case with the camelCase spellings as aliases) are listed in [crates/cli/README.md](crates/cli/README.md).

## Upgrading from the Go build

v0.149 replaces the Go neohugo in place: the binary, the `neohugo version` line and the release archives keep their names and formats. What a site or a script may have to change:

- **Layouts are Tera 2 templates, not Go templates**, with Hugo's v0.146 layout names (`home.html`, `single.html`, `_partials/`, `_shortcodes/`, `_markup/`). A layout with Go template syntax or a legacy name is an error that says what to change. [docs/rust-port/template-api.md](docs/rust-port/template-api.md) lists every function, filter and test with Hugo's name for each and how Go-template idioms translate; `neohugo templates check -s <site>` checks a site's templates against it. Output is escaped by output format (HTML and XML), not by context as in Go's `html/template`: in `<script>` use `jsonify | safe`, in query strings `urlencode`.
- **Commands and flags:** `build` (also with no command), `server`, `templates check`, `config` and `version`; the Go build's `env`, `new`, `mod`, `deploy`, `list`, `gen`, `convert`, `import`, `release`, `server trust` and `config mounts`, and the `completion` and `help` commands are not available (`--help` prints the help). Of Hugo's flags, only those [crates/cli/README.md](crates/cli/README.md) lists are accepted; another, such as `--gc`, `--logLevel` or the build's `-w`/`--watch`, is an error. `config` prints JSON by default (the Go build: TOML) and takes `--format json` or `toml`, not `yaml`, `--lang` or `--printZero`.
- **`server` renders into memory by default.** The Go build wrote the site to the publish directory and served it from there (`-M`/`--renderToMemory` rendered into memory). For the Go behaviour add `--render-to-disk` (`--renderToDisk`), a flag of neohugo, not of the Go build; `-d`/`--destination` without it is an error.
- **[Hugo Modules]** are not downloaded: themes come from the themes directory, `_vendor` or an absolute path.
- **`js.Build` runs an esbuild binary** instead of a built-in esbuild, and the release archives do not include one: install esbuild and set `NEOHUGO_ESBUILD_BINARY` ([External tools](#external-tools)). Sass is compiled in process with dart-sass semantics, whatever `transpiler` says.
- The Docker images (`neohugo/neohugo`, `ghcr.io/neohugo/neohugo`) are no longer updated; they stay at the last Go build.

## Build from source

Prerequisites to build neohugo from source:

- Rust 1.94 or later (`rust-version` in `Cargo.toml`; CI builds with 1.94.1)
- A C compiler (libwebp and ring are compiled with the `cc` crate)

Build neohugo:

```text
cargo build --release --locked -p neohugo
```

The binary is `target/release/neohugo`. [DEVELOPMENT.md](DEVELOPMENT.md) describes the workspace, its tests and the CI and release workflow.

## External tools

Sass is compiled in process. The other asset pipelines run external tools, looked up when a site uses them:

- `js_build` (`js.Build`): esbuild, named by `NEOHUGO_ESBUILD_BINARY` (default: `tools/esbuild/bin/esbuild`, relative to the working directory, which fits only a checkout of this repository; esbuild is not looked up on `PATH`). The tests use esbuild 0.25.6, the version `tools/neohugo/node/package.json` pins and the Go build linked. With a release archive, install that version and point the variable at its binary by an absolute path, e.g. `npm install esbuild@0.25.6` in a directory of your choice, then `NEOHUGO_ESBUILD_BINARY=<dir>/node_modules/@esbuild/<os>-<arch>/bin/esbuild` (`linux-x64`, `linux-arm64`, `darwin-x64`, `darwin-arm64`; on Windows `<dir>\node_modules\@esbuild\win32-x64\esbuild.exe`). In a checkout, `tools/neohugo/node.sh && tools/esbuild/install.sh` installs it at the default path.
- `postcss`, `tailwind` and `babel` (`css.PostCSS`, `css.TailwindCSS`, `js.Babel`): `postcss`, `tailwindcss` and `babel`, named by `NEOHUGO_POSTCSS_BIN`, `NEOHUGO_TAILWINDCSS_BIN` and `NEOHUGO_BABEL_BIN`, else looked up in the project's `node_modules/.bin`, in the `.bin` of each directory of `NEOHUGO_NODE_MODULES` (a path list), then on `PATH`. As in Hugo, `security.exec.allow` must allow them (the default allows `postcss` and `tailwindcss`, not `babel`).

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=gohugoio/hugo&type=Timeline)](https://star-history.com/#gohugoio/hugo&Timeline)

## Documentation

Hugo's [documentation] includes installation instructions, a quick start guide, conceptual explanations, reference information, and examples.

Please submit documentation issues and pull requests to the [documentation repository].

## Support

Please **do not use the issue queue** for questions or troubleshooting. Unless you are certain that your issue is a software defect, use the [forum].

Hugo’s [forum] is an active community of users and developers who answer questions, share knowledge, and provide examples. A quick search of over 20,000 topics will often answer your question. Please be sure to read about [requesting help] before asking your first question.

## Contributing

You can contribute to the Hugo project by:

- Answering questions on the [forum]
- Improving the [documentation]
- Monitoring the [issue queue]
- Creating or improving [themes]
- Squashing [bugs]

Please submit documentation issues and pull requests to the [documentation repository].

If you have an idea for an enhancement or new feature, create a new topic on the [forum] in the "Feature" category. This will help you to:

- Determine if the capability already exists
- Measure interest
- Refine the concept

If there is sufficient interest, [create a proposal]. Do not submit a pull request until the project lead accepts the proposal.

For a complete guide to contributing to Hugo, see the [Contribution Guide](CONTRIBUTING.md).

The code is the Cargo workspace at the repository root: [DEVELOPMENT.md](DEVELOPMENT.md) has the layout, the commands and the CI and release workflow, and [docs/rust-port/HANDOFF.md](docs/rust-port/HANDOFF.md) the crate map, the parity gates, the deviations from Hugo and the open items.

## Dependencies

Neohugo stands on the shoulders of great open source libraries. The Rust crates it uses are declared in [Cargo.toml](Cargo.toml) (`[workspace.dependencies]`, locked by `Cargo.lock`); material taken from other projects is listed in [PROVENANCE.md](PROVENANCE.md), with licences cargo cannot see in [THIRD_PARTY](THIRD_PARTY/README.md). The `THIRD_PARTY_NOTICES.txt` of each release archive holds the licences of the crates linked into that binary.
