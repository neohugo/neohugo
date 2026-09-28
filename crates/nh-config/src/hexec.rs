//! Port of `common/hexec/exec.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

//! Go `common/hexec`: security-checked process execution (postcss, npx) with a filtered environment.
//! The golden build found postcss at `<workingDir>/node_modules/.bin/postcss` (see
//! specs/resources-pipeline.md §4.5 for lookup order, env filter and the inherited cwd).
//!
//! Go reads `os.Environ()` and `$PATH` from the process. [`Exec::new_with_env`] takes both
//! explicitly (the Rust test harness cannot change the process environment safely);
//! [`Exec::new`] reads them from the process like Go.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex, OnceLock};

use nh_common::herrors::ErrorKind;
use nh_common::{Error, Result};

use crate::env::split_env_var;
use crate::goregexp::Regexp;
use crate::security::security_config::Config as SecurityConfig;

/// Options for a command (Go passes `hexec.WithStdin(r)`, `WithStdout(w)`, `WithEnviron(env)`, ...
/// mixed into `arg ...any`).
#[derive(Default)]
pub struct CommandOptions {
    pub args: Vec<String>,
    pub stdin: Option<Box<dyn Read + Send>>,
    pub stdout: Option<Box<dyn Write + Send>>,
    pub stderr: Option<Box<dyn Write + Send>>,
    /// Go: `WithEnviron(env)`: each entry replaces the entries with the same key (all of
    /// them), or is appended.
    pub env: Vec<String>,
    /// Go: `WithDir` (not used by postcss: the child inherits the process cwd).
    pub dir: Option<String>,
}

/// Go: `hexec.Runner`.
pub trait Runner: Send {
    fn run(self: Box<Self>) -> Result<()>;

    /// The resolved command (binary path, `Args[1:]`, environment); for tests and logging.
    fn command_line(&self) -> (String, Vec<String>, Vec<String>) {
        (String::new(), Vec::new(), Vec::new())
    }
}

/// Go: `hexec.binaryLocation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryLocation {
    NodeModules,
    Npx,
    Path,
}

impl BinaryLocation {
    // Go: common/hexec/exec.go:(binaryLocation).String
    pub fn string(self) -> &'static str {
        match self {
            BinaryLocation::NodeModules => "node_modules/.bin",
            BinaryLocation::Npx => "npx",
            BinaryLocation::Path => "PATH",
        }
    }
}

const NPX_NO_INSTALL: &str = "--no-install";
const NPX_BINARY: &str = "npx";
const NODE_MODULES_BIN_PATH: &str = "node_modules/.bin";

/// Go: `hexec.NotFoundError` (`ErrorKind::ExecNotFound`).
// Go: common/hexec/exec.go:(*NotFoundError).Error
fn not_found_error(name: &str, method: &str) -> Error {
    Error::with_kind(
        ErrorKind::ExecNotFound,
        format!(
            "binary with name {} not found {}",
            go_strconv::quote(name),
            method
        ),
    )
}

/// Go: `hexec.IsNotFound(err)`.
// Go: common/hexec/exec.go:IsNotFound
pub fn is_not_found(err: &Error) -> bool {
    err.kind() == ErrorKind::ExecNotFound
}

/// Go: `hexec.Exec`: enforces a security policy for commands run via os/exec.
pub struct Exec {
    pub sc: SecurityConfig,
    pub working_dir: String,
    /// `os.Environ()` filtered by `security.exec.osEnv`.
    pub base_environ: Vec<String>,
    /// `$PATH` for binary lookups (Go reads it from the process on each lookup).
    path_env: Option<String>,
    new_npx_runner_cache: Mutex<HashMap<String, (BinaryLocation, String)>>,
    npx_available: OnceLock<bool>,
}

