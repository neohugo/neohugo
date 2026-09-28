//! Port of `resources/page/pagegroup.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! `page.PageGroup` (`.Key`, `.Pages` + the promoted `page.Pages` methods) and `page.PagesGroup`.
//!
//! `GroupBy` reflects over the `page.Page` interface in Go: the key is a method name, the map key
//! type is the method's first result type, and `sortKeys` sorts int kinds numerically and string
//! kinds with the second collator. [`GROUP_BY_METHODS`] is that interface's method set (name,
//! result class, how `hreflect.CallMethodByName` calls it), checked against Go by the
//! `collections` oracle. Go collects the groups in a map, so keys of other kinds (bool,
//! `time.Time`, pages, ...) come out in random order; the port keeps first-appearance order.

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_time::GoTimeExt;
use go_value::{GoString, HostCtx, Object, SliceType, Time, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::object::{GoResult, NamedMethods, args};

use crate::page::{Page, PageRef, Pages, pages_from_value, pages_to_value};
use crate::pages_sort::{
    by_date, by_expiry_date, by_lastmod, by_publish_date, collator_string_less, page_by_sort,
    reverse,
};

/// Go type string of `page.PageGroup` (a struct value).
pub const PAGE_GROUP_TYPE: &str = "page.PageGroup";
/// Go type string of `page.PagesGroup`.
pub const PAGES_GROUP_TYPE: &str = "page.PagesGroup";

/// Go: `page.PageGroup` (`.Key`, `.Pages`).
#[derive(Clone)]
pub struct PageGroup {
    pub key: Value,
    pub pages: Pages,
}

/// Go: `page.PagesGroup`.
pub type PagesGroup = Vec<PageGroup>;

impl PageGroup {
    pub fn to_value(&self) -> Value {
        Value::object(self.clone())
    }

    /// Go: `PageGroup.ProbablyEq(other)`.
    // Go: resources/page/pagegroup.go:ProbablyEq
    pub fn probably_eq(&self, other: &Value) -> bool {
        let Some(other_p) = other.downcast::<PageGroup>() else {
            return false;
        };

        if !group_key_eq(&self.key, &other_p.key) {
            return false;
        }

        crate::pages::probably_eq(&self.pages, &other_p.pages)
    }

    /// Go: `PageGroup.Slice(in)` — for `collections.Slice`.
    // Go: resources/page/pagegroup.go:Slice
    pub fn slice(&self, input: &Value) -> GoResult<Value> {
        if input.downcast::<PageGroup>().is_some() {
            return Ok(input.clone());
        }
        match input {
            Value::List(l) if l.ty == SliceType::Any => {
                let mut groups = Vec::with_capacity(l.items.len());
                for v in &l.items {
                    match v.downcast::<PageGroup>() {
                        Some(g) => groups.push(g.clone()),
                        None => {
                            return Err(go_value::Error::new(format!(
                                "type {} is not a PageGroup",
                                type_of(v)
                            )));
                        }
                    }
                }
                Ok(pages_group_to_value(&groups))
            }
            _ => Err(go_value::Error::new(format!(
                "invalid slice type {}",
                type_of(input)
            ))),
        }
    }
}

fn type_of(v: &Value) -> String {
    match v {
        Value::Invalid => "<nil>".to_string(),
        _ => v.go_type_name().into_owned(),
    }
}

impl Object for PageGroup {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(PAGE_GROUP_TYPE)
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    /// `ProbablyEq`, `Slice` and the promoted methods of the embedded `Pages`.
    fn has_method(&self, name: &str) -> bool {
        name == "ProbablyEq" || name == "Slice" || crate::pages::pages_has_method(name)
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, a: &[Value]) -> Option<GoResult<Value>> {
        match name {
            "ProbablyEq" => Some(
                args::exactly(a, 1, name)
                    .and_then(|_| Ok(Value::Bool(self.probably_eq(&args::get(a, 0)?)))),
            ),
            "Slice" => Some(args::exactly(a, 1, name).and_then(|_| self.slice(&args::get(a, 0)?))),
            _ => crate::pages::pages_call_method(ctx, &pages_to_value(&self.pages), name, a),
        }
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Key" => Some(self.key.clone()),
            "Pages" => Some(pages_to_value(&self.pages)),
            _ => None,
        }
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("Key"), self.key.clone()),
            (Cow::Borrowed("Pages"), pages_to_value(&self.pages)),
        ])
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `page.PagesGroup` as a template value (`nil` for an empty Go nil result is the caller's
/// choice: see [`pages_group_opt_to_value`]).
pub fn pages_group_to_value(g: &PagesGroup) -> Value {
    Value::list(
        SliceType::Named(Arc::from(PAGES_GROUP_TYPE)),
        g.iter().map(|g| g.to_value()).collect(),
    )
}

/// A Go `PagesGroup` result that is nil when `None`.
pub fn pages_group_opt_to_value(g: Option<&PagesGroup>) -> Value {
    match g {
        Some(g) => pages_group_to_value(g),
        None => Value::TypedNil(Arc::from(PAGES_GROUP_TYPE)),
    }
}

// ---------------------------------------------------------------------------
// Map keys

/// Go `==` on two map keys of the same map (interface values in Go's `map[K]Pages`).
pub fn group_key_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Invalid, Value::Invalid) => true,
        (Value::TypedNil(x), Value::TypedNil(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x, k1), Value::Int(y, k2)) => x == y && k1 == k2,
        (Value::Uint(x, k1), Value::Uint(y, k2)) => x == y && k1 == k2,
        #[allow(clippy::float_cmp)]
        (Value::Float(x, k1), Value::Float(y, k2)) => x == y && k1 == k2,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Safe(k1, x), Value::Safe(k2, y)) => k1 == k2 && x == y,
        (Value::Time(x), Value::Time(y)) => time_eq(x, y),
        (Value::Object(x), Value::Object(y)) => {
            x.type_name() == y.type_name()
                && match (x.underlying(), y.underlying()) {
                    (Some(u1), Some(u2)) => group_key_eq(&u1, &u2),
                    _ => x.identity() == y.identity(),
                }
        }
        _ => false,
    }
}

