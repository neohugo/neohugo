//! Port of `navigation/pagemenus.go`.
//!
//! Owner: Wave B task T12 (page-collections).

use super::menu::PageMenus;

/// Go: `navigation.PageMenusFromPage(p)` (front matter `menus`).
// Go: navigation/pagemenus.go:PageMenusFromPage
pub fn page_menus_from_page(menus: Option<&go_value::Value>) -> nh_common::Result<PageMenus> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: navigation/pagemenus.go (225 lines; 0/8 funcs executed)
//   types: PageMenusProvider, PageMenusGetter, MenusGetter, MenuQueryProvider, pageMenus, nopPageMenus
//    L45-99: PageMenusFromPage(ms any, p Page) (PageMenus, error)
//    L101-111: NewMenuQueryProvider( pagem PageMenusGetter, sitem MenusGetter, p Page, ) MenuQueryProvider
//    L119-158: (pm *pageMenus) HasMenuCurrent(menuID string, me *MenuEntry) bool
//    L160-194: (pm *pageMenus) IsMenuCurrent(menuID string, inme *MenuEntry) bool
//    L196-209: (pm *pageMenus) isSameAsDescendantMenu(inme *MenuEntry, parent *MenuEntry) bool
//    L215-217: (m nopPageMenus) Menus() PageMenus
//    L219-221: (m nopPageMenus) HasMenuCurrent(menuID string, me *MenuEntry) bool
//    L223-225: (m nopPageMenus) IsMenuCurrent(menuID string, inme *MenuEntry) bool
// ---------------------------------------------------------------------------
