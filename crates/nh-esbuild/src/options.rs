//! Port of `internal/js/esbuild/options.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! Go `internal/js/esbuild/options.go`: `js.Build` options (mapstructure WeakDecode, case-insensitive
//! keys) compiled into esbuild BuildOptions (target map, format, loader by media type, minify flags,
//! sourcemap none, bundle=true, platform browser...). See specs/resources-pipeline.md §4.3.
//!
//! Go compiles into `api.BuildOptions`; the port compiles into [`CompiledBuildOptions`], which
//! holds the same settings with esbuild's CLI names (the service rebuilds `api.BuildOptions`
//! from them with `cli.ParseBuildOptions`, see `service::client::build_flags`).

use std::collections::BTreeMap;

use go_value::{Map, MapType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_config::decode::FieldRef;
use nh_media::media::media_type::MediaType;

/// Go: `nameTarget` (the esbuild CLI spelling of each `api.Target`).
// Go: internal/js/esbuild/options.go:nameTarget
fn name_target(s: &str) -> Option<&'static str> {
    Some(match s {
        "" | "esnext" => "esnext",
        "es5" => "es5",
        "es6" | "es2015" => "es2015",
        "es2016" => "es2016",
        "es2017" => "es2017",
        "es2018" => "es2018",
        "es2019" => "es2019",
        "es2020" => "es2020",
        "es2021" => "es2021",
        "es2022" => "es2022",
        "es2023" => "es2023",
        "es2024" => "es2024",
        _ => return None,
    })
}

/// Go: `nameLoader` (the esbuild CLI spelling of each `api.Loader`; Hugo maps `copy` to
/// `api.LoaderFile`). `none` is Go's `api.LoaderNone`, which has no CLI spelling: `Some("")`.
// Go: internal/js/esbuild/options.go:nameLoader
fn name_loader(s: &str) -> Option<&'static str> {
    Some(match s {
        "none" => "",
        "base64" => "base64",
        "binary" => "binary",
        "copy" => "file",
        "css" => "css",
        "dataurl" => "dataurl",
        "default" => "default",
        "empty" => "empty",
        "file" => "file",
        "global-css" => "global-css",
        "js" => "js",
        "json" => "json",
        "jsx" => "jsx",
        "local-css" => "local-css",
        "text" => "text",
        "ts" => "ts",
        "tsx" => "tsx",
        _ => return None,
    })
}

/// Go: `esbuild.ExternalOptions` (user-facing `js.Build` options).
#[derive(Clone, Debug)]
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
    /// Go `Defines map[string]any` (empty = nil: no defines either way).
    pub defines: Map,
    pub drop: String,
    pub shims: BTreeMap<String, String>,
    pub loaders: BTreeMap<String, String>,
    /// Go `Params any` (`Value::Invalid` = nil).
    pub params: Value,
    pub jsx_factory: String,
    pub jsx_fragment: String,
    pub jsx: String,
    pub jsx_import_source: String,
    pub avoid_tdz: bool,
}

impl Default for ExternalOptions {
    fn default() -> Self {
        ExternalOptions {
            target_path: String::new(),
            minify: false,
            source_map: String::new(),
            sources_content: false,
            target: String::new(),
            format: String::new(),
            platform: String::new(),
            externals: Vec::new(),
            inject: Vec::new(),
            defines: Map::new(MapType::StringAny),
            drop: String::new(),
            shims: BTreeMap::new(),
            loaders: BTreeMap::new(),
            params: Value::Invalid,
            jsx_factory: String::new(),
            jsx_fragment: String::new(),
            jsx: String::new(),
            jsx_import_source: String::new(),
            avoid_tdz: false,
        }
    }
}

nh_config::decode_struct!(ExternalOptions, "esbuild.ExternalOptions", |s| vec![
    FieldRef::new("TargetPath", &mut s.target_path),
    FieldRef::new("Minify", &mut s.minify),
    FieldRef::new("SourceMap", &mut s.source_map),
    FieldRef::new("SourcesContent", &mut s.sources_content),
    FieldRef::new("Target", &mut s.target),
    FieldRef::new("Format", &mut s.format),
    FieldRef::new("Platform", &mut s.platform),
    FieldRef::new("Externals", &mut s.externals),
    FieldRef::new("Inject", &mut s.inject),
    FieldRef::new("Defines", &mut s.defines),
    FieldRef::new("Drop", &mut s.drop),
    FieldRef::new("Shims", &mut s.shims),
    FieldRef::new("Loaders", &mut s.loaders),
    FieldRef::new("Params", &mut s.params),
    FieldRef::new("JSXFactory", &mut s.jsx_factory),
    FieldRef::new("JSXFragment", &mut s.jsx_fragment),
    FieldRef::new("JSX", &mut s.jsx),
    FieldRef::new("JSXImportSource", &mut s.jsx_import_source),
    FieldRef::new("AvoidTDZ", &mut s.avoid_tdz),
]);

