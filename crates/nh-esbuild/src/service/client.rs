//! Module `service::client`.
//!
//! NEW: esbuild service client: build request, on-resolve/on-load plugin callbacks
//!
//! Owner: Wave B task T16 (js-css-pipeline).


//! Client for the pinned esbuild 0.25.6 native binary (`esbuild --service=0.25.6 --ping`), started
//! once per build. The binary is located via `NEOHUGO_ESBUILD` env or `<workingDir>/node_modules/
//! @esbuild/<platform>/bin/esbuild` (document the choice in PORTING.md). The process cwd for the
//! build is irrelevant (absWorkingDir is passed), but paths embedded in output are relative to it.

use std::sync::Mutex;

use nh_common::Result;

use crate::build::BuildResult;
use crate::options::CompiledBuildOptions;

/// Plugin callbacks (Hugo's resolver + params plugins) invoked by the service.
pub trait PluginHost: Send + Sync {
    /// onResolve: (path, importer, namespace, resolveDir, kind) -> Some((path, namespace)) or None (native).
    fn on_resolve(&self, path: &str, importer: &str, namespace: &str, resolve_dir: &str) -> Result<Option<(String, String)>>;
    /// onLoad for Hugo namespaces -> (contents, resolveDir, loader).
    fn on_load(&self, path: &str, namespace: &str) -> Result<Option<(Vec<u8>, String, String)>>;
}

/// The running service process.
pub struct ServiceClient {
    pub binary: String,
    pub(crate) state: Mutex<()>,
}

impl ServiceClient {
    pub fn start(binary: &str) -> Result<ServiceClient> {
        todo!()
    }

    pub fn build(&self, opts: &CompiledBuildOptions, plugins: Option<&dyn PluginHost>) -> Result<BuildResult> {
        todo!()
    }
}
