//! Port of `navigation/menu_cache.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Go `navigation.menuCache` (the package-global `smc`): memoizes sorted copies of menus keyed by
//! name + the identity of the input menus (`menuEqual`: the same entry pointers). A hit returns
//! the copy made at first computation. Go never clears it. The apply function runs outside the
//! lock (HUGO_LAYER.md §4.8); the first stored entry wins.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use super::menu::Menu;

/// Go: `menuCacheEntry`.
#[derive(Clone)]
struct MenuCacheEntry {
    input: Vec<Menu>,
    out: Menu,
}

impl MenuCacheEntry {
    /// Go: `entry.matches(menuList)`.
    // Go: navigation/menu_cache.go:matches
    fn matches(&self, menu_list: &[&Menu]) -> bool {
        if self.input.len() != menu_list.len() {
            return false;
        }
        for (i, m) in menu_list.iter().enumerate() {
            if !menu_equal(m, &self.input[i]) {
                return false;
            }
        }

        true
    }
}

/// Go: `menuCache`.
#[derive(Default)]
pub struct MenuCache {
    m: Mutex<HashMap<String, Vec<MenuCacheEntry>>>,
}

/// Go: `newMenuCache()` — the package-level `smc`.
// Go: navigation/menu_cache.go:newMenuCache
pub fn smc() -> &'static MenuCache {
    static SMC: OnceLock<MenuCache> = OnceLock::new();
    SMC.get_or_init(MenuCache::default)
}

/// Go: `menuEqual(m1, m2)` — same length and the same entry pointers.
// Go: navigation/menu_cache.go:menuEqual
pub fn menu_equal(m1: &Menu, m2: &Menu) -> bool {
    if m1.len() != m2.len() {
        return false;
    }

    for i in 0..m1.len() {
        if !Arc::ptr_eq(&m1[i], &m2[i]) {
            return false;
        }
    }

    true
}

impl MenuCache {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Vec<MenuCacheEntry>>> {
        self.m.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn lookup(&self, key: &str, menu_lists: &[&Menu]) -> Option<Menu> {
        let m = self.lock();
        let cached = m.get(key)?;
        cached
            .iter()
            .find(|e| e.matches(menu_lists))
            .map(|e| e.out.clone())
    }

    /// Go: `c.get(key, apply, menuLists...)`.
    // Go: navigation/menu_cache.go:get
    pub fn get(&self, key: &str, apply: &dyn Fn(&mut Menu), menu_lists: &[&Menu]) -> (Menu, bool) {
        self.get_p(key, apply, menu_lists)
    }

    /// Go: `c.getP(key, apply, menuLists...)`.
    // Go: navigation/menu_cache.go:getP
    pub fn get_p(
        &self,
        key: &str,
        apply: &dyn Fn(&mut Menu),
        menu_lists: &[&Menu],
    ) -> (Menu, bool) {
        if let Some(out) = self.lookup(key, menu_lists) {
            return (out, true);
        }

        let m = menu_lists[0];
        let mut menu_copy = m.clone();
        apply(&mut menu_copy);

        let mut c = self.lock();
        if let Some(cached) = c.get(key)
            && let Some(e) = cached.iter().find(|e| e.matches(menu_lists))
        {
            return (e.out.clone(), true);
        }

        let entry = MenuCacheEntry {
            input: menu_lists.iter().map(|m| (*m).clone()).collect(),
            out: menu_copy.clone(),
        };
        c.entry(key.to_string()).or_default().push(entry);

        (menu_copy, false)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: navigation/menu_cache.go (102 lines; 1/5 funcs executed)
//   types: menuCacheEntry, menuCache
// OK L26-37: (entry menuCacheEntry) matches(menuList []Menu) bool
// OK L40-42: newMenuCache() *menuCache
// OK L50-62: menuEqual(m1, m2 Menu) bool
// OK L66-72: (c *menuCache) get(key string, apply func(m Menu), menuLists ...Menu) (Menu, bool)
// OK L75-102: (c *menuCache) getP(key string, apply func(m *Menu), menuLists ...Menu) (Menu, bool)
// ---------------------------------------------------------------------------
