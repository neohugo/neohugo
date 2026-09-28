//! Port of `modules/module.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).

use std::sync::Arc;

use go_value::Time;
use nh_config::config_provider::Provider;

use super::config::{Config, Mount};

/// Go: `modules.Module` (the `*moduleAdapter` after collection). Go modules are never resolved
/// here (no `go` binary): `is_go_mod` is always false, so `Replace()` is nil and `Time()` zero.
#[derive(Clone)]
pub struct Module {
    /// "project" for the main project.
    pub path: String,
    pub dir: String,
    pub config: Config,
    pub config_filenames: Vec<String>,
    /// Effective mounts (after defaults + `_jsconfig` auto-mounts).
    pub mounts: Vec<Mount>,
    pub owner: Option<Arc<Module>>,
    pub is_go_mod: bool,
    pub vendor: bool,
    pub version: String,
    pub watch: bool,
    pub cfg: Option<Arc<dyn Provider>>,
}

impl Module {
    /// Optional config read from the module's config file.
    // Go: modules/module.go:Cfg
    pub fn cfg(&self) -> Option<&Arc<dyn Provider>> {
        self.cfg.as_ref()
    }

    /// The decoded module config and mounts.
    // Go: modules/module.go:Config
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Optional configuration filenames (e.g. "/themes/mytheme/config.json").
    // Go: modules/module.go:ConfigFilenames
    pub fn config_filenames(&self) -> &[String] {
        &self.config_filenames
    }

    /// Directory holding files for this module (may point to the `_vendor` dir).
    // Go: modules/module.go:Dir
    pub fn dir(&self) -> &str {
        &self.dir
    }

    /// Returns whether this is a Go Module.
    // Go: modules/module.go:IsGoMod
    pub fn is_go_mod(&self) -> bool {
        self.is_go_mod
    }

    /// Any directory remappings.
    // Go: modules/module.go:Mounts
    pub fn mounts(&self) -> &[Mount] {
        &self.mounts
    }

    /// In the dependency tree, this is the first module that defines this module as a
    /// dependency.
    // Go: modules/module.go:Owner
    pub fn owner(&self) -> Option<&Arc<Module>> {
        self.owner.as_ref()
    }

    /// Returns the path to this module: the module path or the path below the /themes folder.
    // Go: modules/module.go:Path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Replaced by this module (Go modules only; always nil here).
    // Go: modules/module.go:Replace
    pub fn replace(&self) -> Option<Arc<Module>> {
        None
    }

    /// Returns whether Dir points below the _vendor dir.
    // Go: modules/module.go:Vendor
    pub fn vendor(&self) -> bool {
        self.vendor
    }

    /// The module version.
    // Go: modules/module.go:Version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Time version was created (Go modules only; always the zero time here).
    // Go: modules/module.go:Time
    pub fn time(&self) -> Time {
        Time::zero()
    }

    /// Whether this module's dir is a watch candidate.
    // Go: modules/module.go:Watch
    pub fn watch(&self) -> bool {
        self.watch
    }
}

/// Go: `modules.Modules`.
pub type Modules = Vec<Arc<Module>>;

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: modules/module.go (180 lines; 12/13 funcs executed)
//   types: Module, Modules, moduleAdapter
// OK L93-95: (m *moduleAdapter) Cfg() config.Provider
// OK L97-99: (m *moduleAdapter) Config() Config
// OK L101-103: (m *moduleAdapter) ConfigFilenames() []string
// OK L105-111: (m *moduleAdapter) Dir() string
// OK L113-115: (m *moduleAdapter) IsGoMod() bool
// OK L117-119: (m *moduleAdapter) Mounts() []Mount
// OK L121-123: (m *moduleAdapter) Owner() Module
// OK L125-130: (m *moduleAdapter) Path() string
// OK L132-140: (m *moduleAdapter) Replace() Module
// OK L142-144: (m *moduleAdapter) Vendor() bool
// OK L146-151: (m *moduleAdapter) Version() string
// OK L153-159: (m *moduleAdapter) Time() time.Time
// OK L161-180: (m *moduleAdapter) Watch() bool
// ---------------------------------------------------------------------------
