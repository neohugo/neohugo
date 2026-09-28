//! Port of `tpl/tplimpl/category_string.go`, `tpl/tplimpl/subcategory_string.go`.
//!
//! Owner: Wave B task T13 (tplimpl).

/// Go: `tplimpl.Category`. Go's zero value (no category, e.g. a `TextParse` template) is
/// `Option::<Category>::None` where it can occur.
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SubCategory {
    /// Mostly a placeholder for the default.
    #[default]
    Main,
    /// Internal Hugo templates.
    Embedded,
    /// Inline partials (`{{ define "partials/x" }}`).
    Inline,
}

// Go: tpl/tplimpl/category_string.go:_Category_name
const CATEGORY_NAME: &str = "CategoryLayoutCategoryBaseofCategoryMarkupCategoryShortcodeCategoryPartialCategoryServerCategoryHugo";

// Go: tpl/tplimpl/category_string.go:_Category_index
const CATEGORY_INDEX: [usize; 8] = [0, 14, 28, 42, 59, 74, 88, 100];

// Go: tpl/tplimpl/subcategory_string.go:_SubCategory_name
const SUB_CATEGORY_NAME: &str = "SubCategoryMainSubCategoryEmbeddedSubCategoryInline";

// Go: tpl/tplimpl/subcategory_string.go:_SubCategory_index
const SUB_CATEGORY_INDEX: [usize; 4] = [0, 15, 34, 51];

/// Go: `Category(i).String()` for any integer value (the stringer output).
// Go: tpl/tplimpl/category_string.go:String
pub fn category_string(i: i64) -> String {
    let j = i.wrapping_sub(1);
    if j < 0 || j >= (CATEGORY_INDEX.len() - 1) as i64 {
        return format!("Category({i})");
    }
    let j = j as usize;
    CATEGORY_NAME[CATEGORY_INDEX[j]..CATEGORY_INDEX[j + 1]].to_string()
}

/// Go: `SubCategory(i).String()` for any integer value.
// Go: tpl/tplimpl/subcategory_string.go:String
pub fn sub_category_string(i: i64) -> String {
    if i < 0 || i >= (SUB_CATEGORY_INDEX.len() - 1) as i64 {
        return format!("SubCategory({i})");
    }
    let i = i as usize;
    SUB_CATEGORY_NAME[SUB_CATEGORY_INDEX[i]..SUB_CATEGORY_INDEX[i + 1]].to_string()
}

impl Category {
    /// Go: `Category.String()`.
    pub fn string(self) -> String {
        category_string(self as i64)
    }
}

impl SubCategory {
    /// Go: `SubCategory.String()`.
    pub fn string(self) -> String {
        sub_category_string(self as i64)
    }
}

/// Go: `Category.String()` of an optional category (Go's zero category prints `Category(0)`).
pub fn opt_category_string(c: Option<Category>) -> String {
    match c {
        Some(c) => c.string(),
        None => category_string(0),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/category_string.go (30 lines; 0/2 funcs executed)
// OK L7-18: _() (compile-time check; the enum discriminants)
// OK L24-30: (i Category) String() string
// Source: tpl/tplimpl/subcategory_string.go (25 lines; 0/2 funcs executed)
// OK L7-14: _() (compile-time check; the enum discriminants)
// OK L20-25: (i SubCategory) String() string
// ---------------------------------------------------------------------------
