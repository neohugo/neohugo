//! Port of `internal/js/esbuild/resolve.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! Go `internal/js/esbuild/resolve.go`: the Hugo import resolver plugin (onResolve `.*`: shims,
//! externals, relDir, `resolveComponent` in the assets fs trying `.js/.ts/.tsx/.jsx`, `index.*`,
//! dir index, strip `.js`; onLoad `ns-hugo-imp`) and the `@params` plugin.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use go_value::{Map, MapType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_hugofs::afero::Fs;
use nh_hugofs::fileinfo::FileMeta;
use nh_resources::resource_spec::Spec;

use crate::options::Options;
use crate::service::client::{
    OnLoadArgs, OnLoadFn, OnLoadResult, OnResolveArgs, OnResolveFn, OnResolveResult, Plugin,
};

pub const NS_HUGO_IMPORT: &str = "ns-hugo-imp";
pub const NS_HUGO_IMPORT_RESOLVE_FUNC: &str = "ns-hugo-imp-func";
pub const NS_HUGO_PARAMS: &str = "ns-hugo-params";
const PATH_HUGO_CONFIG_PARAMS: &str = "@params/config";

pub(crate) const STDIN_IMPORTER: &str = "<stdin>";

/// Go: `hugoNamespaces`.
pub(crate) const HUGO_NAMESPACES: [&str; 3] =
    [NS_HUGO_IMPORT, NS_HUGO_IMPORT_RESOLVE_FUNC, NS_HUGO_PARAMS];

pub const PREFIX_HUGO_VIRTUAL: &str = "__hu_v";
pub const PREFIX_HUGO_MEMORY: &str = "__hu_m";

/// Go: `extensionToLoaderMap` (CLI loader names).
pub(crate) fn extension_to_loader(ext: &str) -> Option<&'static str> {
    Some(match ext {
        ".js" | ".mjs" | ".cjs" => "js",
        ".jsx" => "jsx",
        ".ts" => "ts",
        ".tsx" => "tsx",
        ".css" => "css",
        ".json" => "json",
        ".txt" => "text",
        _ => return None,
    })
}

/// This is a common sub-set of ESBuild's default extensions. We assume that imports of JSON,
/// CSS etc. will be using their full name with extension.
const COMMON_EXTENSIONS: [&str; 4] = [".js", ".ts", ".tsx", ".jsx"];

/// Go: `esbuild.ResolveComponent(impPath, resolve)`: `resolve(name)` returns the value and
/// whether it is a directory, or `None` when not found.
// Go: internal/js/esbuild/resolve.go:ResolveComponent
pub fn resolve_component<T>(
    imp_path: &str,
    resolve: &dyn Fn(&str) -> Option<(T, bool)>,
) -> Option<T> {
    let find_first = |base: &str| -> Option<(T, bool)> {
        for ext in COMMON_EXTENSIONS {
            if imp_path.ends_with(ext) {
                // Import of foo.js.js need the full name.
                continue;
            }
            if let Some(v) = resolve(&format!("{base}{ext}")) {
                return Some(v);
            }
        }
        // Not found.
        None
    };

    // We need to check if this is a regular file imported without an extension.
    // There may be ambiguous situations where both foo.js and foo/index.js exists.
    // This import order is in line with both how Node and ESBuild's native
    // import resolver works.

    // It may be a regular file imported without an extension, e.g.
    // foo or foo/index.
    if let Some((v, _)) = find_first(imp_path) {
        return Some(v);
    }

    let base = go_path::filepath::base(imp_path);
    if base == "index" {
        // try index.esm.js etc.
        if let Some((v, _)) = find_first(&format!("{imp_path}.esm")) {
            return Some(v);
        }
    }

    // Check the path as is.
    let mut found = resolve(imp_path);
    if let Some((_, true)) = &found {
        found = find_first(&go_path::filepath::join(&[imp_path, "index"]));
        if found.is_none() {
            found = find_first(&go_path::filepath::join(&[imp_path, "index.esm"]));
        }
    }

    if found.is_none() && base.ends_with(".js") {
        found = find_first(imp_path.strip_suffix(".js").unwrap_or(imp_path));
    }

    found.map(|(v, _)| v)
}

