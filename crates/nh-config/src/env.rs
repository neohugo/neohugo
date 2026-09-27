//! Port of `config/env.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


/// Go: `config.GetNumWorkerMultiplier()` (`HUGO_NUMWORKERMULTIPLIER`, default NumCPU). The Rust
/// port renders sequentially (see HUGO_LAYER.md §8); this only sizes optional worker pools.
// Go: config/env.go:GetNumWorkerMultiplier
pub fn get_num_worker_multiplier() -> usize {
    todo!()
}

/// Go: `config.SetEnvVars(oldVars, keyValues...)`.
pub fn set_env_vars(old: &mut Vec<String>, kv: &[(&str, &str)]) {
    todo!()
}

/// Go: `config.SplitEnvVar("K=V")`.
pub fn split_env_var(v: &str) -> (String, String) {
    match v.split_once('=') {
        Some((k, v)) => (k.to_string(), v.to_string()),
        None => (v.to_string(), String::new()),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/env.go (93 lines; 5/6 funcs executed)
// EX L33-40: GetNumWorkerMultiplier() int
// EX L47-63: GetMemoryLimit() uint64
//    L65-70: stringToGibabyte(f string) uint64
// EX L73-77: SetEnvVars(oldVars *[]string, keyValues ...string)
// EX L79-82: SplitEnvVar(v string) (string, string)
// EX L84-93: setEnvVar(vars *[]string, key, value string)
// ---------------------------------------------------------------------------
