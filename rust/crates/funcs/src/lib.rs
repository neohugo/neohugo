//! The template API: [`spec`] (the single source of truth: `spec::FUNCS`, the render contexts and
//! `spec::EMBEDDED_TEMPLATES`), [`register_placeholders`] for the contract instance, and (T31,
//! feature `runtime`) the pure Tera filters, functions and tests.
//!
//! `spec` has no dependencies and `register_placeholders` needs only tera, so both are available
//! with `default-features = false` (neohugo-testkit's contract test uses that; REWRITE_PLAN.md §4.8).

#![forbid(unsafe_code)]

mod placeholders;
pub mod spec;

pub use placeholders::{check_kwargs, register_placeholders};
