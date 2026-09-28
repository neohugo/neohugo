//! `nh-tplfuncs`: neohugo tpl/<namespace>/** template functions (all namespaces; seeksnack's set fully, the rest as stubs), registry, tplimplinit.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

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
pub mod crypto;
pub mod css;
pub mod data;
pub mod debug;
pub mod diagrams;
pub mod encoding;
pub mod fmt;
pub mod hash;
pub mod hugo;
pub mod images;
pub mod inflect;
pub mod internal;
pub mod js;
pub mod lang;
pub mod math;
pub mod openapi3;
pub mod os;
pub mod page;
pub mod partials;
pub mod path;
pub mod reflect;
pub mod resources;
pub mod safe;
pub mod site;
pub mod strings;
pub mod templates;
pub mod time;
pub mod tplimplinit;
pub mod transform;
pub mod urls;
