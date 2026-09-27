//! Port of `commands/helpers.go`.
//!
//! Owner: Wave B task T25 (commands-cli).


use std::sync::Arc;

use nh_config::config_provider::Provider;

use crate::commandeer::Flags;

/// Go: commands/helpers.go:flagsToCfg
pub fn flags_to_cfg(flags: &Flags, cfg: Arc<dyn Provider>) -> Arc<dyn Provider> {
    flags_to_cfg_with_additional_config_base(flags, cfg, "")
}

/// Go: commands/helpers.go:flagsToCfgWithAdditionalConfigBase — maps flag names to config keys
/// (`minify` -> `minifyOutput`, `destination` -> `publishDir`, `printI18nWarnings` ->
/// `logI18nWarnings`, `printPathWarnings` -> `logPathWarnings`, `theme`, `cacheDir`, ...).
pub fn flags_to_cfg_with_additional_config_base(flags: &Flags, cfg: Arc<dyn Provider>, additional_config_base: &str) -> Arc<dyn Provider> {
    todo!()
}

/// Go: commands/helpers.go:setValueFromFlag
pub fn set_value_from_flag(flags: &Flags, key: &str, cfg: &dyn Provider, target_key: &str, force: bool) {
    todo!()
}

/// Go: commands/helpers.go:mkdir
pub fn mkdir(parts: &[&str]) {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: commands/helpers.go (122 lines; 3/5 funcs executed)
//    L37-39: newUserError(a ...any) *simplecobra.CommandError
// EX L41-67: setValueFromFlag(flags *pflag.FlagSet, key string, cfg config.Provider, targetKey string, force bool)
// EX L69-71: flagsToCfg(cd *simplecobra.Commandeer, cfg config.Provider) config.Provider
// EX L73-114: flagsToCfgWithAdditionalConfigBase(cd *simplecobra.Commandeer, cfg config.Provider, additionalConfigBase string) config.Provider
//    L116-122: mkdir(x ...string)
// ---------------------------------------------------------------------------
