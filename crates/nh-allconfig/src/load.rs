//! Port of `config/allconfig/load.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).


//! Go `config/allconfig/load.go`: config file discovery (`hugo.toml`...), config dir, flags,
//! `HUGO_*` env overrides (applied twice), default merge strategies (`_merge` keys), aliases,
//! modules (project only), then `fromLoadConfigResult` + `Configs.Init`.

use std::sync::Arc;

use nh_common::Result;
use nh_config::config_provider::Provider;

use crate::allconfig::Configs;

/// Go: `allconfig.ConfigSourceDescriptor`.
pub struct ConfigSourceDescriptor {
    /// Flags from the CLI (already mapped: `minify` -> `minifyOutput`, `destination` -> `publishDir`,
    /// `internal.*` keys).
    pub flags: Option<Arc<dyn Provider>>,
    /// Config filename(s) (`--config`), comma separated; default: discovery.
    pub filename: String,
    /// Config dir (`--configDir`, default "config").
    pub config_dir: String,
    /// "production" by default for builds.
    pub environment: String,
    /// `os.Environ()` (for `HUGO_*` overrides).
    pub environ: Vec<String>,
    pub ignore_module_does_not_exist: bool,
    /// The working dir (`--source` or cwd).
    pub working_dir: String,
}

/// Go: `allconfig.LoadConfig(d)`.
// Go: config/allconfig/load.go:LoadConfig
pub fn load_config(d: ConfigSourceDescriptor) -> Result<Configs> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/allconfig/load.go (544 lines; 12/15 funcs executed)
//   types: ConfigSourceDescriptor, configLoader
// EX L43-97: LoadConfig(d ConfigSourceDescriptor) (*Configs, error)
// EX L124-129: (d ConfigSourceDescriptor) configFilenames() []string
// EX L142-158: (l configLoader) applyConfigAliases() error
// EX L160-170: (l configLoader) applyDefaultConfig() error
// EX L172-180: (l configLoader) normalizeCfg(cfg config.Provider) error
// EX L182-187: (l configLoader) cleanExternalConfig(cfg config.Provider) error
// EX L189-194: (l configLoader) applyFlagsOverrides(cfg config.Provider) error
// EX L196-270: (l configLoader) applyOsEnvOverrides(environ []string) error
//    L272-279: (l *configLoader) envValToVal(k string, v any) any
//    L281-292: (l *configLoader) envStringToVal(k, v string) any
// EX L294-414: (l *configLoader) loadConfigMain(d ConfigSourceDescriptor) (config.LoadConfigResult, modules.ModulesConfig, error)
// EX L416-477: (l *configLoader) loadModules(configs *Configs, ignoreModuleDoesNotExist bool) (modules.ModulesConfig, *modules.Client, error)
// EX L479-526: (l configLoader) loadConfig(configName string) (string, error)
// EX L528-533: (l configLoader) deleteMergeStrategies()
//    L535-544: (l configLoader) wrapFileError(err error, filename string) error
// ---------------------------------------------------------------------------
