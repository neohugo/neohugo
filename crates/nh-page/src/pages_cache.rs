//! Port of `resources/page/pages_cache.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Go `pageCache` (the package-global `spc`): memoizes sorted copies keyed by name + the identity
//! of the input lists (`pagesEqual`). PORT IT FAITHFULLY: a hit returns the list sorted when it
//! was first computed, and several comparators depend on mutable state (the collator of
//! `Site.Current()`), so "just recompute" is not equivalent in general.
//!
//! `clear()` is Go `page.Clear()` (resources/page/page.go:39-42); nh-hugolib calls it at the
//! start of EVERY `Site.render` (per site and output format, site.go:1580-1583; T24's
//! `site_render::render_site`).
//!
//! Never sort while holding the lock (HUGO_LAYER.md §4.8): look up under the lock, compute the
//! copy outside it, then insert.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::page::Pages;

/// Go: `pageCacheEntry`.
#[derive(Clone)]
pub struct PageCacheEntry {
    pub input: Vec<Pages>,
    pub out: Pages,
}

/// Go: `pageCache`.
#[derive(Default)]
pub struct PageCache {
    pub m: Mutex<HashMap<String, Vec<PageCacheEntry>>>,
}

/// Go: the package-level `spc = newPageCache()`.
pub fn spc() -> &'static PageCache {
    static SPC: OnceLock<PageCache> = OnceLock::new();
    SPC.get_or_init(PageCache::default)
}

/// Go: `page.Clear()` -> `spc.clear()`.
// Go: resources/page/pages_cache.go:clear
pub fn clear() {
    spc().m.lock().unwrap().clear();
}

impl PageCache {
    /// Go: `get(key, apply, pageLists...)` — a cached sorted copy of `page_lists[0]`, or a new
    /// copy with `apply` applied (and stored).
    // Go: resources/page/pages_cache.go:get
    pub fn get(
        &self,
        key: &str,
        apply: &dyn Fn(&mut Pages),
        page_lists: &[&Pages],
    ) -> (Pages, bool) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_cache.go (136 lines; 6/6 funcs executed)
//   types: pageCacheEntry, pageCache
// EX L26-37: (entry pageCacheEntry) matches(pageLists []Pages) bool
// EX L44-46: newPageCache() *pageCache
// EX L48-52: (c *pageCache) clear()
// EX L63-69: (c *pageCache) get(key string, apply func(p Pages), pageLists ...Pages) (Pages, bool)
// EX L71-110: (c *pageCache) getP(key string, apply func(p *Pages), pageLists ...Pages) (Pages, bool)
// EX L113-136: pagesEqual(p1, p2 Pages) bool
// ---------------------------------------------------------------------------
