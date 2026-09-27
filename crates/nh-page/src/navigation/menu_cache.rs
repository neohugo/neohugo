//! Port of `navigation/menu_cache.go`.
//!
//! Owner: Wave B task T12 (page-collections).


// Go `navigation.menuCache` — memoizes sorted menus; recompute in Rust (same result).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: navigation/menu_cache.go (102 lines; 1/5 funcs executed)
//   types: menuCacheEntry, menuCache
//    L26-37: (entry menuCacheEntry) matches(menuList []Menu) bool
// EX L40-42: newMenuCache() *menuCache
//    L50-62: menuEqual(m1, m2 Menu) bool
//    L66-72: (c *menuCache) get(key string, apply func(m Menu), menuLists ...Menu) (Menu, bool)
//    L75-102: (c *menuCache) getP(key string, apply func(m *Menu), menuLists ...Menu) (Menu, bool)
// ---------------------------------------------------------------------------
