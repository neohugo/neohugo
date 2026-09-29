//! Port of `resources/page/pages.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Methods of the named slice type `page.Pages` for templates (`.Reverse`, `.Related`, `.ByTitle`,
//! `.GroupBy*`, `.Len`, `.Next`, `.Prev`, `.Limit`, ...), registered in the NamedTypeRegistry.
//! The method set is Go's (`pages.go`, `pages_sort.go`, `pages_related.go`, `pagegroup.go`,
//! `pages_prev_next.go`, `pages_language_merge.go`); argument checks follow text/template's
//! `evalCall` for the Go parameter types.

use std::sync::Arc;

use go_value::{HostCtx, SliceType, Value};
use nh_common::object::{GoResult, NamedMethods, args};

use crate::page::{PAGES_TYPE, PageRef, Pages, page_from_value, pages_from_value, pages_to_value};
use crate::pagegroup::{PageGroup, pages_group_opt_to_value};
use crate::weighted::{page_arg, page_opt_value};

/// Go: `Pages.Len()`.
// Go: resources/page/pages.go:Len
pub fn len(p: &Pages) -> i64 {
    p.len() as i64
}

/// Go: `Pages.ToResources()`.
// Go: resources/page/pages.go:ToResources
pub fn to_resources(pages: &Pages) -> nh_resource::resourcetypes::Resources {
    pages
        .iter()
        .map(|p| {
            let r: Arc<dyn nh_resource::resourcetypes::Resource> = p.0.clone();
            r
        })
        .collect()
}

/// Go: `Pages.Group(key, in)` (`collections.Grouper`): a `PageGroup` of `ToPages(in)`.
// Go: resources/page/pages.go:Group
pub fn group(key: &Value, input: &Value) -> nh_common::Result<PageGroup> {
    let pages = pages_from_value(input)?;
    Ok(PageGroup {
        key: key.clone(),
        pages,
    })
}

pub fn pages_has_method(name: &str) -> bool {
    matches!(
        name,
        "Len"
            | "Reverse"
            | "Related"
            | "RelatedIndices"
            | "RelatedTo"
            | "ByWeight"
            | "ByTitle"
            | "ByLinkTitle"
            | "ByDate"
            | "ByPublishDate"
            | "ByExpiryDate"
            | "ByLastmod"
            | "ByLength"
            | "ByLanguage"
            | "ByParam"
            | "Limit"
            | "GroupBy"
            | "GroupByParam"
            | "GroupByDate"
            | "GroupByPublishDate"
            | "GroupByExpiryDate"
            | "GroupByLastmod"
            | "GroupByParamDate"
            | "Next"
            | "Prev"
            | "MergeByLanguage"
            | "MergeByLanguageInterface"
            | "ToResources"
            | "ProbablyEq"
            | "String"
            | "Group"
    )
}

fn err(e: nh_common::herrors::Error) -> go_value::Error {
    go_value::Error::new(e.to_string())
}

fn pages_opt_value(p: Option<Pages>) -> Value {
    match p {
        Some(p) => pages_to_value(&p),
        None => Value::TypedNil(Arc::from(PAGES_TYPE)),
    }
}

fn is_pages_value(v: &Value) -> bool {
    match v {
        Value::List(l) => matches!(&l.ty, SliceType::Named(n) if &**n == PAGES_TYPE),
        Value::TypedNil(t) => &**t == PAGES_TYPE,
        _ => false,
    }
}

/// A `page.Pages` parameter (Go `validateType`: a `page.Pages` value or nil).
fn pages_arg(a: &[Value], i: usize) -> GoResult<Pages> {
    let v = args::get(a, i)?;
    if v.is_invalid() {
        return Ok(Vec::new());
    }
    if !is_pages_value(&v) {
        return Err(args::wrong_type(PAGES_TYPE, &v));
    }
    pages_from_value(&v).map_err(err)
}

/// The receiver as Pages (a `page.Pages` list or typed nil).
fn recv_pages(recv: &Value) -> Option<Pages> {
    match recv {
        Value::TypedNil(t) if &**t == PAGES_TYPE => Some(Vec::new()),
        Value::List(_) => pages_from_value(recv).ok(),
        _ => None,
    }
}

fn strings_vec(a: &[Value], from: usize) -> GoResult<Vec<String>> {
    Ok(args::strings(a, from)?
        .iter()
        .map(|s| s.to_str_lossy().into_owned())
        .collect())
}

/// Dispatch of `page.Pages` methods (receiver: a `page.Pages` list value).
pub fn pages_call_method(
    ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    args: &[Value],
) -> Option<GoResult<Value>> {
    if !pages_has_method(name) {
        return None;
    }
    let p = recv_pages(recv)?;
    Some(call(ctx, recv, &p, name, args))
}

