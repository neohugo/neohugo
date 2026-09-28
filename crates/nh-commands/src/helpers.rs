//! Port of `commands/helpers.go`.
//!
//! Owner: Wave B task T25 (commands-cli).

use std::sync::Arc;

use go_value::Value;
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;

use crate::commandeer::{ExecError, FlagValue, Flags};

/// Go: commands/helpers.go:newUserError — a `*simplecobra.CommandError`.
// Go: commands/helpers.go:newUserError
pub fn new_user_error(msg: impl Into<String>) -> ExecError {
    ExecError::Command(msg.into())
}

/// Go: commands/helpers.go:flagsToCfg
// Go: commands/helpers.go:flagsToCfg
pub fn flags_to_cfg(flags: &Flags, cfg: Arc<dyn Provider>) -> Arc<dyn Provider> {
    flags_to_cfg_with_additional_config_base(flags, cfg, "")
}

/// [`flags_to_cfg`] with Go's `cfg == nil` (a new provider).
pub fn flags_to_new_cfg(flags: &Flags) -> Arc<dyn Provider> {
    flags_to_cfg(flags, Arc::new(DefaultConfigProvider::new()))
}

/// Go: commands/helpers.go:flagsToCfgWithAdditionalConfigBase — maps flag names to config keys
/// (`minify` -> `minifyOutput`, `destination` -> `publishDir`, `editor` -> `newContentEditor`;
/// `quiet`, `verbose`, `watch`, `liveReloadPort`, `renderToMemory` and `clock` go below
/// `internal.`); every other changed flag keeps its name.
// Go: commands/helpers.go:flagsToCfgWithAdditionalConfigBase
pub fn flags_to_cfg_with_additional_config_base(
    flags: &Flags,
    cfg: Arc<dyn Provider>,
    additional_config_base: &str,
) -> Arc<dyn Provider> {
    // Flags with a different name in the config.
    let key_map = |k: &str| -> Option<&'static str> {
        match k {
            "minify" => Some("minifyOutput"),
            "destination" => Some("publishDir"),
            "editor" => Some("newContentEditor"),
            _ => None,
        }
    };
    // Flags that we for some reason don't want to expose in the site config.
    let internal_key_set = |k: &str| {
        matches!(
            k,
            "quiet" | "verbose" | "watch" | "liveReloadPort" | "renderToMemory" | "clock"
        )
    };

    // Go: flags.VisitAll (sorted by name), changed flags only.
    for (name, _) in &flags.set {
        let target_key = if internal_key_set(name) {
            format!("internal.{name}")
        } else if let Some(mapped) = key_map(name) {
            mapped.to_string()
        } else {
            name.clone()
        };
        set_value_from_flag(flags, name, &*cfg, &target_key, false);
        if !additional_config_base.is_empty() {
            set_value_from_flag(
                flags,
                name,
                &*cfg,
                &format!("{additional_config_base}.{target_key}"),
                true,
            );
        }
    }
    cfg
}

/// Go: commands/helpers.go:setValueFromFlag
// Go: commands/helpers.go:setValueFromFlag
pub fn set_value_from_flag(
    flags: &Flags,
    key: &str,
    cfg: &dyn Provider,
    target_key: &str,
    force: bool,
) {
    let key = key.trim();
    let changed = flags.set.iter().find(|(n, _)| n == key);
    let looked_up = flags.all.iter().find(|(n, _)| n == key);
    let f = if changed.is_some() {
        changed
    } else if force {
        looked_up
    } else {
        None
    };
    let Some((_, v)) = f else {
        return;
    };
    let config_key = if !target_key.is_empty() {
        target_key
    } else {
        key
    };
    let value = match v {
        FlagValue::Bool(b) => Value::Bool(*b),
        FlagValue::String(s) => Value::string(s.as_str()),
        FlagValue::StringSlice(s) => Value::string_list(s.iter().map(|x| x.as_str())),
    };
    cfg.set(config_key, value);
}

/// Go: commands/helpers.go:mkdir
// Go: commands/helpers.go:mkdir
pub fn mkdir(parts: &[&str]) -> nh_common::Result<()> {
    let p = go_path::filepath::join(parts);
    std::fs::create_dir_all(&p).map_err(|e| nh_hugofs::oserror::from_io("mkdir", &p, &e))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: commands/helpers.go (122 lines; 3/5 funcs executed)
// OK L37-39: newUserError(a ...any) *simplecobra.CommandError
// OK L41-67: setValueFromFlag(flags *pflag.FlagSet, key string, cfg config.Provider, targetKey string, force bool)
// OK L69-71: flagsToCfg(cd *simplecobra.Commandeer, cfg config.Provider) config.Provider
// OK L73-114: flagsToCfgWithAdditionalConfigBase(cd *simplecobra.Commandeer, cfg config.Provider, additionalConfigBase string) config.Provider
// OK L116-122: mkdir(x ...string) (returns the error; Go log.Fatal)
// ---------------------------------------------------------------------------
