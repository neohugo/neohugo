//! The pipes (T42): shared helpers. A test project is a copy of a fixture site in a temporary
//! directory, with a store whose [`TransformEnv`] the test controls: no inherited tool paths,
//! the process environment (for `PATH`), and fake or real tools as the test decides.
//!
//! Tests that need a real node tool read its binary from `NEOHUGO_POSTCSS_BIN`,
//! `NEOHUGO_TAILWINDCSS_BIN` or `NEOHUGO_BABEL_BIN` and print `SKIPPED` without it; tests with
//! fake tools (small node scripts) need `node` on `PATH` and print `SKIPPED` without it.

mod babel;
mod jsbuild;
mod minify;
mod postcss;
mod postprocess;
mod tailwind;
mod template;
mod tocss;
mod tools;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use neohugo_base::{Idx as _, LangIdx, ResourceId};
use neohugo_resources::pipes::ToolPaths;
use neohugo_resources::{ResourceStore, StoreConfig, TransformEnv};
use neohugo_vfs::Vfs;
use serde_json::Value as J;

use crate::support::config;

/// A copy of a fixture site (or a site used in place) and its store.
pub struct Project {
    pub dir: PathBuf,
    /// `HOME` of the tools (fake tools log their calls to `<home>/tool-calls.log`).
    pub home: PathBuf,
    pub store: ResourceStore,
    _tmp: tempfile::TempDir,
}

impl Project {
    pub fn lang(&self) -> LangIdx {
        LangIdx::from_index(0)
    }

    /// The asset at `path`.
    pub fn asset(&self, path: &str) -> ResourceId {
        self.store
            .get_asset(self.lang(), path)
            .unwrap()
            .unwrap_or_else(|| panic!("no asset {path}"))
    }

    /// The calls fake tools logged (one JSON object per line).
    pub fn tool_calls(&self) -> String {
        std::fs::read_to_string(self.home.join("tool-calls.log")).unwrap_or_default()
    }

    /// `text` with the oracle's `$SITE` replaced by the project directory.
    pub fn site(&self, text: &str) -> String {
        text.replace("$SITE", &self.dir.display().to_string())
    }
}

/// The resource-transformers fixture directory `name` (`t16site`).
pub fn fixture_site(name: &str) -> PathBuf {
    neohugo_testkit::fixture::testdata("oracle/resource-transformers").join(name)
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let name = e.file_name();
        // The fixture's node_modules is stored as _node_modules (it is gitignored otherwise).
        let target = if name == "_node_modules" {
            "node_modules".into()
        } else {
            name
        };
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &to.join(target));
        } else {
            std::fs::copy(e.path(), to.join(target)).unwrap();
        }
    }
}

/// A copy of `src` as `<tmp>/site` (the PostCSS oracle prints the working directory's
/// name), with `edit` applied to its environment.
pub fn project(src: &Path, edit: impl FnOnce(&mut TransformEnv)) -> Project {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().canonicalize().unwrap().join("site");
    copy_tree(src, &dir);
    in_dir(tmp, dir, edit)
}

/// A store over the site at `dir` itself (not copied; its tools must not write into it).
pub fn project_in_place(dir: &Path, edit: impl FnOnce(&mut TransformEnv)) -> Project {
    in_dir(
        tempfile::tempdir().unwrap(),
        dir.canonicalize().unwrap(),
        edit,
    )
}

/// A small site in a temporary directory: `hugo.toml` and the given files.
pub fn mini_site(files: &[(&str, &str)]) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    for (name, body) in files {
        let path = tmp.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
    tmp
}

fn in_dir(tmp: tempfile::TempDir, dir: PathBuf, edit: impl FnOnce(&mut TransformEnv)) -> Project {
    // T00's fixture conversion re-serialised the JSON sources compactly; the oracles read the
    // original text (it shows in source maps and bundles).
    for (file, text) in [
        (
            "assets/js/data.json",
            "{\"items\": [1, 2, 3], \"name\": \"data\"}\n",
        ),
        ("assets/js/data/config.json", "{\"mode\": \"test\"}\n"),
    ] {
        if dir.join(file).is_file() {
            std::fs::write(dir.join(file), text).unwrap();
        }
    }
    let home = tmp.path().canonicalize().unwrap().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let cfg = config(&dir, &home);
    let mut env = TransformEnv::from_config(&cfg);
    env.tools = ToolPaths::default();
    env.os_env.retain(|(k, _)| k != "HOME");
    env.os_env.push(("HOME".into(), home.display().to_string()));
    env.esbuild_binary = esbuild_binary();
    edit(&mut env);
    let store = ResourceStore::new(StoreConfig {
        transforms: Arc::new(env),
        ..StoreConfig::from_config(&cfg, Some(Arc::new(Vfs::new(&cfg).unwrap())), None)
    });
    Project {
        dir,
        home,
        store,
        _tmp: tmp,
    }
}

