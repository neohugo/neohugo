//! Port of `navigation/pagemenus.go`.
//!
//! Owner: Wave B task T12 (page-collections).

use std::sync::Arc;

use go_value::Value;
use nh_common::Result;
use nh_common::herrors::Error;

use super::menu::{MenuConfig, MenuEntry, Menus, PageMenus, set_page_values};
use crate::page::PageRef;

/// Go: `navigation.PageMenusFromPage(p)` (front matter `menus`) — without the page, only a nil
/// `menus` can be handled; use [`page_menus_from_page_for`].
// Go: navigation/pagemenus.go:PageMenusFromPage
pub fn page_menus_from_page(menus: Option<&go_value::Value>) -> nh_common::Result<PageMenus> {
    match menus {
        None => Ok(PageMenus::new()),
        Some(v) if v.is_invalid() => Ok(PageMenus::new()),
        Some(_) => Err(Error::new(
            "neohugo-rs: navigation::pagemenus::page_menus_from_page needs the page (use page_menus_from_page_for)",
        )),
    }
}

/// Go: `navigation.PageMenusFromPage(ms, p)` — `ms` is the front matter `menus` value: a menu
/// name, a list of names (one shared entry, Go's `&me` in the loop, whose `Menu` is the last
/// name), or a map of menu name -> entry config. `None` is Go's nil result (`ms == nil`).
// Go: navigation/pagemenus.go:PageMenusFromPage
pub fn page_menus_from_page_for(ms: &Value, p: &PageRef) -> Result<Option<PageMenus>> {
    if ms.is_invalid() {
        return Ok(None);
    }
    let mut pm = PageMenus::new();
    let mut me = MenuEntry::default();
    set_page_values(&mut me, p);

    // Could be the name of the menu to attach it to
    if let Ok(mname) = nh_common::cast::caste::to_string_e(ms) {
        let mname = mname.to_str_lossy().into_owned();
        me.menu = mname.clone();
        pm.insert(mname, Arc::new(me));
        return Ok(Some(pm));
    }

    // Could be a slice of strings
    if let Ok(mnames) = nh_common::cast::caste::to_string_slice_e(ms) {
        let names: Vec<String> = mnames
            .iter()
            .map(|s| s.to_str_lossy().into_owned())
            .collect();
        if let Some(last) = names.last() {
            me.menu = last.clone();
        }
        let me = Arc::new(me);
        for mname in names {
            pm.insert(mname, me.clone());
        }
        return Ok(Some(pm));
    }

    let wrap_err = |err: Error| -> Error {
        Error::new(format!(
            "unable to process menus for page {}: {}",
            String::from_utf8_lossy(&go_fmt::sprintf("%q", &[Value::string(p.0.path())])),
            err
        ))
    };

    // Could be a structured menu entry
    let menus = nh_common::maps::maps::to_string_map_e(ms).map_err(wrap_err)?;

    for (name, menu) in &menus.entries {
        let name = name.to_str_lossy().into_owned();
        let mut menu_entry = MenuEntry {
            menu: name.clone(),
            ..Default::default()
        };
        if !menu.is_nil() {
            let ime = nh_common::maps::maps::to_string_map_e(menu).map_err(wrap_err)?;
            let mut cfg = MenuConfig::default();
            cfg.weak_decode(&Value::map(ime))?;
            menu_entry.config = cfg;
        }
        set_page_values(&mut menu_entry, p);
        pm.insert(name, Arc::new(menu_entry));
    }

    Ok(Some(pm))
}

/// Go: `pageMenus` (`NewMenuQueryProvider(pagem, sitem, p)`): the page's own menus, the site's
/// menus and the page.
pub struct PageMenusQuery {
    pub pagem: PageMenus,
    pub sitem: Menus,
    pub p: Option<PageRef>,
}

/// Go: `NewMenuQueryProvider(pagem, sitem, p)`.
// Go: navigation/pagemenus.go:NewMenuQueryProvider
pub fn new_menu_query_provider(
    pagem: PageMenus,
    sitem: Menus,
    p: Option<PageRef>,
) -> PageMenusQuery {
    PageMenusQuery { pagem, sitem, p }
}