/// Go `time.Time ==`: the same wall clock, monotonic reading and location pointer.
fn time_eq(a: &Time, b: &Time) -> bool {
    a.go_unix() == b.go_unix()
        && a.nanosecond() == b.nanosecond()
        && Arc::ptr_eq(&a.go_location(), &b.go_location())
}

/// Go: hashing an interface key whose dynamic type is not comparable panics.
fn check_hashable(v: &Value) -> Result<()> {
    match v {
        Value::List(_) | Value::Map(_) => Err(Error::new(format!(
            "runtime error: hash of unhashable type {}",
            v.go_type_name()
        ))),
        _ => Ok(()),
    }
}

/// The groups of a `map[K]Pages` in first-appearance order (Go's map).
#[derive(Default)]
struct GroupMap {
    keys: Vec<Value>,
    pages: Vec<Pages>,
}

impl GroupMap {
    fn add(&mut self, k: Value, p: PageRef) {
        for (i, kk) in self.keys.iter().enumerate() {
            if group_key_eq(kk, &k) {
                self.pages[i].push(p);
                return;
            }
        }
        self.keys.push(k);
        self.pages.push(vec![p]);
    }
}

/// The reflect kind class of a map key that `sortKeys` switches on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    /// `reflect.Int*`.
    Int,
    /// `reflect.String`.
    String,
    /// Anything else: not sorted.
    Other,
}

fn key_int(v: &Value) -> i64 {
    match v {
        Value::Int(n, _) => *n,
        Value::Object(o) => match o.underlying() {
            Some(Value::Int(n, _)) => n,
            _ => 0,
        },
        _ => 0,
    }
}

fn key_str(v: &Value) -> GoString {
    match v {
        Value::String(s) | Value::Safe(_, s) => s.clone(),
        Value::Object(o) => match o.underlying() {
            Some(Value::String(s)) => s,
            _ => GoString::empty(),
        },
        _ => GoString::empty(),
    }
}

