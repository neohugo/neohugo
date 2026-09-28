//! Port of `config/configLoader.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use std::path::Path;
use std::sync::OnceLock;

use go_value::{Map, MapType, Value};
use nh_common::maps::maps::KeyRenamer;
use nh_common::{Error, Result};
use nh_parser::metadecoders::decoder::Decoder;
use nh_parser::metadecoders::format::Format;

use crate::config_provider::Provider;
use crate::default_config_provider::DefaultConfigProvider;

/// Go: `config.DefaultConfigNames` (`hugo`, `config`). Hugo has always used config.toml etc.
/// as the default config file name, but hugo.toml is a more descriptive name, so both are
/// checked.
pub const DEFAULT_CONFIG_NAMES: &[&str] = &["hugo", "config"];
/// Go: `config.ValidConfigFileExtensions`.
pub const VALID_CONFIG_FILE_EXTENSIONS: &[&str] = &["toml", "yaml", "yml", "json"];

/// Go: `config.DefaultConfigNamesSet`.
fn is_default_config_name(name: &str) -> bool {
    DEFAULT_CONFIG_NAMES.contains(&name)
}

/// Go: `config.IsValidConfigFilename(filename)`.
// Go: config/configLoader.go:IsValidConfigFilename
pub fn is_valid_config_filename(filename: &str) -> bool {
    let ext = go_path::filepath::ext(filename);
    let ext = go_unicode::strings::to_lower_str(ext.strip_prefix('.').unwrap_or(ext)).into_owned();
    VALID_CONFIG_FILE_EXTENSIONS.contains(&ext.as_str())
}

/// Go: `config.FromTOMLConfigString(config)` (panics on error, like Go).
// Go: config/configLoader.go:FromTOMLConfigString
pub fn from_toml_config_string(config: &str) -> DefaultConfigProvider {
    match from_config_string(config, "toml") {
        Ok(cfg) => cfg,
        Err(e) => panic!("{e}"),
    }
}

/// Go: `config.FromConfigString(config, configType)`.
// Go: config/configLoader.go:FromConfigString
pub fn from_config_string(config: &str, config_type: &str) -> Result<DefaultConfigProvider> {
    let m = read_config(
        nh_parser::metadecoders::format::format_from_string(config_type),
        config.as_bytes(),
    )?;
    Ok(DefaultConfigProvider::new_from(m))
}

/// Go: `config.FromFile(fs, filename)`.
// Go: config/configLoader.go:FromFile
pub fn from_file(filename: &Path) -> Result<DefaultConfigProvider> {
    match load_config_from_file(filename) {
        Ok(m) => Ok(DefaultConfigProvider::new_from(m)),
        Err(err) => {
            let name = filename.to_string_lossy();
            if let Some(pos) = err.pos() {
                let mut pos = pos.clone();
                pos.filename = name.into_owned();
                return Err(err.at(pos));
            }
            Err(nh_common::herrors::new_file_error_from_name(err, &name))
        }
    }
}

/// Go: `config.FromFileToMap(fs, filename)` (then `RenameKeys`).
// Go: config/configLoader.go:FromFileToMap
pub fn from_file_to_map(filename: &Path) -> Result<Map> {
    load_config_from_file(filename)
}

// Go: config/configLoader.go:readConfig
fn read_config(format: Format, data: &[u8]) -> Result<Map> {
    let mut m = Decoder::default().unmarshal_to_map(data, format)?;
    rename_keys(&mut m);
    Ok(m)
}

fn read_file(filename: &str) -> Result<Vec<u8>> {
    std::fs::read(filename).map_err(Error::from)
}

// Go: config/configLoader.go:loadConfigFromFile
fn load_config_from_file(filename: &Path) -> Result<Map> {
    let name = filename.to_string_lossy();
    let mut m = Decoder::default().unmarshal_file_to_map(&name, &read_file)?;
    rename_keys(&mut m);
    Ok(m)
}

/// A file system entry of the config dir walk.
struct WalkEntry {
    path: String,
    is_dir: bool,
}

/// Go: `afero.Walk(fs, root, fn)` over the OS file system: lexical order, directories before
/// their contents, symlinks not followed; unreadable entries are passed with an error (which the
/// config walk ignores).
fn walk_dir(root: &str) -> Vec<WalkEntry> {
    let mut out = Vec::new();
    let Ok(md) = std::fs::symlink_metadata(root) else {
        return out;
    };
    walk_rec(root, md.is_dir(), &mut out);
    out
}

fn walk_rec(path: &str, is_dir: bool, out: &mut Vec<WalkEntry>) {
    out.push(WalkEntry {
        path: path.to_string(),
        is_dir,
    });
    if !is_dir {
        return;
    }
    let Ok(rd) = std::fs::read_dir(path) else {
        return;
    };
    let mut names: Vec<String> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for name in names {
        let p = go_path::filepath::join(&[path, &name]);
        let Ok(md) = std::fs::symlink_metadata(&p) else {
            continue;
        };
        walk_rec(&p, md.is_dir(), out);
    }
}

