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
//! copy outside it, then insert — unless an equal entry was stored meanwhile, which then wins
//! (Go computes under its write lock after a double check, so the first computation is the one
//! every caller sees; with sequential rendering both are the same).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::page::Pages;
use crate::pages_sort::page_ref_eq;

/// Go: `pageCacheEntry`.
#[derive(Clone)]
pub struct PageCacheEntry {
    pub input: Vec<Pages>,
    pub out: Pages,
}

impl PageCacheEntry {
    /// Go: `entry.matches(pageLists)`.
    // Go: resources/page/pages_cache.go:matches
    pub fn matches(&self, page_lists: &[&Pages]) -> bool {
        if self.input.len() != page_lists.len() {
            return false;
        }
        for (i, p) in page_lists.iter().enumerate() {
            if !pages_equal(p, &self.input[i]) {
                return false;
            }
        }

        true
    }
}

/// Go: `pageCache`.
#[derive(Default)]
pub struct PageCache {
    pub m: Mutex<HashMap<String, Vec<PageCacheEntry>>>,
}

/// Go: the package-level `spc = newPageCache()`.
// Go: resources/page/pages_cache.go:newPageCache
pub fn spc() -> &'static PageCache {
    static SPC: OnceLock<PageCache> = OnceLock::new();
    SPC.get_or_init(PageCache::default)
}

/// Go: `page.Clear()` -> `spc.clear()`.
// Go: resources/page/pages_cache.go:clear
pub fn clear() {
    spc().clear();
}

impl PageCache {
    /// Go: `c.clear()`.
    // Go: resources/page/pages_cache.go:clear
    pub fn clear(&self) {
        self.lock().clear();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Vec<PageCacheEntry>>> {
        self.m.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn lookup(&self, key: &str, page_lists: &[&Pages]) -> Option<Pages> {
        let m = self.lock();
        if let Some(cached) = m.get(key) {
            for entry in cached {
                if entry.matches(page_lists) {
                    return Some(entry.out.clone());
                }
            }
        }
        None
    }

    /// Go: `get(key, apply, pageLists...)` — a cached sorted copy of `page_lists[0]`, or a new
    /// copy with `apply` applied (and stored). The bool is Go's "found in cache".
    // Go: resources/page/pages_cache.go:get
    pub fn get(
        &self,
        key: &str,
        apply: &dyn Fn(&mut Pages),
        page_lists: &[&Pages],
    ) -> (Pages, bool) {
        self.get_p(key, apply, page_lists)
    }

    /// Go: `getP(key, apply, pageLists...)` (the apply func may grow the copy).
    // Go: resources/page/pages_cache.go:getP
    pub fn get_p(
        &self,
        key: &str,
        apply: &dyn Fn(&mut Pages),
        page_lists: &[&Pages],
    ) -> (Pages, bool) {
        if let Some(out) = self.lookup(key, page_lists) {
            return (out, true);
        }

        // Computed without the lock (HUGO_LAYER.md §4.8).
        let p = page_lists[0];
        let mut pages_copy = p.clone();
        apply(&mut pages_copy);

        let mut m = self.lock();
        // double-check
        if let Some(cached) = m.get(key) {
            for entry in cached {
                if entry.matches(page_lists) {
                    return (entry.out.clone(), true);
                }
            }
        }

        let entry = PageCacheEntry {
            input: page_lists.iter().map(|p| (*p).clone()).collect(),
            out: pages_copy.clone(),
        };
        m.entry(key.to_string()).or_default().push(entry);

        (pages_copy, false)
    }
}

/// Go: `pagesEqual(p1, p2)` — same length and the same page values element by element. Go
/// distinguishes a nil list from an empty one here; Rust lists have no nil.
// Go: resources/page/pages_cache.go:pagesEqual
pub fn pages_equal(p1: &Pages, p2: &Pages) -> bool {
    if p1.len() != p2.len() {
        return false;
    }

    if p1.is_empty() {
        return true;
    }

    for i in 0..p1.len() {
        if !page_ref_eq(&p1[i], &p2[i]) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_cache.go (136 lines; 6/6 funcs executed)
//   types: pageCacheEntry, pageCache
// OK L26-37: (entry pageCacheEntry) matches(pageLists []Pages) bool
// OK L44-46: newPageCache() *pageCache
// OK L48-52: (c *pageCache) clear()
// OK L63-69: (c *pageCache) get(key string, apply func(p Pages), pageLists ...Pages) (Pages, bool)
// OK L71-110: (c *pageCache) getP(key string, apply func(p *Pages), pageLists ...Pages) (Pages, bool)
// OK L113-136: pagesEqual(p1, p2 Pages) bool
// ---------------------------------------------------------------------------
