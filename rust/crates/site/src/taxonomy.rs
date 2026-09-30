//! Taxonomies and terms (part of phase B3).
//!
//! Every linked page of a language, in tree order, names its terms in front matter: `tags:
//! [a, b]` (a single value is one term; numbers and booleans are written out; a list holding a
//! list or a map names none). A term's key is `/<plural>/<value>` read as a content path
//! (lower case, spaces as `-`, a `/` nests); the page at that key is the term page (made when
//! missing). A page's weight in a taxonomy is its `<plural>_weight`. The term's `.Data.Term`
//! is the value last written for it; a term page nobody names keeps its path's name.

use std::collections::BTreeMap;

use neohugo_base::diag::Diagnostic;
use neohugo_base::paths::ContentKey;
use neohugo_base::{IdVec, LangIdx, PageId, PageKind, TaxonomyIdx, TermIdx, Value};
use neohugo_config::SiteConfig;
use neohugo_config::sections::TaxonomyDef;

use crate::nodes::Maker;
use crate::relations::{self, Collators};
use crate::{ListScope, Model, ModelError, Removed};

/// A configured taxonomy of one language.
#[derive(Clone, Debug)]
pub struct Taxonomy {
    pub def: TaxonomyDef,
    /// The taxonomy page (`None` when the build removed it).
    pub page: Option<PageId>,
    /// The term pages below the taxonomy page, by key.
    pub terms: IdVec<TermIdx, Term>,
}

/// A term of a taxonomy.
#[derive(Clone, Debug)]
pub struct Term {
    /// The term page's key (`tags/blue-sky`).
    pub key: ContentKey,
    /// `.Data.Term`: the value last written for the term (`Blue Sky`), else its path's name.
    pub term: String,
    pub page: PageId,
    /// The pages that name the term: by weight, then in the default order.
    pub members: Vec<WeightedPage>,
}

/// A page in a term with its weight (`tags_weight`) and position among the page's values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeightedPage {
    pub page: PageId,
    pub weight: i32,
    pub ordinal: u32,
}

impl Taxonomy {
    /// The terms listed in `.Site.Taxonomies`: listed site-wide and named by a page.
    pub fn listed_terms<'a>(&'a self, m: &'a Model) -> impl Iterator<Item = (TermIdx, &'a Term)> {
        self.terms.iter_enumerated().filter(move |(_, t)| {
            !t.members.is_empty() && m.pages[t.page].listed(ListScope::Global)
        })
    }
}

/// A site's taxonomies in the order Hugo walks them: by plural, each plural once.
pub(crate) fn views(site: &SiteConfig) -> Vec<TaxonomyIdx> {
    let mut v: Vec<TaxonomyIdx> = site.taxonomies.ids().collect();
    v.sort_by(|a, b| site.taxonomies[*a].plural.cmp(&site.taxonomies[*b].plural));
    v.dedup_by(|a, b| site.taxonomies[*a].plural == site.taxonomies[*b].plural);
    v
}