impl Exec {
    /// New creates a new Exec using the provided security config and the process environment.
    // Go: common/hexec/exec.go:New
    pub fn new(sc: SecurityConfig, working_dir: &str) -> Arc<Exec> {
        let environ: Vec<String> = std::env::vars_os()
            .map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy()))
            .collect();
        let path = std::env::var_os("PATH").map(|p| p.to_string_lossy().into_owned());
        Exec::new_with_env(sc, working_dir, &environ, path)
    }

    /// [`Exec::new`] with the environment (`os.Environ()`) and `$PATH` given.
    pub fn new_with_env(
        sc: SecurityConfig,
        working_dir: &str,
        environ: &[String],
        path_env: Option<String>,
    ) -> Arc<Exec> {
        let mut base_environ = Vec::new();
        for v in environ {
            let (k, _) = split_env_var(v);
            if sc.exec.os_env.accept(&k) {
                base_environ.push(v.clone());
            }
        }

        Arc::new(Exec {
            sc,
            working_dir: working_dir.to_string(),
            base_environ,
            path_env,
            new_npx_runner_cache: Mutex::new(HashMap::new()),
            npx_available: OnceLock::new(),
        })
    }

    /// Go: `Exec.New(name, args...)` — binary from PATH, security-checked.
    // Go: common/hexec/exec.go:(*Exec).New
    pub fn command(&self, name: &str, opts: CommandOptions) -> Result<Box<dyn Runner>> {
        self.new_runner(name, "", opts)
    }

    /// New will fail if name is not allowed according to the configured security policy. Else
    /// a configured Runner will be returned ready to be Run.
    // Go: common/hexec/exec.go:(*Exec).new
    fn new_runner(
        &self,
        name: &str,
        fully_qualified_name: &str,
        opts: CommandOptions,
    ) -> Result<Box<dyn Runner>> {
        self.sc.check_allowed_exec(name)?;

        let env = self.base_environ.clone();

        let cm = Commandeer {
            name: name.to_string(),
            fully_qualified_name: fully_qualified_name.to_string(),
            env,
            path_env: self.path_env.clone(),
        };

        Ok(Box::new(cm.command(opts)?))
    }

    /// Go: `Exec.Npx(name, args...)`, in order:
    /// 1. the binary in the WORKINGDIR/node_modules/.bin directory;
    /// 2. if not found, and npx is available, `npx --no-install <name> <args>`;
    /// 3. the PATH.
    ///
    /// For "tailwindcss" the PATH is the second option.
    // Go: common/hexec/exec.go:(*Exec).Npx
    pub fn npx(&self, name: &str, opts: CommandOptions) -> Result<Box<dyn Runner>> {
        self.sc.check_allowed_exec(name)?;

        let (loc, path) = self.resolve_npx(name)?;
        match loc {
            BinaryLocation::NodeModules => self.new_runner(name, &path, opts),
            BinaryLocation::Npx => self.npx_runner(name, opts),
            BinaryLocation::Path => self.command(name, opts),
        }
    }

    /// The location Npx uses for `name` (Go: the cached runner factory of `Npx`; failures are
    /// not cached).
    pub fn resolve_npx(&self, name: &str) -> Result<(BinaryLocation, String)> {
        if let Some(r) = self.new_npx_runner_cache.lock().unwrap().get(name) {
            return Ok(r.clone());
        }

        let try_loc = |loc: BinaryLocation| -> Option<(BinaryLocation, String)> {
            match loc {
                BinaryLocation::NodeModules => {
                    let node_bin_filename = go_path::filepath::join(&[
                        self.working_dir.as_str(),
                        NODE_MODULES_BIN_PATH,
                        name,
                    ]);
                    look_path_in(&node_bin_filename, self.path_env.as_deref()).ok()?;
                    Some((loc, node_bin_filename))
                }
                BinaryLocation::Npx => {
                    if !self.check_npx() {
                        return None;
                    }
                    Some((loc, String::new()))
                }
                BinaryLocation::Path => {
                    look_path_in(name, self.path_env.as_deref()).ok()?;
                    Some((loc, String::new()))
                }
            }
        };

        let mut locations = [
            BinaryLocation::NodeModules,
            BinaryLocation::Npx,
            BinaryLocation::Path,
        ];
        if name == "tailwindcss" {
            // See https://github.com/neohugo/neohugo/issues/13221#issuecomment-2574801253
            locations = [
                BinaryLocation::NodeModules,
                BinaryLocation::Path,
                BinaryLocation::Npx,
            ];
        }
        for loc in locations {
            if let Some(r) = try_loc(loc) {
                self.new_npx_runner_cache
                    .lock()
                    .unwrap()
                    .entry(name.to_string())
                    .or_insert_with(|| r.clone());
                return Ok(r);
            }
        }
        Err(not_found_error(
            name,
            &format!("in {}", locations[locations.len() - 1].string()),
        ))
    }

    // Go: common/hexec/exec.go:(*Exec).checkNpx
    fn check_npx(&self) -> bool {
        *self
            .npx_available
            .get_or_init(|| look_path_in(NPX_BINARY, self.path_env.as_deref()).is_ok())
    }

    /// npx is a convenience method to create a Runner running npx --no-install <name> <args>.
    // Go: common/hexec/exec.go:(*Exec).npx
    fn npx_runner(&self, name: &str, mut opts: CommandOptions) -> Result<Box<dyn Runner>> {
        let mut args = vec![NPX_NO_INSTALL.to_string(), name.to_string()];
        args.append(&mut opts.args);
        opts.args = args;
        self.command(NPX_BINARY, opts)
    }

    /// Sec returns the security policies this Exec is configured with.
    // Go: common/hexec/exec.go:Sec
    pub fn sec(&self) -> &SecurityConfig {
        &self.sc
    }
}

