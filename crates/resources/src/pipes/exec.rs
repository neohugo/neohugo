//! External tools: lookup, the security check, the environment, and running one with its
//! input on stdin.

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Condvar, Mutex, PoisonError};

use neohugo_vfs::Component;

use super::{PipeError, TransformEnv};
use crate::store::ResourceStore;

/// An external tool of the pipes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tool {
    /// `postcss` (`postcss-cli`).
    PostCss,
    /// `tailwindcss` (`@tailwindcss/cli`, v4).
    TailwindCss,
    /// `babel` (`@babel/cli`).
    Babel,
}

impl Tool {
    /// The binary's name (`postcss`), as `security.exec.allow` matches it.
    #[must_use]
    pub const fn binary(self) -> &'static str {
        match self {
            Self::PostCss => "postcss",
            Self::TailwindCss => "tailwindcss",
            Self::Babel => "babel",
        }
    }

    /// The environment variable that names the binary explicitly
    /// ([`ToolPaths::from_env`]).
    #[must_use]
    pub const fn env_var(self) -> &'static str {
        match self {
            Self::PostCss => "NEOHUGO_POSTCSS_BIN",
            Self::TailwindCss => "NEOHUGO_TAILWINDCSS_BIN",
            Self::Babel => "NEOHUGO_BABEL_BIN",
        }
    }
}

/// Where the tools are looked up.
#[derive(Clone, Debug, Default)]
pub struct ToolPaths {
    pub postcss: Option<PathBuf>,
    pub tailwindcss: Option<PathBuf>,
    pub babel: Option<PathBuf>,
    /// `node_modules` directories searched after the project's own (their `.bin`), e.g.
    /// `tools/neohugo/node_modules` installed by `tools/neohugo/node.sh`. Also added to
    /// `NODE_PATH` when they exist.
    pub node_modules: Vec<PathBuf>,
}

impl ToolPaths {
    /// Explicit binaries from `NEOHUGO_POSTCSS_BIN`, `NEOHUGO_TAILWINDCSS_BIN` and
    /// `NEOHUGO_BABEL_BIN`, and `node_modules` directories from `NEOHUGO_NODE_MODULES`
    /// (a path list).
    #[must_use]
    pub fn from_env() -> Self {
        let var = |name: &str| {
            std::env::var_os(name)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        Self {
            postcss: var(Tool::PostCss.env_var()),
            tailwindcss: var(Tool::TailwindCss.env_var()),
            babel: var(Tool::Babel.env_var()),
            node_modules: std::env::var_os("NEOHUGO_NODE_MODULES")
                .map(|v| std::env::split_paths(&v).collect())
                .unwrap_or_default(),
        }
    }

    fn explicit(&self, tool: Tool) -> Option<&Path> {
        match tool {
            Tool::PostCss => self.postcss.as_deref(),
            Tool::TailwindCss => self.tailwindcss.as_deref(),
            Tool::Babel => self.babel.as_deref(),
        }
    }
}

/// A counting semaphore: at most `min(4, cpus)` tools run at once (§3.4).
pub(super) struct Slots {
    free: Mutex<usize>,
    freed: Condvar,
}

impl Default for Slots {
    fn default() -> Self {
        let cpus = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        Self {
            free: Mutex::new(cpus.min(4)),
            freed: Condvar::new(),
        }
    }
}

impl Slots {
    fn acquire(&self) -> SlotGuard<'_> {
        let mut free = self.free.lock().unwrap_or_else(PoisonError::into_inner);
        while *free == 0 {
            free = self
                .freed
                .wait(free)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *free -= 1;
        SlotGuard(self)
    }
}

struct SlotGuard<'a>(&'a Slots);

impl Drop for SlotGuard<'_> {
    fn drop(&mut self) {
        *self.0.free.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        self.0.freed.notify_one();
    }
}

/// The binary of `tool`: the explicit path, `<project>/node_modules/.bin/<name>`, each extra
/// `node_modules/.bin/<name>`, then `PATH` (of the inherited environment).
pub(super) fn locate(env: &TransformEnv, tool: Tool) -> Result<PathBuf, PipeError> {
    let name = tool.binary();
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = env.tools.explicit(tool) {
        candidates.push(p.to_owned());
    }
    candidates.push(env.project_dir.join("node_modules/.bin").join(name));
    candidates.extend(
        env.tools
            .node_modules
            .iter()
            .map(|d| d.join(".bin").join(name)),
    );
    if let Some((_, path)) = env.os_env.iter().find(|(k, _)| k == "PATH") {
        candidates.extend(std::env::split_paths(path).map(|d| d.join(name)));
    }
    if let Some(found) = candidates.iter().find(|p| p.is_file()) {
        return Ok(found.clone());
    }
    Err(PipeError::ToolNotFound {
        tool: name,
        env: tool.env_var(),
        searched: candidates
            .iter()
            .take(3)
            .map(|p| p.display().to_string())
            .chain(
                env.os_env
                    .iter()
                    .any(|(k, _)| k == "PATH")
                    .then(|| "PATH".to_owned()),
            )
            .collect::<Vec<_>>()
            .join(", "),
    })
}