/// Go: `esbuild.InternalOptions`.
#[derive(Clone, Debug, Default)]
pub struct InternalOptions {
    pub media_type: MediaType,
    pub out_dir: String,
    pub contents: Vec<u8>,
    pub source_dir: String,
    pub resolve_dir: String,
    pub abs_working_dir: String,
    pub metafile: bool,
    pub stdin_source_path: String,
    /// Set to true to pass in the entry point as a byte slice.
    pub stdin: bool,
    pub splitting: bool,
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

/// The esbuild build request fields Hugo sets (Go `api.BuildOptions` subset), with esbuild's CLI
/// names, serialised to the service protocol by `service::client`.
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
    pub stdin_contents: Option<Vec<u8>>,
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
    /// "transform" | "preserve" | "automatic".
    pub jsx: String,
    pub jsx_factory: String,
    pub jsx_fragment: String,
    pub jsx_import_source: String,
    pub loader: BTreeMap<String, String>,
    pub metafile: bool,
    pub splitting: bool,
    pub entry_points: Vec<String>,
}

/// Go: `esbuild.DecodeExternalOptions(m)`.
// Go: internal/js/esbuild/options.go:DecodeExternalOptions
pub fn decode_external_options(m: &Map) -> Result<ExternalOptions> {
    decode_external_options_value(&Value::map(m.clone()))
}

/// [`decode_external_options`] of any template value (Go's `map[string]any` parameter: a nil
/// map is `Value::TypedNil`/`Value::Invalid`).
pub fn decode_external_options_value(m: &Value) -> Result<ExternalOptions> {
    let mut opts = ExternalOptions {
        sources_content: true,
        ..Default::default()
    };

    nh_config::decode::weak_decode_into(m, &mut opts)?;

    if !opts.target_path.is_empty() {
        opts.target_path = nh_common::paths::path::to_slash_trim_leading(&opts.target_path);
    }

    opts.target = go_unicode::strings::to_lower_str(&opts.target).into_owned();
    opts.format = go_unicode::strings::to_lower_str(&opts.format).into_owned();

    Ok(opts)
}

