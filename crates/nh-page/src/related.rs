//! Port of `related/inverted_index.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Go `related/inverted_index.go`. Candidate collection iterates Go maps (random order) but the
//! result is `sort.Stable`d by (Weight desc, PublishDate desc, Name asc); the port collects the
//! candidates in first-match order, so only full ties (same weight, publish date and name) can
//! come out in another order than a given Go run (Go's order is random there). Integer division
//! of `avgWeight`/`threshold`, `norm`'s float rounding (an FMA on arm64) and the cardinality
//! threshold's `math.Ceil` are ported exactly.

use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use go_time::GoTimeExt;
use go_value::{HostCtx, Map, SliceType, Time, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_config::decode::{FieldRef, weak_decode_into};

pub const TYPE_BASIC: &str = "basic";
pub const TYPE_FRAGMENTS: &str = "fragments";

/// Go: `validTypes`.
fn valid_type(s: &str) -> bool {
    s == TYPE_BASIC || s == TYPE_FRAGMENTS
}

/// Go: `related.Config`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub threshold: i64,
    pub include_newer: bool,
    pub to_lower: bool,
    pub indices: Vec<IndexConfig>,
}

nh_config::decode_struct!(Config, "related.Config", |s| vec![
    FieldRef::new("Threshold", &mut s.threshold),
    FieldRef::new("IncludeNewer", &mut s.include_newer),
    FieldRef::new("ToLower", &mut s.to_lower),
    FieldRef::new("Indices", &mut s.indices),
]);

impl Config {
    /// Go: `related.DefaultConfig`.
    pub fn default_config() -> Config {
        Config {
            threshold: 80,
            include_newer: false,
            to_lower: false,
            indices: vec![
                IndexConfig {
                    name: "keywords".to_string(),
                    weight: 100,
                    type_: TYPE_BASIC.to_string(),
                    ..Default::default()
                },
                IndexConfig {
                    name: "date".to_string(),
                    weight: 10,
                    type_: TYPE_BASIC.to_string(),
                    ..Default::default()
                },
            ],
        }
    }

    /// Go: `Config.Add(index)`.
    // Go: related/inverted_index.go:Add
    pub fn add(&mut self, mut index: IndexConfig) {
        if self.to_lower {
            index.to_lower = true;
        }
        self.indices.push(index);
    }

    /// Go: `Config.HasType(s)`.
    // Go: related/inverted_index.go:HasType
    pub fn has_type(&self, s: &str) -> bool {
        self.indices.iter().any(|i| i.type_ == s)
    }
}

/// Go: `related.IndexConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IndexConfig {
    pub name: String,
    pub type_: String,
    pub apply_filter: bool,
    pub pattern: String,
    pub weight: i64,
    pub cardinality_threshold: i64,
    pub to_lower: bool,
}

nh_config::decode_struct!(IndexConfig, "related.IndexConfig", |s| vec![
    FieldRef::new("Name", &mut s.name),
    FieldRef::new("Type", &mut s.type_),
    FieldRef::new("ApplyFilter", &mut s.apply_filter),
    FieldRef::new("Pattern", &mut s.pattern),
    FieldRef::new("Weight", &mut s.weight),
    FieldRef::new("CardinalityThreshold", &mut s.cardinality_threshold),
    FieldRef::new("ToLower", &mut s.to_lower),
]);

impl IndexConfig {
    /// Go: `cfg.stringToKeyword(s)`.
    // Go: related/inverted_index.go:stringToKeyword
    pub fn string_to_keyword(&self, s: &str) -> Keyword {
        let s = if self.to_lower {
            String::from_utf8_lossy(&go_unicode::strings::to_lower(s.as_bytes())).into_owned()
        } else {
            s.to_string()
        };
        if self.type_ == TYPE_FRAGMENTS {
            return Keyword::Fragment(s);
        }
        Keyword::String(s)
    }

