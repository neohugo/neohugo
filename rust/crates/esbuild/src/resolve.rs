//! Hugo's import resolution for `js.Build`: imports are looked up in the assets filesystem
//! before esbuild's own resolver sees them, and `@params` yields the build's params.
//!
//! An import path is resolved as an asset when the importer is the entry script or itself an
//! asset (a file under one of the assets mounts): relative imports are resolved against the
//! importer's directory, other imports against the assets root. [`resolve_component`] then
//! tries, in order: the path with `.js`, `.ts`, `.tsx`, `.jsx` appended; for an `index` import
//! the `index.esm.*` variants; the path itself (a directory gives its `index.*`, then
//! `index.esm.*`); and for a `.js` import the same path without `.js`. Anything not found is
//! left to esbuild (which looks in `node_modules`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::options::{JsBuildOptions, Loader};
use crate::service::{CallbackError, Hook, LoadArgs, Loaded, Plugin, ResolveArgs, Resolved};

/// The esbuild namespace of modules resolved as assets.
pub const NS_HUGO_IMPORT: &str = "ns-hugo-imp";
/// The esbuild namespace of the `@params` module.
pub const NS_HUGO_PARAMS: &str = "ns-hugo-params";
/// The name esbuild gives the entry script.
pub(crate) const STDIN: &str = "<stdin>";

/// The extensions tried for an import without one, in order.
const EXTENSIONS: [&str; 4] = [".js", ".ts", ".tsx", ".jsx"];

/// What an assets path names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetEntry {
    /// A file, with its real filename.
    File(PathBuf),
    Dir,
}

/// The assets filesystem, as seen by the import resolver.
pub trait Assets {
    /// The entry at a `/`-separated path relative to the assets root (no leading `/`).
    fn entry(&self, path: &str) -> Option<AssetEntry>;

    /// The assets path of a real (absolute) filename, when it lies under an assets mount.
    fn assets_path(&self, filename: &Path) -> Option<String>;
}

/// An [`Assets`] of directories mounted into the assets root; the first mount that has an
/// entry wins.
#[derive(Clone, Debug, Default)]
pub struct MountedDirs {
    mounts: Vec<(PathBuf, String)>,
}

impl MountedDirs {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mounts the directory `source` at `target` (`""` for the assets root, `vendor` for
    /// `assets/vendor`).
    #[must_use]
    pub fn mount(mut self, source: impl Into<PathBuf>, target: &str) -> Self {
        self.mounts
            .push((source.into(), target.trim_matches('/').to_owned()));
        self
    }
}

impl Assets for MountedDirs {
    fn entry(&self, path: &str) -> Option<AssetEntry> {
        let path = clean(path);
        self.mounts.iter().find_map(|(source, target)| {
            let rest = if target.is_empty() {
                path.as_str()
            } else if path == *target {
                ""
            } else {
                path.strip_prefix(target.as_str())?.strip_prefix('/')?
            };
            if rest == ".." || rest.starts_with("../") {
                return None;
            }
            let full = if rest.is_empty() || rest == "." {
                source.clone()
            } else {
                source.join(rest)
            };
            let meta = std::fs::metadata(&full).ok()?;
            Some(if meta.is_dir() {
                AssetEntry::Dir
            } else {
                AssetEntry::File(full)
            })
        })
    }

    fn assets_path(&self, filename: &Path) -> Option<String> {
        self.mounts.iter().find_map(|(source, target)| {
            let rel = filename.strip_prefix(source).ok()?;
            let rel = rel.to_str()?.replace('\\', "/");
            Some(match (target.is_empty(), rel.is_empty()) {
                (true, _) => rel,
                (false, true) => target.clone(),
                (false, false) => format!("{target}/{rel}"),
            })
        })
    }
}

/// Resolves an import path to an asset file (see the module docs for the order).
pub fn resolve_component(imp: &str, entry: impl Fn(&str) -> Option<AssetEntry>) -> Option<PathBuf> {
    let file = |p: &str| match entry(p) {
        Some(AssetEntry::File(f)) => Some(f),
        _ => None,
    };
    // `foo.js.js` must be imported by its full name: an extension the import already has is
    // not appended again.
    let first = |base: &str| {
        EXTENSIONS
            .iter()
            .filter(|ext| !imp.ends_with(*ext))
            .find_map(|ext| file(&format!("{base}{ext}")))
    };

    if let Some(f) = first(imp) {
        return Some(f);
    }
    let base = imp.rsplit('/').next().unwrap_or(imp);
    if base == "index"
        && let Some(f) = first(&format!("{imp}.esm"))
    {
        return Some(f);
    }
    match entry(imp) {
        Some(AssetEntry::File(f)) => return Some(f),
        Some(AssetEntry::Dir) => {
            let index = first(&join(imp, "index")).or_else(|| first(&join(imp, "index.esm")));
            if index.is_some() {
                return index;
            }
        }
        None => {}
    }
    base.strip_suffix(".js")
        .and(imp.strip_suffix(".js"))
        .and_then(first)
}

/// [`resolve_component`] over [`Assets`], memoized per import path for one build.
pub(crate) struct ComponentResolver<'a> {
    assets: &'a dyn Assets,
    cache: RefCell<HashMap<String, Option<PathBuf>>>,
}

