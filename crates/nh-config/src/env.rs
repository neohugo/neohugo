//! Port of `config/env.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

const GIGABYTE: u64 = 1 << 30;

/// Go: `config.GetNumWorkerMultiplier()` (`HUGO_NUMWORKERMULTIPLIER`, default NumCPU). The Rust
/// port renders sequentially (see HUGO_LAYER.md §8); this only sizes optional worker pools.
// Go: config/env.go:GetNumWorkerMultiplier
pub fn get_num_worker_multiplier() -> usize {
    let env = std::env::var("HUGO_NUMWORKERMULTIPLIER").ok();
    get_num_worker_multiplier_from(env.as_deref(), num_cpu()) as usize
}

/// [`get_num_worker_multiplier`] with the environment value and the CPU count given: the value
/// of `HUGO_NUMWORKERMULTIPLIER` if it is a positive integer, else `num_cpu`.
pub fn get_num_worker_multiplier_from(env: Option<&str>, num_cpu: i64) -> i64 {
    if let Some(gmp) = env
        && !gmp.is_empty()
        && let Ok(p) = go_strconv::atoi(gmp)
        && p > 0
    {
        return p;
    }
    num_cpu
}

/// Go: `runtime.NumCPU()`.
fn num_cpu() -> i64 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(1)
}

/// GetMemoryLimit returns the upper memory limit in bytes for Hugo's in-memory caches: the
/// value of `HUGO_MEMORYLIMIT` (in gigabytes), else a quarter of the total system memory.
// Go: config/env.go:GetMemoryLimit
pub fn get_memory_limit() -> u64 {
    let env = std::env::var("HUGO_MEMORYLIMIT").ok();
    get_memory_limit_from(env.as_deref(), total_memory())
}

/// [`get_memory_limit`] with the environment value and the total memory given.
pub fn get_memory_limit_from(env: Option<&str>, total_memory: u64) -> u64 {
    if let Some(mem) = env
        && !mem.is_empty()
    {
        let v = string_to_gibabyte(mem);
        if v > 0 {
            return v;
        }
    }

    // There is a FreeMemory function, but as the kernel in most situations will take whatever
    // memory that is left and use for caching etc., that value is not something that we can
    // use.
    if total_memory != 0 {
        return total_memory / 4;
    }

    2 * GIGABYTE
}

/// Go: `memory.TotalMemory()` (github.com/pbnjay/memory: `sysinfo` on Linux). Read from
/// `/proc/meminfo` here; 0 when unknown.
fn total_memory() -> u64 {
    let Ok(s) = std::fs::read_to_string("/proc/meminfo") else {
        return 0;
    };
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb = rest.trim().trim_end_matches("kB").trim();
            return kb.parse::<u64>().map(|k| k * 1024).unwrap_or(0);
        }
    }
    0
}

// Go: config/env.go:stringToGibabyte
fn string_to_gibabyte(f: &str) -> u64 {
    match go_strconv::parse_float(f, 32) {
        // Go: uint64(v * gigabyte) (saturating on arm64, like Rust's `as`).
        Ok(v) if v > 0.0 => (v * GIGABYTE as f64) as u64,
        _ => 0,
    }
}

/// Go: `config.SetEnvVars(oldVars, keyValues...)`: sets vars on the form key=value in the
/// oldVars slice.
// Go: config/env.go:SetEnvVars
pub fn set_env_vars(old: &mut Vec<String>, kv: &[(&str, &str)]) {
    for (k, v) in kv {
        set_env_var(old, k, v);
    }
}

/// Go: `config.SplitEnvVar("K=V")`.
// Go: config/env.go:SplitEnvVar
pub fn split_env_var(v: &str) -> (String, String) {
    match v.split_once('=') {
        Some((k, v)) => (k.to_string(), v.to_string()),
        None => (v.to_string(), String::new()),
    }
}

// Go: config/env.go:setEnvVar
fn set_env_var(vars: &mut Vec<String>, key: &str, value: &str) {
    let prefix = format!("{key}=");
    for v in vars.iter_mut() {
        if v.starts_with(&prefix) {
            *v = format!("{key}={value}");
            return;
        }
    }
    // New var.
    vars.push(format!("{key}={value}"));
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/env.go (93 lines; 5/6 funcs executed)
// OK L33-40: GetNumWorkerMultiplier() int
// OK L47-63: GetMemoryLimit() uint64
// OK L65-70: stringToGibabyte(f string) uint64
// OK L73-77: SetEnvVars(oldVars *[]string, keyValues ...string)
// OK L79-82: SplitEnvVar(v string) (string, string)
// OK L84-93: setEnvVar(vars *[]string, key, value string)
// ---------------------------------------------------------------------------