    /// Go: `IndexConfig.ToKeywords(v)` — string, []string, []any (cast.ToStringSlice),
    /// `time.Time` (formatted with `Pattern`, default `2006`), nil.
    // Go: related/inverted_index.go:ToKeywords
    pub fn to_keywords(&self, v: &Value) -> Result<Vec<Keyword>> {
        let mut keywords = Vec::new();

        match v {
            Value::String(s) => keywords.push(self.string_to_keyword(&s.to_str_lossy())),
            Value::List(l) if l.ty == SliceType::String => {
                for s in &l.items {
                    let s = match s {
                        Value::String(s) => s.to_str_lossy().into_owned(),
                        _ => String::new(),
                    };
                    keywords.push(self.string_to_keyword(&s));
                }
            }
            Value::TypedNil(t) if &**t == "[]string" => {}
            Value::List(l) if l.ty == SliceType::Any => {
                let ss = nh_common::cast::caste::to_string_slice(v);
                return self.to_keywords(&Value::string_list(
                    ss.iter().map(|s| s.to_str_lossy().into_owned()),
                ));
            }
            Value::TypedNil(t) if &**t == "[]interface {}" => {}
            Value::Time(t) => {
                let mut layout = "2006";
                if !self.pattern.is_empty() {
                    layout = &self.pattern;
                }
                keywords.push(Keyword::String(
                    String::from_utf8_lossy(&t.format_bytes(layout.as_bytes())).into_owned(),
                ));
            }
            Value::Invalid => return Ok(keywords),
            _ => {
                return Err(Error::new(format!(
                    "indexing currently not supported for index {} and type {}",
                    go_quote(&self.name),
                    v.go_type_name()
                )));
            }
        }

        Ok(keywords)
    }

    /// Go: `cfg.StringsToKeywords(s...)`.
    // Go: related/inverted_index.go:StringsToKeywords
    pub fn strings_to_keywords(&self, s: &[&str]) -> Vec<Keyword> {
        s.iter().map(|s| self.string_to_keyword(s)).collect()
    }
}

fn go_quote(s: &str) -> String {
    String::from_utf8_lossy(&go_fmt::sprintf("%q", &[Value::string(s)])).into_owned()
}

/// Go: `related.Keyword` (`StringKeyword` / `FragmentKeyword`). As map keys the two types never
/// compare equal.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Keyword {
    String(String),
    Fragment(String),
}

impl Keyword {
    /// Go: `String()`.
    // Go: related/inverted_index.go:String
    pub fn string(&self) -> &str {
        match self {
            Keyword::String(s) | Keyword::Fragment(s) => s,
        }
    }
}

/// Go: `related.Document` (+ the optional `related.FragmentProvider`).
pub trait Document: Send + Sync {
    fn related_keywords(&self, cfg: &IndexConfig) -> Result<Vec<Keyword>>;
    fn publish_date(&self) -> Time;
    fn name(&self) -> String;
    /// Identity for "skip self" (Go `compare.Eq(doc, self)`) and for the candidate map (Go
    /// compares interface values).
    fn doc_id(&self) -> u64;
    /// Go `FragmentProvider.Fragments(ctx).Identifiers`; `None` when the document is not a
    /// `FragmentProvider`.
    fn fragments(&self, _ctx: HostCtx<'_>) -> Option<Vec<String>> {
        None
    }
    /// Go `FragmentProvider.ApplyFilterToHeadings(ctx, fn)` with the heading ID filter; `None`
    /// when the document is not a `FragmentProvider`.
    fn apply_filter_to_headings(
        &self,
        _ctx: HostCtx<'_>,
        _filter: &dyn Fn(&str) -> bool,
    ) -> Option<Arc<dyn Document>> {
        None
    }
    /// Downcasting (the page behind a document).
    fn as_any(&self) -> &dyn Any;
}

/// Go: `related.InvertedIndex`.
pub struct InvertedIndex {
    pub cfg: Config,
    /// index name -> keyword -> documents (insertion order).
    pub(crate) index: BTreeMap<String, BTreeMap<Keyword, Vec<Arc<dyn Document>>>>,
    /// Go `indexDocCount`: the number of documents added to each index.
    pub(crate) index_doc_count: HashMap<String, i64>,
    pub(crate) min_weight: i64,
    pub(crate) max_weight: i64,
    pub(crate) finalized: bool,
}

/// Go: `related.SearchOpts`.
#[derive(Default)]
pub struct SearchOpts {
    pub document: Option<Arc<dyn Document>>,
    pub named_slices: Vec<nh_common::types::types::KeyValues>,
    pub indices: Vec<String>,
    pub fragments: Vec<String>,
}

/// Go: `queryElement`.
struct QueryElement {
    index: String,
    keywords: Vec<Keyword>,
}

/// Go: `newQueryElement(index, keywords...)`.
// Go: related/inverted_index.go:newQueryElement
fn new_query_element(index: &str, keywords: Vec<Keyword>) -> QueryElement {
    QueryElement {
        index: index.to_string(),
        keywords,
    }
}