impl Options {
    /// Go: `Options.compile()`.
    // Go: internal/js/esbuild/options.go:compile
    pub fn compile(&mut self) -> Result<()> {
        let e = &self.external;
        let i = &self.internal;

        let Some(target) = name_target(&e.target) else {
            return Err(Error::new(format!(
                "invalid target: {}",
                go_strconv::quote(&e.target)
            )));
        };

        let mut loaders = BTreeMap::new();
        for (k, v) in &e.loaders {
            let Some(loader) = name_loader(v) else {
                return Err(Error::new(format!(
                    "invalid loader: {}",
                    go_strconv::quote(v)
                )));
            };
            if loader.is_empty() {
                // api.LoaderNone in the loader map: no CLI spelling.
                return Err(Error::new(
                    "neohugo-rs: js.Build loader \"none\" is not supported",
                ));
            }
            loaders.insert(k.clone(), loader.to_string());
        }

        let mut media_type = i.media_type.clone();
        if media_type.is_zero() {
            media_type = nh_media::media::builtin::builtin().javascript_type.clone();
        }

        let loader = match media_type.sub_type.as_str() {
            s if s == nh_media::media::builtin::builtin().javascript_type.sub_type => "js",
            s if s == nh_media::media::builtin::builtin().typescript_type.sub_type => "ts",
            s if s == nh_media::media::builtin::builtin().tsx_type.sub_type => "tsx",
            s if s == nh_media::media::builtin::builtin().jsx_type.sub_type => "jsx",
            _ => {
                return Err(Error::new(format!(
                    "unsupported Media Type: {}",
                    go_strconv::quote(i.media_type.string())
                )));
            }
        };

        // One of: iife, cjs, esm
        let format = match e.format.as_str() {
            "" | "iife" => "iife",
            "esm" => "esm",
            "cjs" => "cjs",
            _ => {
                return Err(Error::new(format!(
                    "unsupported script output format: {}",
                    go_strconv::quote(&e.format)
                )));
            }
        };

        let jsx = match e.jsx.as_str() {
            "" | "transform" => "transform",
            "preserve" => "preserve",
            "automatic" => "automatic",
            _ => {
                return Err(Error::new(format!(
                    "unsupported jsx type: {}",
                    go_strconv::quote(&e.jsx)
                )));
            }
        };

        let platform = match e.platform.as_str() {
            "" | "browser" => "browser",
            "node" => "node",
            "neutral" => "neutral",
            _ => {
                return Err(Error::new(format!(
                    "unsupported platform type: {}",
                    go_strconv::quote(&e.platform)
                )));
            }
        };

        let mut defines = BTreeMap::new();
        if !e.defines.entries.is_empty() {
            let m = nh_common::maps::maps::to_string_map_string(&Value::map(e.defines.clone()));
            for (k, v) in &m.entries {
                defines.insert(
                    k.to_str_lossy().into_owned(),
                    v.as_go_string()
                        .map(|s| s.to_str_lossy().into_owned())
                        .unwrap_or_default(),
                );
            }
        }

        // Go sets err here without returning: the options are still compiled, and compile
        // returns the error at the end.
        let mut err = None;
        let drop = match e.drop.as_str() {
            "" => vec![],
            "console" => vec!["console".to_string()],
            "debugger" => vec!["debugger".to_string()],
            _ => {
                err = Some(Error::new(format!(
                    "unsupported drop type: {}",
                    go_strconv::quote(&e.drop)
                )));
                vec![]
            }
        };

        // By default we only need to specify outDir and no outFile
        let out_dir = i.out_dir.clone();
        let sourcemap = match e.source_map.as_str() {
            "inline" => "inline",
            "external" => "external",
            "linked" => "linked",
            "" | "none" => "none",
            _ => {
                return Err(Error::new(format!(
                    "unsupported sourcemap type: {}",
                    go_strconv::quote(&e.source_map)
                )));
            }
        };

        let mut compiled = CompiledBuildOptions {
            bundle: true,
            metafile: i.metafile,
            abs_working_dir: i.abs_working_dir.clone(),

            target: target.to_string(),
            format: format.to_string(),
            platform: platform.to_string(),
            sourcemap: sourcemap.to_string(),
            sources_content: e.sources_content,

            loader: loaders,

            minify_whitespace: e.minify,
            minify_identifiers: e.minify,
            minify_syntax: e.minify,

            outdir: out_dir,
            splitting: i.splitting,

            define: defines,
            external: e.externals.clone(),
            drop,

            jsx_factory: e.jsx_factory.clone(),
            jsx_fragment: e.jsx_fragment.clone(),

            jsx: jsx.to_string(),
            jsx_import_source: e.jsx_import_source.clone(),

            tsconfig: i.ts_config.clone(),

            entry_points: i.entry_points.clone(),
            ..Default::default()
        };

        if i.stdin {
            // This makes ESBuild pass `stdin` as the Importer to the import.
            compiled.stdin_contents = Some(i.contents.clone());
            compiled.stdin_resolve_dir = i.resolve_dir.clone();
            compiled.stdin_loader = loader.to_string();
        }
        self.compiled = Some(compiled);
        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// The compiled options (after [`Options::compile`]).
    pub fn compiled(&self) -> Option<&CompiledBuildOptions> {
        self.compiled.as_ref()
    }

    /// Go: `loaderFromFilename(filename)` — the loader for an imported file: the user's loader
    /// for its extension, else Hugo's extension map, else JS.
    // Go: internal/js/esbuild/options.go:loaderFromFilename
    pub fn loader_from_filename(&self, filename: &str) -> String {
        let ext = go_path::filepath::ext(filename);
        if let Some(c) = &self.compiled
            && let Some(l) = c.loader.get(ext)
        {
            return l.clone();
        }
        if let Some(l) = crate::resolve::extension_to_loader(ext) {
            return l.to_string();
        }
        "js".to_string()
    }

    /// Go: `validate()` (the Import*Func pairs are not ported: `js.Batch` only).
    // Go: internal/js/esbuild/options.go:validate
    pub fn validate(&self) -> Result<()> {
        if self.internal.abs_working_dir.is_empty() {
            return Err(Error::new("AbsWorkingDir must be set"));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: internal/js/esbuild/options.go (411 lines; 3/4 funcs executed)
//   types: ErrorMessageResolved, ExternalOptions, InternalOptions, Options
// OK L74-91: DecodeExternalOptions(m map[string]any) (ExternalOptions, error)
// OK L220-384: (opts *Options) compile() (err error)
// OK L386-398: (o Options) loaderFromFilename(filename string) api.Loader
// OK L400-411: (opts *Options) validate() error
// ---------------------------------------------------------------------------
