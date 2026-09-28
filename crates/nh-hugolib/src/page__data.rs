//! Port of `hugolib/page__data.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Go `hugolib/page__data.go`: `.Data` (page.Data): taxonomy -> Singular/Plural/Terms/Pages;
//! term -> Singular/Plural/Term/<singular>/Pages; section/home -> Pages; sitemap -> Pages = site Pages.
//!
//! Go builds the map once per page (`dataInit`); the port builds it on every call (the values
//! are the same: `.Data.Pages` stays the lazy `p.Pages` func, and the taxonomies are the site's
//! lazily created list).

use std::sync::Arc;

use go_value::{GoString, Map, MapType, Value};
use nh_common::kinds;
use nh_page::page::Page;
use nh_page::page_data::{LazyPages, new_data};
use nh_page::taxonomy::{TAXONOMY_TYPE, taxonomy_get, taxonomy_to_value};
use nh_page::weighted::{WEIGHTED_PAGES_TYPE, weighted_pages_to_value};

use crate::content_map_page::site_taxonomies;
use crate::page::{PageHandle, PageWrapper};

// Go: hugolib/page__data.go:Data
pub fn data(p: &PageHandle) -> Value {
    let mut data = Map::new(MapType::StringAny);
    let ps = p.state();

    if ps.meta.kind() == kinds::KIND_PAGE {
        return new_data(data);
    }

    let s = &p.h.sites[ps.site_idx];
    match ps.meta.kind() {
        kinds::KIND_TERM => {
            let path = ps.meta.path();
            let name = s.page_map.cfg.get_taxonomy_config(&path);
            let taxonomies = site_taxonomies(&p.h, ps.site_idx);
            let term = taxonomies.get(&name.plural).and_then(|t| {
                taxonomy_get(
                    t,
                    path.strip_prefix(name.plural_tree_key.as_str())
                        .unwrap_or(&path),
                )
            });
            let term = match term {
                Some(wp) => weighted_pages_to_value(&wp),
                None => Value::TypedNil(Arc::from(WEIGHTED_PAGES_TYPE)),
            };
            data.insert(GoString::from(name.singular.as_str()), term);
            data.insert("Singular", Value::string(name.singular.as_str()));
            data.insert("Plural", Value::string(name.plural.as_str()));
            data.insert("Term", Value::string(ps.meta.term.as_str()));
        }
        kinds::KIND_TAXONOMY => {
            let view_cfg = s.page_map.cfg.get_taxonomy_config(&ps.meta.path());
            data.insert("Singular", Value::string(view_cfg.singular.as_str()));
            data.insert("Plural", Value::string(view_cfg.plural.as_str()));
            let taxonomies = site_taxonomies(&p.h, ps.site_idx);
            let terms = match taxonomies.get(&view_cfg.plural) {
                Some(t) => taxonomy_to_value(t),
                None => Value::TypedNil(Arc::from(TAXONOMY_TYPE)),
            };
            data.insert("Terms", terms.clone());
            // keep the following just for legacy reasons
            data.insert("OrderedIndex", terms.clone());
            data.insert("Index", terms);
        }
        _ => {}
    }

    // Assign the function to the map to make sure it is lazily initialized
    let handle = PageHandle {
        h: p.h.clone(),
        id: p.id,
        wrapper: PageWrapper::None,
    };
    data.insert(
        "pages",
        Value::object(LazyPages(Arc::new(move || Page::pages(&handle)))),
    );

    new_data(data)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__data.go (63 lines; 1/1 funcs executed)
//   types: pageData
// OK L31-63: (p *pageData) Data() any
// ---------------------------------------------------------------------------
