//! Port of `hugolib/page__menus.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Go `pageMenus`: the page's own menu entries (front matter `menus`, else the `menus`/`menu`
//! params), created once (`pmInit`), and the menu queries, which first run the site's lazy
//! menu assembly (`s.init.menus`, `crate::site::site_menus`). In Go the site assembly mutates
//! the page's entries (children, page values); the port keeps the assembled entries of each page
//! it visits in `PageCommon::page_menus_assembled` and answers with those.

use go_value::Value;
use nh_page::navigation::menu::{MenuEntry, PageMenus};
use nh_page::navigation::pagemenus::{new_menu_query_provider, page_menus_from_page_for};

use crate::page::PageHandle;

/// Go: `(p *pageMenus) HasMenuCurrent(menuID, me)`.
// Go: hugolib/page__menus.go:HasMenuCurrent
pub fn has_menu_current(p: &PageHandle, menu_id: &str, me: &MenuEntry) -> bool {
    let site_menus = crate::site::site_menus(&p.h, p.state().site_idx);
    let q = new_menu_query_provider(menus(p).unwrap_or_default(), site_menus, Some(unwrapped(p)));
    q.has_menu_current(menu_id, me)
}

/// Go: `(p *pageMenus) IsMenuCurrent(menuID, inme)`.
// Go: hugolib/page__menus.go:IsMenuCurrent
pub fn is_menu_current(p: &PageHandle, menu_id: &str, inme: &MenuEntry) -> bool {
    let site_menus = crate::site::site_menus(&p.h, p.state().site_idx);
    let q = new_menu_query_provider(menus(p).unwrap_or_default(), site_menus, Some(unwrapped(p)));
    q.is_menu_current(menu_id, inme)
}

/// Go: `(p *pageMenus) Menus()` — runs the site's menu assembly first (`None` is Go's nil
/// `navigation.PageMenus`).
// Go: hugolib/page__menus.go:Menus
pub fn page_menus(p: &PageHandle) -> Option<PageMenus> {
    // There is a reverse dependency here. initMenus will, once, build the
    // site menus and update any relevant page.
    let _ = crate::site::site_menus(&p.h, p.state().site_idx);
    menus(p)
}

/// Go: `(p *pageMenus) menus()` — the page's entries (as the site assembly left them).
// Go: hugolib/page__menus.go:menus
pub fn menus(p: &PageHandle) -> Option<PageMenus> {
    let ps = p.state();
    if let Some(m) = ps.common.page_menus_assembled.get() {
        return Some(m.clone());
    }
    init(p).clone()
}

/// Go: `(p *pageMenus) init()` — the page's own entries, once. A decode error is logged.
// Go: hugolib/page__menus.go:init
pub fn init(p: &PageHandle) -> &Option<PageMenus> {
    let ps = p.state();
    ps.common.page_menus.get_or_init(|| {
        let params = ps.meta.params();

        let menus: Value = match &ps.meta.page_config.menus {
            Some(m) => m.clone(),
            None => {
                let get = |k: &str| {
                    params
                        .and_then(|m| m.get(k.as_bytes()).cloned())
                        .unwrap_or(Value::Invalid)
                };
                match params.and_then(|m| m.get(b"menus".as_slice()).cloned()) {
                    Some(v) => v,
                    None => get("menu"),
                }
            }
        };

        match page_menus_from_page_for(&menus, &unwrapped(p)) {
            Ok(pm) => pm,
            Err(err) => {
                p.h.sites[ps.site_idx]
                    .deps
                    .log
                    .errorf(ps.wrap_error(err).to_string());
                None
            }
        }
    })
}

/// The `*pageState` value of a (possibly wrapped) page handle.
fn unwrapped(p: &PageHandle) -> nh_page::page::PageRef {
    PageHandle {
        h: p.h.clone(),
        id: p.id,
        wrapper: crate::page::PageWrapper::None,
    }
    .page_ref()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__menus.go (91 lines; 0/5 funcs executed)
//   types: pageMenus
// OK L32-38: (p *pageMenus) HasMenuCurrent(menuID string, me *navigation.MenuEntry) bool
// OK L40-46: (p *pageMenus) IsMenuCurrent(menuID string, inme *navigation.MenuEntry) bool
// OK L48-56: (p *pageMenus) Menus() navigation.PageMenus
// OK L58-61: (p *pageMenus) menus() navigation.PageMenus
// OK L63-91: (p *pageMenus) init()
// ---------------------------------------------------------------------------
