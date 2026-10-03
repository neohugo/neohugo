# ssg-npm

A project's npm packages, without Node.js or npm, through Deno's crates used as libraries:

- `ensure_installed(project, cache)` installs the `dependencies`, `devDependencies` and
  `optionalDependencies` of `<project>/package.json` into `<project>/node_modules`
  (`deno_npm_installer`). It resolves against the registry of `.npmrc`, verifies each tarball's
  integrity, uses the npm-style hoisted layout and installs only the optional packages of the
  current platform. The cache directory is `<cache>/packages`.
- `run_program(node_modules, package, bin, args)` runs a package's program on `deno_runtime` with
  its Node compatibility layer (`node:` modules, CommonJS, N-API addons). The child process
  `fugo __run-package <node_modules> <package> <bin> [args…]` (`run_main`,
  `ssg_base::RUN_PACKAGE_COMMAND`) calls it. `ssg-resources`' Tailwind and Babel pipes spawn
  that process when `ssg_resources::pipes::set_package_runner` named the binary.

`ssg-cli` uses the crate behind its default feature `npm`:

- the install runs in `ssg_build::Prepare` before the build reads the project, and in the
  server when it starts and when `package.json` changes;
- `main` handles the hidden command;
- `build.rs` exports the N-API symbols (`deno_napi::print_linker_flags`).

User documentation: `docs/content/asset-pipelines/npm-packages.md`.

## API

```rust
pub const LOCK_FILE: &str = "npm.lock";
pub enum Installed { NoPackages, External(&'static str), UpToDate, Installed { elapsed: Duration } }
pub enum InstallError { Io { path, source }, PackageJson { path, message }, Install { path, message } }
pub fn ensure_installed(project: &Path, cache: &Path) -> Result<Installed, InstallError>;
pub fn run_main(args: &[OsString]) -> i32;  // the child process: <node_modules> <package> <bin> [args…]
pub fn run_program(node_modules: &Path, package: &str, bin: &str, args: Vec<String>) -> anyhow::Result<i32>;
```

## Behaviour

**Who owns `node_modules`.** The installer manages a `node_modules` that it wrote (Deno's
`.deno` directory is in it) or one that does not exist yet. It reports `External` and changes
nothing in these cases:

- a link (the test harness links `tools/dev/node_modules`);
- a directory with npm's, pnpm's or yarn's state file (`.package-lock.json`, `.modules.yaml`,
  `.yarn-state.yml`, `.yarn-integrity`);
- any other non-empty directory.

**Stamp.** `node_modules/.deno/.install-stamp` holds the SHA-256 of the installer version, the
platform, `package.json` and the lock file. When it matches, `ensure_installed` returns
`UpToDate` without starting a runtime or reading the network. A second build of a small
Tailwind site took 12 ms in all.

**Lock file.** Deno's lockfile format, written to `npm.lock` next to `package.json` (`lock_arg`).
When `npm.lock` is missing, `package-lock.json` seeds it (`import_npm_lockfile`). A lock file
records the registry origin of packages from registries other than npmjs.org.

**Not done.** Install scripts never run (`NullLifecycleScriptsExecutor`). Workspaces of several
`package.json` files are not installed. Theme `package.json` files are not merged into the
project's.

**Runtime.** A "bring your own node_modules" resolver (`NodeModulesDirMode::Manual`, rooted at
the given `node_modules`). The module loader (`loader.rs`) resolves with `node_resolver` and loads
with `deno_resolver`'s npm module loader, which translates CommonJS for `import` and uses
`deno_ast` for the export analysis. The worker comes from `deno_lib`'s `LibMainWorkerFactory`,
with the CLI snapshot of `deno_snapshots` and every permission. The program sees `process.argv`
as `["node", <bin path>, …args]`, the process's working directory and its environment.
`process.exit()` ends the child process. In a release build, Tailwind compiles a small
stylesheet in about 0.15 s (0.20 s under Node 26) and produces the same bytes. Gates A-D2 and
A-D3 run Tailwind this way.

## Gotchas

- Deno's workspace discovery caches `package.json` per thread
  (`node_resolver::PackageJsonThreadLocalCache`). `install` clears the cache first, so that a
  server reinstalling on the same thread sees the edited file.
- `deno_snapshots`' build script runs `deno_runtime` in the host graph, where nothing selects
  `deno_core`'s `v8` feature. This crate's build dependency on `deno_core` (default features)
  selects it; that dependency is why `build.rs` exists.
- `node_modules` binaries without the exported N-API symbols fail with
  ``symbol not found in flat namespace '_napi_create_error'``. Only `fugo` exports them, so the
  tests here use pure JavaScript packages, and the gates cover the addons.
- The Deno crates are pinned exactly, as one release (deno 2.9.7): `deno_core`, `deno_runtime`,
  `deno_lib`, `deno_resolver`, `node_resolver`, `deno_npm_installer`, `deno_npm_cache` and
  `deno_snapshots` move together. Upgrade them all at once, to the versions a deno CLI release
  pins.
- The first build downloads V8's prebuilt static library (`librusty_v8_*.a`, about 150 MB per
  profile) from the rusty_v8 GitHub releases. Set `RUSTY_V8_ARCHIVE` to a local copy to build
  offline.

## Tests

`tests/it` (no network: `ssg_testkit::registry` serves package documents and tarballs on
`127.0.0.1`, and an `.npmrc` points at it):

- `install`: a hoisted install and its lock file; nothing fetched when up to date; installing
  again after `package.json` changes (and removing what left it); the lock file keeping versions;
  no packages; another manager's or a linked `node_modules`; a missing package; a broken
  `package.json`.
- `run`: the test binary runs again as the child (`run::child`) for a CommonJS program (stdin,
  `require` of a dependency, the environment, the working directory), an ES module program
  importing CommonJS with an exit code, an uncaught exception, and a program name the package
  does not have.

`ssg-cli`'s `tests/it/npm.rs` runs the binary: a build that installs and runs a stand-in
`@tailwindcss/cli` with no Node.js on `PATH`, an unreachable registry, and a server that
installs again after `package.json` changes.
