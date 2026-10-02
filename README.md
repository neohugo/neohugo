[hugo]: https://github.com/gohugoio/hugo
[hugo documentation]: https://gohugo.io/documentation/
[hugo modules]: https://gohugo.io/hugo-modules/
[discussions]: https://github.com/neohugo/neohugo/discussions
[issue tracker]: https://github.com/neohugo/neohugo/issues
[releases]: https://github.com/neohugo/neohugo/releases
[rust]: https://www.rust-lang.org/
[static site generator]: https://en.wikipedia.org/wiki/Static_site_generator

# neohugo

[Website](https://neohugo.github.io) |
[Discussions][discussions] |
[Issues][issue tracker] |
[Installation](#installation) |
[Contributing](CONTRIBUTING.md) |
[Neohugo vs Hugo](https://github.com/neohugo/neohugo/wiki/Diff-hugo-neohugo)

[![CI](https://github.com/neohugo/neohugo/actions/workflows/ci.yml/badge.svg)](https://github.com/neohugo/neohugo/actions/workflows/ci.yml)

## Overview

neohugo is a [static site generator] written in [Rust]. It began as a fork of [Hugo] and was then rewritten: it reads Hugo's project layout, content and configuration keys, and its tests compare its output with Hugo's, but its layouts are Tera templates and it is developed separately. neohugo is not affiliated with or endorsed by the Hugo project ([Relationship to Hugo](#relationship-to-hugo)). With its templating system and fast asset pipelines, neohugo renders a complete site in seconds, often less.

With its multilingual support and taxonomy system, neohugo suits:

- Corporate, government, nonprofit, education, news, event, and project sites
- Documentation sites
- Image portfolios
- Landing pages
- Business, professional, and personal blogs
- Resumes and CVs

Use neohugo's built-in web server (`neohugo server`) during development to instantly see changes to content, structure, behavior, and presentation. Then deploy the site to your host, or push changes to your Git provider for automated builds and deployment.

Neohugo's fast asset pipelines include:

- Image processing &ndash; Convert, resize, crop, rotate, adjust colors, apply filters, overlay text and images, and extract EXIF data
- JavaScript bundling &ndash; Transpile TypeScript and JSX to JavaScript, bundle, tree shake, minify, create source maps, and perform SRI hashing.
- Sass processing &ndash; Transpile Sass to CSS, bundle, tree shake, minify, create source maps, perform SRI hashing, and integrate with PostCSS
- Tailwind CSS processing &ndash; Compile Tailwind CSS utility classes into standard CSS, bundle, tree shake, optimize, minify, perform SRI hashing, and integrate with PostCSS

Neohugo reads Hugo's project layout and configuration keys from `neohugo.toml` (or `config.toml`; Hugo's `hugo.toml` is not read), and its own names throughout: the `neohugo` template object, `NEOHUGO_*` environment variables and `neohugo_stats.json`. Its templates are Tera 2 with Hugo's v0.146 layout names instead of Go templates; [Upgrading from the Go build](#upgrading-from-the-go-build) says what else changed with v0.149. The known differences from Hugo are listed in [docs/rust-port/HANDOFF.md](docs/rust-port/HANDOFF.md#7-known-deviations-from-hugo).

Hugo's [documentation][hugo documentation] describes the concepts neohugo shares with Hugo (content organization, front matter, taxonomies, configuration keys); where neohugo differs, the documents listed under [Documentation](#documentation) apply.

Minified CSS (`--minify`, `resources.Minify`) is prepared for the browsers of the project's [browserslist](https://github.com/browserslist/browserslist#queries) configuration (`.browserslistrc`, a `browserslist` file or the `browserslist` key of `package.json`; the section named like the environment applies, else the default queries): vendor prefixes those browsers need are added, newer syntax they lack is lowered and prefixes none of them needs are removed, as autoprefixer does, so PostCSS is not needed for that. A style rule declaring a property twice (a value and its fallback) is kept as written. Without a browserslist configuration, prefixes stay as written.

`purge_css` cuts a stylesheet down, for each page, to the rules that page uses (its elements' tags, classes and ids, the words of its scripts), with PurgeCSS's `safelist`, `greedy`, `blocklist`, `content` and `variables` options: `<style>{{ get_asset(path="main.scss") | to_css | minify | purge_css(content=[get_asset(path="js/main.js")]) }}</style>`. It needs no `neohugo_stats.json` and no PostCSS, and pages are written as they are rendered. See [template-api.md](docs/rust-port/template-api.md).

## Installation

Download the archive for your platform from the [releases] page. Releases are tagged `v<version>`; v0.149 and later are the Rust implementation, v0.148.2 and earlier are the former Go implementation. The archives are named as before, `neohugo_<version>_<os>-<arch>.tar.gz` (`.zip` for Windows), and hold the `neohugo` binary, `README.md`, `LICENSE`, `NOTICE`, `THIRD_PARTY_NOTICES.txt`, `PROVENANCE.md` and `THIRD_PARTY/`:

- `linux-amd64`, `linux-arm64` (glibc 2.35 or later)
- `darwin-amd64`, `darwin-arm64`
- `windows-amd64`

Check a download with `sha256sum -c neohugo_<version>_checksums.txt --ignore-missing` (on macOS: `shasum -a 256 -c neohugo_<version>_checksums.txt --ignore-missing`), then put `neohugo` on your `PATH`:

```text
neohugo version
neohugo -s <site>                # build into the publish directory
neohugo server -s <site>         # development server with live reload
```

The commands and flags (Hugo's, in kebab-case with the camelCase spellings as aliases) are listed in [crates/cli/README.md](crates/cli/README.md).

## Upgrading from the Go build

v0.149 replaces the Go neohugo in place: the binary, the `neohugo version` line and the release archives keep their names and formats. What a site or a script may have to change:

- **Layouts are Tera 2 templates, not Go templates**, with Hugo's v0.146 layout names (`home.html`, `single.html`, `_partials/`, `_shortcodes/`, `_markup/`). A layout with Go template syntax or a legacy name is an error that says what to change. [docs/rust-port/template-api.md](docs/rust-port/template-api.md) lists every function, filter and test with Hugo's name for each and how Go-template idioms translate; `neohugo templates check -s <site>` checks a site's templates against it. Output is escaped by output format (HTML and XML), not by context as in Go's `html/template`: in `<script>` use `jsonify | safe`, in query strings `urlencode`.
- **Commands and flags:** `build` (also with no command), `server`, `templates check`, `config` and `version`; the Go build's `env`, `new`, `mod`, `deploy`, `list`, `gen`, `convert`, `import`, `release`, `server trust` and `config mounts`, and the `completion` and `help` commands are not available (`--help` prints the help). Of Hugo's flags, those [crates/cli/README.md](crates/cli/README.md) lists are accepted, including the logging and housekeeping flags (`--gc`, `--logLevel`, `--noBuildLock`, `--noChmod`, `--noTimes`, `--printI18nWarnings`, `--printPathWarnings`, `--printUnusedTemplates`, `--templateMetrics`, `--templateMetricsHints`), so `neohugo --gc --minify` still works; those neohugo does not act on print a warning. Boolean flags take an explicit value as before (`--minify=false`, `--buildDrafts=true`), and `=false` overrides the configuration as before (`-D=false` against `buildDrafts = true`). `--quiet` (also `-q` in neohugo) only hides the build summary: warnings and errors are still printed, where the Go build discarded them too. Another, such as `--enableGitInfo`, `--contentDir`, `--disableKinds`, `--panicOnWarning` or the build's `-w`/`--watch`, is an error. As before, flags may come before or after the command (`neohugo -s <site> server`), and every command takes the persistent flags (`-s`, `-d`, `-e`, `--config`, `--configDir`, `--themesDir`, `--clock`, `--quiet`, `-M`, `--logLevel`, `--noBuildLock`). A build prints one summary line (`pages … | files … | … | static files …`, then `Total in N ms`) instead of the Go build's per-language statistics table. `config` prints neohugo's resolved configuration model (snake_case fields, one entry per site under `sites`, the merged user keys under `raw`), not the Go build's lower-cased Hugo keys (`baseurl`, `publishdir`, …), so a script that reads its output must change; it prints JSON by default (the Go build: TOML) and takes `--format json` or `toml`, not `yaml`, `--lang` or `--printZero`. A usage error exits with 2 (the Go build exited with 1 on every error), and error messages start with `error:` (usage) or `ERROR` (build) instead of `Error:`.
- **`server` renders into memory by default.** The Go build wrote the site to the publish directory and served it from there (`-M`/`--renderToMemory` rendered into memory). For the Go behaviour add `--render-to-disk` (`--renderToDisk`), a flag of neohugo, not of the Go build; `-d`/`--destination` without it is an error.
- **[Hugo Modules]** are not downloaded: themes come from the themes directory, `_vendor` or an absolute path.
- **`js.Build` bundles in process with [rolldown](https://rolldown.rs)** (the Go build linked esbuild 0.25.6): nothing to install, and the options are the same. Scripts behave as before, but their bytes differ, so fingerprinted names, `Data.Integrity` and source maps change. Other visible differences: the IIFE wrapper is `(function() { … })();`; legal comments stay where they are instead of moving to the end; error texts are rolldown's, except unresolved imports (`Could not resolve "x"`) and the `es5` target's errors, which keep esbuild's wording and positions. As with esbuild, TC39 decorators are lowered, `target: es5` checks and lowers the bundle, and CSS imported from scripts is dropped (`local-css` modules give their class names). Sass is compiled in process with dart-sass semantics, whatever `transpiler` says.
- The Docker images (`neohugo/neohugo`, `ghcr.io/neohugo/neohugo`) are no longer updated; they stay at the last Go build.
- The website [neohugo.github.io](https://neohugo.github.io), with its documentation and installation guide, is no longer redeployed on release tags; it documents the Go build. neohugo builds the same site from `docs/` with `tools/docs/build.sh` (Tera layouts in `sites/docs`), and gate A-D3 checks every page of that build against the published one.

## Build from source

Prerequisites to build neohugo from source:

- Rust 1.96 or later (`rust-version` in `Cargo.toml`; CI builds with 1.96.0)
- A C compiler (libwebp and ring are compiled with the `cc` crate)

Build neohugo:

```text
cargo build --release --locked -p neohugo
```

The binary is `target/release/neohugo`. [DEVELOPMENT.md](DEVELOPMENT.md) describes the workspace, its tests and the CI and release workflow.

## External tools

Sass and `js.Build` run in process. The other asset pipelines run external tools, looked up when a site uses them:

- `postcss`, `tailwind` and `babel` (`css.PostCSS`, `css.TailwindCSS`, `js.Babel`): `postcss`, `tailwindcss` and `babel`, named by `NEOHUGO_POSTCSS_BIN`, `NEOHUGO_TAILWINDCSS_BIN` and `NEOHUGO_BABEL_BIN`, else looked up in the project's `node_modules/.bin`, in the `.bin` of each directory of `NEOHUGO_NODE_MODULES` (a path list), then on `PATH`. As in Hugo, `security.exec.allow` must allow them (the default allows `postcss` and `tailwindcss`, not `babel`).

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=neohugo/neohugo&type=Timeline)](https://star-history.com/#neohugo/neohugo&Timeline)

## Documentation

- [crates/cli/README.md](crates/cli/README.md): the commands and flags.
- [docs/rust-port/template-api.md](docs/rust-port/template-api.md): every template function, filter and test, with Hugo's name for each and how Go-template idioms translate.
- [Upgrading from the Go build](#upgrading-from-the-go-build), above, and the known differences from Hugo in [docs/rust-port/HANDOFF.md](docs/rust-port/HANDOFF.md#7-known-deviations-from-hugo).
- The website [neohugo.github.io](https://neohugo.github.io), which documents the Go build (v0.148.2 and earlier).
- Hugo's [documentation][hugo documentation], for the concepts neohugo shares with Hugo. Report problems with neohugo, or with how it differs from these pages, to neohugo's [issue tracker], not to the Hugo project.

## Support

Please **do not use the issue tracker** for questions or troubleshooting: ask in neohugo's [discussions]. Use the [issue tracker] for defects of neohugo and for feature requests. Hugo's forum and issue tracker are for Hugo; its maintainers do not maintain neohugo.

## Contributing

You can contribute to neohugo by answering questions in the [discussions], reporting and fixing bugs, improving the documentation and proposing features. Before you work on a feature, open an issue with the feature request template so that it can be discussed first. The [Contribution Guide](CONTRIBUTING.md) covers the code guidelines, the commit messages and the checks a pull request must pass.

The code is the Cargo workspace at the repository root: [DEVELOPMENT.md](DEVELOPMENT.md) has the layout, the commands and the CI and release workflow, and [docs/rust-port/HANDOFF.md](docs/rust-port/HANDOFF.md) the crate map, the parity gates, the deviations from Hugo and the open items.

## Relationship to Hugo

neohugo began as a fork of [Hugo] (gohugoio/hugo): v0.148.2 and earlier were Hugo's Go code with neohugo's changes, and v0.149 and later are a rewrite in Rust. Hugo is copyright The Hugo Authors and licensed under the Apache License 2.0. Parts of neohugo derive from it: templates rewritten from Hugo's embedded templates, Hugo's LiveReload plugin, behaviour transcribed from Hugo's Go sources, and Hugo's test data and documentation used as test fixtures. Each of these is listed in [PROVENANCE.md](PROVENANCE.md), and [NOTICE](NOTICE) carries the attribution.

The Hugo name and logos belong to their owners. neohugo uses the name only to describe where it comes from and what it is compatible with; it is not affiliated with, sponsored by or endorsed by the Hugo project, its maintainers or its sponsors.

## License and dependencies

neohugo is licensed under the [Apache License 2.0](LICENSE); [NOTICE](NOTICE) holds the attribution notices of the work it derives from. It stands on the shoulders of great open source libraries. The Rust crates it uses are declared in [Cargo.toml](Cargo.toml) (`[workspace.dependencies]`, locked by `Cargo.lock`); material taken from other projects is listed in [PROVENANCE.md](PROVENANCE.md), with licences cargo cannot see in [THIRD_PARTY](THIRD_PARTY/README.md). The `THIRD_PARTY_NOTICES.txt` of each release archive holds the licences of the crates linked into that binary.
