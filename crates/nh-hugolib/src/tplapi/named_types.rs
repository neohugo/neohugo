//! Module `tplapi::named_types`.
//!
//! NEW: NamedTypeRegistry assembly (page.Pages, resource.Resources, maps.Params, page.Taxonomy ...)
//!
//! Owner: Wave B task T23 (hugolib-site).


//! Builds the `NamedTypeRegistry` handed to the template store: methods declared on Go named
//! slice/map types represented as `Value::List`/`Value::Map`.

use nh_common::object::NamedTypeRegistry;

/// Registry for: `maps.Params` (nh-common), `page.Pages` (nh-page pages), `page.Data`,
/// `page.Taxonomy`, `page.TaxonomyList`, `page.WeightedPages`, `page.OutputFormats`,
/// `page.pagers`, `resource.Resources` (nh-resource), `langs.Languages`, `[]hooks.TableRow` ...
pub fn registry() -> NamedTypeRegistry {
    let mut r = NamedTypeRegistry::new();
    r.register("maps.Params", nh_common::maps::params::PARAMS_METHODS);
    r.register(nh_page::page::PAGES_TYPE, nh_page::pages::PAGES_METHODS);
    r.register(nh_page::page_data::DATA_TYPE, nh_page::page_data::DATA_METHODS);
    r.register(nh_page::taxonomy::TAXONOMY_TYPE, nh_page::taxonomy::TAXONOMY_METHODS);
    r.register(nh_page::weighted::WEIGHTED_PAGES_TYPE, nh_page::weighted::WEIGHTED_PAGES_METHODS);
    // Wave B (T23): resource.Resources, page.OutputFormats (Get), page.TaxonomyList, langs.Languages.
    r
}