/// Go: `hexec.commandeer`.
struct Commandeer {
    name: String,
    fully_qualified_name: String,
    env: Vec<String>,
    path_env: Option<String>,
}

impl Commandeer {
    // Go: common/hexec/exec.go:(*commandeer).command
    fn command(mut self, opts: CommandOptions) -> Result<CmdWrapper> {
        // Go: WithEnviron.
        for s in &opts.env {
            let (k1, _) = split_env_var(s);
            let mut found = false;
            for v in self.env.iter_mut() {
                let (k2, _) = split_env_var(v);
                if k1 == k2 {
                    found = true;
                    *v = s.clone();
                }
            }
            if !found {
                self.env.push(s.clone());
            }
        }

        let bin = if !self.fully_qualified_name.is_empty() {
            self.fully_qualified_name.clone()
        } else {
            match look_path_in(&self.name, self.path_env.as_deref()) {
                Ok(b) => b,
                Err(_) => return Err(not_found_error(&self.name, "in PATH")),
            }
        };

        Ok(CmdWrapper {
            name: self.name,
            bin,
            args: opts.args,
            env: self.env,
            dir: opts.dir,
            stdin: opts.stdin,
            stdout: opts.stdout,
            stderr: opts.stderr,
        })
    }
}

/// Go: `hexec.cmdWrapper` (a configured `*exec.Cmd`).
struct CmdWrapper {
    name: String,
    bin: String,
    args: Vec<String>,
    env: Vec<String>,
    dir: Option<String>,
    stdin: Option<Box<dyn Read + Send>>,
    stdout: Option<Box<dyn Write + Send>>,
    stderr: Option<Box<dyn Write + Send>>,
}

fn not_found_re() -> &'static Regexp {
    static R: OnceLock<Regexp> = OnceLock::new();
    R.get_or_init(|| {
        Regexp::compile("(?s)not found:|could not determine executable").expect("valid regexp")
    })
}

