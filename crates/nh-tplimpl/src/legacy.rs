//! Port of `tpl/tplimpl/legacy.go`.
//!
//! Owner: Wave B task T13 (tplimpl).


//! Go `legacy.go`: pre-0.146 layout path mappings (`_default/`, `partials/` -> `_partials/`,
//! `shortcodes/` -> `_shortcodes/`, `x-baseof.html` -> `baseof.x.html`, taxonomy/term/section
//! legacy files such as `taxonomy/list.html` -> kind taxonomy, `term/term.html` -> kind term).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/legacy.go (130 lines; 0/0 funcs executed)
//   types: layoutLegacyMapping, layoutLegacyMappingTarget, legacyTargetPathIdentifiers, legacyOrdinalMapping,
//          legacyOrdinalMappingFi
// ---------------------------------------------------------------------------
