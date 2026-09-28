//! `nh-parser`: neohugo parser/* : metadecoders (YAML via go-yaml, TOML via a port of pelletier/go-toml/v2, JSON via go-json), pageparser lexer.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

pub mod frontmatter;
pub mod metadecoders;
pub mod pageparser;