/// Go: `sortKeys(examplePage, v, order)` — `sort.Sort` (pdqsort) of the map keys by int value,
/// or by the second collator (`CompareStrings < 1`) for string kinds; other kinds keep their
/// order.
// Go: resources/page/pagegroup.go:sortKeys
fn sort_keys(example_page: &dyn Page, kind: KeyKind, v: &mut GroupMap, order: &str) {
    if v.keys.len() <= 1 {
        return;
    }

    let mut idx: Vec<usize> = (0..v.keys.len()).collect();
    match kind {
        KeyKind::Int => {
            let ints: Vec<i64> = v.keys.iter().map(key_int).collect();
            if order == "desc" {
                go_sort::sort_by(&mut idx, |a, b| ints[*b] < ints[*a]);
            } else {
                go_sort::sort_by(&mut idx, |a, b| ints[*a] < ints[*b]);
            }
        }
        KeyKind::String => {
            let strs: Vec<GoString> = v.keys.iter().map(key_str).collect();
            let string_less = collator_string_less(example_page);
            let mut g = string_less.lock();
            if order == "desc" {
                go_sort::sort_by(&mut idx, |a, b| {
                    g.less(strs[*b].as_bytes(), strs[*a].as_bytes())
                });
            } else {
                go_sort::sort_by(&mut idx, |a, b| {
                    g.less(strs[*a].as_bytes(), strs[*b].as_bytes())
                });
            }
        }
        KeyKind::Other => return,
    }
    let keys = idx.iter().map(|&i| v.keys[i].clone()).collect();
    let pages = idx.iter().map(|&i| v.pages[i].clone()).collect();
    v.keys = keys;
    v.pages = pages;
}

/// Go: `PagesGroup.Reverse()`.
// Go: resources/page/pagegroup.go:Reverse
pub fn pages_group_reverse(p: &PagesGroup) -> PagesGroup {
    let mut p = p.clone();
    p.reverse();
    p
}

/// How `hreflect.CallMethodByName` treats a `page.Page` method and what `GroupBy` does with its
/// first result type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupByKey {
    /// The first result type sorts as an int kind.
    Int,
    /// The first result type sorts as a string kind.
    String,
    /// A comparable type of another kind (not sorted).
    Other,
    /// Not comparable: `reflect.MapOf` panics (`invalid key type <type>`).
    NotComparable(&'static str),
}

/// How the method is called with only the context (Go `CallMethodByName`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupByCall {
    /// No arguments, or the context, or only a variadic parameter.
    Ok,
    /// One non-context parameter: `reflect: Call with too few input arguments`.
    TooFew,
    /// More than one parameter: `not supported`.
    NotSupported,
}

use GroupByCall::{NotSupported as NS, Ok as C, TooFew as TF};
use GroupByKey::{Int as KI, NotComparable as NC, Other as KO, String as KS};

