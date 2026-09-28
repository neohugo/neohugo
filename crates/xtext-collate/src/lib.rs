//! Port of `golang.org/x/text@v0.26.0/collate` and `internal/colltab`
//! (CLDR 23 / UCA 6.2.0 tables), with the subsets of `unicode/norm`
//! (Unicode 15.0.0 tables) and `language`/`internal/language` (CLDR 32
//! tables) that collation uses, for byte-parity with neohugo's Go build.
//!
//! Typical use (what neohugo's `langs.NewLanguage` does):
//!
//! ```
//! use xtext_collate::Collator;
//! let mut c = Collator::for_hugo_language("th");
//! assert_eq!(c.compare_string("a", "b"), -1);
//! let mut und = Collator::new("und");
//! assert_eq!(c.compare_string("เนย", "ปาร์ตี้"), -1);
//! assert_eq!(und.compare_string("a", "A"), -1);
//! ```
//!
//! See `PORTING.md` for the Go file map, deviations and gaps.

// Faithful-port lints: keep Go's structure, names and comparisons (Go has no
// range-contains / is_multiple_of / late-init idioms; mirroring the Go
// expressions keeps the Rust diffable against Go).
#![allow(
    // Go conditions kept verbatim (`!(base.String() <= ...)` in sort.Search).
    clippy::nonminimal_bool,
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::module_inception,
    clippy::manual_range_contains,
    clippy::manual_is_multiple_of,
    clippy::needless_late_init,
    clippy::collapsible_match,
    clippy::manual_is_ascii_check,
    clippy::wrong_self_convention
)]

pub(crate) mod blob;
pub mod collate;
pub mod colltab;
pub mod goutf8;
pub mod language;
pub mod norm;

pub use collate::option::{
    CollOption, FORCE, IGNORE_CASE, IGNORE_DIACRITICS, IGNORE_WIDTH, LOOSE, NUMERIC,
    options_from_tag,
};
pub use collate::{Buffer, Collator, Lister, cldr_version, supported, unicode_version};
