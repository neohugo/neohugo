//! Dev-only test support for the neohugo workspace (docs/rust-port/REWRITE_PLAN.md §2.1, §7.1).
//!
//! - [`fixture`]: reads the plain-JSON Go-oracle fixtures under `rust/testdata/` (`.json`,
//!   `.jsonl`, either optionally gzipped) and decodes the `$nh:` tagged leaves of the neohugo
//!   schema (see `tools/neohugo/fixtures2json.py`).
//! - [`txtar`]: Go's txtar archive format, used for small test sites.
//! - [`snapshot`]: the shared insta settings.
//! - [`contract`]: the template contract (REWRITE_PLAN.md §4.8): converted templates load against
//!   `neohugo_funcs::spec::FUNCS` and call only declared kwargs.

#![forbid(unsafe_code)]

pub mod contract;
pub mod fixture;
pub mod snapshot;
pub mod txtar;