/// Go: `rank`.
struct Rank {
    doc: Arc<dyn Document>,
    weight: i64,
    matches: i64,
}

impl Rank {
    // Go: related/inverted_index.go:addWeight
    fn add_weight(&mut self, w: i64) {
        self.weight += w;
        self.matches += 1;
    }
}

/// Go: `getRank(doc, weight)` (the `sync.Pool` is an allocation detail).
// Go: related/inverted_index.go:getRank
fn get_rank(doc: Arc<dyn Document>, weight: i64) -> Rank {
    Rank {
        doc,
        weight,
        matches: 1,
    }
}

/// Go: `ranks.Less(i, j)`.
// Go: related/inverted_index.go:Less
fn ranks_less(a: &Rank, b: &Rank) -> bool {
    if a.weight == b.weight {
        let (pa, pb) = (a.doc.publish_date(), b.doc.publish_date());
        if pa.go_equal(&pb) {
            return a.doc.name().as_bytes() < b.doc.name().as_bytes();
        }
        return pa.go_after(&pb);
    }
    a.weight > b.weight
}

impl InvertedIndex {
    /// Go: `idx.getIndexCfg(name)`.
    // Go: related/inverted_index.go:getIndexCfg
    fn get_index_cfg(&self, name: &str) -> Option<IndexConfig> {
        self.cfg.indices.iter().find(|c| c.name == name).cloned()
    }

    // Go: related/inverted_index.go:NewInvertedIndex
    pub fn new(cfg: Config) -> InvertedIndex {
        let mut idx = InvertedIndex {
            cfg: cfg.clone(),
            index: BTreeMap::new(),
            index_doc_count: HashMap::new(),
            min_weight: 0,
            max_weight: 0,
            finalized: false,
        };
        for conf in &cfg.indices {
            idx.index.insert(conf.name.clone(), BTreeMap::new());
            if conf.weight < idx.min_weight {
                // By default, the weight scale starts at 0, but we allow negative weights.
                idx.min_weight = conf.weight;
            }
            if conf.weight > idx.max_weight {
                idx.max_weight = conf.weight;
            }
        }
        idx
    }

    /// Go: `Add(ctx, docs...)` without a context.
    // Go: related/inverted_index.go:Add
    pub fn add(&mut self, docs: &[Arc<dyn Document>]) -> Result<()> {
        self.add_ctx(&(), docs)
    }

