//! Port of `tpl/tplimpl/category_string.go`, `tpl/tplimpl/subcategory_string.go`.
//!
//! Owner: Wave B task T13 (tplimpl).


/// Go: `tplimpl.Category`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    Layout = 1,
    Baseof,
    Markup,
    Shortcode,
    Partial,
    /// E.g. `_server/error.html`.
    Server,
    Hugo,
}

/// Go: `tplimpl.SubCategory`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SubCategory {
    /// Mostly a placeholder for the default.
    #[default]
    Main,
    /// Internal Hugo templates.
    Embedded,
    /// Inline partials (`{{ define "partials/x" }}`).
    Inline,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/category_string.go (30 lines; 0/2 funcs executed)
//    L7-18: _()
//    L24-30: (i Category) String() string
// Source: tpl/tplimpl/subcategory_string.go (25 lines; 0/2 funcs executed)
//    L7-14: _()
//    L20-25: (i SubCategory) String() string
// ---------------------------------------------------------------------------
