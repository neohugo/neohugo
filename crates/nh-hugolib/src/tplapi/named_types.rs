//! Module `tplapi::named_types`.
//!
//! NEW: NamedTypeRegistry assembly (page.Pages, resource.Resources, maps.Params, page.Taxonomy ...)
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Builds the `NamedTypeRegistry` handed to the template store: methods declared on Go named
//! slice/map types represented as `Value::List`/`Value::Map` (Go `hreflect.GetMethodByName` on
//! the named type). Every named slice/map type a template can receive is registered, with the
//! tables the owning crates export; `langs.Languages`, `page.Sites` and the `tableofcontents`
//! types are defined here. Named types without methods (`navigation.Menus`,
//! `navigation.PageMenus`, `page.pagers`, `page.AuthorList`, ...) need no entry: their method
//! lookup finds nothing, as in Go (tests/methodsets.rs checks every method set).
//!
//! [`init`] also runs the reflect-type registrations other crates ask the host to make once
//! (`nh_page::page::init`: the `page.Page`/`resource.Resource` interfaces for `hreflect` and the
//! resource value converter).

use go_value::{HostCtx, Map, MapType, Value};
use nh_common::object::{GoResult, NamedMethods, NamedTypeRegistry};

use super::values::arity;

/// Go type string of `page.Sites`.
pub const SITES_TYPE: &str = "page.Sites";

/// Go type string of `langs.Languages`.
pub const LANGUAGES_TYPE: &str = "langs.Languages";

/// Registry for: `maps.Params` (nh-common), `page.Pages`, `page.PagesGroup`, `page.Data`,
/// `page.Taxonomy`, `page.TaxonomyList`, `page.OrderedTaxonomy`, `page.WeightedPages`,
/// `page.OutputFormats`, `navigation.Menu` (nh-page), `resource.Resources` (nh-resource),
/// `langs.Languages`, `page.Sites`, `tableofcontents.Headings`,
/// `collections.SortedStringSlice` (here).
pub fn registry() -> NamedTypeRegistry {
    init();
    let mut r = NamedTypeRegistry::new();
    r.register("maps.Params", nh_common::maps::params::PARAMS_METHODS);
    r.register(nh_page::page::PAGES_TYPE, nh_page::pages::PAGES_METHODS);
    r.register(
        nh_page::pagegroup::PAGES_GROUP_TYPE,
        nh_page::pagegroup::PAGES_GROUP_METHODS,
    );
    r.register(
        nh_page::page_data::DATA_TYPE,
        nh_page::page_data::DATA_METHODS,
    );
    r.register(
        nh_page::taxonomy::TAXONOMY_TYPE,
        nh_page::taxonomy::TAXONOMY_METHODS,
    );
    r.register(
        nh_page::taxonomy::TAXONOMY_LIST_TYPE,
        nh_page::taxonomy::TAXONOMY_LIST_METHODS,
    );
    r.register(
        nh_page::taxonomy::ORDERED_TAXONOMY_TYPE,
        nh_page::taxonomy::ORDERED_TAXONOMY_METHODS,
    );
    r.register(
        nh_page::weighted::WEIGHTED_PAGES_TYPE,
        nh_page::weighted::WEIGHTED_PAGES_METHODS,
    );
    r.register(
        nh_page::page_outputformat::OUTPUT_FORMATS_TYPE,
        nh_page::page_outputformat::OUTPUT_FORMATS_METHODS,
    );
    r.register(
        nh_page::navigation::menu::MENU_TYPE,
        nh_page::navigation::menu::MENU_METHODS,
    );
    r.register(
        nh_resource::resourcetypes::RESOURCES_TYPE,
        nh_resource::resources::RESOURCES_METHODS,
    );
    r.register(LANGUAGES_TYPE, LANGUAGES_METHODS);
    r.register(SITES_TYPE, SITES_METHODS);
    r.register(
        super::objects::HEADINGS_TYPE,
        super::objects::HEADINGS_METHODS,
    );
    r.register(
        super::objects::SORTED_STRING_SLICE_TYPE,
        super::objects::SORTED_STRING_SLICE_METHODS,
    );
    r
}

/// The process-wide registrations the host makes once (idempotent).
pub fn init() {
    nh_page::page::init();
}

fn languages_has_method(name: &str) -> bool {
    matches!(name, "AsIndexSet" | "AsSet")
}

/// The languages of a `langs.Languages` value.
fn languages_of(recv: &Value) -> nh_langs::language::Languages {
    match recv {
        Value::List(l) => l
            .items
            .iter()
            .filter_map(nh_langs::language::language_from_value)
            .collect(),
        _ => Vec::new(),
    }
}

// Go: langs/language.go:AsSet / AsIndexSet
fn languages_call(
    _ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    a: &[Value],
) -> Option<GoResult<Value>> {
    if !languages_has_method(name) {
        return None;
    }
    Some((|| {
        arity(a, 0, false, name)?;
        let ls = languages_of(recv);
        let mut m = Map::new(MapType::Named(std::sync::Arc::from(if name == "AsSet" {
            "map[string]bool"
        } else {
            "map[string]int"
        })));
        if name == "AsSet" {
            for (k, v) in nh_langs::language::as_set(&ls) {
                m.insert(k.as_str(), Value::Bool(v));
            }
        } else {
            for (k, v) in nh_langs::language::as_index_set(&ls) {
                m.insert(k.as_str(), Value::int(v as i64));
            }
        }
        Ok(Value::map(m))
    })())
}

/// The methods of `langs.Languages` (`AsSet`, `AsIndexSet`).
pub const LANGUAGES_METHODS: NamedMethods = NamedMethods {
    has_method: languages_has_method,
    call: languages_call,
};

fn sites_has_method(name: &str) -> bool {
    matches!(name, "Default" | "First")
}

// Go: resources/page/site.go:Default / First
fn sites_call(_ctx: HostCtx<'_>, recv: &Value, name: &str, a: &[Value]) -> Option<GoResult<Value>> {
    if !sites_has_method(name) {
        return None;
    }
    Some((|| {
        arity(a, 0, false, name)?;
        if name == "First" {
            nh_config::neohugo::neohugo::deprecate(
                ".Sites.First",
                "Use .Sites.Default instead.",
                "v0.127.0",
            );
        }
        // Go: a nil `page.Site` interface for an empty list.
        Ok(match recv {
            Value::List(l) if !l.items.is_empty() => l.items[0].clone(),
            _ => super::values::nil_of("page.Site"),
        })
    })())
}

/// The methods of `page.Sites` (`Default`, the deprecated `First`).
pub const SITES_METHODS: NamedMethods = NamedMethods {
    has_method: sites_has_method,
    call: sites_call,
};