/// Go: `config.LoadConfigFromDir(sourceFs, configDir, environment)`: merges
/// `<configDir>/_default` and `<configDir>/<environment>` (least to most specific). Returns
/// `None` when neither exists, and the visited directories.
// Go: config/configLoader.go:LoadConfigFromDir
pub fn load_config_from_dir(
    config_dir: &Path,
    environment: &str,
) -> Result<(Option<Box<dyn Provider>>, Vec<String>)> {
    let config_dir = config_dir.to_string_lossy().into_owned();
    let default_config_dir = go_path::filepath::join(&[config_dir.as_str(), "_default"]);
    let environment_config_dir = go_path::filepath::join(&[config_dir.as_str(), environment]);
    let cfg = DefaultConfigProvider::new();

    let mut config_dirs: Vec<String> = Vec::new();
    // Merge from least to most specific.
    for dir in [default_config_dir, environment_config_dir] {
        if std::fs::metadata(&dir).is_ok() {
            config_dirs.push(dir);
        }
    }

    if config_dirs.is_empty() {
        return Ok((None, Vec::new()));
    }

    // Keep track of these so we can watch them for changes.
    let mut dirnames: Vec<String> = Vec::new();

    for config_dir in &config_dirs {
        for entry in walk_dir(config_dir) {
            let path = entry.path;
            if entry.is_dir {
                dirnames.push(path);
                continue;
            }

            if !is_valid_config_filename(&path) {
                continue;
            }

            let name = nh_common::paths::path::filename(go_path::filepath::base(&path));

            let item = match Decoder::default().unmarshal_file_to_map(&path, &read_file) {
                Ok(item) => item,
                Err(err) => {
                    return Err(Error::new(format!(
                        "failed to unmarshal config for path {}: {}",
                        go_strconv::quote(&path),
                        err
                    )));
                }
            };

            let mut key_path: Vec<String> = Vec::new();
            if !is_default_config_name(&name) {
                // Can be params.jp, menus.en etc.
                let (name, lang) = nh_common::paths::path::file_and_ext_no_delimiter(&name);

                key_path = vec![name.clone()];

                if !lang.is_empty() {
                    key_path = vec!["languages".to_string(), lang];
                    match name.as_str() {
                        "menu" | "menus" => key_path.push("menus".to_string()),
                        "params" => key_path.push("params".to_string()),
                        _ => {}
                    }
                }
            }

            let mut root = item;
            if !key_path.is_empty() {
                let mut v = Value::map(root);
                for key in key_path.iter().rev() {
                    let mut nm = Map::new(MapType::StringAny);
                    nm.insert(key.as_str(), v);
                    v = Value::map(nm);
                }
                root = match v {
                    Value::Map(m) => (*m).clone(),
                    _ => unreachable!(),
                };
            }

            // Migrate menu => menus etc.
            rename_keys(&mut root);

            // Set will overwrite keys with the same name, recursively.
            cfg.set("", Value::map(root));
        }
    }

    Ok((Some(Box::new(cfg)), dirnames))
}

/// Go: `keyAliases`: before 0.53 Hugo used singular for "menu".
fn key_aliases() -> &'static KeyRenamer {
    static K: OnceLock<KeyRenamer> = OnceLock::new();
    K.get_or_init(|| {
        KeyRenamer::new(&["{menu,languages/*/menu}", "menus"]).expect("valid key renamer")
    })
}

/// Go: `config.RenameKeys(m)` (`menu` -> `menus`, `languages/*/menu` -> `menus`).
// Go: config/configLoader.go:RenameKeys
pub fn rename_keys(m: &mut Map) {
    key_aliases().rename(m);
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/configLoader.go (231 lines; 6/11 funcs executed)
// OK L43-51: init()
// OK L55-58: IsValidConfigFilename(filename string) bool
// OK L60-66: FromTOMLConfigString(config string) Provider
// OK L69-75: FromConfigString(config, configType string) (Provider, error)
// OK L78-92: FromFile(fs afero.Fs, filename string) (Provider, error)
// OK L96-98: FromFileToMap(fs afero.Fs, filename string) (map[string]any, error)
// OK L100-109: readConfig(format metadecoders.Format, data []byte) (map[string]any, error)
// OK L111-118: loadConfigFromFile(fs afero.Fs, filename string) (map[string]any, error)
// OK L120-212: LoadConfigFromDir(sourceFs afero.Fs, configDir, environment string) (Provider, []string, error)
// OK L216-225: init()
// OK L229-231: RenameKeys(m map[string]any)
// ---------------------------------------------------------------------------
