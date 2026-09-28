//! Shared helpers for the T04 nh-config oracle tests.

#![allow(dead_code, unused_imports)]

pub mod dump;
pub mod goval;

use serde_json::Value as J;

pub use goval::{decode, encode, fixture, str_enc};

/// A goval string as a Rust string (lossy).
pub fn j_string(j: &J) -> String {
    String::from_utf8_lossy(&goval::bytes(j)).into_owned()
}

/// Runs `f`, turning a panic into `Err(message)` (Go's recovered panic value).
pub fn catch<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    thread_local! {
        static IN_CATCH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    static HOOK: std::sync::Once = std::sync::Once::new();
    HOOK.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !IN_CATCH.with(|c| c.get()) || std::env::var_os("NH_SHOW_PANICS").is_some() {
                prev(info);
            }
        }));
    });
    IN_CATCH.with(|c| c.set(true));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    IN_CATCH.with(|c| c.set(false));
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
