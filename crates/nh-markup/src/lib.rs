//! `nh-markup`: neohugo markup/**: converter + hooks API, goldmark glue (render hooks, autoid, attributes, tables, blockquotes, hugocontext, images, TOC), highlight config (+ stub highlighter), markup_config; asciidoc/pandoc/rst/org stubs.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Lints that fight a faithful port (README rule 11): Go's argument lists and constructors, and
// the skeleton's module layout (`converter::converter`, `highlight::highlight` mirror the Go
// packages `markup/converter` and `markup/highlight`).
#![allow(
    clippy::too_many_arguments,
    clippy::new_without_default,
    clippy::type_complexity,
    clippy::module_inception
)]

pub mod asciidocext;
pub mod blackfriday;
pub mod converter;
pub mod goldmark;
pub mod highlight;
pub mod internal;
pub mod markup;
pub mod markup_config;
pub mod org;
pub mod pandoc;
pub mod rst;
pub mod tableofcontents;
