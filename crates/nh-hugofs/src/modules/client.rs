//! Port of `modules/client.go`.
//!
//! only the project-module path (no go/npm)
//!
//! Owner: Wave B task T09 (allconfig-modules).
//!
//! Modules are collected without the `go` binary: the project, themes below `themesDir`
//! (and absolute/relative module paths of the project), and modules vendored in `_vendor`
//! (`modules.txt`) resolve like Go. Everything that makes Go run `go mod download`/`go list`/
//! `go get` (a `go.mod` in the project together with an import that looks like a Go module path
//! and is not vendored) returns an explicit unsupported error, as do the `hugo mod` commands
//! that manage Go modules.

use std::sync::Arc;

use nh_common::glob::glob::Glob;
use nh_common::herrors::ErrorKind;
use nh_common::loggers::Logger;
use nh_common::{Error, Result};

use super::config::Config;
use super::module::Modules;
use crate::afero::Fs;

/// Go: `modules.vendord`.
pub(crate) const VENDORD: &str = "_vendor";

/// Go: `modules.goModFilename`.
const GO_MOD_FILENAME: &str = "go.mod";

/// Go: `ClientConfig.HookBeforeFinalize`: run on the collected modules before duplicate mounts
/// are removed (allconfig merges theme configs here).
pub type HookBeforeFinalize = Arc<dyn Fn(&Modules) -> Result<()> + Send + Sync>;

/// Go: `modules.ClientConfig`.
#[derive(Clone, Default)]
pub struct ClientConfig {
    pub working_dir: String,
    pub themes_dir: String,
    pub publish_dir: String,
    pub environment: String,
    pub cache_dir: String,
    pub module_config: Config,
    pub ignore_module_does_not_exist: bool,
    /// Go: `Fs` (`hugofs.Os` in a build).
    pub fs: Option<Arc<dyn Fs>>,
    /// Go: `Logger`.
    pub logger: Option<Logger>,
    /// Go: `HookBeforeFinalize`.
    pub hook_before_finalize: Option<HookBeforeFinalize>,
    /// Go: `IgnoreVendor`: ignore any _vendor directory for module paths matching the given
    /// pattern. This can be nil.
    pub ignore_vendor: Option<Glob>,
}

impl ClientConfig {
    // Go: modules/client.go:shouldIgnoreVendor
    pub(crate) fn should_ignore_vendor(&self, path: &str) -> bool {
        self.ignore_vendor.as_ref().is_some_and(|g| g.matches(path))
    }

    /// Go: `ClientConfig.toEnv()`: the environment of the `go` commands (never run here).
    // Go: modules/client.go:toEnv
    pub fn to_env(&self) -> Vec<String> {
        let mcfg = &self.module_config;
        let mut env: Vec<String> = Vec::new();
        let gocache = go_path::filepath::join(&[self.cache_dir.as_str(), "pkg", "mod"]);
        let mut key_vals: Vec<(&str, &str)> = vec![
            ("PWD", self.working_dir.as_str()),
            ("GO111MODULE", "on"),
            ("GOPATH", self.cache_dir.as_str()),
            ("GOWORK", mcfg.workspace.as_str()), // Requires Go 1.18, see https://tip.golang.org/doc/go1.18
            // GOCACHE was introduced in Go 1.15. This matches the location derived from GOPATH above.
            ("GOCACHE", gocache.as_str()),
        ];

        if !mcfg.proxy.is_empty() {
            key_vals.push(("GOPROXY", mcfg.proxy.as_str()));
        }
        if !mcfg.private.is_empty() {
            key_vals.push(("GOPRIVATE", mcfg.private.as_str()));
        }
        if !mcfg.no_proxy.is_empty() {
            key_vals.push(("GONOPROXY", mcfg.no_proxy.as_str()));
        }
        if !mcfg.auth.is_empty() {
            // GOAUTH was introduced in Go 1.24, see https://tip.golang.org/doc/go1.24.
            key_vals.push(("GOAUTH", mcfg.auth.as_str()));
        }

        nh_config::env::set_env_vars(&mut env, &key_vals);

        env
    }
}