/// The `page.Page` interface's method set as `GroupBy` sees it (Go reflect, generated with the
/// `collections` oracle and checked by it).
pub const GROUP_BY_METHODS: &[(&str, GroupByKey, GroupByCall)] = &[
    ("Aliases", NC("[]string"), C),
    ("AllTranslations", NC("page.Pages"), C),
    ("AlternativeOutputFormats", NC("page.OutputFormats"), C),
    ("Ancestors", NC("page.Pages"), C),
    ("BundleType", KS, C),
    ("CodeOwners", NC("[]string"), C),
    ("Content", KO, C),
    ("ContentWithoutSummary", KS, C),
    ("CurrentSection", KO, C),
    ("Data", KO, C),
    ("Date", KO, C),
    ("Description", KS, C),
    ("Draft", KO, C),
    ("Eq", KO, TF),
    ("ExpiryDate", KO, C),
    ("File", KO, C),
    ("FirstSection", KO, C),
    ("Fragments", KO, C),
    ("FuzzyWordCount", KI, C),
    ("GetPage", KO, TF),
    ("GetTerms", NC("page.Pages"), TF),
    ("GitInfo", KO, C),
    ("HasMenuCurrent", KO, NS),
    ("HasShortcode", KO, TF),
    ("HeadingsFiltered", NC("tableofcontents.Headings"), C),
    ("InSection", KO, TF),
    ("IsAncestor", KO, TF),
    ("IsDescendant", KO, TF),
    ("IsHome", KO, C),
    ("IsMenuCurrent", KO, NS),
    ("IsNode", KO, C),
    ("IsPage", KO, C),
    ("IsSection", KO, C),
    ("IsTranslated", KO, C),
    ("Keywords", NC("[]string"), C),
    ("Kind", KS, C),
    ("Lang", KS, C),
    ("Language", KO, C),
    ("Lastmod", KO, C),
    ("Layout", KS, C),
    ("Len", KI, C),
    ("LinkTitle", KS, C),
    ("Markup", KO, C),
    ("MediaType", KO, C),
    ("Menus", NC("navigation.PageMenus"), C),
    ("Name", KS, C),
    ("Next", KO, C),
    ("NextInSection", KO, C),
    ("NextPage", KO, C),
    ("OutputFormats", NC("page.OutputFormats"), C),
    ("Page", KO, C),
    ("Pages", NC("page.Pages"), C),
    ("Paginate", KO, NS),
    ("Paginator", KO, C),
    ("Param", KO, TF),
    ("Params", NC("maps.Params"), C),
    ("Parent", KO, C),
    ("Path", KS, C),
    ("PathInfo", KO, C),
    ("Permalink", KS, C),
    ("Plain", KS, C),
    ("PlainWords", NC("[]string"), C),
    ("Prev", KO, C),
    ("PrevInSection", KO, C),
    ("PrevPage", KO, C),
    ("PublishDate", KO, C),
    ("RawContent", KS, C),
    ("ReadingTime", KI, C),
    ("Ref", KS, TF),
    ("RefFrom", KS, NS),
    ("RegularPages", NC("page.Pages"), C),
    ("RegularPagesRecursive", NC("page.Pages"), C),
    ("RelPermalink", KS, C),
    ("RelRef", KS, TF),
    ("RelRefFrom", KS, NS),
    ("RelatedKeywords", NC("[]related.Keyword"), TF),
    ("Render", KS, NS),
    ("RenderShortcodes", KS, C),
    ("RenderString", KS, NS),
    ("ResourceType", KS, C),
    ("Resources", NC("resource.Resources"), C),
    ("Scratch", KO, C),
    ("Section", KS, C),
    ("Sections", NC("page.Pages"), C),
    ("SectionsEntries", NC("[]string"), C),
    ("SectionsPath", KS, C),
    ("Site", KO, C),
    ("Sitemap", KO, C),
    ("Sites", NC("page.Sites"), C),
    ("Slug", KS, C),
    ("Store", KO, C),
    ("String", KS, C),
    ("Summary", KS, C),
    ("TableOfContents", KS, C),
    ("Title", KS, C),
    ("TranslationKey", KS, C),
    ("Translations", NC("page.Pages"), C),
    ("Truncated", KO, C),
    ("Type", KS, C),
    ("Weight", KI, C),
    ("WordCount", KI, C),
];

/// Go: `strings.ToLower(order[0])` in ("desc", "rev", "reverse").
fn direction(order: &[String]) -> &'static str {
    if let Some(o) = order.first() {
        let o: &[u8] = &go_unicode::strings::to_lower(o.as_bytes());
        if o == b"desc" || o == b"rev" || o == b"reverse" {
            return "desc";
        }
    }
    "asc"
}

