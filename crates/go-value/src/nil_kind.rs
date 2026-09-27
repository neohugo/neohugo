//! The reflect kind of a [`crate::Value::TypedNil`].
//!
//! Go distinguishes a nil pointer from a nil slice, map, func, chan or
//! interface; `fmt` (`[]` vs `<nil>`), `encoding/json` (`null` vs `[]`) and
//! `hashstructure` depend on it. A `TypedNil` only carries its Go type string,
//! so the kind is derived from the type syntax, and for named types
//! (`page.Pages`, `maps.Params`, ...) from a registry that host crates extend
//! with [`register_named_kind`].

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Kind of a typed nil value (Go: `reflect.Kind` of the nil).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NilKind {
    Ptr,
    Slice,
    Map,
    Func,
    Chan,
    Interface,
}

/// Named neohugo slice types that may appear as typed nils.
const NAMED_SLICES: &[&str] = &[
    "page.Pages",
    "page.PagesGroup",
    "page.WeightedPages",
    "page.OrderedTaxonomy",
    "page.OutputFormats",
    "page.Sites",
    "resource.Resources",
    "langs.Languages",
    "output.Formats",
    "media.Types",
    "navigation.Menu",
    "tableofcontents.Headings",
    "hooks.TableRow",
    "collections.SortedStringSlice",
    "json.RawMessage",
];

/// Named neohugo map types that may appear as typed nils.
const NAMED_MAPS: &[&str] = &[
    "maps.Params",
    "page.Taxonomy",
    "page.TaxonomyList",
    "page.Data",
    "page.AuthorList",
    "page.AuthorSocial",
    "navigation.Menus",
    "navigation.PageMenus",
    "exif.Tags",
    "attributes.Attributes",
];

fn registry() -> &'static RwLock<HashMap<String, NilKind>> {
    static REG: OnceLock<RwLock<HashMap<String, NilKind>>> = OnceLock::new();
    REG.get_or_init(|| {
        let mut m = HashMap::new();
        for t in NAMED_SLICES {
            m.insert((*t).to_string(), NilKind::Slice);
        }
        for t in NAMED_MAPS {
            m.insert((*t).to_string(), NilKind::Map);
        }
        RwLock::new(m)
    })
}

/// Declare the kind of a named Go type, so typed nils of it behave like Go.
/// Host crates call this for their named slice/map/func types.
pub fn register_named_kind(type_name: &str, kind: NilKind) {
    registry()
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(type_name.to_string(), kind);
}

/// The kind of a typed nil with this Go type string. Unnamed types are read
/// from their syntax; named types come from the registry; anything unknown is
/// treated as an interface (an interface nil prints `<nil>`).
pub fn typed_nil_kind(type_name: &str) -> NilKind {
    let ty = type_name;
    if ty.starts_with('*') {
        NilKind::Ptr
    } else if ty.starts_with("[]") {
        NilKind::Slice
    } else if ty.starts_with("map[") {
        NilKind::Map
    } else if ty.starts_with("func(") || ty == "func()" {
        NilKind::Func
    } else if ty.starts_with("chan ") || ty.starts_with("<-chan ") || ty.starts_with("chan<- ") {
        NilKind::Chan
    } else {
        registry()
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(ty)
            .copied()
            .unwrap_or(NilKind::Interface)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(typed_nil_kind("*source.File"), NilKind::Ptr);
        assert_eq!(typed_nil_kind("[]string"), NilKind::Slice);
        assert_eq!(typed_nil_kind("map[string]interface {}"), NilKind::Map);
        assert_eq!(typed_nil_kind("page.Pages"), NilKind::Slice);
        assert_eq!(typed_nil_kind("maps.Params"), NilKind::Map);
        assert_eq!(typed_nil_kind("error"), NilKind::Interface);
        register_named_kind("test.MySlice", NilKind::Slice);
        assert_eq!(typed_nil_kind("test.MySlice"), NilKind::Slice);
    }
}