/// Go: `fsResolver` — `ResolveComponent` over a filesystem, cached by import path.
pub struct FsResolver {
    fs: Arc<dyn Fs>,
    resolved: Mutex<HashMap<String, Option<Arc<FileMeta>>>>,
}

// Go: internal/js/esbuild/resolve.go:newFSResolver
pub(crate) fn new_fs_resolver(fs: Arc<dyn Fs>) -> Arc<FsResolver> {
    Arc::new(FsResolver {
        fs,
        resolved: Mutex::new(HashMap::new()),
    })
}

impl FsResolver {
    // Go: internal/js/esbuild/resolve.go:(*fsResolver).resolveComponent
    pub fn resolve_component(&self, imp_path: &str) -> Option<Arc<FileMeta>> {
        if let Some(v) = self
            .resolved
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(imp_path)
        {
            return v.clone();
        }
        let resolve = |name: &str| -> Option<(Arc<FileMeta>, bool)> {
            match self.fs.stat(name) {
                Ok(fi) => Some((fi.meta.clone(), fi.is_dir)),
                Err(_) => None,
            }
        };
        let v = resolve_component(imp_path, &resolve);
        // maps.Cache.GetOrCreate: the first stored value wins.
        self.resolved
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(imp_path.to_string())
            .or_insert(v)
            .clone()
    }
}

