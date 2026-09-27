//! `nh-hugolib`: neohugo hugolib/** : HugoSites/Site/pageState, content capture, content map + assembly, content rendering + shortcodes, render loop, aliases, postProcess, build stats.
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! generated GO PORTING CHECKLIST at the bottom of each module lists the Go functions to port.

// Skeleton-time allowances. Remove `unused`/`dead_code` once the crate is ported (README rule 11).
#![allow(unused, dead_code, clippy::too_many_arguments, clippy::new_without_default, clippy::type_complexity)]
// Module names mirror Go file names such as `page__meta.go`.
#![allow(non_snake_case)]

pub mod hugo_sites;
pub mod hugo_sites_data;
pub mod hugo_sites_build;
pub mod site;
pub mod site_output;
pub mod site_render;
pub mod site_sections;
pub mod alias;
pub mod build_process;
pub mod build_assemble;
pub mod template_exec;
pub mod pages_capture;
pub mod content_map;
pub mod content_map_trees;
pub mod content_map_page;
pub mod pagecollections;
pub mod collections;
pub mod page;
pub mod page__common;
pub mod page__content;
pub mod page__content_parse;
pub mod page__data;
pub mod page__init;
pub mod page__menus;
pub mod page__meta;
pub mod page__meta_post;
pub mod page__new;
pub mod page__output;
pub mod page__paginator;
pub mod page__paths;
pub mod page__per_output;
pub mod page__position;
pub mod page__ref;
pub mod page__tree;
pub mod page_kinds;
pub mod page_unwrap;
pub mod shortcode;
pub mod shortcode_parse;
pub mod shortcode_page;
pub mod file_info;
pub mod gitinfo;
pub mod codeowners;
pub mod permalinker;
pub mod tplapi;

pub use hugo_sites::HugoSites;
pub use page::{PageHandle, PageId, PageState};
pub use site::Site;
