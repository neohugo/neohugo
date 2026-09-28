//! `nh-images`: neohugo resources/images/** (image config, spec parsing, processing, filters, encoders, exif stub).
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Lints that fight a faithful port (README rule 11); the skeleton's `unused`/`dead_code`
// allowances are gone now that the crate is ported.
#![allow(
    clippy::too_many_arguments,
    clippy::new_without_default,
    clippy::type_complexity
)]

pub mod auto_orient;
pub mod color;
pub mod config;
pub mod dither;
pub mod exif;
mod exif_fields;
pub mod filters;
mod gif;
pub mod image;
pub mod image_resource;
pub mod mask;
pub mod opacity;
pub mod overlay;
pub mod padding;
pub mod process;
pub mod resampling;
pub mod smartcrop;
pub mod text;
pub mod webp;
