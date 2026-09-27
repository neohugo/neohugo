//! `nh-markup`: neohugo markup/**: converter + hooks API, goldmark glue (render hooks, autoid, attributes, tables, blockquotes, hugocontext, images, TOC), highlight config (+ stub highlighter), markup_config; asciidoc/pandoc/rst/org stubs.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(unused, dead_code, clippy::too_many_arguments, clippy::new_without_default, clippy::type_complexity)]

pub mod markup;
pub mod converter;
pub mod markup_config;
pub mod tableofcontents;
pub mod highlight;
pub mod internal;
pub mod goldmark;
pub mod asciidocext;
pub mod pandoc;
pub mod rst;
pub mod org;
pub mod blackfriday;
