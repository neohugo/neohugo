//! Module `funcmap`.
//!
//! NEW: the template func map factory the `neohugo-rs` binary hands to `NewHugoSites`.
//!
//! Owner: Wave B task T25 (commands-cli); switched to production by I01.
//!
//! Production uses Go's `tplimplinit.CreateFuncMap` (`nh_tplfuncs::tplimplinit::
//! create_func_map`, T19), which hugolib calls when `NewHugoSitesCfg.func_map_factory` is `None`.
//! The interim factory T25 used while T19 was in progress is gone (I01). Tests and tools that
//! need another func map pass their own factory through `ExecOptions.func_map_factory`.

use nh_hugolib::hugo_sites::FuncMapFactory;

/// The func map factory of the `neohugo-rs` binary: `None` = hugolib's default,
/// `tplimplinit::create_func_map` (Go `CreateFuncMap`).
pub fn production_func_map_factory() -> Option<FuncMapFactory> {
    None
}