/// Go: `createBuildPlugins(rs, assetsResolver, depsManager, opts)`: the `hugo-import-resolver`
/// and `hugo-params-plugin` plugins (identity tracking is not ported).
// Go: internal/js/esbuild/resolve.go:createBuildPlugins
pub(crate) fn create_build_plugins<'a>(
    rs: &'a Spec,
    assets_resolver: &'a FsResolver,
    opts: &'a Options,
) -> Result<Vec<Plugin<'a>>> {
    let fs = rs.path_spec.base_fs.assets.clone();

    let resolve_import = move |args: &OnResolveArgs| -> Result<OnResolveResult> {
        let mut imp_path = args.path.clone();
        let mut shimmed = false;
        if let Some(over) = opts.external.shims.get(&imp_path) {
            imp_path = over.clone();
            shimmed = true;
        }

        if opts.external.externals.contains(&imp_path) {
            return Ok(OnResolveResult {
                path: imp_path,
                external: true,
                ..Default::default()
            });
        }

        let importer = args.importer.as_str();

        let is_stdin = importer == STDIN_IMPORTER;
        let rel_dir: String;
        if !is_stdin {
            if let Some(rest) = importer.strip_prefix(PREFIX_HUGO_VIRTUAL) {
                rel_dir = go_path::filepath::dir(rest).to_string();
            } else {
                match fs.make_path_relative(importer, true) {
                    None => {
                        if shimmed {
                            rel_dir = opts.internal.source_dir.clone();
                        } else {
                            // Not in any of the /assets folders.
                            // This is an import from a node_modules, let
                            // ESBuild resolve this.
                            return Ok(OnResolveResult::default());
                        }
                    }
                    Some(rel) => {
                        rel_dir = go_path::filepath::dir(&rel).to_string();
                    }
                }
            }
        } else {
            rel_dir = opts.internal.source_dir.clone();
        }

        // Imports not starting with a "." is assumed to live relative to /assets.
        // Hugo makes no assumptions about the directory structure below /assets.
        if !rel_dir.is_empty() && imp_path.starts_with('.') {
            imp_path = go_path::filepath::join(&[rel_dir.as_str(), imp_path.as_str()]);
        }

        if let Some(m) = assets_resolver.resolve_component(&imp_path) {
            // Store the source root so we can create a jsconfig.json
            // to help IntelliSense when the build is done.
            rs.common
                .post_build_assets
                .js_config_builder
                .add_source_root(&m.source_root);
            return Ok(OnResolveResult {
                path: m.filename.clone(),
                namespace: NS_HUGO_IMPORT.to_string(),
                ..Default::default()
            });
        }

        // Fall back to ESBuild's resolve.
        Ok(OnResolveResult::default())
    };

    let load_import = move |args: &OnLoadArgs| -> Result<OnLoadResult> {
        let b = std::fs::read(&args.path).map_err(|e| {
            Error::new(format!(
                "failed to read {}: {}",
                go_strconv::quote(&args.path),
                nh_hugofs::oserror::from_io("open", &args.path, &e)
            ))
        })?;
        Ok(OnLoadResult {
            // See https://github.com/evanw/esbuild/issues/502
            // This allows all modules to resolve dependencies
            // in the main project's node_modules.
            resolve_dir: opts.internal.resolve_dir.clone(),
            contents: Some(b),
            loader: opts.loader_from_filename(&args.path),
        })
    };

    let import_resolver = Plugin {
        name: "hugo-import-resolver".to_string(),
        on_resolve: vec![(
            ".*".to_string(),
            String::new(),
            Box::new(resolve_import) as OnResolveFn<'a>,
        )],
        // The ns-hugo-imp-func loader (ImportOnLoadFunc) belongs to js.Batch, which is not
        // ported; Go registers it too, but nothing resolves into its namespace.
        on_load: vec![(
            ".*".to_string(),
            NS_HUGO_IMPORT.to_string(),
            Box::new(load_import) as OnLoadFn<'a>,
        )],
    };

    let params = if opts.external.params.is_invalid() {
        // This way @params will always resolve to something.
        Value::map(Map::new(MapType::StringAny))
    } else {
        opts.external.params.clone()
    };

    let b = go_json::marshal(&params)
        .map_err(|e| Error::new(format!("failed to marshal params: {e}")))?;

    let resolve_params = |args: &OnResolveArgs| -> Result<OnResolveResult> {
        let mut resolved_path = args.importer.clone();

        if args.path == PATH_HUGO_CONFIG_PARAMS {
            resolved_path = PATH_HUGO_CONFIG_PARAMS.to_string();
        }

        Ok(OnResolveResult {
            path: resolved_path,
            namespace: NS_HUGO_PARAMS.to_string(),
            ..Default::default()
        })
    };
    let load_params = move |_args: &OnLoadArgs| -> Result<OnLoadResult> {
        // (ImportParamsOnLoadFunc belongs to js.Batch.)
        let mut s = b.clone();
        if s.is_empty() {
            s = b"{}".to_vec();
        }
        Ok(OnLoadResult {
            contents: Some(s),
            loader: "json".to_string(),
            ..Default::default()
        })
    };

    let params_plugin = Plugin {
        name: "hugo-params-plugin".to_string(),
        on_resolve: vec![(
            "^@params(/config)?$".to_string(),
            String::new(),
            Box::new(resolve_params) as OnResolveFn<'a>,
        )],
        on_load: vec![(
            ".*".to_string(),
            NS_HUGO_PARAMS.to_string(),
            Box::new(load_params) as OnLoadFn<'a>,
        )],
    };

    Ok(vec![import_resolver, params_plugin])
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: internal/js/esbuild/resolve.go (323 lines; 4/5 funcs executed)
//   types: fsResolver
// OK L67-119: ResolveComponent[T any](impPath string, resolve func(string) (v T, found, isDir bool)) (v T, found bool)
//    L122-132: ResolveResource(impPath string, resourceGetter resource.ResourceGetter) (r resource.Resource)
// OK L134-136: newFSResolver(fs afero.Fs) *fsResolver
// OK L143-155: (r *fsResolver) resolveComponent(impPath string) *hugofs.FileMeta
// OK L157-323: createBuildPlugins(rs *resources.Spec, assetsResolver *fsResolver, depsManager identity.Manager, opts Options) ([]api.Plugin, error)
// ---------------------------------------------------------------------------