/// A config file of a tool (`postcss.config.js`): absolute as given, else the project root's
/// file mounted at `assets/_jsconfig/<name>`, else `<project>/<name>`.
pub(super) fn config_file(
    store: &ResourceStore,
    env: &TransformEnv,
    name: &str,
) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_absolute() {
        return p.is_file().then(|| p.to_owned());
    }
    let clean = neohugo_base::paths::clean(&format!("/{}", name.replace('\\', "/")));
    let clean = clean.trim_start_matches('/');
    store
        .cfg
        .vfs
        .as_ref()
        .and_then(|v| v.open(Component::Assets, &format!("_jsconfig/{clean}")))
        .map(|f| f.abs)
        .or_else(|| {
            let p = env.project_dir.join(clean);
            p.is_file().then_some(p)
        })
}

/// The tool's environment: the allowed inherited variables, then the build's.
fn environment(store: &ResourceStore, env: &TransformEnv) -> Vec<(String, String)> {
    let mut vars: Vec<(String, String)> = env
        .os_env
        .iter()
        .filter(|(k, _)| env.security.exec_os_env.accepts(k))
        .cloned()
        .collect();
    let set = |vars: &mut Vec<(String, String)>, k: &str, v: String| {
        vars.retain(|(name, _)| name != k);
        vars.push((k.to_owned(), v));
    };
    // Only directories that exist: Tailwind 4 (`@tailwindcss/node`) reads NODE_PATH as one
    // directory, so a missing project `node_modules` in front of the tools' would hide them.
    // With neither, the project's, as Go sets it.
    let project_modules = env.project_dir.join("node_modules");
    let mut node_path: Vec<PathBuf> = std::iter::once(project_modules.clone())
        .chain(env.tools.node_modules.iter().cloned())
        .filter(|dir| dir.is_dir())
        .collect();
    if node_path.is_empty() {
        node_path.push(project_modules);
    }
    if let Some((_, np)) = env.os_env.iter().find(|(k, _)| k == "NODE_PATH") {
        node_path.extend(std::env::split_paths(np));
    }
    let node_path = std::env::join_paths(node_path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    set(&mut vars, "NODE_PATH", node_path);
    set(&mut vars, "PWD", env.project_dir.display().to_string());
    set(&mut vars, "HUGO_ENVIRONMENT", env.environment.clone());
    set(&mut vars, "HUGO_ENV", env.environment.clone());
    set(
        &mut vars,
        "HUGO_PUBLISHDIR",
        env.publish_dir.display().to_string(),
    );
    if let Some(vfs) = &store.cfg.vfs {
        for (_, m) in vfs.mounts_of(Component::Assets) {
            let Some(name) = m.target_dir().strip_prefix("_jsconfig/") else {
                continue;
            };
            if m.abs.is_file() {
                let key = format!(
                    "HUGO_FILE_{}",
                    name.to_ascii_uppercase().replace(['.', '-'], "_")
                );
                if !vars.iter().any(|(k, _)| *k == key) {
                    vars.push((key, m.abs.display().to_string()));
                }
            }
        }
    }
    vars
}

/// Runs `tool` with `args` in the project directory, `stdin` as its input; returns its
/// standard output.
pub(super) fn run(
    store: &ResourceStore,
    env: &TransformEnv,
    tool: Tool,
    args: &[String],
    stdin: &[u8],
) -> Result<Vec<u8>, PipeError> {
    let name = tool.binary();
    if !env.security.exec_allow.accepts(name) {
        return Err(PipeError::ExecDenied { tool: name });
    }
    let path = locate(env, tool)?;
    let vars = environment(store, env);
    let _slot = env.slots.acquire();
    let spawn_err = |source| PipeError::Spawn {
        tool: name,
        path: path.clone(),
        source,
    };
    let mut child = Command::new(&path)
        .args(args)
        .current_dir(&env.project_dir)
        .env_clear()
        .envs(vars)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(spawn_err)?;
    let mut input = child.stdin.take();
    let mut out = child.stdout.take();
    let mut err = child.stderr.take();
    let (stdout, stderr) = std::thread::scope(|s| {
        s.spawn(move || {
            if let Some(i) = input.as_mut() {
                // A tool may exit without reading all of its input; its status tells.
                let _ = i.write_all(stdin);
            }
            drop(input);
        });
        let e = s.spawn(move || {
            let mut buf = Vec::new();
            if let Some(e) = err.as_mut() {
                let _ = e.read_to_end(&mut buf);
            }
            buf
        });
        let mut buf = Vec::new();
        if let Some(o) = out.as_mut() {
            let _ = o.read_to_end(&mut buf);
        }
        (buf, e.join().unwrap_or_default())
    });
    let status = child.wait().map_err(spawn_err)?;
    if !status.success() {
        return Err(PipeError::ToolFailed {
            tool: name,
            status: status.to_string(),
            stderr: String::from_utf8_lossy(&stderr).trim_end().to_owned(),
        });
    }
    Ok(stdout)
}