    /// Go: `Add(ctx, docs...)` — Go returns the error of the LAST `RelatedKeywords` call (the
    /// documents whose keywords fail are skipped).
    // Go: related/inverted_index.go:Add
    pub fn add_ctx(&mut self, ctx: HostCtx<'_>, docs: &[Arc<dyn Document>]) -> Result<()> {
        if self.finalized {
            panic!("index is finalized");
        }
        let mut err: Option<Error> = None;
        for config in self.cfg.indices.clone() {
            if config.weight == 0 {
                // Disabled
                continue;
            }

            for doc in docs {
                let mut added = false;
                let words = match doc.related_keywords(&config) {
                    Ok(w) => {
                        err = None;
                        w
                    }
                    Err(e) => {
                        err = Some(e);
                        continue;
                    }
                };

                // Go: `setm := idx.index[config.Name]` (a missing index is created by the first
                // assignment only in NewInvertedIndex; every configured index exists).
                let setm = self.index.entry(config.name.clone()).or_default();
                for keyword in words {
                    added = true;
                    setm.entry(keyword).or_default().push(doc.clone());
                }

                if config.type_ == TYPE_FRAGMENTS
                    && let Some(fragments) = doc.fragments(ctx)
                {
                    for fragment in fragments {
                        added = true;
                        setm.entry(Keyword::Fragment(fragment))
                            .or_default()
                            .push(doc.clone());
                    }
                }

                if added {
                    *self.index_doc_count.entry(config.name.clone()).or_insert(0) += 1;
                }
            }
        }

        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Go: `Finalize(ctx)` — removes the keywords used by more than `CardinalityThreshold`
    /// percent of the documents of an index.
    // Go: related/inverted_index.go:Finalize
    pub fn finalize(&mut self) -> Result<()> {
        if self.finalized {
            return Ok(());
        }

        for config in &self.cfg.indices {
            if config.cardinality_threshold == 0 {
                continue;
            }
            let num_docs = self.index_doc_count.get(&config.name).copied().unwrap_or(0);
            if num_docs == 0 {
                continue;
            }
            let Some(setm) = self.index.get_mut(&config.name) else {
                continue;
            };

            // Remove high cardinality terms.
            setm.retain(|_, v| {
                let percentage_with_keyword =
                    ((v.len() as f64 / num_docs as f64) * 100.0).ceil() as i64;
                percentage_with_keyword <= config.cardinality_threshold
            });
        }

        self.finalized = true;

        Ok(())
    }

    /// Go: `Search(ctx, opts)` without a context.
    // Go: related/inverted_index.go:Search
    pub fn search(&self, opts: &SearchOpts) -> Result<Vec<Arc<dyn Document>>> {
        self.search_ctx(&(), opts)
    }

    /// Go: `Search(ctx, opts)` -> `searchDate`.
    // Go: related/inverted_index.go:Search
    pub fn search_ctx(
        &self,
        ctx: HostCtx<'_>,
        opts: &SearchOpts,
    ) -> Result<Vec<Arc<dyn Document>>> {
        let mut query_elements = Vec::new();

        let configs: Vec<IndexConfig> = if opts.indices.is_empty() {
            self.cfg.indices.clone()
        } else {
            let mut configs = Vec::with_capacity(opts.indices.len());
            for index_name in &opts.indices {
                let Some(cfg) = self.get_index_cfg(index_name) else {
                    return Err(Error::new(format!(
                        "index {} not found",
                        go_quote(index_name)
                    )));
                };
                configs.push(cfg);
            }
            configs
        };

        for cfg in &configs {
            let mut keywords = Vec::new();
            if let Some(doc) = &opts.document {
                keywords.extend(doc.related_keywords(cfg)?);
            }
            if cfg.type_ == TYPE_FRAGMENTS {
                for fragment in &opts.fragments {
                    keywords.push(Keyword::Fragment(fragment.clone()));
                }
                if let Some(doc) = &opts.document
                    && let Some(fragments) = doc.fragments(ctx)
                {
                    for fragment in fragments {
                        keywords.push(Keyword::Fragment(fragment));
                    }
                }
            }
            query_elements.push(new_query_element(&cfg.name, keywords));
        }
        for slice in &opts.named_slices {
            let mut keywords = Vec::new();
            let key = slice.key_string();
            if key.is_empty() {
                return Err(Error::new(format!(
                    "index {} not valid",
                    String::from_utf8_lossy(&go_fmt::sprintf(
                        "%q",
                        std::slice::from_ref(&slice.key)
                    ))
                )));
            }
            let key = key.to_str_lossy().into_owned();
            let Some(conf) = self.get_index_cfg(&key) else {
                return Err(Error::new(format!("index {} not found", go_quote(&key))));
            };

            for val in &slice.values {
                keywords.extend(conf.to_keywords(val)?);
            }
            query_elements.push(new_query_element(&conf.name, keywords));
        }

        if let Some(doc) = &opts.document {
            return self.search_date(ctx, Some(doc), doc.publish_date(), &query_elements);
        }
        self.search_date(ctx, None, Time::zero(), &query_elements)
    }

    /// Go: `searchDate(ctx, self, upperDate, query...)`.
    // Go: related/inverted_index.go:searchDate
    fn search_date(
        &self,
        ctx: HostCtx<'_>,
        self_doc: Option<&Arc<dyn Document>>,
        upper_date: Time,
        query: &[QueryElement],
    ) -> Result<Vec<Arc<dyn Document>>> {
        // Go: `map[Document]*rank`; kept in first-match order here.
        let mut matchm: Vec<Rank> = Vec::new();
        let mut matchi: HashMap<u64, usize> = HashMap::new();

        let apply_date_filter = !self.cfg.include_newer && !upper_date.go_is_zero();
        let mut fragments_filter: Vec<Vec<u8>> = Vec::new();

        for el in query {
            let Some(setm) = self.index.get(&el.index) else {
                return Err(Error::new(format!(
                    "index for {} not found",
                    go_quote(&el.index)
                )));
            };

            let Some(config) = self.get_index_cfg(&el.index) else {
                return Err(Error::new(format!(
                    "index config for {} not found",
                    go_quote(&el.index)
                )));
            };

            for kw in &el.keywords {
                if let Some(docs) = setm.get(kw) {
                    for doc in docs {
                        if let Some(s) = self_doc
                            && doc.doc_id() == s.doc_id()
                        {
                            continue;
                        }

                        if apply_date_filter {
                            // Exclude newer than the limit given
                            if doc.publish_date().go_after(&upper_date) {
                                continue;
                            }
                        }

                        if config.type_ == TYPE_FRAGMENTS
                            && config.apply_filter
                            && let Keyword::Fragment(fkw) = kw
                        {
                            fragments_filter.push(fkw.as_bytes().to_vec());
                        }

                        match matchi.get(&doc.doc_id()) {
                            None => {
                                matchi.insert(doc.doc_id(), matchm.len());
                                matchm.push(get_rank(doc.clone(), config.weight));
                            }
                            Some(&i) => matchm[i].add_weight(config.weight),
                        }
                    }
                }
            }
        }

        if matchm.is_empty() {
            return Ok(Vec::new());
        }

        let mut matches: Vec<Rank> = Vec::with_capacity(100);

        for v in matchm {
            let avg_weight = v.weight / v.matches;
            let weight = norm(avg_weight, self.min_weight, self.max_weight);
            let threshold = self.cfg.threshold / v.matches;

            if weight >= threshold {
                matches.push(v);
            }
        }

        go_sort::stable_by(&mut matches, ranks_less);
        fragments_filter.sort();
        let fragments_filter = nh_common::collections::slice::SortedStringSlice(
            fragments_filter
                .into_iter()
                .map(go_value::GoString::from)
                .collect(),
        );

        let mut result: Vec<Arc<dyn Document>> = Vec::with_capacity(matches.len());

        for m in matches {
            let mut doc = m.doc;

            if !fragments_filter.0.is_empty()
                && let Some(d) = doc
                    .apply_filter_to_headings(ctx, &|id| fragments_filter.contains(id.as_bytes()))
            {
                doc = d;
            }
            result.push(doc);
        }

        Ok(result)
    }
}

impl InvertedIndex {
    /// Go: `idx.search(ctx, query...)` (no document: no date filter, no self skip).
    // Go: related/inverted_index.go:search
    fn search_query(&self, query: &[QueryElement]) -> Result<Vec<Arc<dyn Document>>> {
        self.search_date(&(), None, Time::zero(), query)
    }
}

/// Go: `norm(num, min, max)` — normalizes num to a number between 0 and 100
/// (`int(math.Floor(float64(num-min)/float64(max-min)*100 + 0.5))`; the arm64 compiler fuses the
/// multiply-add).
// Go: related/inverted_index.go:norm
pub fn norm(num: i64, min: i64, max: i64) -> i64 {
    if min > max {
        panic!("min > max");
    }
    let q = (num - min) as f64 / (max - min) as f64;
    q.mul_add(100.0, 0.5).floor() as i64
}

/// Go: `related.DecodeConfig(m)` (`m` is a `maps.Params`; `Value::Invalid` is Go's nil).
// Go: related/inverted_index.go:DecodeConfig
pub fn decode_config(m: &Value) -> Result<Config> {
    let len = match m {
        Value::Invalid | Value::TypedNil(_) => {
            return Err(Error::new("no related config provided"));
        }
        Value::Map(mm) => mm.len(),
        _ => 1,
    };

    if len == 0 {
        return Err(Error::new("empty related config provided"));
    }

    let mut c = Config::default();

    weak_decode_into(m, &mut c)?;

    if c.threshold < 0 || c.threshold > 100 {
        return Err(Error::new("related threshold must be between 0 and 100"));
    }

    if c.to_lower {
        for i in c.indices.iter_mut() {
            i.to_lower = true;
        }
    }
    for i in 0..c.indices.len() {
        // Lower case name.
        c.indices[i].name =
            String::from_utf8_lossy(&go_unicode::strings::to_lower(c.indices[i].name.as_bytes()))
                .into_owned();

        let icfg = c.indices[i].clone();
        if icfg.type_.is_empty() {
            c.indices[i].type_ = TYPE_BASIC.to_string();
        }
        if !valid_type(&c.indices[i].type_) {
            // Go prints the keys of the validTypes map (random order).
            return Err(Error::new(format!(
                "invalid index type {}. Must be one of [basic fragments]",
                go_quote(&c.indices[i].type_)
            )));
        }
        if icfg.cardinality_threshold < 0 || icfg.cardinality_threshold > 100 {
            return Err(Error::new(
                "cardinalityThreshold threshold must be between 0 and 100",
            ));
        }
    }

    Ok(c)
}

/// `related.DecodeConfig` of a `maps.Params` map.
pub fn decode_config_map(m: &Map) -> Result<Config> {
    decode_config(&Value::map(m.clone()))
}

// Ports of related/inverted_index_test.go.
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