fn call(ctx: HostCtx<'_>, recv: &Value, p: &Pages, name: &str, a: &[Value]) -> GoResult<Value> {
    use crate::pages_sort as ps;
    let sorted = |f: fn(&Pages) -> Pages| -> GoResult<Value> {
        args::exactly(a, 0, name)?;
        Ok(pages_to_value(&f(p)))
    };
    match name {
        "Len" => {
            args::exactly(a, 0, name)?;
            Ok(Value::int(len(p)))
        }
        "String" => {
            args::exactly(a, 0, name)?;
            Ok(Value::string(ps::pages_string(p)))
        }
        "ToResources" => {
            args::exactly(a, 0, name)?;
            Ok(nh_resource::resourcetypes::resources_to_value(
                &to_resources(p),
            ))
        }
        "ProbablyEq" => {
            args::exactly(a, 1, name)?;
            Ok(Value::Bool(probably_eq_value(p, &args::get(a, 0)?)))
        }
        "Group" => {
            args::exactly(a, 2, name)?;
            let g = group(&args::get(a, 0)?, &args::get(a, 1)?).map_err(err)?;
            Ok(g.to_value())
        }
        "Reverse" => sorted(ps::reverse),
        "ByWeight" => sorted(ps::by_weight),
        "ByTitle" => sorted(ps::by_title),
        "ByLinkTitle" => sorted(ps::by_link_title),
        "ByDate" => sorted(ps::by_date),
        "ByPublishDate" => sorted(ps::by_publish_date),
        "ByExpiryDate" => sorted(ps::by_expiry_date),
        "ByLastmod" => sorted(ps::by_lastmod),
        "ByLanguage" => sorted(ps::by_language),
        "ByLength" => {
            exactly_ctx(a, 0, name)?;
            Ok(pages_to_value(&ps::by_length(ctx, p)))
        }
        "ByParam" => {
            args::exactly(a, 1, name)?;
            let r = ps::by_param(p, &args::get(a, 0)?);
            if p.len() < 2 {
                // Go returns the receiver itself.
                return Ok(recv.clone());
            }
            Ok(pages_to_value(&r))
        }
        "Limit" => {
            args::exactly(a, 1, name)?;
            let n = args::int(a, 0)?;
            if (p.len() as i64) > n {
                if n < 0 {
                    return Err(go_value::Error::new(format!(
                        "runtime error: slice bounds out of range [:{n}]"
                    )));
                }
                return Ok(pages_to_value(&ps::limit(p, n as usize)));
            }
            Ok(recv.clone())
        }
        "GroupBy" => {
            at_least_ctx(a, 1, name)?;
            let key = args::string(a, 0)?;
            let order = strings_vec(a, 1)?;
            let g =
                crate::pagegroup::group_by_ctx(ctx, p, &key.to_str_lossy(), &order).map_err(err)?;
            Ok(pages_group_opt_to_value(g.as_ref()))
        }
        "GroupByParam" => {
            args::at_least(a, 1, name)?;
            let key = args::string(a, 0)?;
            let order = strings_vec(a, 1)?;
            let g =
                crate::pagegroup::group_by_param(p, &key.to_str_lossy(), &order).map_err(err)?;
            Ok(pages_group_opt_to_value(g.as_ref()))
        }
        "GroupByDate" | "GroupByPublishDate" | "GroupByExpiryDate" | "GroupByLastmod" => {
            args::at_least(a, 1, name)?;
            let format = args::string(a, 0)?.to_str_lossy().into_owned();
            let order = strings_vec(a, 1)?;
            let f = match name {
                "GroupByDate" => crate::pagegroup::group_by_date,
                "GroupByPublishDate" => crate::pagegroup::group_by_publish_date,
                "GroupByExpiryDate" => crate::pagegroup::group_by_expiry_date,
                _ => crate::pagegroup::group_by_lastmod,
            };
            let g = f(p, &format, &order).map_err(err)?;
            Ok(pages_group_opt_to_value(g.as_ref()))
        }
        "GroupByParamDate" => {
            args::at_least(a, 2, name)?;
            let key = args::string(a, 0)?.to_str_lossy().into_owned();
            let format = args::string(a, 1)?.to_str_lossy().into_owned();
            let order = strings_vec(a, 2)?;
            let g = crate::pagegroup::group_by_param_date(p, &key, &format, &order).map_err(err)?;
            Ok(pages_group_opt_to_value(g.as_ref()))
        }
        "Next" | "Prev" => {
            args::exactly(a, 1, name)?;
            let Some(cur) = page_arg(a, 0)? else {
                // Go: a nil page is not found (`c.Eq(nil)` is false).
                return Ok(page_opt_value(None));
            };
            let r = if name == "Next" {
                crate::pages_prev_next::next(p, &*cur.0)
            } else {
                crate::pages_prev_next::prev(p, &*cur.0)
            };
            Ok(page_opt_value(r))
        }
        "MergeByLanguage" => {
            args::exactly(a, 1, name)?;
            let p2 = pages_arg(a, 0)?;
            Ok(pages_to_value(
                &crate::pages_language_merge::merge_by_language(p, &p2),
            ))
        }
        "MergeByLanguageInterface" => {
            args::exactly(a, 1, name)?;
            crate::pages_language_merge::merge_by_language_interface(p, &args::get(a, 0)?)
                .map_err(err)
        }
        "Related" => {
            exactly_ctx(a, 1, name)?;
            let r = crate::pages_related::related_opt(ctx, p, &args::get(a, 0)?).map_err(err)?;
            Ok(pages_opt_value(r))
        }
        "RelatedIndices" => {
            at_least_ctx(a, 1, name)?;
            let doc = crate::pages_related::document_arg(&args::get(a, 0)?)
                .map_err(|t| args::wrong_type("related.Document", &Value::string(t)))?;
            let r = crate::pages_related::related_indices(ctx, p, doc, args::rest(a, 1))
                .map_err(err)?;
            Ok(pages_opt_value(r))
        }
        "RelatedTo" => {
            let mut kvs = Vec::with_capacity(a.len());
            for v in a {
                match crate::pages_related::key_values_arg(v) {
                    Some(kv) => kvs.push(kv),
                    None => return Err(args::wrong_type("types.KeyValues", v)),
                }
            }
            let r = crate::pages_related::related_to(ctx, p, kvs).map_err(err)?;
            Ok(pages_opt_value(r))
        }
        _ => Err(go_value::Error::new(format!("{name}: no such method"))),
    }
}

