//! `nh-page`: neohugo resources/page/** (Page/Site traits, Pages ops, paths, permalinks, pagination, taxonomies), pagemeta, navigation, related.
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

pub mod navigation;
pub mod page;
pub mod page_author;
pub mod page_data;
pub mod page_kinds;
pub mod page_lazy_contentprovider;
pub mod page_markup;
pub mod page_matcher;
pub mod page_nop;
pub mod page_outputformat;
pub mod page_paths;
pub mod pagegroup;
pub mod pagemeta;
pub mod pages;
pub mod pages_cache;
pub mod pages_language_merge;
pub mod pages_prev_next;
pub mod pages_related;
pub mod pages_sort;
pub mod pages_sort_search;
pub mod pagination;
pub mod permalinks;
pub mod related;
pub mod site;
pub mod taxonomy;
pub mod weighted;
