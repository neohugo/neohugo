//! `js.Build`: bundles an asset with esbuild, resolving imports in the assets first.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::options::{JsBuildOptions, Loader, SourceMap};
use crate::resolve::{
    AssetEntry, Assets, ComponentResolver, NS_HUGO_IMPORT, NS_HUGO_PARAMS, PluginContext, STDIN,
    dir, hugo_plugins,
};
use crate::service::{BuildRequest, Message, Service, ServiceError, Stdin};
use crate::sourcemap;

/// A position in a source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub file: PathBuf,
    /// 1-based.
    pub line: u32,
    /// 0-based, in bytes.
    pub column: u32,
}

/// An esbuild error or warning, with its position mapped to a real file where there is one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub text: String,
    /// The plugin that raised it.
    pub plugin: Option<String>,
    pub position: Option<Position>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = &self.position {
            write!(f, "{}:{}:{}: ", p.file.display(), p.line, p.column)?;
        }
        f.write_str(&self.text)
    }
}

/// A failed `js.Build`.
#[derive(Debug, thiserror::Error)]
pub enum JsBuildError {
    /// The asset is not JavaScript, TypeScript, JSX or TSX.
    #[error("js.Build cannot build {0:?} content")]
    UnsupportedMediaType(String),
    /// An `inject` path is absolute.
    #[error("js.Build inject {0:?}: the path must be relative to the assets directory")]
    InjectAbsolute(String),
    /// An `inject` path is not an asset.
    #[error("js.Build inject {0:?}: no such asset")]
    InjectNotFound(String),
    /// esbuild reported errors (at least one).
    #[error("{}{}", .0[0], more(.0.len()))]
    Build(Vec<Diagnostic>),
    #[error(transparent)]
    Service(#[from] ServiceError),
    /// esbuild's output is not what was asked for.
    #[error("unexpected esbuild output: {0}")]
    Output(String),
}

fn more(n: usize) -> String {
    match n {
        0 | 1 => String::new(),
        n => format!(" (and {} more errors)", n - 1),
    }
}

/// The script to build.
#[derive(Clone, Copy, Debug)]
pub struct Source<'a> {
    /// The asset path (`js/main.js`; a leading `/` is ignored).
    pub path: &'a str,
    /// Its media type (`text/javascript`, `text/typescript`, `text/tsx`, `text/jsx`).
    pub media_type: &'a str,
    pub contents: &'a [u8],
}

/// A built script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsBuildOutput {
    /// Where the script is published: `targetPath`, else the source path with a `.js`
    /// extension.
    pub target_path: String,
    pub code: Vec<u8>,
    /// For `external` and `linked` source maps: published at `target_path` + `.map`.
    pub source_map: Option<Vec<u8>>,
    pub warnings: Vec<Diagnostic>,
}

/// Runs `js.Build` for one project on an esbuild service.
#[derive(Clone, Debug)]
pub struct JsBuilder {
    service: Arc<Service>,
    working_dir: PathBuf,
    publish_dir: PathBuf,
    tsconfig: Option<PathBuf>,
}

impl JsBuilder {
    /// `working_dir` is the project directory (its `node_modules` serve imports that are not
    /// assets); `publish_dir` is where output paths are relative to.
    #[must_use]
    pub fn new(service: Arc<Service>, working_dir: PathBuf, publish_dir: PathBuf) -> Self {
        Self {
            service,
            working_dir,
            publish_dir,
            tsconfig: None,
        }
    }

    /// Uses this `tsconfig.json` (or `jsconfig.json`).
    #[must_use]
    pub fn with_tsconfig(mut self, tsconfig: Option<PathBuf>) -> Self {
        self.tsconfig = tsconfig;
        self
    }

    #[must_use]
    pub fn service(&self) -> &Arc<Service> {
        &self.service
    }