pub const PAGES_METHODS: NamedMethods = NamedMethods {
    has_method: pages_has_method,
    call: pages_call_method,
};

/// Go: `Pages.ProbablyEq(other)` — same length and the same page at every index up to 51, then
/// every 50th.
// Go: resources/page/pages.go:ProbablyEq
pub fn probably_eq(a: &Pages, b: &Pages) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut step = 1;

    let mut i = 0;
    while i < a.len() {
        if a[i].0.page_id() != b[i].0.page_id() {
            return false;
        }

        if i > 50 {
            // This is most likely the same.
            step = 50;
        }
        i += step;
    }

    true
}

/// Go: `Pages.ProbablyEq(other any)` — false unless `other` is a `page.Pages`.
pub fn probably_eq_value(a: &Pages, other: &Value) -> bool {
    if !is_pages_value(other) {
        return false;
    }
    match pages_from_value(other) {
        Ok(b) => probably_eq(a, &b),
        Err(_) => false,
    }
}

/// A page from a value, for callers that hold one (`Eq` semantics: the unwrapped page).
pub fn same_page(a: &PageRef, b: &Value) -> bool {
    page_from_value(b).is_some_and(|b| a.same_page(&b))
}

/// The argument count of a method whose first parameter is a `context.Context` (passed by the
/// template engine): `n` more parameters; Go's counts include the context.
// Go: text/template/exec.go:evalCall (argument count)
fn exactly_ctx(a: &[Value], n: usize, name: &str) -> GoResult<()> {
    if a.len() != n {
        return Err(go_value::Error::eval_call(
            format!(
                "wrong number of args for {name}: want {} got {}",
                n + 1,
                a.len() + 1
            ),
            go_value::EvalCallError::Call,
        ));
    }
    Ok(())
}

/// [`exactly_ctx`] for a variadic method with `n` fixed parameters after the context: Go's
/// "want at least" counts the context, its "got" does not.
// Go: text/template/exec.go:evalCall (variadic argument count)
fn at_least_ctx(a: &[Value], n: usize, name: &str) -> GoResult<()> {
    if a.len() < n {
        return Err(go_value::Error::eval_call(
            format!(
                "wrong number of args for {name}: want at least {} got {}",
                n + 1,
                a.len()
            ),
            go_value::EvalCallError::Call,
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages.go (140 lines; 2/7 funcs executed)
//   types: Pages, PagesFactory
// OK L30-32: (ps Pages) String() string
//    L35-40: (ps Pages) shuffle()   (test helper; not ported)
// OK L44-50: (pages Pages) ToResources() resource.Resources
// OK L53-88: ToPages(seq any) (Pages, error)   (T11: page::pages_from_value)
// OK L92-98: (p Pages) Group(key any, in any) (any, error)
// OK L101-103: (p Pages) Len() int
// OK L107-131: (pages Pages) ProbablyEq(other any) bool
// ---------------------------------------------------------------------------
