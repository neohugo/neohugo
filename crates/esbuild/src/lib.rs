//! esbuild for neohugo: a client of `esbuild --service` and Hugo's `js.Build`.
//!
//! - [`service`]: the esbuild child process and its stdio protocol ([`Service`]: start, ping,
//!   builds with plugin callbacks). The esbuild version is read from the binary at runtime.
//! - [`JsBuildOptions`]: the typed `js.Build` options, decoded from the template's map.
//! - [`JsBuilder`]: `js.Build` itself: bundles an asset, resolving imports in the assets
//!   ([`Assets`]) before `node_modules`, with `@params` and external/linked source maps.
//!
//! The binary is named by `$NEOHUGO_ESBUILD_BINARY`, by default `tools/esbuild/bin/esbuild`
//! (installed by `tools/esbuild/install.sh`), see [`binary_path`].

#![forbid(unsafe_code)]

mod build;
mod options;
mod resolve;
pub mod service;
mod sourcemap;

use std::path::PathBuf;

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

/// The binary used when [`BINARY_ENV`] is unset: the pinned build that `tools/esbuild/install.sh`
/// installs, relative to the current directory (the repository root).
pub const DEFAULT_BINARY: &str = "tools/esbuild/bin/esbuild";

/// The esbuild binary: `$NEOHUGO_ESBUILD_BINARY` when set and not empty, else
/// [`DEFAULT_BINARY`].
#[must_use]
pub fn binary_path() -> PathBuf {
    std::env::var_os(BINARY_ENV)
        .filter(|v| !v.is_empty())
        .map_or_else(|| PathBuf::from(DEFAULT_BINARY), PathBuf::from)
}
