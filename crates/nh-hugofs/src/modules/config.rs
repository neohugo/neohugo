//! Port of `modules/config.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).

use go_value::{Map, Value};
use nh_common::Result;
use nh_config::config_provider::Provider;

/// Go: `modules.Mount`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mount {
    /// Relative path in source repo, e.g. "scss".
    pub source: String,
    /// Relative target path, e.g. "assets/bootstrap/scss".
    pub target: String,
    /// Any file in this mount will be associated with this language.
    pub lang: String,
    /// Include only files matching the given Glob patterns (Go `any`: string or slice; normalised at decode).
    pub include_files: Vec<String>,
    /// Exclude all files matching the given Glob patterns (Go `any`: string or slice; normalised at decode).
    pub exclude_files: Vec<String>,
    pub disable_watch: bool,
}

impl Mount {
    /// Go: `Mount.Component()` — first path element of Target.
    pub fn component(&self) -> String {
        todo!()
    }

    /// Go: `Mount.ComponentAndName()`.
    pub fn component_and_name(&self) -> (String, String) {
        todo!()
    }
}

/// Go: `modules.Import`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Import {
    pub path: String,
    pub ignore_config: bool,
    pub ignore_imports: bool,
    pub no_mounts: bool,
    pub no_vendor: bool,
    pub disable: bool,
    pub mounts: Vec<Mount>,
}

/// Go: `modules.Config`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub mounts: Vec<Mount>,
    pub imports: Vec<Import>,
    /// Module params (`[module.params]`), `None` if unset.
    pub params: Option<Map>,
    pub hugo_version_min: String,
    pub hugo_version_max: String,
    pub no_vendor: String,
    pub vendor_closest: bool,
    pub replacements: Vec<String>,
    pub proxy: String,
    pub no_proxy: String,
    pub private: String,
    pub auth: String,
    pub workspace: String,
}

/// Go: `modules.DecodeConfig(cfg)` (maps legacy `contentDir`, `staticDir*` ... to mounts).
// Go: modules/config.go:DecodeConfig
pub fn decode_config(cfg: &dyn Provider) -> Result<Config> {
    todo!()
}

/// Go: `modules.ApplyProjectConfigDefaults(mod, cfgs...)` — default mounts for components without one.
// Go: modules/config.go:ApplyProjectConfigDefaults
pub fn apply_project_config_defaults(
    m: &mut super::module::Module,
    cfgs: &[&dyn nh_config::config_provider::AllProvider],
) -> Result<()> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: modules/config.go (412 lines; 6/9 funcs executed)
//   types: Config, HugoVersion, Import, Mount
// EX L59-165: ApplyProjectConfigDefaults(mod Module, cfgs ...config.AllProvider) error
// EX L168-170: DecodeConfig(cfg config.Provider) (Config, error)
// EX L172-252: decodeConfig(cfg config.Provider, pathReplacements map[string]string) (Config, error)
//    L314-321: (c Config) hasModuleImport() bool
//    L332-345: (v HugoVersion) String() string
// EX L349-359: (v HugoVersion) IsValid() bool
// EX L401-403: (m Mount) key() string
// EX L405-407: (m Mount) Component() string
//    L409-412: (m Mount) ComponentAndName() (string, string)
// ---------------------------------------------------------------------------
