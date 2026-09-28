//! `nh-langs`: neohugo langs/{language,config}.go: Language, Languages, collators (x/text collate CLDR 23), locale translators.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

pub mod config;
pub mod language;
