//! External tools: lookup, the security check, the environment, and running one with its
//! input on stdin.

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Condvar, Mutex, OnceLock, PoisonError};

use ssg_vfs::Component;

use super::{PipeError, TransformEnv};
use crate::store::ResourceStore;

/// An external tool of the pipes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tool {
    /// `tailwindcss` (`@tailwindcss/cli`, v4).
    TailwindCss,
    /// `babel` (`@babel/cli`).
    Babel,
}

impl Tool {
    /// The binary's name (`tailwindcss`), as `security.exec.allow` matches it.
    #[must_use]
    pub const fn binary(self) -> &'static str {
        match self {
            Self::TailwindCss => "tailwindcss",
            Self::Babel => "babel",
        }
    }

    /// The npm package whose program ([`Tool::binary`]) the tool is.
    #[must_use]
    pub const fn package(self) -> &'static str {
        match self {
            Self::TailwindCss => "@tailwindcss/cli",
            Self::Babel => "@babel/cli",
        }
    }
}

/// The runner of [`ToolPaths::of_process`] ([`set_package_runner`]).
static RUNNER: OnceLock<PathBuf> = OnceLock::new();

/// Makes [`ToolPaths::of_process`] run the tools' npm packages with `runner`: the binary itself,
/// when it embeds the JavaScript runtime (`ssg-npm`). Only the first call counts.
pub fn set_package_runner(runner: PathBuf) {
    let _ = RUNNER.set(runner);
}

/// Where the tools are looked up: the project's `node_modules`, then these.
#[derive(Clone, Debug, Default)]
pub struct ToolPaths {
    /// `node_modules` directories searched after the project's own (tests set them; the
    /// binary has none). Also added to `NODE_PATH` when they exist.
    pub node_modules: Vec<PathBuf>,
    /// The program that runs an npm package's program with the embedded JavaScript runtime,
    /// as `<runner> __run-package <node_modules> <package> <bin> [args…]`
    /// ([`ssg_base::RUN_PACKAGE_COMMAND`]). Without one, a package's program runs from
    /// `node_modules/.bin`, which needs Node.js.
    pub runner: Option<PathBuf>,
}

impl ToolPaths {
    /// The process's: no extra `node_modules`, the runner of [`set_package_runner`].
    #[must_use]
    pub fn of_process() -> Self {
        Self {
            node_modules: Vec::new(),
            runner: RUNNER.get().cloned(),
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

/// How a tool runs.
#[derive(Debug)]
pub(super) enum Program {
    /// A `node_modules/.bin` entry (Node.js runs it).
    Binary(PathBuf),
    /// The tool's package in `node_modules`, run by the runner ([`ToolPaths::runner`]).
    Package {
        runner: PathBuf,
        node_modules: PathBuf,
    },
}

impl Program {
    /// The command, without the tool's arguments.
    fn command(&self, tool: Tool) -> Command {
        match self {
            Self::Binary(path) => Command::new(path),
            Self::Package {
                runner,
                node_modules,
            } => {
                let mut c = Command::new(runner);
                c.arg(ssg_base::RUN_PACKAGE_COMMAND)
                    .arg(node_modules)
                    .args([tool.package(), tool.binary()]);
                c
            }
        }
    }

    /// The executable (for errors).
    fn path(&self) -> &Path {
        match self {
            Self::Binary(path) => path,
            Self::Package { runner, .. } => runner,
        }
    }
}

/// How `tool` runs: in the project's `node_modules` and then in each extra one, the tool's
/// package with the runner (when there is one) or its `.bin` entry. A program on `PATH` does
/// not count: the tools come from `package.json`.
pub(super) fn locate(env: &TransformEnv, tool: Tool) -> Result<Program, PipeError> {
    let name = tool.binary();
    let mut searched: Vec<PathBuf> = Vec::new();
    for node_modules in std::iter::once(env.project_dir.join("node_modules"))
        .chain(env.tools.node_modules.iter().cloned())
    {
        if let Some(runner) = &env.tools.runner {
            let package = node_modules.join(tool.package());
            if package.join("package.json").is_file() {
                return Ok(Program::Package {
                    runner: runner.clone(),
                    node_modules,
                });
            }
            searched.push(package);
        }
        let bin = node_modules.join(".bin").join(name);
        if bin.is_file() {
            return Ok(Program::Binary(bin));
        }
        searched.push(bin);
    }
    Err(PipeError::ToolNotFound {
        tool: name,
        package: tool.package(),
        searched: searched
            .iter()
            .take(4)
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
    })
}

/// A config file of a tool (`babel.config.js`): absolute as given, else the project root's
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
    let clean = ssg_base::paths::clean(&format!("/{}", name.replace('\\', "/")));
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
    set(
        &mut vars,
        ssg_base::env_var!("PUBLISHDIR"),
        env.publish_dir.display().to_string(),
    );
    if let Some(vfs) = &store.cfg.vfs {
        for (_, m) in vfs.mounts_of(Component::Assets) {
            let Some(name) = m.target_dir().strip_prefix("_jsconfig/") else {
                continue;
            };
            if m.abs.is_file() {
                let key = format!(
                    concat!(ssg_base::env_var!("FILE_"), "{}"),
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
    let program = locate(env, tool)?;
    let vars = environment(store, env);
    let _slot = env.slots.acquire();
    let spawn_err = |source| PipeError::Spawn {
        tool: name,
        path: program.path().to_owned(),
        source,
    };
    let mut child = program
        .command(tool)
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
