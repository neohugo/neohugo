//! Port of `internal/js/esbuild/build.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).


use std::sync::Arc;

use nh_common::Result;
use nh_hugofs::filesystems::basefs::SourceFilesystem;
use nh_resources::resource_spec::Spec;

use crate::options::Options;

/// Go: `esbuild.BuildClient`.
pub struct BuildClient {
    pub rs: Arc<Spec>,
    /// The assets source filesystem (Hugo import resolver).
    pub sfs: Arc<SourceFilesystem>,
    pub service: Arc<crate::service::client::ServiceClient>,
}

/// Go: `api.BuildResult` subset.
#[derive(Clone, Debug, Default)]
pub struct BuildResult {
    pub output_files: Vec<OutputFile>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct OutputFile {
    pub path: String,
    pub contents: Vec<u8>,
}

impl BuildClient {
    // Go: internal/js/esbuild/build.go:NewBuildClient
    pub fn new(sfs: Arc<SourceFilesystem>, rs: Arc<Spec>, service: Arc<crate::service::client::ServiceClient>) -> Arc<BuildClient> {
        todo!()
    }

    /// Go: `BuildClient.Build(opts)` — OutDir = AbsPublishDir, ResolveDir = AbsWorkingDir = WorkingDir,
    /// TsConfig from `ResolveJSConfigFile("tsconfig.json")`; plugins `hugo-import-resolver` and
    /// `hugo-params-plugin`.
    // Go: internal/js/esbuild/build.go:Build
    pub fn build(&self, opts: Options) -> Result<BuildResult> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: internal/js/esbuild/build.go (236 lines; 2/2 funcs executed)
//   types: BuildClient
// EX L35-40: NewBuildClient(fs *filesystems.SourceFilesystem, rs *resources.Spec) *BuildClient
// EX L49-236: (c *BuildClient) Build(opts Options) (api.BuildResult, error)
// ---------------------------------------------------------------------------