/// Go: `modules.Client` (project-module subset: no `go`/`npm` invocations).
pub struct Client {
    pub ccfg: ClientConfig,
    /// The top level module config.
    pub module_config: Config,
    pub(crate) fs: Arc<dyn Fs>,
    pub(crate) logger: Logger,
    /// Go: `noVendor` (only used by `hugo mod vendor`).
    pub(crate) no_vendor: Option<Glob>,
    /// Environment variables used in "go get" etc.
    pub environ: Vec<String>,
    /// Set when Go modules are initialized in the current repo, that is: a go.mod file exists.
    pub go_modules_filename: String,
}

impl Client {
    // Go: modules/client.go:NewClient
    pub fn new(cfg: ClientConfig) -> Client {
        let fs = cfg.fs.clone().unwrap_or_else(crate::afero::new_os_fs);
        let n = go_path::filepath::join(&[cfg.working_dir.as_str(), GO_MOD_FILENAME]);
        let go_mod_enabled = crate::afero::exists(fs.as_ref(), &n).unwrap_or(false);
        let mut go_mod_filename = String::new();
        if go_mod_enabled {
            go_mod_filename = n;
        }

        let logger = cfg.logger.clone().unwrap_or_else(Logger::new_default);

        let mut no_vendor = None;
        if !cfg.module_config.no_vendor.is_empty() {
            no_vendor = nh_common::glob::glob::get_glob(&nh_common::glob::glob::normalize_path(
                &cfg.module_config.no_vendor,
            ))
            .ok();
        }

        Client {
            fs,
            module_config: cfg.module_config.clone(),
            environ: cfg.to_env(),
            logger,
            no_vendor,
            go_modules_filename: go_mod_filename,
            ccfg: cfg,
        }
    }

    /// Go: `Client.Graph(w)`: one `owner module` line per non-project module.
    // Go: modules/client.go:Graph
    pub fn graph(&self, w: &mut dyn std::io::Write) -> Result<()> {
        let mc = self.collect_modules()?;
        for module in &mc.all_modules {
            let Some(owner) = module.owner() else {
                continue;
            };
            let dep = format!("{} {}", path_version(owner), path_version(module));
            // Replace() is always nil without Go modules.
            writeln!(w, "{dep}").map_err(Error::from)?;
        }
        Ok(())
    }

    // Go: modules/client.go:Tidy
    pub fn tidy(&self) -> Result<()> {
        Err(unsupported("hugo mod tidy"))
    }

    // Go: modules/client.go:Vendor
    pub fn vendor(&self) -> Result<()> {
        Err(unsupported("hugo mod vendor"))
    }

    // Go: modules/client.go:Get
    pub fn get(&self, _args: &[&str]) -> Result<()> {
        Err(unsupported("hugo mod get"))
    }

    // Go: modules/client.go:Init
    pub fn init(&self, _path: &str) -> Result<()> {
        Err(unsupported("hugo mod init"))
    }

    // Go: modules/client.go:Verify
    pub fn verify(&self, _clean: bool) -> Result<()> {
        Err(unsupported("hugo mod verify"))
    }

    // Go: modules/client.go:Clean
    pub fn clean(&self, _pattern: &str) -> Result<()> {
        Err(unsupported("hugo mod clean"))
    }

    /// Go: `Client.listGoMods()`: nothing to list unless the project has a go.mod and imports
    /// a Go module; listing those needs the `go` binary (unsupported).
    // Go: modules/client.go:listGoMods
    pub(crate) fn list_go_mods(&self) -> Result<()> {
        if self.go_modules_filename.is_empty() || !self.module_config.has_module_import() {
            return Ok(());
        }
        Err(unsupported(
            "Go modules (go.mod with module imports; resolving them runs `go mod download` and `go list`)",
        ))
    }

    /// Go: `Client.get(args...)` (`go get`).
    // Go: modules/client.go:get
    pub(crate) fn go_get(&self, arg: &str) -> Result<()> {
        Err(unsupported(&format!("go get {arg}")))
    }

    /// Go: `Client.createThemeDirname(modulePath, isProjectMod)`.
    // Go: modules/client.go:createThemeDirname
    pub(crate) fn create_theme_dirname(
        &self,
        module_path: &str,
        is_project_mod: bool,
    ) -> Result<String> {
        let invalid = || {
            Error::new(format!(
                "invalid module path {}; must be relative to themesDir when defined outside of the project",
                go_strconv::quote(module_path.as_bytes())
            ))
        };

        let module_path = go_path::filepath::clean(module_path);
        if go_path::filepath::is_abs(&module_path) {
            if is_project_mod {
                return Ok(module_path);
            }
            return Err(invalid());
        }

        let module_dir =
            go_path::filepath::join(&[self.ccfg.themes_dir.as_str(), module_path.as_str()]);
        if !is_project_mod && !module_dir.starts_with(&self.ccfg.themes_dir) {
            return Err(invalid());
        }
        Ok(module_dir)
    }
}

