//! `nh-hugofs`: neohugo hugofs/*, hugolib/filesystems (BaseFs), hugolib/paths, modules (project module mounts), plus afero / bep/overlayfs / spf13/fsync semantics.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(
    unused,
    dead_code,
    clippy::too_many_arguments,
    clippy::new_without_default,
    clippy::type_complexity
)]

pub mod afero;
pub mod component_fs;
pub mod decorators;
pub mod dirsmerger;
pub mod fileinfo;
pub mod filename_filter_fs;
pub mod filesystems;
pub mod fs;
pub mod glob;
pub mod hasbytes_fs;
pub mod modules;
pub mod nfc;
pub mod oserror;
pub mod overlayfs;
pub mod paths;
pub mod rootmapping_fs;
pub mod walk;
