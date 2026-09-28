//! Port of `internal/js/esbuild/build.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::{Error, FilePos};
use nh_hugofs::filesystems::basefs::SourceFilesystem;
use nh_resources::resource_spec::Spec;

use crate::options::Options;
use crate::resolve::{
    HUGO_NAMESPACES, PREFIX_HUGO_MEMORY, PREFIX_HUGO_VIRTUAL, STDIN_IMPORTER, create_build_plugins,
    new_fs_resolver,
};
use crate::service::client::ServiceClient;

/// Go: `esbuild.BuildClient`.
pub struct BuildClient {
    pub rs: Arc<Spec>,
    /// The assets source filesystem (Hugo import resolver).
    pub sfs: Arc<SourceFilesystem>,
    pub service: Arc<ServiceClient>,
}

/// Go: `api.Location`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Location {
    pub file: String,
    pub namespace: String,
    /// 1-based
    pub line: i64,
    /// 0-based, in bytes
    pub column: i64,
    /// in bytes
    pub length: i64,
    pub line_text: String,
    pub suggestion: String,
}

/// Go: `api.Note`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Note {
    pub text: String,
    pub location: Option<Location>,
}

/// Go: `api.Message`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Message {
    pub id: String,
    pub plugin_name: String,
    pub text: String,
    pub location: Option<Location>,
    pub notes: Vec<Note>,
}

/// Go: `api.BuildResult` subset.
#[derive(Clone, Debug, Default)]
pub struct BuildResult {
    pub output_files: Vec<OutputFile>,
    pub errors: Vec<Message>,
    pub warnings: Vec<Message>,
}

/// Go: `api.OutputFile`.
#[derive(Clone, Debug, Default)]
pub struct OutputFile {
    pub path: String,
    pub contents: Vec<u8>,
    pub hash: String,
}

impl BuildClient {
    // Go: internal/js/esbuild/build.go:NewBuildClient
    pub fn new(
        sfs: Arc<SourceFilesystem>,
        rs: Arc<Spec>,
        service: Arc<ServiceClient>,
    ) -> Arc<BuildClient> {
        Arc::new(BuildClient { rs, sfs, service })
    }

    /// Go's `NewBuildClient(fs, rs)`: the esbuild service is found and started on the first
    /// build (see [`ServiceClient::lazy`]).
    pub fn new_default(sfs: Arc<SourceFilesystem>, rs: Arc<Spec>) -> Arc<BuildClient> {
        let wd = rs.path_spec.cfg.base_config().working_dir.clone();
        Self::new(sfs, rs, Arc::new(ServiceClient::lazy(&wd)))
    }

