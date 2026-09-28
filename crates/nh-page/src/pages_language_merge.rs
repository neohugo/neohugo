//! Port of `resources/page/pages_language_merge.go`.
//!
//! Owner: Wave B task T12 (page-collections).

use go_value::Value;
use nh_common::Result;
use nh_common::herrors::Error;

use crate::page::{PAGES_TYPE, Pages, pages_to_value};
use crate::pages_cache::spc;
use crate::pages_sort::sort_by_default;

/// Go: `Pages.MergeByLanguage(other)` — supplies missing translations in `p1` with values from
/// `p2` (by `TranslationKey`); the result is sorted by the default sort (cached:
/// `pages.MergeByLanguage` over both lists).
// Go: resources/page/pages_language_merge.go:MergeByLanguage
pub fn merge_by_language(p1: &Pages, p2: &Pages) -> Pages {
    let merge = |pages: &mut Pages| {
        let mut m = std::collections::HashSet::new();
        for p in pages.iter() {
            m.insert(p.0.translation_key().unwrap_or_default());
        }

        for p in p2 {
            if !m.contains(&p.0.translation_key().unwrap_or_default()) {
                pages.push(p.clone());
            }
        }

        sort_by_default(pages);
    };

    let (out, _) = spc().get_p("pages.MergeByLanguage", &merge, &[p1, p2]);

    out
}

/// Go: `Pages.MergeByLanguageInterface(in)` — nil gives `p1` back; anything but `page.Pages` is
/// an error.
// Go: resources/page/pages_language_merge.go:MergeByLanguageInterface
pub fn merge_by_language_interface(p1: &Pages, input: &Value) -> Result<Value> {
    if input.is_invalid() {
        return Ok(pages_to_value(p1));
    }
    let p2 = match input {
        Value::List(l) if matches!(&l.ty, go_value::SliceType::Named(n) if &**n == PAGES_TYPE) => {
            crate::page::pages_from_value(input).ok()
        }
        Value::TypedNil(t) if &**t == PAGES_TYPE => Some(Vec::new()),
        _ => None,
    };
    let Some(p2) = p2 else {
        return Err(Error::new(format!(
            "{} cannot be merged by language",
            input.go_type_name()
        )));
    };
    Ok(pages_to_value(&merge_by_language(p1, &p2)))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_language_merge.go (63 lines; 0/2 funcs executed)
//   types: pagesLanguageMerger
// OK L30-49: (p1 Pages) MergeByLanguage(p2 Pages) Pages
// OK L54-63: (p1 Pages) MergeByLanguageInterface(in any) (any, error)
// ---------------------------------------------------------------------------
