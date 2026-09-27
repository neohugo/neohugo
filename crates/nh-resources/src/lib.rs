//! `nh-resources`: neohugo resources/*.go (Spec, genericResource, resourceAdapter + transformation chain, caches, image resource), resources/postpub, resources/jsconfig.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(unused, dead_code, clippy::too_many_arguments, clippy::new_without_default, clippy::type_complexity)]

pub mod resource;
pub mod resource_spec;
pub mod transform;
pub mod resource_cache;
pub mod resource_metadata;
pub mod image;
pub mod image_cache;
pub mod post_publish;
pub mod postpub;
pub mod jsconfig;