    /// Bundles `source` with `options`.
    ///
    /// # Errors
    /// An unsupported media type, a bad `inject` path, esbuild errors, or a stopped service.
    pub fn build(
        &self,
        assets: &dyn Assets,
        source: &Source<'_>,
        options: &JsBuildOptions,
    ) -> Result<JsBuildOutput, JsBuildError> {
        let loader = Loader::from_media_type(source.media_type)
            .ok_or_else(|| JsBuildError::UnsupportedMediaType(source.media_type.to_owned()))?;
        let source_path = source.path.trim_start_matches('/');
        let resolver = ComponentResolver::new(assets);

        let inject = options
            .inject
            .iter()
            .map(|p| {
                if p.starts_with('/') || Path::new(p).is_absolute() {
                    return Err(JsBuildError::InjectAbsolute(p.clone()));
                }
                resolver
                    .resolve(&p.replace('\\', "/"))
                    .ok_or_else(|| JsBuildError::InjectNotFound(p.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let working_dir = self.working_dir.to_string_lossy().into_owned();
        let mut req = BuildRequest::new(
            Stdin {
                contents: source.contents.to_vec(),
                loader,
                resolve_dir: Some(self.working_dir.clone()),
            },
            self.working_dir.clone(),
        );
        req.out_dir = Some(self.publish_dir.clone());
        req.tsconfig.clone_from(&self.tsconfig);
        req.target = options.target;
        req.format = options.format;
        req.platform = options.platform;
        req.minify = options.minify;
        req.source_map = options.source_map;
        req.sources_content = options.sources_content;
        req.defines.clone_from(&options.defines);
        req.externals.clone_from(&options.externals);
        req.inject = inject;
        req.drop = options.drop;
        req.jsx = options.jsx;
        req.jsx_factory.clone_from(&options.jsx_factory);
        req.jsx_fragment.clone_from(&options.jsx_fragment);
        req.jsx_import_source.clone_from(&options.jsx_import_source);
        req.loaders.clone_from(&options.loaders);

        let params = match &options.params {
            Some(p) => serde_json::to_vec(p),
            None => Ok(b"{}".to_vec()),
        }
        .map_err(|e| JsBuildError::Output(format!("params: {e}")))?;
        let ctx = PluginContext {
            resolver,
            options,
            source_dir: dir(source_path).to_owned(),
            resolve_dir: working_dir,
            params,
        };
        let result = {
            let plugins = hugo_plugins(&ctx);
            self.service.build(&req, &plugins)?
        };

        let stdin_file = match assets.entry(source_path) {
            Some(AssetEntry::File(f)) => Some(f),
            _ => None,
        };
        let diagnostic = |m: &Message| self.diagnostic(m, stdin_file.as_deref());
        if !result.errors.is_empty() {
            return Err(JsBuildError::Build(
                result.errors.iter().map(diagnostic).collect(),
            ));
        }
        let warnings = result.warnings.iter().map(diagnostic).collect();

        // The script, its map, and CSS imported by the script (which js.Build does not publish).
        let (maps, files): (Vec<_>, Vec<_>) = result
            .output_files
            .into_iter()
            .partition(|f| f.path.ends_with(".map"));
        let script = files
            .into_iter()
            .find(|f| f.path.ends_with(".js"))
            .ok_or_else(|| JsBuildError::Output("no script".to_owned()))?;
        let target_path = options
            .target_path
            .clone()
            .unwrap_or_else(|| with_js_extension(source_path));

        let mut code = script.contents;
        let source_map = match options.source_map {
            SourceMap::None | SourceMap::Inline => None,
            SourceMap::External | SourceMap::Linked => {
                let map_path = format!("{}.map", script.path);
                let map = maps
                    .into_iter()
                    .find(|m| m.path == map_path)
                    .ok_or_else(|| JsBuildError::Output("no source map".to_owned()))?;
                if options.source_map == SourceMap::Linked {
                    let name = target_path.rsplit('/').next().unwrap_or(&target_path);
                    code = relink(&code, &format!("{name}.map"));
                }
                Some(
                    sourcemap::fix_sources(&map.contents, stdin_file.as_deref(), &self.publish_dir)
                        .map_err(|e| JsBuildError::Output(format!("source map: {e}")))?,
                )
            }
        };
        Ok(JsBuildOutput {
            target_path,
            code,
            source_map,
            warnings,
        })
    }

    /// Maps an esbuild message's location to a real file: `<stdin>` is the source asset,
    /// `ns-hugo-imp:<path>` is that path, and other paths are relative to the working dir.
    fn diagnostic(&self, m: &Message, stdin_file: Option<&Path>) -> Diagnostic {
        let text = strip_namespaces(&m.text);
        let position = m.location.as_ref().and_then(|loc| {
            let file = if loc.file == STDIN {
                stdin_file?.to_owned()
            } else if let Some(p) = strip_namespace(&loc.file) {
                PathBuf::from(p)
            } else {
                self.working_dir.join(&loc.file)
            };
            file.is_file().then_some(Position {
                file,
                line: loc.line,
                column: loc.column,
            })
        });
        Diagnostic {
            text,
            plugin: (!m.plugin_name.is_empty()).then(|| m.plugin_name.clone()),
            position,
        }
    }
}

fn strip_namespace(path: &str) -> Option<&str> {
    [NS_HUGO_IMPORT, NS_HUGO_PARAMS]
        .into_iter()
        .find_map(|ns| path.strip_prefix(ns)?.strip_prefix(':'))
}

fn strip_namespaces(text: &str) -> String {
    text.replace(&format!("{NS_HUGO_IMPORT}:"), "")
        .replace(&format!("{NS_HUGO_PARAMS}:"), "")
}

fn with_js_extension(path: &str) -> String {
    let name_start = path.rfind('/').map_or(0, |i| i + 1);
    match path[name_start..].rfind('.') {
        Some(dot) => format!("{}.js", &path[..name_start + dot]),
        None => format!("{path}.js"),
    }
}

/// Points every `//# sourceMappingURL=` comment at `map_name`.
fn relink(code: &[u8], map_name: &str) -> Vec<u8> {
    const MARK: &[u8] = b"//# sourceMappingURL=";
    let replacement = format!("//# sourceMappingURL={map_name}\n");
    let mut out = Vec::with_capacity(code.len() + replacement.len());
    let mut rest = code;
    while let Some(i) = rest.windows(MARK.len()).position(|w| w == MARK) {
        out.extend_from_slice(&rest[..i]);
        out.extend_from_slice(replacement.as_bytes());
        let after = &rest[i..];
        rest = match after.iter().position(|&b| b == b'\n') {
            Some(nl) => &after[nl + 1..],
            None => &[],
        };
    }
    out.extend_from_slice(rest);
    out
}
