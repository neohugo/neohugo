//! esbuild for neohugo: a client of `esbuild --service` and Hugo's `js.Build`.
//!
//! - [`service`]: the esbuild child process and its stdio protocol ([`Service`]: start, ping,
//!   builds with plugin callbacks). The esbuild version is read from the binary at runtime.
//! - [`JsBuildOptions`]: the typed `js.Build` options, decoded from the template's map.
//! - [`JsBuilder`]: `js.Build` itself: bundles an asset, resolving imports in the assets
//!   ([`Assets`]) before `node_modules`, with `@params` and external/linked source maps.
//!
//! The binary is `$NEOHUGO_ESBUILD_BINARY`, else `esbuild` next to the running executable, on
//! `PATH`, or at `tools/esbuild/bin/esbuild` (installed by `tools/esbuild/install.sh`), see
//! [`binary_path`].

#![forbid(unsafe_code)]

mod build;
mod options;
mod resolve;
pub mod service;
mod sourcemap;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub use build::{Diagnostic, JsBuildError, JsBuildOutput, JsBuilder, Position, Source};
pub use options::{
    DropKind, Format, JsBuildOptions, Jsx, Loader, OptionsError, Platform, SourceMap, Target,
};
pub use resolve::{
    AssetEntry, Assets, MountedDirs, NS_HUGO_IMPORT, NS_HUGO_PARAMS, resolve_component,
};
pub use service::{Service, ServiceError};
pub use sourcemap::file_url;

/// The environment variable naming the esbuild binary.
pub const BINARY_ENV: &str = "NEOHUGO_ESBUILD_BINARY";

/// The binary used when no other is found: the pinned build that `tools/esbuild/install.sh`
/// installs, relative to the current directory (the repository root).
pub const DEFAULT_BINARY: &str = "tools/esbuild/bin/esbuild";

/// The file name looked up next to the executable and on `PATH`.
pub const BINARY_FILE: &str = if cfg!(windows) {
    "esbuild.exe"
} else {
    "esbuild"
};

/// The esbuild binary, the first of: `$NEOHUGO_ESBUILD_BINARY` when set and not empty;
/// [`BINARY_FILE`] next to the running executable (a release archive unpacked with esbuild
/// beside `neohugo`); [`BINARY_FILE`] in an absolute directory of `PATH`; else
/// [`DEFAULT_BINARY`].
#[must_use]
pub fn binary_path() -> PathBuf {
    let var = std::env::var_os(BINARY_ENV);
    let exe = std::env::current_exe().ok();
    let path = std::env::var_os("PATH");
    find_binary(
        var.as_deref(),
        exe.as_deref().and_then(Path::parent),
        path.as_deref(),
    )
}

/// [`binary_path`] for the given value of [`BINARY_ENV`], directory of the executable and
/// `PATH`.
#[must_use]
pub fn find_binary(var: Option<&OsStr>, exe_dir: Option<&Path>, path: Option<&OsStr>) -> PathBuf {
    if let Some(v) = var.filter(|v| !v.is_empty()) {
        return PathBuf::from(v);
    }
    // Relative and empty `PATH` entries name the working directory, a site's: skipped.
    let on_path = path
        .into_iter()
        .flat_map(std::env::split_paths)
        .filter(|d| d.is_absolute());
    exe_dir
        .map(Path::to_path_buf)
        .into_iter()
        .chain(on_path)
        .map(|d| d.join(BINARY_FILE))
        .find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BINARY))
}
