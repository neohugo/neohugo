//! Page list orders and groupings (`by_title` … `by_weight`, `group_by_date`,
//! `group_by_param`) and taxonomy term orders (`by_count`, `alphabetical`).
//!
//! Page lists are ordered by what the model knows about each page (its id), collated in the
//! list's language; every order is a stable sort, so ties keep the input order.

use std::cmp::Ordering;
use std::sync::Arc;

use jiff::Zoned;
use neohugo_base::{Collate, IdVec, Idx, LangIdx, PageId};
use neohugo_locale::Collator;
use neohugo_page::{SortKey, default_order};
use neohugo_view::ViewCache;
use tera::{Kwargs, State, TeraResult, Value};

use crate::Handles;
use crate::call::{
    Registrar, SiteFilter, chain, field, list, map_value, msg, page_id, render_lang, scope,
};

/// Unix seconds of Hugo's zero date (pages without the date).
const ZERO_DATE_SECONDS: i64 = -62_135_596_800;

/// The collators of the site languages.
pub(crate) struct Collators(IdVec<LangIdx, Collator>);

impl Collators {
    fn new(views: &ViewCache) -> Self {
        Self(
            views
                .model()
                .config
                .sites
                .iter()
                .map(|s| Collator::for_language(&s.language.key))
                .collect(),
        )
    }

    fn get(&self, lang: LangIdx) -> &Collator {
        self.0
            .get(lang)
            .unwrap_or_else(|| &self.0[LangIdx::from_index(0)])
    }
}

pub(crate) fn register(r: &mut Registrar<'_>, h: &Handles) {
    let collators = Arc::new(Collators::new(&h.views));
    for (name, by) in [
        ("by_title", By::Title),
        ("by_link_title", By::LinkTitle),
        ("by_date", By::Date),
        ("by_publish_date", By::PublishDate),
        ("by_lastmod", By::Lastmod),
        ("by_weight", By::Weight),
    ] {
        r.filter(
            name,
            SortPages {
                views: Arc::clone(&h.views),
                collators: Arc::clone(&collators),
                by,
            },
        );
    }
    r.filter(
        "group_by_date",
        GroupByDate {
            views: Arc::clone(&h.views),
        },
    );
    r.filter(
        "group_by_param",
        GroupByParam {
            views: Arc::clone(&h.views),
            collators: Arc::clone(&collators),
        },
    );
    r.filter(
        "by_count",
        SortTerms {
            views: Arc::clone(&h.views),
            collators: Arc::clone(&collators),
            by_count: true,
        },
    );
    r.filter(
        "alphabetical",
        SortTerms {
            views: Arc::clone(&h.views),
            collators,
            by_count: false,
        },
    );
}

/// The pages of a list value with their ids.
fn pages(views: &ViewCache, v: &Value, what: &str) -> TeraResult<Vec<(Value, PageId)>> {
    if v.is_none() || v.is_undefined() {
        return Ok(Vec::new());
    }
    list(v, what)?
        .iter()
        .map(|p| Ok((p.clone(), page_id(views, p, what)?)))
        .collect()
}

/// The language a list is collated in: the render's, else its first page's.
fn list_lang(views: &ViewCache, st: &State, items: &[(Value, PageId)]) -> TeraResult<LangIdx> {
    if let Some(s) = scope(st)? {
        return Ok(s.lang);
    }
    match items.first() {
        Some((_, id)) => Ok(views.model().pages[*id].lang),
        None => render_lang(views.model(), st),
    }
}

fn seconds(d: Option<&Zoned>) -> i64 {
    d.map_or(ZERO_DATE_SECONDS, |d| d.timestamp().as_second())
}

/// What Hugo's default order reads from a page.
fn sort_key(p: &neohugo_site::Page) -> SortKey<'_> {
    SortKey {
        weight: p.meta.weight,
        date: p.meta.dates.date.as_ref(),
        link_title: &p.link_title,
        path: &p.path_info.path,
        ordinal: None,
        weight0: None,
    }
}

#[derive(Clone, Copy)]
enum By {
    Title,
    LinkTitle,
    Date,
    PublishDate,
    Lastmod,
    Weight,
}

/// `by_title`, `by_link_title` (collation), `by_date`, `by_publish_date`, `by_lastmod`
/// (oldest first), `by_weight` (Hugo's default order).
struct SortPages {
    views: Arc<ViewCache>,
    collators: Arc<Collators>,
    by: By,
}

impl SiteFilter for SortPages {
    fn call(&self, v: Value, _: &Kwargs, st: &State) -> TeraResult<Value> {
        let mut items = pages(&self.views, &v, "a page order filter")?;
        let c = self.collators.get(list_lang(&self.views, st, &items)?);
        let model = self.views.model();
        let page = |id: PageId| &model.pages[id];
        items.sort_by(|(_, a), (_, b)| {
            let (a, b) = (page(*a), page(*b));
            match self.by {
                By::Title => c.compare(&a.title, &b.title),
                By::LinkTitle => c.compare(&a.link_title, &b.link_title),
                By::Date => {
                    seconds(a.meta.dates.date.as_ref()).cmp(&seconds(b.meta.dates.date.as_ref()))
                }
                By::PublishDate => seconds(a.meta.dates.publish_date.as_ref())
                    .cmp(&seconds(b.meta.dates.publish_date.as_ref())),
                By::Lastmod => seconds(a.meta.dates.lastmod.as_ref())
                    .cmp(&seconds(b.meta.dates.lastmod.as_ref())),
                By::Weight => default_order(&sort_key(a), &sort_key(b), c),
            }
        });
        Ok(Value::from(
            items.into_iter().map(|(v, _)| v).collect::<Vec<_>>(),
        ))
    }
}