/// The esbuild binary of the repository (`NEOHUGO_ESBUILD_BINARY` or `tools/esbuild/bin`).
pub fn esbuild_binary() -> PathBuf {
    std::env::var_os(neohugo_esbuild::BINARY_ENV)
        .filter(|v| !v.is_empty())
        .map_or_else(
            || crate::support::repo_dir().join(neohugo_esbuild::DEFAULT_BINARY),
            PathBuf::from,
        )
}

/// A real tool named by its environment variable, or `None` (printing `SKIPPED`).
pub fn real_tool(var: &str, test: &str) -> Option<PathBuf> {
    match std::env::var_os(var).filter(|v| !v.is_empty()) {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!(
                "SKIPPED {test}: {var} is not set (install the node tools with tools/neohugo/node.sh)"
            );
            None
        }
    }
}

/// Whether `node` is on `PATH` (fake tools are node scripts); prints `SKIPPED` otherwise.
pub fn have_node(test: &str) -> bool {
    let found = std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("node").is_file()));
    if !found {
        eprintln!("SKIPPED {test}: no node on PATH for the fake tools");
    }
    found
}

/// Writes an executable node script `name` into `<dir>/bin` and returns its path.
pub fn fake_tool(dir: &Path, name: &str, script: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let bin = dir.join("fake-bin");
    std::fs::create_dir_all(&bin).unwrap();
    let path = bin.join(name);
    std::fs::write(&path, format!("#!/usr/bin/env node\n{script}")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// A fixture case's steps run on the store: `get`, `concat` (of earlier cases), `tocss`,
/// `postcss`, `js`, `minify`, `fingerprint`. The error is the failing step's.
pub fn run_steps(
    p: &Project,
    case: &J,
    done: &BTreeMap<String, ResourceId>,
) -> Result<ResourceId, (usize, String)> {
    use neohugo_resources::pipes::{JsBuildSpec, PostCssOptions, ToCssOptions};
    use neohugo_resources::{CallSite, HashAlgo, Transform};
    let s = &p.store;
    let mut id = None;
    for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
        let err = |e: &dyn std::fmt::Display| (i, e.to_string());
        let t = match step["op"].as_str().unwrap() {
            "get" => {
                let path = step["path"].as_str().unwrap();
                id = Some(p.asset(path));
                continue;
            }
            "concat" => {
                let refs: Vec<ResourceId> = step["refs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| done[r.as_str().unwrap()])
                    .collect();
                let target = step["target"].as_str().unwrap();
                id = Some(
                    s.concat(target, &refs, &CallSite::in_lang(p.lang()))
                        .map_err(|e| err(&e))?,
                );
                continue;
            }
            "tocss" => {
                Transform::ToCss(ToCssOptions::from_json(&step["opts"]).map_err(|e| err(&e))?)
            }
            "postcss" => {
                Transform::PostCss(PostCssOptions::from_json(&step["opts"]).map_err(|e| err(&e))?)
            }
            "js" => Transform::JsBuild(JsBuildSpec::from_json(&step["opts"]).map_err(|e| err(&e))?),
            "minify" => Transform::Minify,
            "fingerprint" => Transform::Fingerprint(HashAlgo::Sha256),
            op => panic!("unknown op {op}"),
        };
        let next = s.transform(id.unwrap(), t).map_err(|e| err(&e))?;
        s.realize(next).map_err(|e| err(&e))?;
        id = Some(next);
    }
    Ok(id.unwrap())
}

/// The oracle's `data` document (`"null"` or a JSON object) as a value.
pub fn data(v: &J) -> J {
    crate::support::json_doc(v)
}
