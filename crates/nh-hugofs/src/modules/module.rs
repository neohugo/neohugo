//! Port of `modules/module.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).


use std::sync::Arc;

use nh_config::config_provider::Provider;

use super::config::{Config, Mount};

/// Go: `modules.Module` (only the project module exists for seeksnack; themes/imports not supported yet).
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

/// Go: `modules.Modules`.
pub type Modules = Vec<Arc<Module>>;

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: modules/module.go (180 lines; 12/13 funcs executed)
//   types: Module, Modules, moduleAdapter
//    L93-95: (m *moduleAdapter) Cfg() config.Provider
// EX L97-99: (m *moduleAdapter) Config() Config
// EX L101-103: (m *moduleAdapter) ConfigFilenames() []string
// EX L105-111: (m *moduleAdapter) Dir() string
// EX L113-115: (m *moduleAdapter) IsGoMod() bool
// EX L117-119: (m *moduleAdapter) Mounts() []Mount
// EX L121-123: (m *moduleAdapter) Owner() Module
// EX L125-130: (m *moduleAdapter) Path() string
// EX L132-140: (m *moduleAdapter) Replace() Module
// EX L142-144: (m *moduleAdapter) Vendor() bool
// EX L146-151: (m *moduleAdapter) Version() string
// EX L153-159: (m *moduleAdapter) Time() time.Time
// EX L161-180: (m *moduleAdapter) Watch() bool
// ---------------------------------------------------------------------------
