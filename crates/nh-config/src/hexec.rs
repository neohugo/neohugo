//! Port of `common/hexec/exec.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


//! Go `common/hexec`: security-checked process execution (postcss, npx) with a filtered environment.
//! The golden build found postcss at `<workingDir>/node_modules/.bin/postcss` (see
//! specs/resources-pipeline.md §4.5 for lookup order, env filter and the inherited cwd).

use std::io::{Read, Write};
use std::sync::Arc;

use nh_common::Result;

use crate::security::security_config::Config as SecurityConfig;

/// Options for a command (Go passes `hexec.WithStdin(r)`, `WithStdout(w)`, `WithEnviron(env)`, ...
/// mixed into `arg ...any`).
#[derive(Default)]
pub struct CommandOptions {
    pub args: Vec<String>,
    pub stdin: Option<Box<dyn Read + Send>>,
    pub stdout: Option<Box<dyn Write + Send>>,
    pub stderr: Option<Box<dyn Write + Send>>,
    pub env: Vec<String>,
    /// Go: `WithDir` (not used by postcss: the child inherits the process cwd).
    pub dir: Option<String>,
}

/// Go: `hexec.Runner`.
pub trait Runner: Send {
    fn run(self: Box<Self>) -> Result<()>;
}

/// Go: `hexec.Exec`.
pub struct Exec {
    pub sc: SecurityConfig,
    pub working_dir: String,
    /// `os.Environ()` filtered by `security.exec.osEnv`.
    pub base_environ: Vec<String>,
}

impl Exec {
    // Go: common/hexec/exec.go:New
    pub fn new(sc: SecurityConfig, working_dir: &str) -> Arc<Exec> {
        todo!()
    }

    /// Go: `Exec.New(name, args...)` — binary from PATH, security-checked.
    // Go: common/hexec/exec.go:New (method)
    pub fn command(&self, name: &str, opts: CommandOptions) -> Result<Box<dyn Runner>> {
        todo!()
    }

    /// Go: `Exec.Npx(name, args...)`: `<workingDir>/node_modules/.bin/<name>`, then `npx --no-install`,
    /// then PATH. Not found -> `ErrorKind::ExecNotFound`.
    // Go: common/hexec/exec.go:Npx
    pub fn npx(&self, name: &str, opts: CommandOptions) -> Result<Box<dyn Runner>> {
        todo!()
    }

    // Go: common/hexec/exec.go:Sec
    pub fn sec(&self) -> &SecurityConfig {
        &self.sc
    }
}

/// Go: `hexec.InPath(binaryName)`.
pub fn in_path(binary: &str) -> bool {
    todo!()
}

/// Go: `hexec.LookPath(binaryName)`.
pub fn look_path(binary: &str) -> Option<String> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/hexec/exec.go (389 lines; 8/15 funcs executed)
//   types: Exec, binaryLocation, NotFoundError, Runner, cmdWrapper, commandeer
// EX L91-107: New(cfg security.Config, workingDir string, log loggers.Logger) *Exec
//    L110-113: IsNotFound(err error) bool
//    L129-131: (e *Exec) New(name string, arg ...any) (Runner, error)
// EX L135-150: (e *Exec) new(name string, fullyQualifiedName string, arg ...any) (Runner, error)
//    L154-164: (b binaryLocation) String() string
// EX L177-232: (e *Exec) Npx(name string, arg ...any) (Runner, error)
//    L240-244: (e *Exec) checkNpx()
//    L247-250: (e *Exec) npx(name string, arg ...any) (Runner, error)
// EX L253-255: (e *Exec) Sec() security.Config
//    L262-264: (e *NotFoundError) Error() string
// EX L281-296: (c *cmdWrapper) Run() error
// EX L298-300: (c *cmdWrapper) StdinPipe() (io.WriteCloser, error)
// EX L314-367: (c *commandeer) command(arg ...any) (*cmdWrapper, error)
// EX L370-376: InPath(binaryName string) bool
//    L380-389: LookPath(binaryName string) string
// ---------------------------------------------------------------------------
