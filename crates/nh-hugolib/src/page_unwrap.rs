//! Port of `hugolib/page_unwrap.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Go `unwrapPage(in any)`: a `*pageState` is itself; the `pageWrapper`s (`pageWithWeight0`,
//! `*pageWithOrdinal`) give their page; `types.Unwrapper`s (`*pageForShortcode`,
//! `*pageForRenderHooks`) their `Unwrapv()`; any other `page.Page` (the nop page) is itself;
//! `nil` is nil; anything else is an error.
//!
//! In the port every page value is a `nh_page::page::PageRef`, and `Page::unwrap_page` is the
//! wrapper-free page (`PageWrapper::None` for nh-hugolib's handles, the nop page itself).

use go_value::Value;
use nh_common::Result;
use nh_common::herrors::Error;
use nh_page::page::PageRef;

/// Go: `unwrapPage(in)` — `Ok(None)` is Go's `(nil, nil)`.
// Go: hugolib/page_unwrap.go:unwrapPage
pub fn unwrap_page(v: &Value) -> Result<Option<PageRef>> {
    match v {
        Value::Invalid => Ok(None),
        // A typed nil `*pageState` (or a nil page.Page interface converted to `any`).
        Value::TypedNil(t) if nh_page::page::implements_page(t) => Ok(None),
        _ => match nh_page::page::page_from_value(v) {
            Some(p) => Ok(Some(PageRef(p.0.unwrap_page()))),
            None => Err(Error::new(format!(
                "unwrapPage: {} not supported",
                go_type_string(v)
            ))),
        },
    }
}

/// Go: `mustUnwrapPage(in)` — panics on error (as Go does).
// Go: hugolib/page_unwrap.go:mustUnwrapPage
pub fn must_unwrap_page(v: &Value) -> Option<PageRef> {
    match unwrap_page(v) {
        Ok(p) => p,
        Err(err) => panic!("{}", err),
    }
}

/// Go's `%T` of a value (`<nil>` for an untyped nil).
pub(crate) fn go_type_string(v: &Value) -> String {
    match v {
        Value::Invalid => "<nil>".to_string(),
        _ => v.go_type_name().into_owned(),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page_unwrap.go (53 lines; 1/2 funcs executed)
//   types: pageWrapper
// OK L29-44: unwrapPage(in any) (page.Page, error)
// OK L46-53: mustUnwrapPage(in any) page.Page
// ---------------------------------------------------------------------------