/// `group_by_date(format=, attribute=?)`: `[{key, pages}]` by the date (`date` by default;
/// `publish_date`, `lastmod`, `expiry_date`) formatted with strftime `format`, newest first.
struct GroupByDate {
    views: Arc<ViewCache>,
}

impl SiteFilter for GroupByDate {
    fn call(&self, v: Value, kw: &Kwargs, _: &State) -> TeraResult<Value> {
        let format = kw.must_get::<&str>("format")?;
        let attribute = kw.get::<&str>("attribute")?.unwrap_or("date");
        let mut items = pages(&self.views, &v, "group_by_date")?;
        let model = self.views.model();
        let date = |id: PageId| -> TeraResult<Option<&Zoned>> {
            let d = &model.pages[id].meta.dates;
            Ok(
                match attribute.to_ascii_lowercase().replace('_', "").as_str() {
                    "date" => d.date.as_ref(),
                    "publishdate" => d.publish_date.as_ref(),
                    "lastmod" => d.lastmod.as_ref(),
                    "expirydate" => d.expiry_date.as_ref(),
                    other => {
                        return Err(msg(format!(
                            "group_by_date(attribute=\"{other}\"): expected date, publish_date, \
                         lastmod or expiry_date"
                        )));
                    }
                },
            )
        };
        let mut keyed = Vec::with_capacity(items.len());
        for (v, id) in items.drain(..) {
            let d = date(id)?;
            let key = match d {
                Some(d) => jiff::fmt::strtime::format(format, d)
                    .map_err(|e| chain(format!("group_by_date(format=\"{format}\")"), e))?,
                None => String::new(),
            };
            keyed.push((seconds(d), key, v));
        }
        keyed.sort_by(|a, b| b.0.cmp(&a.0));
        let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
        for (_, key, v) in keyed {
            match groups.last_mut() {
                Some((k, pages)) if *k == key => pages.push(v),
                _ => groups.push((key, vec![v])),
            }
        }
        Ok(groups_value(
            groups
                .into_iter()
                .map(|(k, p)| (Value::from(k), p))
                .collect(),
        ))
    }
}

fn groups_value(groups: Vec<(Value, Vec<Value>)>) -> Value {
    Value::from(
        groups
            .into_iter()
            .map(|(key, pages)| map_value([("key", key), ("pages", Value::from(pages))]))
            .collect::<Vec<_>>(),
    )
}

/// `group_by_param(param=)`: `[{key, pages}]` by the page param `param` (a dotted path), keys
/// ascending (numbers numerically, text by collation); pages without it are left out.
struct GroupByParam {
    views: Arc<ViewCache>,
    collators: Arc<Collators>,
}

impl SiteFilter for GroupByParam {
    fn call(&self, v: Value, kw: &Kwargs, st: &State) -> TeraResult<Value> {
        let param = kw.must_get::<&str>("param")?.to_lowercase();
        let items = pages(&self.views, &v, "group_by_param")?;
        let c = self.collators.get(list_lang(&self.views, st, &items)?);
        let model = self.views.model();
        let mut groups: Vec<(Value, Vec<Value>)> = Vec::new();
        for (v, id) in items {
            let params = model.pages[id].params();
            let Some(key) = params.get(&param).or_else(|| params.get_path(&param)) else {
                continue;
            };
            let key = key.to_tera();
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, pages)) => pages.push(v),
                None => groups.push((key, vec![v])),
            }
        }
        groups.sort_by(|(a, _), (b, _)| compare_keys(a, b, c));
        Ok(groups_value(groups))
    }
}

fn compare_keys(a: &Value, b: &Value, c: &Collator) -> Ordering {
    match (
        a.as_f64().filter(|_| a.is_number()),
        b.as_f64().filter(|_| b.is_number()),
    ) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        _ => c.compare(&a.to_string(), &b.to_string()),
    }
}

/// `by_count` (most pages first, then name) and `alphabetical` (name) over the terms of a
/// taxonomy (`site.taxonomies.tags`, a map of term entries, or a list of them).
struct SortTerms {
    views: Arc<ViewCache>,
    collators: Arc<Collators>,
    by_count: bool,
}

impl SiteFilter for SortTerms {
    fn call(&self, v: Value, _: &Kwargs, st: &State) -> TeraResult<Value> {
        let what = if self.by_count {
            "by_count"
        } else {
            "alphabetical"
        };
        let mut terms: Vec<Value> = if let Some(m) = v.as_map() {
            m.values().cloned().collect()
        } else if v.is_none() || v.is_undefined() {
            Vec::new()
        } else {
            list(&v, what)?.to_vec()
        };
        let lang = match scope(st)? {
            Some(s) => s.lang,
            None => render_lang(self.views.model(), st)?,
        };
        let c = self.collators.get(lang);
        let name = |t: &Value| {
            field(t, "name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let count = |t: &Value| field(t, "count").and_then(Value::as_u64).unwrap_or(0);
        if terms.iter().any(|t| field(t, "name").is_none()) {
            return Err(msg(format!("{what}: expected taxonomy terms")));
        }
        terms.sort_by(|a, b| {
            let by_name = || c.compare(&name(a), &name(b));
            if self.by_count {
                count(b).cmp(&count(a)).then_with(by_name)
            } else {
                by_name()
            }
        });
        Ok(Value::from(terms))
    }
}
