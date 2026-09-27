//! Port of `config/configLoader.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


use go_value::Map;
use nh_common::Result;

use crate::config_provider::Provider;

/// Go: `config.DefaultConfigNames` (`hugo`, `config`).
pub const DEFAULT_CONFIG_NAMES: &[&str] = &["hugo", "config"];
/// Go: `config.ValidConfigFileExtensions`.
pub const VALID_CONFIG_FILE_EXTENSIONS: &[&str] = &["toml", "yaml", "yml", "json"];

/// Go: `config.FromFileToMap(fs, filename)` (then `RenameKeys`).
// Go: config/configLoader.go:FromFileToMap
pub fn from_file_to_map(filename: &std::path::Path) -> Result<Map> {
    todo!()
}

/// Go: `config.LoadConfigFromDir(sourceFs, configDir, environment)`.
// Go: config/configLoader.go:LoadConfigFromDir
pub fn load_config_from_dir(config_dir: &std::path::Path, environment: &str) -> Result<(Box<dyn Provider>, Vec<String>)> {
    todo!()
}

/// Go: `config.RenameKeys(m)` (`menu` -> `menus`, `languages/*/menu` -> `menus`).
// Go: config/configLoader.go:RenameKeys
pub fn rename_keys(m: &mut Map) {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/configLoader.go (231 lines; 6/11 funcs executed)
// EX L43-51: init()
//    L55-58: IsValidConfigFilename(filename string) bool
//    L60-66: FromTOMLConfigString(config string) Provider
//    L69-75: FromConfigString(config, configType string) (Provider, error)
//    L78-92: FromFile(fs afero.Fs, filename string) (Provider, error)
// EX L96-98: FromFileToMap(fs afero.Fs, filename string) (map[string]any, error)
//    L100-109: readConfig(format metadecoders.Format, data []byte) (map[string]any, error)
// EX L111-118: loadConfigFromFile(fs afero.Fs, filename string) (map[string]any, error)
// EX L120-212: LoadConfigFromDir(sourceFs afero.Fs, configDir, environment string) (Provider, []string, error)
// EX L216-225: init()
// EX L229-231: RenameKeys(m map[string]any)
// ---------------------------------------------------------------------------