fn unsupported(what: &str) -> Error {
    Error::with_kind(
        ErrorKind::FeatureNotAvailable,
        format!("neohugo-rs: {what} is not supported"),
    )
}

/// Go: `pathVersion(m)`.
// Go: modules/client.go:pathVersion
fn path_version(m: &super::module::Module) -> String {
    let mut version_str = m.version().to_string();
    if m.vendor() {
        version_str.push_str("+vendor");
    }
    if version_str.is_empty() {
        return m.path().to_string();
    }
    format!("{}@{}", m.path(), version_str)
}

/// Go: `isProbablyModule(path)` = `module.CheckPath(path) == nil`.
// Go: modules/client.go:isProbablyModule
pub(crate) fn is_probably_module(path: &str) -> bool {
    check_path(path).is_ok()
}

// ---------------------------------------------------------------------------
// golang.org/x/mod@v0.25.0/module: CheckPath and SplitPathVersion (the error texts are not
// needed: Hugo only asks whether the path is valid).

#[derive(Clone, Copy, PartialEq, Eq)]
enum PathKind {
    ModulePath,
}

// Go: golang.org/x/mod/module/module.go:CheckPath
fn check_path(path: &str) -> std::result::Result<(), ()> {
    check_path_kind(path, PathKind::ModulePath)?;
    let i = path.find('/').unwrap_or(path.len());
    if i == 0 {
        return Err(()); // leading slash
    }
    if !path[..i].contains('.') {
        return Err(()); // missing dot in first path element
    }
    if path.starts_with('-') {
        return Err(()); // leading dash in first path element
    }
    if !path[..i].chars().all(first_path_ok) {
        return Err(()); // invalid char in first path element
    }
    if !split_path_version(path).2 {
        return Err(()); // invalid version
    }
    Ok(())
}

// Go: golang.org/x/mod/module/module.go:firstPathOK
fn first_path_ok(r: char) -> bool {
    r == '-' || r == '.' || r.is_ascii_digit() || r.is_ascii_lowercase()
}

// Go: golang.org/x/mod/module/module.go:modPathOK
fn mod_path_ok(r: char) -> bool {
    if (r as u32) < 0x80 {
        return r == '-'
            || r == '.'
            || r == '_'
            || r == '~'
            || r.is_ascii_digit()
            || r.is_ascii_uppercase()
            || r.is_ascii_lowercase();
    }
    false
}

// Go: golang.org/x/mod/module/module.go:checkPath
fn check_path_kind(path: &str, kind: PathKind) -> std::result::Result<(), ()> {
    // A Rust &str is valid UTF-8.
    if path.is_empty() {
        return Err(());
    }
    if path.starts_with('-') {
        return Err(());
    }
    if path.contains("//") {
        return Err(());
    }
    if path.ends_with('/') {
        return Err(());
    }
    for elem in path.split('/') {
        check_elem(elem, kind)?;
    }
    Ok(())
}

const BAD_WINDOWS_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

// Go: golang.org/x/mod/module/module.go:checkElem
fn check_elem(elem: &str, kind: PathKind) -> std::result::Result<(), ()> {
    if elem.is_empty() {
        return Err(());
    }
    if elem.bytes().filter(|&b| b == b'.').count() == elem.len() {
        return Err(());
    }
    if elem.starts_with('.') && kind == PathKind::ModulePath {
        return Err(());
    }
    if elem.ends_with('.') {
        return Err(());
    }
    if !elem.chars().all(mod_path_ok) {
        return Err(());
    }

    // Windows disallows a bunch of path elements, sadly.
    let short = match elem.find('.') {
        Some(i) => &elem[..i],
        None => elem,
    };
    for bad in BAD_WINDOWS_NAMES {
        if go_unicode::strings::equal_fold_str(bad, short) {
            return Err(());
        }
    }

    // Reject path components that look like Windows short-names.
    if let Some(tilde) = short.rfind('~')
        && tilde < short.len() - 1
    {
        let suffix = &short[tilde + 1..];
        if suffix.bytes().all(|b| b.is_ascii_digit()) {
            return Err(());
        }
    }

    Ok(())
}

