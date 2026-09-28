//! Port of `resources/page/pages_related.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Go `pages_related.go`: `.Site.RegularPages.Related .` — an inverted index per page list
//! (cached by list equality), searched with the related config (threshold, indices weights).

use std::any::Any;
use std::sync::{Arc, Mutex};

use go_value::{HostCtx, MapType, Time, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::types::types::KeyValues;

use crate::page::{PageRef, Pages, page_from_value};
use crate::pages_cache::pages_equal;
use crate::related::{
    Config, Document, IndexConfig, InvertedIndex, Keyword, SearchOpts, TYPE_FRAGMENTS,
};

/// A page as a `related.Document` (Go: every `page.Page` is one; `*pageState` is also a
/// `related.FragmentProvider`).
#[derive(Clone)]
pub struct PageDocument(pub PageRef);

impl Document for PageDocument {
    fn related_keywords(&self, cfg: &IndexConfig) -> Result<Vec<Keyword>> {
        self.0.0.related_keywords(cfg)
    }
    fn publish_date(&self) -> Time {
        self.0.0.publish_date()
    }
    fn name(&self) -> String {
        nh_resource::resourcetypes::Resource::name(&*self.0.0)
    }
    fn doc_id(&self) -> u64 {
        self.0.0.page_id()
    }
    fn fragments(&self, ctx: HostCtx<'_>) -> Option<Vec<String>> {
        self.0.0.fragments_identifiers(ctx)
    }
    fn apply_filter_to_headings(
        &self,
        ctx: HostCtx<'_>,
        filter: &dyn Fn(&str) -> bool,
    ) -> Option<Arc<dyn Document>> {
        let p = self.0.0.apply_filter_to_headings(ctx, filter)?;
        Some(Arc::new(PageDocument(p)))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `page.RelatedDocsHandler`.
pub struct RelatedDocsHandler {
    pub cfg: Config,
    /// Go `postingLists` (`cachedPostingList`: the page list and its index).
    pub(crate) posting_lists: Mutex<Vec<(Pages, Arc<InvertedIndex>)>>,
}

impl RelatedDocsHandler {
    // Go: resources/page/pages_related.go:NewRelatedDocsHandler
    pub fn new(cfg: Config) -> Arc<RelatedDocsHandler> {
        Arc::new(RelatedDocsHandler {
            cfg,
            posting_lists: Mutex::new(Vec::new()),
        })
    }

    /// Go: `s.Clone()` — a new handler with the same config (and no indices).
    // Go: resources/page/pages_related.go:Clone
    pub fn clone_handler(&self) -> Arc<RelatedDocsHandler> {
        RelatedDocsHandler::new(self.cfg.clone())
    }

    /// Go: `s.getIndex(p)` (the caller holds the lock).
    // Go: resources/page/pages_related.go:getIndex
    fn get_index(lists: &[(Pages, Arc<InvertedIndex>)], p: &Pages) -> Option<Arc<InvertedIndex>> {
        for (ci_p, posting_list) in lists {
            if pages_equal(p, ci_p) {
                return Some(posting_list.clone());
            }
        }
        None
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<(Pages, Arc<InvertedIndex>)>> {
        self.posting_lists.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Go: `s.getOrCreateIndex(ctx, p)` — the index of this page list, created (and cached) on
    /// first use. The index is built outside the lock (HUGO_LAYER.md §4.8); the first one stored
    /// wins.
    // Go: resources/page/pages_related.go:getOrCreateIndex
    pub fn get_or_create_index(&self, ctx: HostCtx<'_>, p: &Pages) -> Result<Arc<InvertedIndex>> {
        if let Some(cached_index) = Self::get_index(&self.lock(), p) {
            return Ok(cached_index);
        }

        for c in &self.cfg.indices {
            if c.type_ == TYPE_FRAGMENTS {
                // This will trigger building the Pages' fragment map (Go runs these in the
                // workers pool, one worker with HUGO_NUMWORKERMULTIPLIER=1).
                for page in p {
                    let _ = page.0.fragments_identifiers(ctx);
                }

                break;
            }
        }

        let mut search_index = InvertedIndex::new(self.cfg.clone());

        for page in p {
            let doc: Arc<dyn Document> = Arc::new(PageDocument(page.clone()));
            search_index.add_ctx(ctx, &[doc])?;
        }

        search_index.finalize()?;
        let search_index = Arc::new(search_index);

        let mut lists = self.lock();
        // Double check.
        if let Some(cached_index) = Self::get_index(&lists, p) {
            return Ok(cached_index);
        }
        lists.push((p.clone(), search_index.clone()));

        Ok(search_index)
    }
}

/// Go: `p.withInvertedIndex(ctx, search)` — `None` is Go's nil result.
// Go: resources/page/pages_related.go:withInvertedIndex
fn with_inverted_index(
    ctx: HostCtx<'_>,
    p: &Pages,
    search: &dyn Fn(&InvertedIndex) -> Result<Vec<Arc<dyn Document>>>,
) -> Result<Option<Pages>> {
    if p.is_empty() {
        return Ok(None);
    }

    let Some(cache) = p[0].0.related_docs_handler() else {
        return Err(Error::new(format!(
            "invalid type {} in related search",
            p[0].0.tpl_type_name()
        )));
    };

    let search_index = cache.get_or_create_index(ctx, p)?;

    let result = search(&search_index)?;

    if !result.is_empty() {
        let mut mp = Vec::with_capacity(result.len());
        for m in &result {
            match m.as_any().downcast_ref::<PageDocument>() {
                Some(d) => mp.push(d.0.clone()),
                None => {
                    return Err(Error::new(
                        "interface conversion: related.Document is not page.Page",
                    ));
                }
            }
        }
        return Ok(Some(mp));
    }

    Ok(None)
}

/// Go: `p.search(ctx, opts)`.
// Go: resources/page/pages_related.go:search
fn search(ctx: HostCtx<'_>, p: &Pages, opts: &SearchOpts) -> Result<Option<Pages>> {
    with_inverted_index(ctx, p, &|idx| idx.search_ctx(ctx, opts))
}

/// A `related.Document` template argument (a page).
fn document_from_value(v: &Value) -> Option<Arc<dyn Document>> {
    let p = page_from_value(v)?;
    Some(Arc::new(PageDocument(p)))
}

/// Go: `mapstructure.WeakDecode(v, &opts)` for `related.SearchOpts` from a `map[string]any`
/// (field names matched case-insensitively).
fn decode_search_opts(m: &go_value::Map) -> Result<SearchOpts> {
    let mut opts = SearchOpts::default();
    let mut errors: Vec<String> = Vec::new();
    for (name, _) in [
        ("Document", 0),
        ("NamedSlices", 1),
        ("Indices", 2),
        ("Fragments", 3),
    ] {
        let Some(v) = nh_config::decode::field(m, name) else {
            continue;
        };
        if v.is_invalid() {
            continue;
        }
        match name {
            "Document" => match document_from_value(v) {
                Some(d) => opts.document = Some(d),
                None => errors.push(format!(
                    "'Document' expected type 'related.Document', got '{}'",
                    v.go_type_name()
                )),
            },
            "NamedSlices" => {
                let items: Vec<Value> = match v {
                    Value::List(l) => l.items.clone(),
                    other => vec![other.clone()],
                };
                for (i, it) in items.iter().enumerate() {
                    match key_values_from_value(it) {
                        Some(kv) => opts.named_slices.push(kv),
                        None => errors.push(format!(
                            "'NamedSlices[{i}]' expected a map, got '{}'",
                            reflect_kind_name(it)
                        )),
                    }
                }
            }
            _ => match nh_config::decode::weak_string_slice(v) {
                Ok(s) => {
                    if name == "Indices" {
                        opts.indices = s;
                    } else {
                        opts.fragments = s;
                    }
                }
                Err(e) => errors.push(e.to_string()),
            },
        }
    }
    if !errors.is_empty() {
        let mut points: Vec<String> = errors.iter().map(|e| format!("* {e}")).collect();
        points.sort();
        return Err(Error::new(format!(
            "{} error(s) decoding:\n\n{}",
            errors.len(),
            points.join("\n")
        )));
    }
    Ok(opts)
}

fn reflect_kind_name(v: &Value) -> &'static str {
    match v {
        Value::Invalid => "invalid",
        Value::Bool(_) => "bool",
        Value::Int(..) => "int",
        Value::Uint(..) => "uint",
        Value::Float(..) => "float64",
        Value::String(_) | Value::Safe(..) => "string",
        Value::List(_) | Value::TypedNil(_) => "slice",
        Value::Map(_) => "map",
        _ => "struct",
    }
}

/// A `types.KeyValues` (an object with `Key` and `Values` fields, as `keyVals` returns).
fn key_values_from_value(v: &Value) -> Option<KeyValues> {
    let o = v.as_object()?;
    if o.type_name() != "types.KeyValues" {
        return None;
    }
    let key = o.field("Key")?;
    let values = match o.field("Values")? {
        Value::List(l) => l.items.clone(),
        _ => Vec::new(),
    };
    Some(KeyValues { key, values })
}

/// Go: `Pages.Related(ctx, optsv)` — `optsv` is a page (document) or an options map.
// Go: resources/page/pages_related.go:Related
pub fn related(ctx: HostCtx<'_>, p: &Pages, opts: &Value) -> Result<Pages> {
    Ok(related_opt(ctx, p, opts)?.unwrap_or_default())
}

/// Go: `Pages.Related(ctx, optsv)`; `None` is Go's nil result.
// Go: resources/page/pages_related.go:Related
pub fn related_opt(ctx: HostCtx<'_>, p: &Pages, optsv: &Value) -> Result<Option<Pages>> {
    if p.is_empty() {
        return Ok(None);
    }

    let opts = if let Some(d) = document_from_value(optsv) {
        SearchOpts {
            document: Some(d),
            ..Default::default()
        }
    } else {
        match optsv {
            Value::Map(m) if m.ty == MapType::StringAny => decode_search_opts(m)?,
            _ => {
                return Err(Error::new(format!(
                    "invalid argument type {}",
                    match optsv {
                        Value::Invalid => "<nil>".into(),
                        v => v.go_type_name(),
                    }
                )));
            }
        }
    };

    search(ctx, p, &opts)
}

/// Go: `Pages.RelatedIndices(ctx, doc, indices...)` (deprecated).
// Go: resources/page/pages_related.go:RelatedIndices
pub fn related_indices(
    ctx: HostCtx<'_>,
    p: &Pages,
    doc: Option<Arc<dyn Document>>,
    indices: &[Value],
) -> Result<Option<Pages>> {
    let indices_str =
        nh_common::cast::caste::to_string_slice_e(&Value::any_list(indices.to_vec()))?;

    let opts = SearchOpts {
        document: doc,
        indices: indices_str
            .iter()
            .map(|s| s.to_str_lossy().into_owned())
            .collect(),
        ..Default::default()
    };

    search(ctx, p, &opts)
}

/// Go: `Pages.RelatedTo(ctx, args...)` (deprecated).
// Go: resources/page/pages_related.go:RelatedTo
pub fn related_to(ctx: HostCtx<'_>, p: &Pages, args: Vec<KeyValues>) -> Result<Option<Pages>> {
    if p.is_empty() {
        return Ok(None);
    }

    let opts = SearchOpts {
        named_slices: args,
        ..Default::default()
    };

    search(ctx, p, &opts)
}

/// Template argument helpers for `pages.rs`.
pub(crate) fn document_arg(v: &Value) -> std::result::Result<Option<Arc<dyn Document>>, String> {
    if v.is_invalid() {
        return Ok(None);
    }
    document_from_value(v)
        .map(Some)
        .ok_or_else(|| v.go_type_name().into_owned())
}

pub(crate) fn key_values_arg(v: &Value) -> Option<KeyValues> {
    key_values_from_value(v)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_related.go (244 lines; 6/9 funcs executed)
//   types: PageGenealogist, cachedPostingList, RelatedDocsHandler
// OK L56-79: (p Pages) Related(ctx context.Context, optsv any) (Pages, error)
// OK L84-101: (p Pages) RelatedIndices(ctx context.Context, doc related.Document, indices ...any) (Pages, error)
// OK L105-115: (p Pages) RelatedTo(ctx context.Context, args ...types.KeyValues) (Pages, error)
// OK L117-121: (p Pages) search(ctx context.Context, opts related.SearchOpts) (Pages, error)
// OK L123-154: (p Pages) withInvertedIndex(ctx context.Context, search func(idx *related.InvertedIndex) ([]related.Document, error)) (Pages, error)
// OK L171-173: NewRelatedDocsHandler(cfg related.Config) *RelatedDocsHandler
// OK L175-177: (s *RelatedDocsHandler) Clone() *RelatedDocsHandler
// OK L180-187: (s *RelatedDocsHandler) getIndex(p Pages) *related.InvertedIndex
// OK L189-244: (s *RelatedDocsHandler) getOrCreateIndex(ctx context.Context, p Pages) (*related.InvertedIndex, error)
// ---------------------------------------------------------------------------
