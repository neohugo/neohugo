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

[![Rust](https://github.com/neohugo/neohugo/actions/workflows/rust.yml/badge.svg)](https://github.com/neohugo/neohugo/actions/workflows/rust.yml)

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

Neohugo reads Hugo's project layout and configuration (a `neohugo.toml` wins over a `hugo.toml` next to it). Its templates are Tera 2 with Hugo's v0.146 layout names instead of Go templates: [rust/docs/template-api.md](rust/docs/template-api.md) lists every function, filter and test with Hugo's name for each, and `neohugo-rs templates check` checks a site's templates against it. Themes come from the themes directory or `_vendor`; [Hugo Modules] are not downloaded. The known differences from Hugo are listed in [docs/rust-port/HANDOFF.md](docs/rust-port/HANDOFF.md#7-known-deviations-from-hugo).

See the [features] section of the documentation for a comprehensive summary of Hugo's capabilities.

## Sponsors

<p>&nbsp;</p>
<p float="left">
  <a href="https://www.linode.com/?utm_campaign=hugosponsor&utm_medium=banner&utm_source=hugogithub" target="_blank"><img src="https://raw.githubusercontent.com/gohugoio/gohugoioTheme/master/assets/images/sponsors/linode-logo_standard_light_medium.png" width="200" alt="Linode"></a>
&nbsp;&nbsp;&nbsp;
  <a href="https://www.jetbrains.com/go/?utm_source=OSS&utm_medium=referral&utm_campaign=hugo" target="_blank"><img src="https://raw.githubusercontent.com/gohugoio/gohugoioTheme/master/assets/images/sponsors/goland.svg" width="200" alt="The complete IDE crafted for professional Go developers."></a>
</p>

## Installation

Download the archive for your platform from the [releases] page. Releases are tagged `v<version>`; v0.149 and later are built from `rust/`, v0.148.2 and earlier are the former Go implementation. Each archive, `neohugo-rs-<version>-<target>.tar.gz` (`.zip` for Windows), holds the `neohugo-rs` binary, `LICENSE`, `THIRD_PARTY_NOTICES.txt`, `PROVENANCE.md` and `THIRD_PARTY/`:

- `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` (glibc 2.35 or later)
- `x86_64-apple-darwin`, `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

Check a download with `sha256sum -c SHA256SUMS --ignore-missing`, then put `neohugo-rs` on your `PATH`:

```text
neohugo-rs version
neohugo-rs -s <site>                # build into the publish directory
neohugo-rs server -s <site>         # development server with live reload
```

The commands and flags (Hugo's, in kebab-case with the camelCase spellings as aliases) are listed in [rust/crates/cli/README.md](rust/crates/cli/README.md).

## Build from source

Prerequisites to build neohugo from source:

- Rust 1.94 or later (`rust-version` in `rust/Cargo.toml`; CI builds with 1.94.1)
- A C compiler (libwebp and ring are compiled with the `cc` crate)

Build neohugo:

```text
cd rust
cargo build --release --locked -p neohugo
```

The binary is `rust/target/release/neohugo-rs`.

## External tools

Sass is compiled in process. The other asset pipelines run external tools, looked up when a site uses them:

- `js_build` (`js.Build`): esbuild, named by `NEOHUGO_ESBUILD_BINARY` (default: `tools/esbuild/bin/esbuild`, relative to the working directory). The tests use esbuild 0.25.6, the version `tools/neohugo/node/package.json` pins; in a checkout, `tools/neohugo/node.sh && tools/esbuild/install.sh` installs it.
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

The code is the Cargo workspace in [rust/](rust/README.md): its README has the layout, the commands and the CI and release workflow, and [docs/rust-port/HANDOFF.md](docs/rust-port/HANDOFF.md) the crate map, the parity gates, the deviations from Hugo and the open items.

## Dependencies

Neohugo stands on the shoulders of great open source libraries. The Rust crates it uses are declared in [rust/Cargo.toml](rust/Cargo.toml) (`[workspace.dependencies]`, locked by `rust/Cargo.lock`); material taken from other projects is listed in [rust/PROVENANCE.md](rust/PROVENANCE.md), with licences cargo cannot see in [rust/THIRD_PARTY](rust/THIRD_PARTY/README.md). The `THIRD_PARTY_NOTICES.txt` of each release archive holds the licences of the crates linked into that binary.