    /// Go: `BuildClient.Build(opts)` — OutDir = AbsPublishDir, ResolveDir = AbsWorkingDir = WorkingDir,
    /// TsConfig from `ResolveJSConfigFile("tsconfig.json")`; plugins `hugo-import-resolver` and
    /// `hugo-params-plugin`.
    // Go: internal/js/esbuild/build.go:Build
    pub fn build(&self, mut opts: Options) -> Result<BuildResult> {
        opts.internal.out_dir = self.rs.path_spec.abs_publish_dir.clone();
        // where node_modules gets resolved
        opts.internal.resolve_dir = self.rs.path_spec.cfg.base_config().working_dir.clone();
        opts.internal.abs_working_dir = opts.internal.resolve_dir.clone();
        opts.internal.ts_config = self
            .rs
            .path_spec
            .base_fs
            .resolve_js_config_file("tsconfig.json");
        let assets_resolver = new_fs_resolver(self.rs.path_spec.base_fs.assets.fs.clone());

        opts.validate()?;

        opts.compile()?;

        if !opts.external.inject.is_empty() {
            // Resolve the absolute filenames.
            let mut inject = opts.external.inject.clone();
            for (i, ext) in opts.external.inject.iter().enumerate() {
                let imp_path = go_path::filepath::from_slash(ext).to_string();
                if go_path::filepath::is_abs(&imp_path) {
                    return Err(Error::new(
                        "inject: absolute paths not supported, must be relative to /assets",
                    ));
                }

                let Some(m) = assets_resolver.resolve_component(&imp_path) else {
                    return Err(Error::new(format!(
                        "inject: file {} not found",
                        go_strconv::quote(ext)
                    )));
                };

                inject[i] = m.filename.clone();
            }
            opts.external.inject = inject.clone();
            if let Some(c) = opts.compiled.as_mut() {
                c.inject = inject;
            }
        }

        let result = {
            let plugins = create_build_plugins(&self.rs, &assets_resolver, &opts)?;
            let compiled = opts.compiled.clone().unwrap_or_default();
            self.service.build_with_plugins(&compiled, &plugins)?
        };

        if !result.errors.is_empty() {
            let errors: Vec<Error> = result
                .errors
                .iter()
                .map(|msg| self.create_err(&opts, msg))
                .collect();

            // Return 1, log the rest.
            for (i, err) in errors.iter().enumerate() {
                if i > 0 {
                    self.rs.logger.errorf(format!("js.Build failed: {err}"));
                }
            }

            return Err(errors.into_iter().next().unwrap_or_else(|| Error::new("")));
        }

        let mut result = result;
        let out_dir = opts.internal.out_dir.clone();
        let stdin_source_path = opts.internal.stdin_source_path.clone();
        let resolve_source_map_source = |s: &str| -> String {
            if let Some(m) = assets_resolver.resolve_component(s) {
                return m.filename.clone();
            }
            String::new()
        };

        for o in result.output_files.iter_mut() {
            crate::sourcemap::fix_output_file(o, &|s: &str| {
                if s == STDIN_IMPORTER {
                    return resolve_source_map_source(&stdin_source_path);
                }
                let mut s = s.to_string();
                let mut is_ns_hugo = false;
                if s.starts_with("ns-hugo") {
                    is_ns_hugo = true;
                    let idx_colon = s.find(':').map(|i| i as isize).unwrap_or(-1);
                    s = s[(idx_colon + 1) as usize..].to_string();
                }

                if !s.starts_with(PREFIX_HUGO_VIRTUAL) && !go_path::filepath::is_abs(&s) {
                    s = go_path::filepath::join(&[out_dir.as_str(), s.as_str()]);
                }

                if is_ns_hugo {
                    let ss = resolve_source_map_source(&s);
                    if !ss.is_empty() {
                        if ss.starts_with(PREFIX_HUGO_MEMORY) {
                            // File not on disk, mark it for removal from the sources slice.
                            return String::new();
                        }
                        return ss;
                    }
                    return String::new();
                }
                s
            })?;
        }

        Ok(result)
    }

    // Go: internal/js/esbuild/build.go:Build (createErr)
    fn create_err(&self, opts: &Options, msg: &crate::build::Message) -> Error {
        let Some(loc) = &msg.location else {
            return Error::new(msg.text.clone());
        };
        let mut error_path = loc.file.clone();
        if error_path == STDIN_IMPORTER {
            error_path = opts.internal.stdin_source_path.clone();
        }

        let mut error_message = msg.text.clone();

        let mut namespace = "";
        for ns in HUGO_NAMESPACES {
            if error_path.starts_with(ns) {
                namespace = ns;
                break;
            }
        }

        let ok = if !namespace.is_empty() {
            let namespace = format!("{namespace}:");
            error_message = error_message.replace(&namespace, "");
            error_path = error_path
                .strip_prefix(&namespace)
                .unwrap_or(&error_path)
                .to_string();
            std::fs::File::open(&error_path).is_ok()
        } else {
            match self.sfs.fs.stat(&error_path) {
                Ok(fi) => {
                    error_path = fi.meta.filename.clone();
                    fi.meta.open().is_ok()
                }
                Err(_) => false,
            }
        };

        if ok {
            // NewFileErrorFromName(...).UpdatePosition(line, column).UpdateContent(content,
            // SimpleLineMatcher): the matcher finds the line but no column, so the position
            // stays.
            return Error::new(error_message).at(FilePos {
                filename: error_path,
                line: loc.line,
                column: loc.column,
            });
        }

        Error::new(error_message)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: internal/js/esbuild/build.go (236 lines; 2/2 funcs executed)
//   types: BuildClient
// OK L35-40: NewBuildClient(fs *filesystems.SourceFilesystem, rs *resources.Spec) *BuildClient
// OK L49-236: (c *BuildClient) Build(opts Options) (api.BuildResult, error)
// ---------------------------------------------------------------------------