/// Go: `Pages.GroupBy(ctx, key, order...)` — groups by the value of the `page.Page` method
/// `key` (exact case); `None` is Go's nil result for an empty list. Go panics for a key that is
/// not a method (`reflect: Elem of invalid type page.Page`), for result types that cannot be map
/// keys and for methods that need arguments; the port returns those panic messages as errors.
// Go: resources/page/pagegroup.go:GroupBy
pub fn group_by_ctx(
    ctx: HostCtx<'_>,
    p: &Pages,
    key: &str,
    order: &[String],
) -> Result<Option<PagesGroup>> {
    if p.is_empty() {
        return Ok(None);
    }

    let direction = direction(order);

    let Some(&(_, kind, call)) = GROUP_BY_METHODS.iter().find(|(n, _, _)| *n == key) else {
        // Go: `pagePtrType.Elem()` on an interface type.
        return Err(Error::new("reflect: Elem of invalid type page.Page"));
    };
    let kind = match kind {
        GroupByKey::NotComparable(t) => {
            return Err(Error::new(format!("reflect.MapOf: invalid key type {t}")));
        }
        GroupByKey::Int => KeyKind::Int,
        GroupByKey::String => KeyKind::String,
        GroupByKey::Other => KeyKind::Other,
    };

    let mut tmp = GroupMap::default();
    for e in p {
        match call {
            GroupByCall::TooFew => {
                return Err(Error::new("reflect: Call with too few input arguments"));
            }
            GroupByCall::NotSupported => return Err(Error::new("not supported")),
            GroupByCall::Ok => {}
        }
        let fv = match e.0.tpl_call_method(ctx, key, &[]) {
            Some(Ok(v)) => v,
            // Go uses the first result only; with an error it is the zero value.
            Some(Err(_)) | None => match kind {
                KeyKind::Int => Value::int(0),
                KeyKind::String => Value::string(""),
                KeyKind::Other => Value::Invalid,
            },
        };
        if kind == KeyKind::Other {
            check_hashable(&fv)?;
        }
        tmp.add(fv, e.clone());
    }

    sort_keys(&*p[0].0, kind, &mut tmp, direction);
    let r = tmp
        .keys
        .into_iter()
        .zip(tmp.pages)
        .map(|(key, pages)| PageGroup { key, pages })
        .collect();

    Ok(Some(r))
}

/// Go: `Pages.GroupBy(ctx, key, order...)` without a template context.
// Go: resources/page/pagegroup.go:GroupBy
pub fn group_by(p: &Pages, key: &str, order: &[String]) -> Result<PagesGroup> {
    Ok(group_by_ctx(&(), p, key, order)?.unwrap_or_default())
}

/// The kind `sortKeys` sees for a param value's dynamic type.
fn param_key_kind(v: &Value) -> KeyKind {
    match v {
        Value::Int(..) => KeyKind::Int,
        Value::String(_) | Value::Safe(..) => KeyKind::String,
        Value::Object(o) => match o.underlying() {
            Some(Value::Int(..)) => KeyKind::Int,
            Some(Value::String(_)) => KeyKind::String,
            _ => KeyKind::Other,
        },
        _ => KeyKind::Other,
    }
}

/// Go: `Pages.GroupByParam(key, order...)` — the key type is the dynamic type of the first
/// non-nil, non-`[]string` param (lower-cased strings); pages whose param has another type are
/// left out. `None` is Go's nil result.
// Go: resources/page/pagegroup.go:GroupByParam
pub fn group_by_param(p: &Pages, key: &str, order: &[String]) -> Result<Option<PagesGroup>> {
    if p.is_empty() {
        return Ok(None);
    }

    let direction = direction(order);

    let mut keyt: Option<Cow<'static, str>> = None;
    for e in p {
        let param = nh_resource::resource_helpers::get_param_to_lower(&*e.0, key);
        if !param.is_invalid() && param.go_type_name() != "[]string" {
            let t = param.go_type_name().into_owned();
            if matches!(param, Value::List(_) | Value::Map(_)) {
                return Err(Error::new(format!("reflect.MapOf: invalid key type {t}")));
            }
            keyt = Some(Cow::Owned(t));
            break;
        }
    }
    let Some(keyt) = keyt else {
        return Ok(None);
    };

    let mut tmp = GroupMap::default();
    let mut kind = KeyKind::Other;
    for e in p {
        let param = nh_resource::resource_helpers::get_param_of(&*e.0, key);

        if param.is_invalid() || param.go_type_name() != keyt {
            continue;
        }
        kind = param_key_kind(&param);
        tmp.add(param, e.clone());
    }

    sort_keys(&*p[0].0, kind, &mut tmp, direction);
    let r: PagesGroup = tmp
        .keys
        .into_iter()
        .zip(tmp.pages)
        .map(|(key, pages)| PageGroup { key, pages })
        .collect();

    // Go: `var r []PageGroup` stays nil without keys.
    if r.is_empty() {
        return Ok(None);
    }
    Ok(Some(r))
}

