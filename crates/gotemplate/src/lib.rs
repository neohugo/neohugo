//! Faithful port of Go's `text/template` and `html/template` as forked in
//! neohugo (`tpl/internal/go_templates`: go1.24.0 sources plus Hugo's
//! `hugo_template.go` hooks), over [`go_value::Value`].
//!
//! | Go package | Rust module |
//! |---|---|
//! | `texttemplate/parse` | [`parse`] |
//! | `texttemplate` | [`text`] |
//! | `htmltemplate` | [`html`] |
//!
//! The host contract with the Hugo layer (`crates/GOTEMPLATE_CONTRACT.md`)
//! is documented in `PORTING.md` under "Host contract".

// Lints that fight a faithful line-by-line port.
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::type_complexity)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::nonminimal_bool)]

pub mod parse;
pub mod text;

mod error;

pub use error::{Error, ExecError};
