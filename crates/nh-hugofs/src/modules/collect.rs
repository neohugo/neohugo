//! Port of `modules/collect.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).


use std::sync::Arc;

use nh_common::Result;

use super::client::Client;
use super::module::{Module, Modules};

/// Go: `modules.ModulesConfig`.
#[derive(Clone, Default)]
pub struct ModulesConfig {
    pub all_modules: Modules,
    pub go_modules_filename: String,
    pub go_workspace_filename: String,
}

impl Client {
    /// Go: `Client.Collect()` — only the project module (`createProjectModule` + `applyMounts`:
    /// `normalizeMounts` (Clean, silently skip missing sources) and `mountCommonJSConfig`
    /// (auto-mount package.json / postcss.config.js etc. to `assets/_jsconfig/<name>`)).
    // Go: modules/collect.go:Collect
    pub fn collect(&self) -> Result<ModulesConfig> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: modules/collect.go (755 lines; 14/22 funcs executed)
//   types: ModulesConfig, collected, collector, vendoredModule
// EX L51-72: (h *Client) Collect() (ModulesConfig, error)
// EX L74-105: (h *Client) collect(tidy bool) (ModulesConfig, *collector)
//    L118-125: (m ModulesConfig) HasConfigFile() bool
// EX L127-135: (m *ModulesConfig) setActiveMods(logger loggers.Logger) error
// EX L137-143: (m *ModulesConfig) finalize(logger loggers.Logger) error
// EX L145-156: filterUnwantedMounts(mounts []Mount) []Mount
// EX L187-201: (c *collector) initModules() error
//    L203-210: (c *collector) isSeen(path string) bool
//    L212-215: (c *collector) getVendoredDir(path string) (vendoredModule, bool)
//    L217-332: (c *collector) add(owner *moduleAdapter, moduleImport Import) (*moduleAdapter, error)
// EX L334-360: (c *collector) addAndRecurse(owner *moduleAdapter) error
// EX L362-405: (c *collector) applyMounts(moduleImport Import, mod *moduleAdapter) error
//    L407-506: (c *collector) applyThemeConfig(tc *moduleAdapter) error
// EX L508-530: (c *collector) collect()
// EX L532-535: (c *collector) isVendored(dir string) bool
//    L537-586: (c *collector) collectModulesTXT(owner Module) error
// EX L588-595: (c *collector) loadModules() error
// EX L600-637: (c *collector) mountCommonJSConfig(owner *moduleAdapter, mounts []Mount) ([]Mount, error)
// EX L639-702: (c *collector) normalizeMounts(owner *moduleAdapter, mounts []Mount) ([]Mount, error)
//    L704-723: (c *collector) wrapModuleNotFound(err error) error
// EX L731-745: createProjectModule(gomod *goModule, workingDir string, conf Config) *moduleAdapter
//    L751-755: pathKey(p string) string
// ---------------------------------------------------------------------------