/// Go: `module.SplitPathVersion(path)`: `(prefix, pathMajor, ok)`.
// Go: golang.org/x/mod/module/module.go:SplitPathVersion
pub(crate) fn split_path_version(path: &str) -> (&str, &str, bool) {
    if path.starts_with("gopkg.in/") {
        return split_gopkg_in(path);
    }

    let b = path.as_bytes();
    let mut i = b.len();
    let mut dot = false;
    while i > 0 && (b[i - 1].is_ascii_digit() || b[i - 1] == b'.') {
        if b[i - 1] == b'.' {
            dot = true;
        }
        i -= 1;
    }
    if i <= 1 || i == b.len() || b[i - 1] != b'v' || b[i - 2] != b'/' {
        return (path, "", true);
    }
    let (prefix, path_major) = (&path[..i - 2], &path[i - 2..]);
    if dot || path_major.len() <= 2 || path_major.as_bytes()[2] == b'0' || path_major == "/v1" {
        return (path, "", false);
    }
    (prefix, path_major, true)
}

// Go: golang.org/x/mod/module/module.go:splitGopkgIn
fn split_gopkg_in(path: &str) -> (&str, &str, bool) {
    if !path.starts_with("gopkg.in/") {
        return (path, "", false);
    }
    let b = path.as_bytes();
    let mut i = b.len();
    if path.ends_with("-unstable") {
        i -= "-unstable".len();
    }
    while i > 0 && b[i - 1].is_ascii_digit() {
        i -= 1;
    }
    if i <= 1 || b[i - 1] != b'v' || b[i - 2] != b'.' {
        // All gopkg.in paths must end in vN for some N.
        return (path, "", false);
    }
    let (prefix, path_major) = (&path[..i - 2], &path[i - 2..]);
    if path_major.len() <= 2 || (path_major.as_bytes()[2] == b'0' && path_major != ".v0") {
        return (path, "", false);
    }
    (prefix, path_major, true)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: modules/client.go (873 lines; 4/26 funcs executed)
//   types: Client, goOutputReplacerWriter, ClientConfig, goBinaryStatus, goModule, goModuleError, goModules
// OK L67-95: NewClient(cfg ClientConfig) *Client
// OK L124-147: (c *Client) Graph(w io.Writer) error
// STUB L150-161: (c *Client) Tidy() error
// STUB L175-290: (c *Client) Vendor() error
// STUB L293-344: (c *Client) Get(args ...string) error
// STUB L346-351: (c *Client) get(args ...string) error
// STUB L356-365: (c *Client) Init(path string) error
// STUB L372-390: (c *Client) Verify(clean bool) error
// STUB L392-422: (c *Client) Clean(pattern string) error
// STUB L424-426: (c *Client) runVerify() error
// OK L428-430: isProbablyModule(path string) bool
// OK L432-517: (c *Client) listGoMods() (goModules, error)   (no Go modules: nil, or unsupported)
// STUB L519-531: (c *Client) rewriteGoMod(name string, isGoMod map[string]bool) error
// STUB L533-581: (c *Client) rewriteGoModRewrite(name string, isGoMod map[string]bool) ([]byte, error)
// STUB L583-598: (c *Client) rmVendorDir(vendorDir string) error
// STUB L600-657: (c *Client) runGo( ctx context.Context, stdout io.Writer, args ...string, ) error
// STUB L668-675: (w goOutputReplacerWriter) Write(p []byte) (n int, err error)
// STUB L677-703: (c *Client) tidy(mods Modules, goModOnly bool) error
//    L705-707: (c *Client) shouldVendor(path string) bool   (vendoring only)
// OK L709-725: (c *Client) createThemeDirname(modulePath string, isProjectMod bool) (string, error)
// OK L761-763: (c ClientConfig) shouldIgnoreVendor(path string) bool
// OK L765-794: (cfg ClientConfig) toEnv() []string
// OK L818-830: (modules goModules) GetByPath(p string) *goModule   (always empty: nil)
// OK L832-840: (modules goModules) GetMain() *goModule   (always empty: nil)
//    L842-862: getModlineSplitter(isGoMod bool) func(line string) []string   (go.mod rewriting only)
// OK L864-873: pathVersion(m Module) string
// ---------------------------------------------------------------------------
