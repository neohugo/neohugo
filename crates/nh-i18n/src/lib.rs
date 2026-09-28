//! `nh-i18n`: neohugo langs/i18n + gohugoio/go-i18n/v2 fork (bundle, localizer, message templates, CLDR plural rules).
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.
//!
//! - [`i18n`] and [`translation_provider`]: neohugo `langs/i18n`.
//! - [`goi18n`]: the port of `github.com/gohugoio/go-i18n/v2@v2.1.3-0.20230805085216-e63c13218d0e`
//!   (`i18n/{bundle,localizer,message,message_template,parse}.go`, `internal/template.go`,
//!   `internal/plural`).
//! - [`xlanguage`]: the port of the `golang.org/x/text@v0.26.0/language` matcher and likely-subtags code that
//!   go-i18n uses and `xtext-collate` does not export.

pub mod goi18n;
pub mod i18n;
pub mod translation_provider;
pub mod xlanguage;
