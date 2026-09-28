//! `nh-common`: neohugo common/* + compare + resources/kinds + hugofs/files + hugofs/glob + identity(stub) + cache/dynacache, plus ports of spf13/cast, gobuffalo/flect, jdkato/prose/transform, gohugoio/locales (en, th).
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port. The module map
//! and the deviations are in `PORTING.md`.
//!
//! [`object`] is the template-API foundation of every nh-* crate: `go_methods!`,
//! `object_basics!`, the `args` helpers and the `NamedTypeRegistry`.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(
    unused,
    dead_code,
    clippy::too_many_arguments,
    clippy::new_without_default,
    clippy::type_complexity
)]

pub mod cast;
pub mod collections;
pub mod compare;
pub mod constants;
pub mod dynacache;
pub mod files;
pub mod flect;
pub mod glob;
pub mod hashing;
pub mod herrors;
pub mod hreflect;
pub mod hstrings;
pub mod htime;
pub mod hugio;
pub mod identity;
pub mod kinds;
pub mod locales;
pub mod loggers;
pub mod maps;
pub mod math;
pub mod object;
pub mod paths;
pub mod predicate;
pub mod prose;
pub mod text;
pub mod types;
pub mod urls;

// Re-exports used by every Hugo-layer crate.
pub use go_value::{
    FloatKind, GoString, HostCtx, IntKind, Kind, List, Location, Map, MapType, Object, SafeKind,
    SliceType, Time, UintKind, Value,
};
pub use herrors::{Error, Result};
