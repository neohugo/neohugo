//! Integration tests of `neohugo-esbuild` (the crate's single test binary, REWRITE_PLAN.md §2.2).
//!
//! Tests that run esbuild use `$NEOHUGO_ESBUILD_BINARY`, else the repository's
//! `tools/esbuild/bin/esbuild` (installed by `tools/esbuild/install.sh`); without a binary they
//! print `SKIPPED` and pass.

mod jsbuild;
mod options;
mod protocol;
mod resolve;
mod service;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use neohugo_esbuild::{BINARY_ENV, DEFAULT_BINARY, Service};

/// The repository root.
fn repo_root() -> PathBuf {
    neohugo_testkit::fixture::repo_dir()
}

/// The esbuild binary, or `None` (with a note on stderr) when there is none.
fn esbuild_binary(test: &str) -> Option<PathBuf> {
    let path = std::env::var_os(BINARY_ENV)
        .filter(|v| !v.is_empty())
        .map_or_else(|| repo_root().join(DEFAULT_BINARY), PathBuf::from);
    if path.is_file() {
        Some(path)
    } else {
        eprintln!(
            "SKIPPED {test}: no esbuild binary at {} (set {BINARY_ENV} or run tools/neohugo/node.sh && tools/esbuild/install.sh)",
            path.display()
        );
        None
    }
}

/// A started service, or `None` when there is no binary.
fn service(test: &str) -> Option<Arc<Service>> {
    let binary = esbuild_binary(test)?;
    Some(Arc::new(Service::start(&binary).unwrap_or_else(|e| {
        panic!("starting {}: {e}", binary.display())
    })))
}

/// A fresh scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("neohugo-esbuild-{}", std::process::id()))
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}