    static CLOCK: AtomicI64 = AtomicI64::new(1_700_000_000);
    static IDS: AtomicU64 = AtomicU64::new(1);

    /// Go `testDoc` (`newTestDoc` sleeps a millisecond so that every doc is newer).
    #[derive(Clone)]
    struct TestDoc {
        id: u64,
        keywords: HashMap<String, Vec<Keyword>>,
        date: Time,
        name: String,
    }

    impl TestDoc {
        fn new(name: &str, keywords: &[&str]) -> TestDoc {
            let t = CLOCK.fetch_add(1, Ordering::SeqCst);
            TestDoc::with_date(name, go_time::unix(t, 0), keywords)
        }
        fn with_date(name: &str, date: Time, keywords: &[&str]) -> TestDoc {
            let mut d = TestDoc {
                id: IDS.fetch_add(1, Ordering::SeqCst),
                keywords: HashMap::new(),
                date,
                name: String::new(),
            };
            d.add_keywords(name, keywords);
            d
        }
        fn add_keywords(&mut self, name: &str, keywords: &[&str]) -> &mut Self {
            self.keywords.insert(
                name.to_string(),
                keywords
                    .iter()
                    .map(|k| Keyword::String(k.to_string()))
                    .collect(),
            );
            self
        }
        fn doc(self) -> Arc<dyn Document> {
            Arc::new(self)
        }
    }

