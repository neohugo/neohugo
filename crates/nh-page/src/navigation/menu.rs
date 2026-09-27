//! Port of `navigation/menu.go`.
//!
//! Owner: Wave B task T12 (page-collections).


//! Go `navigation`: menus from config (`[languages.X.menus.main]`) and front matter. Not
//! referenced by seeksnack templates (`.Site.Menus` unused) but decoded; port faithfully later.

use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{GoString, Map, Value};
use nh_common::Result;

/// Go: `navigation.MenuConfig`.
#[derive(Clone, Debug, Default)]
pub struct MenuConfig {
    pub identifier: String,
    pub parent: String,
    pub name: String,
    pub pre: GoString,
    pub post: GoString,
    pub url: String,
    pub page_ref: String,
    pub weight: i64,
    pub title: String,
    pub params: Option<Map>,
}

/// Go: `navigation.MenuEntry`.
#[derive(Clone, Default)]
pub struct MenuEntry {
    pub config: MenuConfig,
    pub menu: String,
    pub configured_url: String,
    /// The page (a page.Page value) if the entry points to one.
    pub page: Option<Value>,
    pub children: Menu,
}

/// Go: `navigation.Menu` (sorted: weight (0 last), `compare.Strings(Name)`, Identifier).
pub type Menu = Vec<Arc<MenuEntry>>;
/// Go: `navigation.Menus`.
pub type Menus = Arc<BTreeMap<String, Menu>>;
/// Go: `navigation.PageMenus`.
pub type PageMenus = BTreeMap<String, Arc<MenuEntry>>;

/// Go: `navigation.DecodeConfig(in)`.
// Go: navigation/menu.go:DecodeConfig
pub fn decode_config(input: &Value) -> Result<BTreeMap<String, Menu>> {
    todo!()
}

/// Go: `Menu.Sort()` / `ByWeight` / `ByName`.
// Go: navigation/menu.go:Sort
pub fn sort(m: &mut Menu) {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: navigation/menu.go (318 lines; 7/20 funcs executed)
//   types: MenuEntry, Page, Menu, Menus, PageMenus, MenuConfig, menuSorter, menuEntryBy
//    L53-66: (m *MenuEntry) URL() string
//    L69-80: SetPageValues(m *MenuEntry, p Page)
//    L106-108: (m *MenuEntry) HasChildren() bool
//    L111-116: (m *MenuEntry) KeyName() string
//    L118-126: (m *MenuEntry) hopefullyUniqueID() string
//    L129-131: (m *MenuEntry) isEqual(inme *MenuEntry) bool
//    L135-141: (m *MenuEntry) isSameResource(inme *MenuEntry) bool
//    L143-148: (m *MenuEntry) isSamePage(p Page) bool
// EX L168-173: (m Menu) Add(me *MenuEntry) Menu
// EX L188-194: (by menuEntryBy) Sort(menu Menu)
// EX L216-216: (ms *menuSorter) Len() int
// EX L217-217: (ms *menuSorter) Swap(i, j int)
// EX L220-220: (ms *menuSorter) Less(i, j int) bool
// EX L223-226: (m Menu) Sort() Menu
//    L229-234: (m Menu) Limit(n int) Menu
//    L237-242: (m Menu) ByWeight() Menu
//    L245-254: (m Menu) ByName() Menu
//    L257-267: (m Menu) Reverse() Menu
//    L271-273: (m Menu) Clone() Menu
// EX L275-318: DecodeConfig(in any) (*config.ConfigNamespace[map[string]MenuConfig, Menus], error)
// ---------------------------------------------------------------------------
