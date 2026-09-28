//! Shared helpers for the T02 oracle tests (paths, urls, glob, textmisc): fixture strings,
//! Go-panic expectations and quiet `catch_unwind`.

#![allow(dead_code)]

use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Once;

use serde_json::Value as J;

thread_local! {
    static QUIET: Cell<bool> = const { Cell::new(false) };
}

/// Installs (once) a panic hook that stays silent while [`catch`] runs on this thread.
fn install_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let prev = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if !QUIET.with(|q| q.get()) {
                prev(info);
            }
        }));
    });
}

/// Runs `f`, returning `Err(message)` if it panics (without printing the panic).
pub fn catch<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    install_hook();
    QUIET.with(|q| q.set(true));
    let r = panic::catch_unwind(AssertUnwindSafe(f));
    QUIET.with(|q| q.set(false));
    r.map_err(|e| {
        if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else {
            "panic".to_string()
        }
    })
}

/// Decodes a fixture hex string.
pub fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// A goval string (`"s"` or `{"hex": ...}`) as bytes.
pub fn bytes(v: &J) -> Vec<u8> {
    match v {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(o) => hex(o["hex"].as_str().unwrap()),
        _ => panic!("bad string {v}"),
    }
}

/// Bytes as a goval string (a JSON string when valid UTF-8).
pub fn enc(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => {
            let h: String = b.iter().map(|c| format!("{c:02x}")).collect();
            serde_json::json!({ "hex": h })
        }
    }
}

/// Whether a fixture value is a recorded Go panic.
pub fn is_panic(v: &J) -> bool {
    v.as_object().is_some_and(|o| o.contains_key("panic"))
}

/// Compares a Rust result with a fixture value: a Go panic must be a Rust panic (messages
/// differ), anything else must be equal.
pub fn same(want: &J, got: &Result<J, String>) -> bool {
    match got {
        Err(_) => is_panic(want),
        Ok(g) => !is_panic(want) && want == g,
    }
}

/// Reads `tests/fixtures/<rel>` (gunzipped when it ends in `.gz`).
pub fn fixture(rel: &str) -> J {
    use std::io::Read;
    let path = format!("{}/tests/fixtures/{rel}", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let text = if rel.ends_with(".gz") {
        let mut s = String::new();
        flate2::read::GzDecoder::new(&raw[..])
            .read_to_string(&mut s)
            .unwrap();
        s
    } else {
        String::from_utf8(raw).unwrap()
    };
    serde_json::from_str(&text).unwrap()
}
