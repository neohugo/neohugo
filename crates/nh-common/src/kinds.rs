//! Port of `resources/kinds/kinds.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

use go_unicode::strings::to_lower;

pub const KIND_PAGE: &str = "page";

// The rest are node types; home page, sections etc.

pub const KIND_HOME: &str = "home";
pub const KIND_SECTION: &str = "section";

// Note that before Hugo 0.73 these were confusingly named
// taxonomy (now: term)
// taxonomyTerm (now: taxonomy)
pub const KIND_TAXONOMY: &str = "taxonomy";
pub const KIND_TERM: &str = "term";

// The following are (currently) temporary nodes,
// i.e. nodes we create just to render in isolation.
pub const KIND_TEMPORARY: &str = "temporary";
pub const KIND_RSS: &str = "rss";
pub const KIND_SITEMAP: &str = "sitemap";
pub const KIND_SITEMAP_INDEX: &str = "sitemapindex";
pub const KIND_ROBOTS_TXT: &str = "robotstxt";
pub const KIND_STATUS_404: &str = "404";

/// Go: `kindMapMain`.
const KIND_MAP_MAIN: [(&str, &str); 6] = [
    (KIND_PAGE, KIND_PAGE),
    (KIND_HOME, KIND_HOME),
    (KIND_SECTION, KIND_SECTION),
    (KIND_TAXONOMY, KIND_TAXONOMY),
    (KIND_TERM, KIND_TERM),
    // Legacy, pre v0.53.0.
    ("taxonomyterm", KIND_TAXONOMY),
];

/// Go: `kindMapTemporary`.
const KIND_MAP_TEMPORARY: [(&str, &str); 4] = [
    (KIND_RSS, KIND_RSS),
    (KIND_SITEMAP, KIND_SITEMAP),
    (KIND_ROBOTS_TXT, KIND_ROBOTS_TXT),
    (KIND_STATUS_404, KIND_STATUS_404),
];

// Go: resources/kinds/kinds.go:init
/// Go: `kinds.AllKindsInPages` — all the kinds we can expect to find in `.Site.Pages` (the keys of
/// `kindMapMain`, sorted in `init`).
pub const ALL_KINDS_IN_PAGES: [&str; 6] = [
    "home",
    "page",
    "section",
    "taxonomy",
    "taxonomyterm",
    "term",
];

/// Go: `kinds.AllKinds` — all the kinds, including the temporary ones (sorted in `init`).
pub const ALL_KINDS: [&str; 10] = [
    "404",
    "home",
    "page",
    "robotstxt",
    "rss",
    "section",
    "sitemap",
    "taxonomy",
    "taxonomyterm",
    "term",
];

fn lookup(m: &[(&'static str, &'static str)], key: &[u8]) -> &'static str {
    m.iter()
        .find(|(k, _)| k.as_bytes() == key)
        .map(|(_, v)| *v)
        .unwrap_or("")
}

/// Go: `kinds.GetKindMain(s)` — the page kind given a string, empty if not found. Note that this
/// will not return any temporary kinds (e.g. robotstxt).
// Go: resources/kinds/kinds.go:GetKindMain
pub fn get_kind_main(s: &str) -> &'static str {
    lookup(&KIND_MAP_MAIN, &to_lower(s.as_bytes()))
}

/// Go: `kinds.GetKindAny(s)` — the page kind given a string, empty if not found.
// Go: resources/kinds/kinds.go:GetKindAny
pub fn get_kind_any(s: &str) -> &'static str {
    let pkind = get_kind_main(s);
    if !pkind.is_empty() {
        return pkind;
    }
    lookup(&KIND_MAP_TEMPORARY, &to_lower(s.as_bytes()))
}

/// Go: `kinds.IsBranch(kind)` — home, section, taxonomy, term.
// Go: resources/kinds/kinds.go:IsBranch
pub fn is_branch(kind: &str) -> bool {
    matches!(kind, KIND_HOME | KIND_SECTION | KIND_TAXONOMY | KIND_TERM)
}

/// Go: `kinds.IsDeprecatedAndReplacedWith` — the new kind if the given kind is deprecated.
// Go: resources/kinds/kinds.go:IsDeprecatedAndReplacedWith
pub fn is_deprecated_and_replaced_with(s: &str) -> &'static str {
    match &*to_lower(s.as_bytes()) {
        b"taxonomyterm" => KIND_TAXONOMY,
        _ => "",
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/kinds/kinds.go (119 lines; 5/5 funcs executed)
// OK L52-65: init()
// OK L87-89: GetKindMain(s string) string
// OK L92-97: GetKindAny(s string) string
// OK L100-107: IsBranch(kind string) bool
// OK L110-119: IsDeprecatedAndReplacedWith(s string) string
// ---------------------------------------------------------------------------