/// Go: `Pages.groupByDateField(format, sorter, getDate, order...)` — sorted by the date, reversed
/// unless the order is `asc`, `rev` or `reverse` (sic), grouped by consecutive equal formatted
/// dates (the current site's time formatter).
// Go: resources/page/pagegroup.go:groupByDateField
fn group_by_date_field(
    p: &Pages,
    format: &str,
    sorter: &dyn Fn(&Pages) -> Pages,
    get_date: &dyn Fn(&dyn Page) -> Time,
    order: &[String],
) -> Result<Option<PagesGroup>> {
    if p.is_empty() {
        return Ok(None);
    }

    let mut sp = sorter(p);

    let keep = order.first().is_some_and(|o| {
        let o: &[u8] = &go_unicode::strings::to_lower(o.as_bytes());
        o == b"asc" || o == b"rev" || o == b"reverse"
    });
    if !keep {
        sp = reverse(&sp);
    }

    let first_page = &sp[0];
    let date = get_date(&*first_page.0);

    // Pages may be a mix of multiple languages, so we need to use the language for the
    // currently rendered Site.
    let current_site = first_page.0.site().0.current();
    let lang = current_site.0.language();
    let formatter = lang.time_formatter();
    let formatted = formatter.format_bytes(&date, format.as_bytes());
    let mut r: PagesGroup = vec![PageGroup {
        key: Value::String(GoString::from(formatted.clone())),
        pages: vec![sp[0].clone()],
    }];
    let mut cur_key = formatted;

    let mut i = 0;
    for e in &sp[1..] {
        let date = get_date(&*e.0);
        let formatted = formatter.format_bytes(&date, format.as_bytes());
        if cur_key != formatted {
            r.push(PageGroup {
                key: Value::String(GoString::from(formatted.clone())),
                pages: Vec::new(),
            });
            cur_key = formatted;
            i += 1;
        }
        r[i].pages.push(e.clone());
    }
    Ok(Some(r))
}

/// Go: `Pages.GroupByDate(format, order...)`.
// Go: resources/page/pagegroup.go:GroupByDate
pub fn group_by_date(p: &Pages, format: &str, order: &[String]) -> Result<Option<PagesGroup>> {
    group_by_date_field(p, format, &by_date, &|p| p.date(), order)
}

/// Go: `Pages.GroupByPublishDate(format, order...)`.
// Go: resources/page/pagegroup.go:GroupByPublishDate
pub fn group_by_publish_date(
    p: &Pages,
    format: &str,
    order: &[String],
) -> Result<Option<PagesGroup>> {
    group_by_date_field(p, format, &by_publish_date, &|p| p.publish_date(), order)
}

/// Go: `Pages.GroupByExpiryDate(format, order...)`.
// Go: resources/page/pagegroup.go:GroupByExpiryDate
pub fn group_by_expiry_date(
    p: &Pages,
    format: &str,
    order: &[String],
) -> Result<Option<PagesGroup>> {
    group_by_date_field(p, format, &by_expiry_date, &|p| p.expiry_date(), order)
}

/// Go: `Pages.GroupByLastmod(format, order...)`.
// Go: resources/page/pagegroup.go:GroupByLastmod
pub fn group_by_lastmod(p: &Pages, format: &str, order: &[String]) -> Result<Option<PagesGroup>> {
    group_by_date_field(p, format, &by_lastmod, &|p| p.lastmod(), order)
}

