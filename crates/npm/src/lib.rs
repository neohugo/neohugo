//! The project's npm packages, without Node.js or npm: [`ensure_installed`] installs the
//! dependencies of `package.json` into `node_modules` before a build, and [`run_program`] runs
//! a package's program (`tailwindcss` of `@tailwindcss/cli`, `babel` of `@babel/cli`) with an
//! embedded JavaScript runtime.
//!
//! Both are Deno's, as libraries: the installer is `deno_npm_installer` (registry resolution,
//! `.npmrc`, tarballs checked against their integrity, an npm-style hoisted `node_modules`, the
//! platform's optional packages), the runtime `deno_runtime` with its Node compatibility layer
//! (`node:` modules, CommonJS, N-API addons such as Tailwind's oxide and lightningcss).
//!
//! **Who owns `node_modules`.** The installer manages a `node_modules` it created (it holds
//! Deno's `.deno` directory) or one that does not exist yet. One that npm, pnpm or yarn wrote
//! (their state files), or a link, is the user's: it is left alone. A stamp of what was
//! installed (the hash of `package.json` and the lock file) skips the work when nothing changed.
//!
//! **The lock file** is [`LOCK_FILE`] next to `package.json`, in Deno's lockfile format. When it
//! does not exist, a `package-lock.json` seeds it, so a project moving from npm keeps its
//! versions.
//!
//! **Programs run in a child process**: the binary runs itself with
//! [`ssg_base::RUN_PACKAGE_COMMAND`] (`ssg_resources`' tool pipes spawn it), so a program's
//! `process.exit()`, its working directory and its environment stay its own. Lifecycle scripts
//! (`postinstall`) never run.

#![forbid(unsafe_code)]

mod http;
mod install;
mod loader;
mod run;

pub use install::{InstallError, Installed, LOCK_FILE, ensure_installed};
pub use run::{run_main, run_program};