impl Runner for CmdWrapper {
    // Go: common/hexec/exec.go:(*cmdWrapper).Run
    fn run(self: Box<Self>) -> Result<()> {
        let me = *self;
        let (ok, outerr) = run_command(
            &me.bin,
            &me.args,
            &me.env,
            me.dir.as_deref(),
            me.stdin,
            me.stdout,
            me.stderr,
        );
        if ok {
            return Ok(());
        }
        let outerr = String::from_utf8_lossy(&outerr).into_owned();
        let mut name = me.name.as_str();
        let mut method = "in PATH";
        if name == NPX_BINARY {
            // c.c.Args[2]: Args[0] is the binary.
            name = me.args.get(1).map(String::as_str).unwrap_or("");
            method = "using npx";
        }
        if not_found_re().match_string(&outerr) {
            return Err(not_found_error(name, method));
        }
        Err(Error::new(format!(
            "failed to execute binary {} with args [{}]: {}",
            go_strconv::quote(&me.name),
            me.args.join(" "),
            outerr
        )))
    }

    fn command_line(&self) -> (String, Vec<String>, Vec<String>) {
        (self.bin.clone(), self.args.clone(), self.env.clone())
    }
}

/// Go: `exec.Cmd.Run()` with `Env` set: runs `bin` with the deduplicated environment
/// (`dedupEnv`: the last value of a key wins), the given stdio (nil = the null device), and
/// stderr also captured. Returns whether the command started and exited successfully, and the
/// captured stderr.
fn run_command(
    bin: &str,
    args: &[String],
    env: &[String],
    dir: Option<&str>,
    stdin: Option<Box<dyn Read + Send>>,
    stdout: Option<Box<dyn Write + Send>>,
    stderr: Option<Box<dyn Write + Send>>,
) -> (bool, Vec<u8>) {
    use std::process::{Command, Stdio};

    let mut cmd = Command::new(bin);
    cmd.args(args).env_clear();
    for kv in dedup_env(env) {
        if let Some((k, v)) = kv.split_once('=') {
            cmd.env(k, v);
        }
    }
    if let Some(d) = dir
        && !d.is_empty()
    {
        cmd.current_dir(d);
    }
    cmd.stdin(if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    cmd.stdout(if stdout.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return (false, Vec::new()),
    };

    let stdin_thread = match (child.stdin.take(), stdin) {
        (Some(mut pipe), Some(mut r)) => Some(std::thread::spawn(move || {
            let _ = std::io::copy(&mut r, &mut pipe);
        })),
        _ => None,
    };
    let stdout_thread = match (child.stdout.take(), stdout) {
        (Some(mut pipe), Some(mut w)) => Some(std::thread::spawn(move || {
            let _ = std::io::copy(&mut pipe, &mut w);
            let _ = w.flush();
        })),
        _ => None,
    };
    let mut stderr_writer = stderr;
    let mut outerr = Vec::new();
    if let Some(mut pipe) = child.stderr.take() {
        let mut buf = [0u8; 8192];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if let Some(w) = stderr_writer.as_mut() {
                        let _ = w.write_all(&buf[..n]);
                    }
                    outerr.extend_from_slice(&buf[..n]);
                }
            }
        }
    }
    if let Some(t) = stdin_thread {
        let _ = t.join();
    }
    if let Some(t) = stdout_thread {
        let _ = t.join();
    }
    let ok = matches!(child.wait(), Ok(status) if status.success());
    (ok, outerr)
}

/// Go: `os/exec.dedupEnv` (case-sensitive): the last occurrence of each key wins.
fn dedup_env(env: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(env.len());
    let mut saw: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for kv in env.iter().rev() {
        let mut i = kv.find('=');
        if i == Some(0) {
            i = kv[1..].find('=').map(|j| j + 1);
        }
        let Some(i) = i else {
            if !kv.is_empty() {
                out.push(kv.clone());
            }
            continue;
        };
        let k = &kv[..i];
        if !saw.insert(k) {
            continue;
        }
        out.push(kv.clone());
    }
    out.reverse();
    out
}