/// Go: `Pages.GroupByParamDate(key, format, order...)` — the param as a `time.Time` (or
/// `cast.ToTime` of it; the zero time when missing), stable-sorted by its Unix time.
// Go: resources/page/pagegroup.go:GroupByParamDate
pub fn group_by_param_date(
    p: &Pages,
    key: &str,
    format: &str,
    order: &[String],
) -> Result<Option<PagesGroup>> {
    // Cache the dates (Go: `map[Page]time.Time`, keyed by the page value).
    let dates: std::cell::RefCell<Vec<(PageRef, Time)>> = std::cell::RefCell::new(Vec::new());
    let lookup = |p: &dyn Page, dates: &[(PageRef, Time)]| -> Time {
        dates
            .iter()
            .rev()
            .find(|(q, _)| q.0.page_id() == p.page_id() && q.0.tpl_type_name() == p.tpl_type_name())
            .map(|(_, t)| t.clone())
            .unwrap_or_else(Time::zero)
    };

    let sorter = |pages: &Pages| -> Pages {
        let mut r: Pages = Vec::new();

        for p in pages {
            let param = nh_resource::resource_helpers::get_param_of(&*p.0, key);
            let mut t = Time::zero();

            if !param.is_invalid() {
                t = match &param {
                    Value::Time(t) => t.clone(),
                    // Probably a string. Try to convert it to time.Time.
                    _ => nh_common::cast::time::to_time(&param),
                };
            }

            dates.borrow_mut().push((p.clone(), t));
            r.push(p.clone());
        }

        let d = dates.borrow();
        let pdate =
            |p1: &dyn Page, p2: &dyn Page| lookup(p1, &d).go_unix() < lookup(p2, &d).go_unix();
        page_by_sort(&pdate, &mut r);
        r
    };
    let get_date = |p: &dyn Page| lookup(p, &dates.borrow());
    group_by_date_field(p, format, &sorter, &get_date, order)
}

/// Go: `PagesGroup.Len()` — the number of pages in all groups.
// Go: resources/page/pagegroup.go:Len
pub fn pages_group_len(psg: &PagesGroup) -> i64 {
    psg.iter().map(|pg| pg.pages.len() as i64).sum()
}

/// Go: `PagesGroup.ProbablyEq(other)`.
// Go: resources/page/pagegroup.go:ProbablyEq
pub fn pages_group_probably_eq(psg: &PagesGroup, other: &Value) -> bool {
    let Some(other_psg) = pages_group_from_value(other) else {
        return false;
    };
    if !is_pages_group(other) {
        return false;
    }

    if psg.len() != other_psg.len() {
        return false;
    }

    for i in 0..psg.len() {
        if !psg[i].probably_eq(&other_psg[i].to_value()) {
            return false;
        }
    }

    true
}

fn is_pages_group(v: &Value) -> bool {
    match v {
        Value::List(l) => matches!(&l.ty, SliceType::Named(n) if &**n == PAGES_GROUP_TYPE),
        Value::TypedNil(t) => &**t == PAGES_GROUP_TYPE,
        _ => false,
    }
}

/// `page.PagesGroup` (or `[]page.PageGroup`) from a template value.
pub fn pages_group_from_value(v: &Value) -> Option<PagesGroup> {
    match v {
        Value::List(l) if matches!(&l.ty, SliceType::Named(n) if &**n == PAGES_GROUP_TYPE || &**n == "[]page.PageGroup") => {
            l.items
                .iter()
                .map(|it| it.downcast::<PageGroup>().cloned())
                .collect()
        }
        Value::TypedNil(t) if &**t == PAGES_GROUP_TYPE || &**t == "[]page.PageGroup" => {
            Some(Vec::new())
        }
        _ => None,
    }
}

