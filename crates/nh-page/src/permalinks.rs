//! Port of `resources/page/permalinks.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::Map;
use nh_common::Result;

use crate::page::Page;

/// Go: `page.PermalinkExpander` (`[permalinks]`: `:year`, `:month`, `:title`, `:slug`, `:sections`...).
#[derive(Clone)]
pub struct PermalinkExpander {
    /// kind -> section -> compiled pattern.
    pub(crate) expanders: BTreeMap<String, BTreeMap<String, Arc<dyn Fn(&dyn Page) -> Result<String> + Send + Sync>>>,
    pub(crate) urlize: Arc<dyn Fn(&str) -> String + Send + Sync>,
}

impl PermalinkExpander {
    // Go: resources/page/permalinks.go:NewPermalinkExpander
    pub fn new(urlize: Arc<dyn Fn(&str) -> String + Send + Sync>, patterns: &BTreeMap<String, BTreeMap<String, String>>) -> Result<PermalinkExpander> {
        todo!()
    }

    /// Go: `Expand(key, p)` — `key` is the section or taxonomy; "" if no pattern.
    // Go: resources/page/permalinks.go:Expand
    pub fn expand(&self, key: &str, p: &dyn Page) -> Result<String> {
        todo!()
    }
}

/// Go: `page.DecodePermalinksConfig(m)` — legacy flat form `posts = "..."` registers for `page` and `term`.
// Go: resources/page/permalinks.go:DecodePermalinksConfig
pub fn decode_permalinks_config(m: &Map) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/permalinks.go (480 lines; 8/22 funcs executed)
//   types: PermalinkExpander, pageToPermaAttribute, permalinkExpandError
// EX L53-71: (p PermalinkExpander) callback(attr string) (pageToPermaAttribute, bool)
// EX L75-110: NewPermalinkExpander(urlize func(uri string) string, patterns map[string]map[string]string) (PermalinkExpander, error)
// EX L115-118: (l PermalinkExpander) normalizeEscapeSequencesIn(s string) (string, bool)
//    L120-122: (l PermalinkExpander) normalizeEscapeSequencesOut(result string) string
//    L125-132: (l PermalinkExpander) ExpandPattern(pattern string, p Page) (string, error)
// EX L136-148: (l PermalinkExpander) Expand(key string, p Page) (string, error)
// EX L153-157: init()
// EX L159-211: (l PermalinkExpander) getOrParsePattern(pattern string) (func(Page) (string, error), error)
// EX L213-228: (l PermalinkExpander) parse(patterns map[string]string) (map[string]func(Page) (string, error), error)
//    L241-243: (pee *permalinkExpandError) Error() string
//    L247-267: (l PermalinkExpander) pageToPermalinkDate(p Page, dateField string) (string, error)
//    L270-272: (l PermalinkExpander) pageToPermalinkTitle(p Page, _ string) (string, error)
//    L275-287: (l PermalinkExpander) pageToPermalinkFilename(p Page, _ string) (string, error)
//    L290-295: (l PermalinkExpander) pageToPermalinkSlugElseTitle(p Page, a string) (string, error)
//    L298-303: (l PermalinkExpander) pageToPermalinkSlugElseFilename(p Page, a string) (string, error)
//    L305-307: (l PermalinkExpander) pageToPermalinkSection(p Page, _ string) (string, error)
//    L309-311: (l PermalinkExpander) pageToPermalinkSections(p Page, _ string) (string, error)
//    L314-316: (l PermalinkExpander) pageToPermalinkContentBaseName(p Page, _ string) (string, error)
//    L319-328: (l PermalinkExpander) pageToPermalinkSlugOrContentBaseName(p Page, a string) (string, error)
//    L330-335: (l PermalinkExpander) translationBaseName(p Page) string
//    L353-432: (l PermalinkExpander) toSliceFunc(cut string) func(s []string) []string
// EX L437-480: DecodePermalinksConfig(m map[string]any) (map[string]map[string]string, error)
// ---------------------------------------------------------------------------
