//! `nh-common`: neohugo common/* + compare + resources/kinds + hugofs/files + hugofs/glob + identity(stub) + cache/dynacache, plus ports of spf13/cast, gobuffalo/flect, jdkato/prose/transform, gohugoio/locales (en, th).
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(unused, dead_code, clippy::too_many_arguments, clippy::new_without_default, clippy::type_complexity)]

pub mod object;
pub mod herrors;
pub mod constants;
pub mod collections;
pub mod hashing;
pub mod hreflect;
pub mod htime;
pub mod maps;
pub mod math;
pub mod predicate;
pub mod types;
pub mod compare;
pub mod identity;
pub mod dynacache;
pub mod cast;
pub mod locales;
pub mod paths;
pub mod urls;
pub mod text;
pub mod hstrings;
pub mod hugio;
pub mod loggers;
pub mod kinds;
pub mod files;
pub mod glob;
pub mod flect;
pub mod prose;

// Re-exports used by every Hugo-layer crate.
pub use go_value::{
    FloatKind, GoString, HostCtx, IntKind, Kind, List, Location, Map, MapType, Object, SafeKind, SliceType, Time,
    UintKind, Value,
};
pub use herrors::{Error, Result};