impl<'a> ComponentResolver<'a> {
    pub(crate) fn new(assets: &'a dyn Assets) -> Self {
        Self {
            assets,
            cache: RefCell::default(),
        }
    }

    pub(crate) fn resolve(&self, imp: &str) -> Option<PathBuf> {
        if let Some(hit) = self.cache.borrow().get(imp) {
            return hit.clone();
        }
        let found = resolve_component(imp, |p| self.assets.entry(p));
        self.cache
            .borrow_mut()
            .insert(imp.to_owned(), found.clone());
        found
    }

    pub(crate) fn assets(&self) -> &'a dyn Assets {
        self.assets
    }
}

/// What the Hugo plugins need for one build.
pub(crate) struct PluginContext<'a> {
    pub(crate) resolver: ComponentResolver<'a>,
    pub(crate) options: &'a JsBuildOptions,
    /// The entry script's directory in the assets (`.` at the root).
    pub(crate) source_dir: String,
    /// Where modules loaded as assets resolve their `node_modules` imports.
    pub(crate) resolve_dir: String,
    /// The JSON text of `@params`.
    pub(crate) params: Vec<u8>,
}

/// The `hugo-import-resolver` and `hugo-params-plugin` plugins.
pub(crate) fn hugo_plugins<'a>(ctx: &'a PluginContext<'a>) -> Vec<Plugin<'a>> {
    let resolve_import = move |args: &ResolveArgs| -> Result<Option<Resolved>, CallbackError> {
        let (mut imp, shimmed) = match ctx.options.shims.get(&args.path) {
            Some(shim) => (shim.clone(), true),
            None => (args.path.clone(), false),
        };
        if ctx.options.externals.contains(&imp) {
            return Ok(Some(Resolved::External { path: imp }));
        }
        let rel_dir = if args.importer == STDIN {
            ctx.source_dir.clone()
        } else {
            match ctx.resolver.assets().assets_path(Path::new(&args.importer)) {
                Some(p) => dir(&p).to_owned(),
                None if shimmed => ctx.source_dir.clone(),
                // An import from outside the assets (node_modules): esbuild's job.
                None => return Ok(None),
            }
        };
        if !rel_dir.is_empty() && imp.starts_with('.') {
            imp = join(&rel_dir, &imp);
        }
        Ok(ctx.resolver.resolve(&imp).map(|file| Resolved::Module {
            path: file.to_string_lossy().into_owned(),
            namespace: Some(NS_HUGO_IMPORT.to_owned()),
        }))
    };
    let load_import = move |args: &LoadArgs| -> Result<Option<Loaded>, CallbackError> {
        let contents =
            std::fs::read(&args.path).map_err(|e| format!("cannot read {}: {e}", args.path))?;
        Ok(Some(Loaded {
            contents,
            // Imports of assets resolve node_modules from the project, not the asset's dir.
            resolve_dir: Some(ctx.resolve_dir.clone()),
            loader: Some(ctx.options.loader_for(&args.path)),
        }))
    };
    let resolve_params = |args: &ResolveArgs| -> Result<Option<Resolved>, CallbackError> {
        let path = if args.path == "@params/config" {
            args.path.clone()
        } else {
            args.importer.clone()
        };
        Ok(Some(Resolved::Module {
            path,
            namespace: Some(NS_HUGO_PARAMS.to_owned()),
        }))
    };
    let load_params = move |_: &LoadArgs| -> Result<Option<Loaded>, CallbackError> {
        Ok(Some(Loaded {
            contents: ctx.params.clone(),
            resolve_dir: None,
            loader: Some(Loader::Json),
        }))
    };

    vec![
        Plugin {
            name: "hugo-import-resolver".to_owned(),
            on_resolve: vec![Hook {
                filter: ".*".to_owned(),
                namespace: None,
                callback: Box::new(resolve_import),
            }],
            on_load: vec![Hook {
                filter: ".*".to_owned(),
                namespace: Some(NS_HUGO_IMPORT.to_owned()),
                callback: Box::new(load_import),
            }],
        },
        Plugin {
            name: "hugo-params-plugin".to_owned(),
            on_resolve: vec![Hook {
                filter: "^@params(/config)?$".to_owned(),
                namespace: None,
                callback: Box::new(resolve_params),
            }],
            on_load: vec![Hook {
                filter: ".*".to_owned(),
                namespace: Some(NS_HUGO_PARAMS.to_owned()),
                callback: Box::new(load_params),
            }],
        },
    ]
}

/// The directory of a `/`-separated path (`.` for a bare name).
pub(crate) fn dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) => "/",
        Some(i) => &path[..i],
        None => ".",
    }
}

/// Joins two `/`-separated paths and cleans the result.
pub(crate) fn join(a: &str, b: &str) -> String {
    if a.is_empty() {
        clean(b)
    } else {
        clean(&format!("{a}/{b}"))
    }
}

/// Removes `.` segments, empty segments and `x/..` pairs; `..` above a relative root is kept.
pub(crate) fn clean(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => match out.last() {
                Some(&last) if last != ".." => {
                    out.pop();
                }
                _ if absolute => {}
                _ => out.push(".."),
            },
            s => out.push(s),
        }
    }
    let joined = out.join("/");
    match (absolute, joined.is_empty()) {
        (true, _) => format!("/{joined}"),
        (false, true) => ".".to_owned(),
        (false, false) => joined,
    }
}
