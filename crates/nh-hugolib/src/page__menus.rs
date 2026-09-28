//! Port of `hugolib/page__menus.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

// Wave B: port per the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__menus.go (91 lines; 0/5 funcs executed)
//   types: pageMenus
//    L32-38: (p *pageMenus) HasMenuCurrent(menuID string, me *navigation.MenuEntry) bool
//    L40-46: (p *pageMenus) IsMenuCurrent(menuID string, inme *navigation.MenuEntry) bool
//    L48-56: (p *pageMenus) Menus() navigation.PageMenus
//    L58-61: (p *pageMenus) menus() navigation.PageMenus
//    L63-91: (p *pageMenus) init()
// ---------------------------------------------------------------------------