impl PageMenusQuery {
    fn p_value(&self) -> Value {
        match &self.p {
            Some(p) => p.to_value(),
            None => Value::Invalid,
        }
    }

    /// Go: `pm.HasMenuCurrent(menuID, me)`.
    // Go: navigation/pagemenus.go:HasMenuCurrent
    pub fn has_menu_current(&self, menu_id: &str, me: &MenuEntry) -> bool {
        if let Some(mp) = me.page_ref()
            && mp.0.is_section()
            && mp.0.is_ancestor(&self.p_value())
        {
            return true;
        }

        if !me.has_children() {
            return false;
        }

        let menus = &self.pagem;

        if let Some(m) = menus.get(menu_id) {
            for child in &me.children {
                if child.is_equal(m) {
                    return true;
                }
                if self.has_menu_current(menu_id, child) {
                    return true;
                }
            }
        }

        let Some(p) = &self.p else {
            return false;
        };

        for child in &me.children {
            if child.is_same_page(Some(p)) {
                return true;
            }

            if self.has_menu_current(menu_id, child) {
                return true;
            }
        }

        false
    }

    /// Go: `pm.IsMenuCurrent(menuID, inme)`.
    // Go: navigation/pagemenus.go:IsMenuCurrent
    pub fn is_menu_current(&self, menu_id: &str, inme: &MenuEntry) -> bool {
        let menus = &self.pagem;

        if let Some(me) = menus.get(menu_id)
            && me.is_equal(inme)
        {
            return true;
        }

        let Some(p) = &self.p else {
            return false;
        };

        if !inme.is_same_page(Some(p)) {
            return false;
        }

        // This resource may be included in several menus. Search for it to make sure that it is
        // in the menu with the given menuId.
        if let Some(menu) = self.sitem.get(menu_id) {
            for menu_entry in menu {
                if menu_entry.is_same_resource(inme) {
                    return true;
                }

                let descendant_found = self.is_same_as_descendant_menu(inme, menu_entry);
                if descendant_found {
                    return descendant_found;
                }
            }
        }

        false
    }

    // Go: navigation/pagemenus.go:isSameAsDescendantMenu
    fn is_same_as_descendant_menu(&self, inme: &MenuEntry, parent: &MenuEntry) -> bool {
        if parent.has_children() {
            for child in &parent.children {
                if child.is_same_resource(inme) {
                    return true;
                }
                let descendant_found = self.is_same_as_descendant_menu(inme, child);
                if descendant_found {
                    return descendant_found;
                }
            }
        }
        false
    }
}

/// Go: `NopPageMenus` — no menus, never current.
pub struct NopPageMenus;

impl NopPageMenus {
    // Go: navigation/pagemenus.go:Menus
    pub fn menus(&self) -> PageMenus {
        PageMenus::new()
    }
    // Go: navigation/pagemenus.go:HasMenuCurrent
    pub fn has_menu_current(&self, _menu_id: &str, _me: &MenuEntry) -> bool {
        false
    }
    // Go: navigation/pagemenus.go:IsMenuCurrent
    pub fn is_menu_current(&self, _menu_id: &str, _inme: &MenuEntry) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: navigation/pagemenus.go (225 lines; 0/8 funcs executed)
//   types: PageMenusProvider, PageMenusGetter, MenusGetter, MenuQueryProvider, pageMenus, nopPageMenus
// OK L45-99: PageMenusFromPage(ms any, p Page) (PageMenus, error)
// OK L101-111: NewMenuQueryProvider( pagem PageMenusGetter, sitem MenusGetter, p Page, ) MenuQueryProvider
// OK L119-158: (pm *pageMenus) HasMenuCurrent(menuID string, me *MenuEntry) bool
// OK L160-194: (pm *pageMenus) IsMenuCurrent(menuID string, inme *MenuEntry) bool
// OK L196-209: (pm *pageMenus) isSameAsDescendantMenu(inme *MenuEntry, parent *MenuEntry) bool
// OK L215-217: (m nopPageMenus) Menus() PageMenus
// OK L219-221: (m nopPageMenus) HasMenuCurrent(menuID string, me *MenuEntry) bool
// OK L223-225: (m nopPageMenus) IsMenuCurrent(menuID string, inme *MenuEntry) bool
// ---------------------------------------------------------------------------