/// Go: `page.ToPagesGroup(seq)` — `Ok(Some(groups))` is Go's `ok == true` (a nil `seq` gives
/// empty groups), `Ok(None)` is `ok == false`.
// Go: resources/page/pagegroup.go:ToPagesGroup
pub fn to_pages_group(v: &Value) -> Result<Option<PagesGroup>> {
    if v.is_invalid() {
        return Ok(Some(Vec::new()));
    }
    if let Some(g) = pages_group_from_value(v) {
        return Ok(Some(g));
    }
    if let Value::List(l) = v
        && l.ty == SliceType::Any
        && let Some(first) = l.items.first()
        && first.downcast::<PageGroup>().is_some()
    {
        let mut pages_group = Vec::with_capacity(l.items.len());
        for ipg in &l.items {
            match ipg.downcast::<PageGroup>() {
                Some(pg) => pages_group.push(pg.clone()),
                None => {
                    return Err(Error::new(format!(
                        "unsupported type in paginate from slice, got {} instead of PageGroup",
                        type_of(ipg)
                    )));
                }
            }
        }
        return Ok(Some(pages_group));
    }

    Ok(None)
}

/// Methods of the named slice `page.PagesGroup`.
pub fn pages_group_has_method(name: &str) -> bool {
    matches!(name, "Reverse" | "Len" | "ProbablyEq")
}

pub fn pages_group_call_method(
    _ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    a: &[Value],
) -> Option<GoResult<Value>> {
    if !pages_group_has_method(name) {
        return None;
    }
    let g = pages_group_from_value(recv)?;
    Some(match name {
        "Reverse" => args::exactly(a, 0, name).map(|_| {
            if matches!(recv, Value::TypedNil(_)) {
                return recv.clone();
            }
            pages_group_to_value(&pages_group_reverse(&g))
        }),
        "Len" => args::exactly(a, 0, name).map(|_| Value::int(pages_group_len(&g))),
        _ => args::exactly(a, 1, name)
            .and_then(|_| Ok(Value::Bool(pages_group_probably_eq(&g, &args::get(a, 0)?)))),
    })
}

pub const PAGES_GROUP_METHODS: NamedMethods = NamedMethods {
    has_method: pages_group_has_method,
    call: pages_group_call_method,
};

/// `Pages` from a PageGroup value (Go `ToPages` of a `PageGroup`).
pub fn page_group_pages(v: &Value) -> Result<Pages> {
    pages_from_value(v)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pagegroup.go (461 lines; 1/19 funcs executed)
//   types: PageGroup, mapKeyValues, mapKeyByInt, mapKeyByStr, PagesGroup
// OK L53-53: (v mapKeyValues) Len() int
// OK L54-54: (v mapKeyValues) Swap(i, j int)
// OK L58-58: (s mapKeyByInt) Less(i, j int) bool
// OK L65-67: (s mapKeyByStr) Less(i, j int) bool
// OK L69-91: sortKeys(examplePage Page, v []reflect.Value, order string) []reflect.Value
// OK L98-104: (p PagesGroup) Reverse() PagesGroup
// OK L114-180: (p Pages) GroupBy(ctx context.Context, key string, order ...string) (PagesGroup, error)
// OK L184-230: (p Pages) GroupByParam(key string, order ...string) (PagesGroup, error)
// OK L232-270: (p Pages) groupByDateField(format string, sorter func(p Pages) Pages, getDate func(p Page) time.Time, order ...string) (PagesGroup, error)
// OK L276-284: (p Pages) GroupByDate(format string, order ...string) (PagesGroup, error)
// OK L290-298: (p Pages) GroupByPublishDate(format string, order ...string) (PagesGroup, error)
// OK L304-312: (p Pages) GroupByExpiryDate(format string, order ...string) (PagesGroup, error)
// OK L318-326: (p Pages) GroupByLastmod(format string, order ...string) (PagesGroup, error)
// OK L332-365: (p Pages) GroupByParamDate(key string, format string, order ...string) (PagesGroup, error)
// OK L369-380: (p PageGroup) ProbablyEq(other any) bool
// OK L384-401: (p PageGroup) Slice(in any) (any, error)
// OK L404-410: (psg PagesGroup) Len() int
// OK L413-430: (psg PagesGroup) ProbablyEq(other any) bool
// OK L433-461: ToPagesGroup(seq any) (PagesGroup, bool, error)
// ---------------------------------------------------------------------------
