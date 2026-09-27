//! Port of `internal/js/esbuild/options.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).


//! Go `internal/js/esbuild/options.go`: `js.Build` options (mapstructure WeakDecode, case-insensitive
//! keys) compiled into esbuild BuildOptions (target map, format, loader by media type, minify flags,
//! sourcemap none, bundle=true, platform browser...). See specs/resources-pipeline.md §4.3.

use std::collections::BTreeMap;

use go_value::{Map, Value};
use nh_common::Result;
use nh_media::media::media_type::MediaType;

/// Go: `esbuild.ExternalOptions` (user-facing `js.Build` options).
#[derive(Clone, Debug, Default)]
pub struct ExternalOptions {
    pub target_path: String,
    pub minify: bool,
    pub source_map: String,
    pub sources_content: bool,
    pub target: String,
    pub format: String,
    pub platform: String,
    pub externals: Vec<String>,
    pub inject: Vec<String>,
    pub defines: Option<Map>,
    pub drop: String,
    pub shims: BTreeMap<String, String>,
    pub loaders: BTreeMap<String, String>,
    pub params: Option<Value>,
    pub jsx_factory: String,
    pub jsx_fragment: String,
    pub jsx: String,
    pub jsx_import_source: String,
    pub avoid_tdz: bool,
}

/// Go: `esbuild.InternalOptions`.
#[derive(Clone, Debug, Default)]
pub struct InternalOptions {
    pub media_type: MediaType,
    pub out_dir: String,
    pub contents: String,
    pub source_dir: String,
    pub resolve_dir: String,
    pub abs_working_dir: String,
    pub stdin: bool,
    pub ts_config: String,
    pub entry_points: Vec<String>,
}

/// Go: `esbuild.Options`.
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub external: ExternalOptions,
    pub internal: InternalOptions,
    pub(crate) compiled: Option<CompiledBuildOptions>,
}

/// The esbuild build request fields Hugo sets (Go `api.BuildOptions` subset), serialised to the
/// service protocol by `service::client`.
#[derive(Clone, Debug, Default)]
pub struct CompiledBuildOptions {
    pub bundle: bool,
    /// "iife" | "esm" | "cjs".
    pub format: String,
    /// "browser" | "node" | "neutral".
    pub platform: String,
    /// e.g. "es2015", "esnext".
    pub target: String,
    pub minify_whitespace: bool,
    pub minify_identifiers: bool,
    pub minify_syntax: bool,
    /// "none" | "inline" | "external" | "linked".
    pub sourcemap: String,
    pub sources_content: bool,
    pub stdin_contents: Option<String>,
    pub stdin_resolve_dir: String,
    /// "js" | "ts" | "tsx" | "jsx".
    pub stdin_loader: String,
    pub outdir: String,
    pub abs_working_dir: String,
    pub tsconfig: String,
    pub define: BTreeMap<String, String>,
    pub external: Vec<String>,
    pub inject: Vec<String>,
    pub drop: Vec<String>,
    pub jsx: String,
    pub jsx_factory: String,
    pub jsx_fragment: String,
    pub jsx_import_source: String,
    pub loader: BTreeMap<String, String>,
}

/// Go: `esbuild.DecodeExternalOptions(m)`.
// Go: internal/js/esbuild/options.go:DecodeExternalOptions
pub fn decode_external_options(m: &Map) -> Result<ExternalOptions> {
    todo!()
}

impl Options {
    /// Go: `Options.compile()`.
    // Go: internal/js/esbuild/options.go:compile
    pub fn compile(&mut self) -> Result<()> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: internal/js/esbuild/options.go (411 lines; 3/4 funcs executed)
//   types: ErrorMessageResolved, ExternalOptions, InternalOptions, Options
// EX L74-91: DecodeExternalOptions(m map[string]any) (ExternalOptions, error)
// EX L220-384: (opts *Options) compile() (err error)
//    L386-398: (o Options) loaderFromFilename(filename string) api.Loader
// EX L400-411: (opts *Options) validate() error
// ---------------------------------------------------------------------------