/// The term values of a front matter value: `None` when it names no terms.
fn term_values(v: &Value) -> Option<Vec<String>> {
    fn scalar(v: &Value) -> Option<String> {
        match v {
            Value::Null => Some(String::new()),
            Value::String(s) => Some(s.to_string()),
            Value::Int(i) => Some(i.to_string()),
            Value::Float(f) => Some(f.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Date(_) | Value::Array(_) | Value::Map(_) => None,
        }
    }
    match v {
        Value::Null => None,
        Value::Array(items) => items.iter().map(scalar).collect(),
        other => scalar(other).map(|s| vec![s]),
    }
}

/// A `<plural>_weight` value as an integer; `Err` for a value that is not one.
fn weight_of(v: Option<&Value>) -> Result<i32, ()> {
    let w = match v {
        None | Some(Value::Null) => 0,
        Some(Value::Int(i)) => *i,
        #[allow(clippy::cast_possible_truncation)]
        Some(Value::Float(f)) => f.trunc() as i64,
        Some(Value::Bool(b)) => i64::from(*b),
        Some(Value::String(s)) => {
            let s = s.trim();
            let s = s.strip_suffix(".0").unwrap_or(s);
            s.parse::<i64>().map_err(|_| ())?
        }
        Some(Value::Date(_) | Value::Array(_) | Value::Map(_)) => return Err(()),
    };
    i32::try_from(w).map_err(|_| ())
}

/// The term pages of one language, their members and the language's taxonomy tables.
pub(crate) fn assemble_terms(
    m: &mut Model,
    maker: &Maker<'_>,
    lang: LangIdx,
    removed: &[Removed],
) -> Result<(), ModelError> {
    let cfg = m.config.clone();
    let site = &cfg.sites[lang];
    // (term page, member) → (weight, ordinal); the last value naming the term.
    let mut entries: BTreeMap<(PageId, PageId), (i32, u32)> = BTreeMap::new();
    let mut last_value: BTreeMap<PageId, String> = BTreeMap::new();
    if !site.disable_kinds.contains(PageKind::Term) {
        let pages: Vec<PageId> = m.sites[lang].tree.iter().map(|(_, id)| id).collect();
        let views = views(site);
        for id in pages {
            if !m.pages[id].linked() {
                continue;
            }
            for &idx in &views {
                let plural = &site.taxonomies[idx].plural;
                let Some(values) = m.pages[id].params().get(plural).and_then(term_values) else {
                    continue;
                };
                let weight = match weight_of(m.pages[id].params().get(&format!("{plural}_weight")))
                {
                    Ok(w) => w,
                    Err(()) => {
                        m.diagnostics.push(
                            Diagnostic::warning(format!(
                                "{}: {plural}_weight is not an integer",
                                m.pages[id].path()
                            ))
                            .with_id("taxonomy-weight"),
                        );
                        0
                    }
                };
                for (i, v) in values.into_iter().enumerate() {
                    if v.is_empty() {
                        continue;
                    }
                    let Some(info) = maker.parse(&format!("/{plural}/{v}/_index.md")) else {
                        continue;
                    };
                    let key = info.key.clone();
                    // A content term page the build filter removed takes its values with it.
                    let gone = || {
                        removed
                            .iter()
                            .any(|r| r.lang == lang && r.kind == PageKind::Term && r.key == key)
                    };
                    let term = match m.sites[lang].tree.get(&key) {
                        Some(t) => t,
                        None if gone() => continue,
                        None => match maker.make(m, lang, PageKind::Term, key, info)? {
                            Some(t) => t,
                            None => continue,
                        },
                    };
                    last_value.insert(term, v);
                    let ordinal = u32::try_from(i).unwrap_or(u32::MAX);
                    entries.insert((term, id), (weight, ordinal));
                }
            }
        }
    }

    // The tables, with the term pages in key order.
    let mut taxonomies: IdVec<TaxonomyIdx, Taxonomy> = IdVec::new();
    for (idx, def) in site.taxonomies.iter_enumerated() {
        let plural = ContentKey::from_source(&def.plural);
        let page = m.sites[lang].tree.get(&plural);
        if let Some(p) = page {
            m.pages[p].taxonomy = Some(idx);
        }
        let mut terms: IdVec<TermIdx, Term> = IdVec::new();
        let below: Vec<(ContentKey, PageId)> = m.sites[lang]
            .tree
            .descendants(&plural)
            .filter(|_| !plural.is_home())
            .map(|(k, id)| (k.clone(), id))
            .collect();
        for (key, id) in below {
            if m.pages[id].kind != PageKind::Term {
                continue;
            }
            let term = last_value.get(&id).cloned().unwrap_or_else(|| {
                let base = &m.pages[id].path_info.original.base;
                base.strip_prefix(&format!("/{}", def.plural))
                    .unwrap_or(base)
                    .trim_start_matches('/')
                    .to_owned()
            });
            let members = entries
                .range((id, PageId::from_raw(0))..=(id, PageId::from_raw(u32::MAX)))
                .map(|(&(_, page), &(weight, ordinal))| WeightedPage {
                    page,
                    weight,
                    ordinal,
                })
                .collect();
            let tidx = terms.push(Term {
                key,
                term,
                page: id,
                members,
            });
            let p = &mut m.pages[id];
            p.taxonomy = Some(idx);
            p.term = Some(tidx);
        }
        taxonomies.push(Taxonomy {
            def: def.clone(),
            page,
            terms,
        });
    }

    // `.GetTerms` of every member.
    let mut by_member: BTreeMap<PageId, Vec<(TaxonomyIdx, u32, TermIdx)>> = BTreeMap::new();
    for (idx, t) in taxonomies.iter_enumerated() {
        for (tidx, term) in t.terms.iter_enumerated() {
            for w in &term.members {
                by_member
                    .entry(w.page)
                    .or_default()
                    .push((idx, w.ordinal, tidx));
            }
        }
    }
    for (page, mut terms) in by_member {
        terms.sort_unstable();
        m.pages[page].terms = terms.into_iter().map(|(t, _, term)| (t, term)).collect();
    }
    m.sites[lang].taxonomies = taxonomies;
    Ok(())
}

/// Sorts every term's members: by weight, then in the default order.
pub(crate) fn sort_members(m: &mut Model) {
    let collators = Collators::new(&m.config);
    for lang in m.config.sites.ids() {
        let mut taxonomies = std::mem::take(&mut m.sites[lang].taxonomies);
        for t in taxonomies.iter_mut() {
            for term in t.terms.iter_mut() {
                term.members.sort_by(|a, b| {
                    a.weight.cmp(&b.weight).then_with(|| {
                        relations::default_order(m, &collators[lang], a.page, None, b.page, None)
                    })
                });
            }
        }
        m.sites[lang].taxonomies = taxonomies;
    }
}
