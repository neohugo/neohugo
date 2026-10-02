//! Environment overrides, `<PREFIX>_*` ([`PREFIX`], `ssg_base::ENV_PREFIX`; `FUGO`):
//! `FUGO_TITLE=x` sets `title`, `FUGO_PARAMS_API_KEY=k` sets `params.api.key`. The character
//! after the prefix is the key delimiter, so `FUGOxPARAMSxAPI_KEY=k` sets `params.api_key`.
//! Hugo's `HUGO_*` variables are not read, and the program's own settings ([`RESERVED`]) are not
//! overrides.
//!
//! Each value is parsed into the type of the value it overrides: a boolean, a number, a list
//! (`["a", "b"]`) or a table (JSON or TOML); when it does not parse as that type it stays a
//! string. New keys are strings, or a table or list when the value is written as one.

use std::sync::Arc;

use ssg_base::{Map, Value};

use crate::tree;

/// The prefix of the override variables (and of every variable the program reads).
pub const PREFIX: &str = ssg_base::ENV_PREFIX;

/// The variable that chooses the build environment when `--environment` is not given.
pub const ENVIRONMENT: &str = ssg_base::env_var!("ENVIRONMENT");

/// The program's own settings and the build and test harness's variables: not configuration
/// overrides, although they carry the prefix.
pub const RESERVED: &[&str] = &[
    ssg_base::env_var!("BABEL_BIN"),
    ssg_base::env_var!("BINARY"),
    ssg_base::env_var!("BUILD_COMMIT"),
    ssg_base::env_var!("BUILD_DATE"),
    ssg_base::env_var!("COMPARE_WORK"),
    ssg_base::env_var!("NODE_MODULES"),
    ssg_base::env_var!("POSTCSS_BIN"),
    ssg_base::env_var!("REPO_DIR"),
    ssg_base::env_var!("SITES"),
    ssg_base::env_var!("STRUCTURE_OUT"),
    ssg_base::env_var!("TAILWINDCSS_BIN"),
    ssg_base::env_var!("TARGET_LIMIT_MB"),
    ssg_base::env_var!("TASK"),
    ssg_base::env_var!("TIMINGS"),
    ssg_base::env_var!("VENDOR_INFO"),
];

/// Applies every `<PREFIX>*` override variable of `env` to `root` (keys lower-case).
pub fn apply(root: &mut Map, env: &[(String, String)]) {
    let mut vars: Vec<(Vec<String>, &str)> = env
        .iter()
        .filter_map(|(k, v)| Some((key_path(k)?, v.as_str())))
        .collect();
    // Shorter paths first, so `FUGO_PARAMS` is merged before `FUGO_PARAMS_X` refines it.
    vars.sort_by(|a, b| a.0.len().cmp(&b.0.len()).then_with(|| a.0.cmp(&b.0)));
    for (path, raw) in vars {
        let segs: Vec<&str> = path.iter().map(String::as_str).collect();
        let existing = tree::get_path(root, &path.join("."));
        let value = typed(existing, raw);
        match (existing, &value) {
            (Some(Value::Map(_)), Value::Map(over)) => {
                let over = tree::normalize_keys(over);
                if let Some(Value::Map(target)) = get_path_mut(root, &segs) {
                    tree::merge_deep(Arc::make_mut(target), &over);
                }
            }
            _ => tree::set_segments(root, &segs, value),
        }
    }
}

/// The lower-case key path of an override variable, or `None` when `name` is not one.
#[must_use]
pub fn key_path(name: &str) -> Option<Vec<String>> {
    if RESERVED.contains(&name) {
        return None;
    }
    let rest = name.strip_prefix(PREFIX)?;
    let mut chars = rest.chars();
    let delim = chars.next()?;
    let key = chars.as_str();
    if key.is_empty() {
        return None;
    }
    let segs: Vec<String> = key.split(delim).map(str::to_lowercase).collect();
    if segs.iter().any(String::is_empty) {
        return None;
    }
    Some(segs)
}

fn get_path_mut<'a>(m: &'a mut Map, segs: &[&str]) -> Option<&'a mut Value> {
    let (first, rest) = segs.split_first()?;
    let v = m.get_mut(first)?;
    if rest.is_empty() {
        return Some(v);
    }
    match v {
        Value::Map(child) => get_path_mut(Arc::make_mut(child), rest),
        _ => None,
    }
}

/// `raw` as a value of the type of `existing`.
fn typed(existing: Option<&Value>, raw: &str) -> Value {
    let fallback = || Value::string(raw);
    match existing {
        Some(Value::Bool(_)) => {
            crate::de::weak_bool(&Value::string(raw)).map_or_else(fallback, Value::Bool)
        }
        Some(Value::Int(_)) => raw
            .trim()
            .parse::<i64>()
            .map_or_else(|_| fallback(), Value::Int),
        Some(Value::Float(_)) => raw
            .trim()
            .parse::<f64>()
            .map_or_else(|_| fallback(), Value::Float),
        Some(Value::Map(_)) => parse_table(raw).unwrap_or_else(fallback),
        Some(Value::Array(_)) => parse_list(raw).unwrap_or_else(fallback),
        _ => {
            let t = raw.trim_start();
            if t.starts_with('{') || t.contains('=') {
                parse_table(raw).unwrap_or_else(fallback)
            } else if t.starts_with('[') {
                parse_list(raw).unwrap_or_else(fallback)
            } else {
                fallback()
            }
        }
    }
}

/// A table written as JSON or TOML.
fn parse_table(raw: &str) -> Option<Value> {
    if let Ok(v @ Value::Map(_)) = Value::from_json_str(raw) {
        return Some(v);
    }
    match Value::from_toml_str(raw) {
        Ok(Value::Map(m)) if !m.is_empty() => Some(Value::Map(m)),
        _ => None,
    }
}

/// A list written as JSON or as a TOML array (`['a', "b"]`).
fn parse_list(raw: &str) -> Option<Value> {
    if let Ok(v @ Value::Array(_)) = Value::from_json_str(raw) {
        return Some(v);
    }
    let doc = format!("v = {raw}");
    let v = Value::from_toml_str(&doc).ok()?;
    match v.as_map()?.get("v")? {
        a @ Value::Array(_) => Some(a.clone()),
        _ => None,
    }
}