/// Go: `os/exec.findExecutable` (unix): an existing non-directory with an execute bit.
/// (Go asks `faccessat(AT_EACCESS, X_OK)`; for root, and for a file owned by the caller, that
/// is the same test.)
fn find_executable(file: &str) -> std::result::Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let md = std::fs::metadata(file).map_err(|e| e.to_string())?;
    if md.is_dir() {
        return Err("is a directory".to_string());
    }
    if md.permissions().mode() & 0o111 != 0 {
        return Ok(());
    }
    Err("permission denied".to_string())
}

/// Go: `exec.LookPath(file)` with `$PATH` given (`None` = unset): a name with a slash is
/// checked as it is; otherwise each PATH entry is tried (an empty entry is "."). A match in a
/// relative directory is Go's `exec.ErrDot` error.
pub fn look_path_in(file: &str, path_env: Option<&str>) -> std::result::Result<String, String> {
    if file.contains('/') {
        return match find_executable(file) {
            Ok(()) => Ok(file.to_string()),
            Err(e) => Err(format!("exec: {}: {}", go_strconv::quote(file), e)),
        };
    }
    let path = path_env.unwrap_or("");
    for dir in go_path::filepath::split_list(path) {
        let dir = if dir.is_empty() { "." } else { dir };
        let p = go_path::filepath::join(&[dir, file]);
        if find_executable(&p).is_ok() {
            if !go_path::filepath::is_abs(&p) {
                return Err(format!(
                    "exec: {}: cannot run executable found relative to current directory",
                    go_strconv::quote(&p)
                ));
            }
            return Ok(p);
        }
    }
    Err(format!(
        "exec: {}: executable file not found in $PATH",
        go_strconv::quote(file)
    ))
}

/// Go: `hexec.InPath(binaryName)`: whether binaryName is in $PATH.
// Go: common/hexec/exec.go:InPath
pub fn in_path(binary: &str) -> bool {
    if binary.contains('/') {
        panic!("binary name should not contain any slash");
    }
    let path = std::env::var_os("PATH").map(|p| p.to_string_lossy().into_owned());
    look_path_in(binary, path.as_deref()).is_ok()
}

/// Go: `hexec.LookPath(binaryName)`: the path to binaryName in $PATH ("" -> `None`).
// Go: common/hexec/exec.go:LookPath
pub fn look_path(binary: &str) -> Option<String> {
    if binary.contains('/') {
        panic!("binary name should not contain any slash");
    }
    let path = std::env::var_os("PATH").map(|p| p.to_string_lossy().into_owned());
    look_path_in(binary, path.as_deref()).ok()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/hexec/exec.go (389 lines; 8/15 funcs executed)
//   types: Exec, binaryLocation, NotFoundError, Runner, cmdWrapper, commandeer
// OK L91-107: New(cfg security.Config, workingDir string, log loggers.Logger) *Exec
// OK L110-113: IsNotFound(err error) bool
// OK L129-131: (e *Exec) New(name string, arg ...any) (Runner, error)
// OK L135-150: (e *Exec) new(name string, fullyQualifiedName string, arg ...any) (Runner, error)
// OK L154-164: (b binaryLocation) String() string
// OK L177-232: (e *Exec) Npx(name string, arg ...any) (Runner, error)
// OK L240-244: (e *Exec) checkNpx()
// OK L247-250: (e *Exec) npx(name string, arg ...any) (Runner, error)
// OK L253-255: (e *Exec) Sec() security.Config
// OK L262-264: (e *NotFoundError) Error() string
// OK L281-296: (c *cmdWrapper) Run() error
// OK L298-300: (c *cmdWrapper) StdinPipe() (io.WriteCloser, error)  (as `CommandOptions::stdin`)
// OK L314-367: (c *commandeer) command(arg ...any) (*cmdWrapper, error)
// OK L370-376: InPath(binaryName string) bool
// OK L380-389: LookPath(binaryName string) string
// ---------------------------------------------------------------------------