    fn with_kw(mut d: TestDoc, name: &str, keywords: &[&str]) -> TestDoc {
        d.add_keywords(name, keywords);
        d
    }

    impl Document for TestDoc {
        fn related_keywords(&self, cfg: &IndexConfig) -> Result<Vec<Keyword>> {
            Ok(self.keywords.get(&cfg.name).cloned().unwrap_or_default())
        }
        fn publish_date(&self) -> Time {
            self.date.clone()
        }
        fn name(&self) -> String {
            self.name.clone()
        }
        fn doc_id(&self) -> u64 {
            self.id
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn ic(name: &str, weight: i64, card: i64) -> IndexConfig {
        IndexConfig {
            name: name.to_string(),
            weight,
            cardinality_threshold: card,
            ..Default::default()
        }
    }

    fn same(a: &Arc<dyn Document>, b: &Arc<dyn Document>) -> bool {
        a.doc_id() == b.doc_id()
    }

    // Go: related/inverted_index_test.go:TestCardinalityThreshold
    #[test]
    fn cardinality_threshold() {
        let config = Config {
            threshold: 90,
            include_newer: false,
            indices: vec![ic("tags", 50, 79), ic("keywords", 65, 90)],
            ..Default::default()
        };
        let mut idx = InvertedIndex::new(config);
        let docs = vec![
            TestDoc::new("tags", &["a", "b", "c", "d"]).doc(),
            TestDoc::new("tags", &["b", "d", "g"]).doc(),
            TestDoc::new("tags", &["b", "d", "g"]).doc(),
            with_kw(TestDoc::new("tags", &["b", "h"]), "keywords", &["a"]).doc(),
            with_kw(
                TestDoc::new("tags", &["g", "h"]),
                "keywords",
                &["a", "b", "z"],
            )
            .doc(),
        ];
        idx.add(&docs).unwrap();
        idx.finalize().unwrap();
        let has =
            |index: &str, kw: &str| idx.index[index].contains_key(&Keyword::String(kw.into()));
        // Only tags=b should be removed.
        assert!(has("tags", "a"));
        assert!(!has("tags", "b"));
        assert!(has("tags", "d"));
        assert!(has("keywords", "b"));
    }

    // Go: related/inverted_index_test.go:TestSearch
    #[test]
    fn search() {
        let config = Config {
            threshold: 90,
            include_newer: false,
            indices: vec![ic("tags", 50, 0), ic("keywords", 65, 0)],
            ..Default::default()
        };
        let mut idx = InvertedIndex::new(config.clone());
        let docs = vec![
            TestDoc::new("tags", &["a", "b", "c", "d"]).doc(),
            TestDoc::new("tags", &["b", "d", "g"]).doc(),
            with_kw(TestDoc::new("tags", &["b", "h"]), "keywords", &["a"]).doc(),
            with_kw(TestDoc::new("tags", &["g", "h"]), "keywords", &["a", "b"]).doc(),
        ];
        idx.add(&docs).unwrap();

        // count
        assert_eq!(idx.index.len(), 2);
        assert_eq!(idx.index["tags"].len(), 6);
        assert_eq!(idx.index["keywords"].len(), 2);

        let cfg = IndexConfig::default();
        // search-tags
        let m = idx
            .search_query(&[new_query_element(
                "tags",
                cfg.strings_to_keywords(&["a", "b", "d", "z"]),
            )])
            .unwrap();
        assert_eq!(m.len(), 2);
        assert!(same(&m[0], &docs[0]));
        assert!(same(&m[1], &docs[1]));

        // search-tags-and-keywords
        let m = idx
            .search_query(&[
                new_query_element("tags", cfg.strings_to_keywords(&["a", "b", "z"])),
                new_query_element("keywords", cfg.strings_to_keywords(&["a", "b"])),
            ])
            .unwrap();
        assert_eq!(m.len(), 3);
        assert!(same(&m[0], &docs[3]));
        assert!(same(&m[1], &docs[2]));
        assert!(same(&m[2], &docs[0]));

        // searchdoc-all
        let doc = with_kw(TestDoc::new("tags", &["a"]), "keywords", &["a"]).doc();
        let m = idx
            .search(&SearchOpts {
                document: Some(doc),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(m.len(), 2);
        assert!(same(&m[0], &docs[3]));
        assert!(same(&m[1], &docs[2]));

        // searchdoc-tags
        let doc = with_kw(
            TestDoc::new("tags", &["a", "b", "d", "z"]),
            "keywords",
            &["a", "b"],
        )
        .doc();
        let m = idx
            .search(&SearchOpts {
                document: Some(doc),
                indices: vec!["tags".into()],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(m.len(), 2);
        assert!(same(&m[0], &docs[0]));
        assert!(same(&m[1], &docs[1]));

        // searchdoc-keywords-date
        let doc = with_kw(
            TestDoc::new("tags", &["a", "b", "d", "z"]),
            "keywords",
            &["a", "b"],
        )
        .doc();
        // This will get a date newer than the others.
        let new_doc = TestDoc::new("keywords", &["a", "b"]).doc();
        idx.add(&[new_doc]).unwrap();
        let m = idx
            .search(&SearchOpts {
                document: Some(doc),
                indices: vec!["keywords".into()],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(m.len(), 2);
        assert!(same(&m[0], &docs[3]));

        // searchdoc-keywords-same-date
        let mut idx = InvertedIndex::new(config);
        let date = go_time::unix(CLOCK.fetch_add(1, Ordering::SeqCst), 0);
        let mut doc = TestDoc::with_date("keywords", date, &["a", "b"]);
        doc.name = "thedoc".into();
        for i in 0..10 {
            let mut docc = doc.clone();
            docc.id = IDS.fetch_add(1, Ordering::SeqCst);
            docc.name = format!("doc{i}");
            idx.add(&[docc.doc()]).unwrap();
        }
        let m = idx
            .search(&SearchOpts {
                document: Some(doc.doc()),
                indices: vec!["keywords".into()],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(m.len(), 10);
        for (i, d) in m.iter().enumerate() {
            assert_eq!(d.name(), format!("doc{i}"));
        }
    }

    // Go: related/inverted_index_test.go:TestToKeywordsToLower
    #[test]
    fn to_keywords_to_lower() {
        let slice = Value::string_list(["A", "B", "C"]);
        let config = IndexConfig {
            to_lower: true,
            ..Default::default()
        };
        let keywords = config.to_keywords(&slice).unwrap();
        assert_eq!(
            keywords,
            vec![
                Keyword::String("a".into()),
                Keyword::String("b".into()),
                Keyword::String("c".into())
            ]
        );
    }

    // Go: related/inverted_index_test.go:TestDecodeConfig (the TOML as the decoded maps.Params)
    #[test]
    fn decode_config_params() {
        let idx = |name: &str, typ: &str, weight: i64, card: i64, af: bool| {
            let mut m = Map::new(go_value::MapType::StringAny);
            m.insert("applyFilter", Value::Bool(af));
            m.insert("cardinalityThreshold", Value::int64(card));
            m.insert("name", Value::string(name));
            m.insert("pattern", Value::string(""));
            m.insert("toLower", Value::Bool(false));
            m.insert("type", Value::string(typ));
            m.insert("weight", Value::int64(weight));
            Value::map(m)
        };
        let mut m = Map::new(go_value::MapType::Params);
        m.insert("includenewer", Value::Bool(true));
        m.insert("threshold", Value::int64(32));
        m.insert("tolower", Value::Bool(false));
        m.insert(
            "indices",
            Value::any_list(vec![
                idx("KeyworDs", "basic", 100, 0, false),
                idx("date", "basic", 10, 32, true),
                idx("tags", "fragments", 80, 0, false),
            ]),
        );
        let conf = decode_config(&Value::map(m)).unwrap();
        assert!(conf.include_newer);
        assert_eq!(conf.indices[0].name, "keywords");
        assert_eq!(conf.indices[1].cardinality_threshold, 32);
        assert!(conf.indices[1].apply_filter);
        assert_eq!(conf.indices[2].type_, TYPE_FRAGMENTS);
    }

    // Go: related/inverted_index_test.go:TestToKeywordsAnySlice
    #[test]
    fn to_keywords_any_slice() {
        let config = IndexConfig::default();
        let slice = Value::any_list(vec![Value::string("A"), Value::int(32), Value::string("C")]);
        let keywords = config.to_keywords(&slice).unwrap();
        assert_eq!(
            keywords,
            vec![
                Keyword::String("A".into()),
                Keyword::String("32".into()),
                Keyword::String("C".into())
            ]
        );
    }

    /// `norm` rounds with an arm64 FMA (`float64(num-min)/float64(max-min)*100 + 0.5` is one
    /// FMADDD, inlined into searchDate, in the arm64 build): -0.5000000000000001 + 0.5 floors to
    /// -1 there, where the unfused amd64 sum floors to 0.
    #[test]
    fn norm_fma() {
        assert_eq!(norm(-1, 0, 200), -1);
        assert_eq!(norm(50, 0, 100), 50);
        assert_eq!(norm(1, 0, 3), 33);
        assert_eq!(norm(2, 0, 3), 67);
        assert_eq!(norm(-10, -10, 100), 0);
        // max == min: 0/0 = NaN -> 0 (arm64 and Rust `as`).
        assert_eq!(norm(0, 0, 0), 0);
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: related/inverted_index.go (632 lines; 17/23 funcs executed)
//   types: Config, IndicesConfig, IndexConfig, Document, FragmentProvider, InvertedIndex, queryElement, ranks, rank,
//          SearchOpts, StringKeyword, FragmentKeyword, Keyword
// OK L81-86: (c *Config) Add(index IndexConfig)
// OK L88-95: (c *Config) HasType(s string) bool
// OK L168-176: (idx *InvertedIndex) getIndexCfg(name string) (IndexConfig, bool)
// OK L180-194: NewInvertedIndex(cfg Config) *InvertedIndex
// OK L198-239: (idx *InvertedIndex) Add(ctx context.Context, docs ...Document) error
// OK L241-269: (idx *InvertedIndex) Finalize(ctx context.Context) error
// OK L278-280: newQueryElement(index string, keywords ...Keyword) queryElement
// OK L290-293: (r *rank) addWeight(w int)
// OK L301-307: getRank(doc Document, weight int) *rank
// OK L309-311: putRank(r *rank)
// OK L313-313: (r ranks) Len() int
// OK L314-314: (r ranks) Swap(i, j int)
// OK L315-323: (r ranks) Less(i, j int) bool
// OK L348-416: (idx *InvertedIndex) Search(ctx context.Context, opts SearchOpts) ([]Document, error)
// OK L418-426: (cfg IndexConfig) stringToKeyword(s string) Keyword
// OK L429-456: (cfg IndexConfig) ToKeywords(v any) ([]Keyword, error)
// OK L458-460: (idx *InvertedIndex) search(ctx context.Context, query ...queryElement) ([]Document, error)
// OK L462-550: (idx *InvertedIndex) searchDate(ctx context.Context, self Document, upperDate time.Time, query ...queryElement) ([]Document, error)
// OK L553-558: norm(num, min, max int) int
// OK L561-602: DecodeConfig(m maps.Params) (Config, error)
// OK L607-609: (s StringKeyword) String() string
// OK L614-616: (f FragmentKeyword) String() string
// OK L624-632: (cfg IndexConfig) StringsToKeywords(s ...string) []Keyword
// ---------------------------------------------------------------------------
