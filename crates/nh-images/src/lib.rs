//! `nh-images`: neohugo resources/images/** (image config, spec parsing, processing, filters, encoders, exif stub).
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(unused, dead_code, clippy::too_many_arguments, clippy::new_without_default, clippy::type_complexity)]

pub mod config;
pub mod image;
pub mod filters;
pub mod overlay;
pub mod process;
pub mod resampling;
pub mod color;
pub mod image_resource;
pub mod auto_orient;
pub mod dither;
pub mod mask;
pub mod opacity;
pub mod padding;
pub mod smartcrop;
pub mod text;
pub mod exif;
pub mod webp;
