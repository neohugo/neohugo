//! Port of `resources/kinds/kinds.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

pub const KIND_PAGE: &str = "page";
pub const KIND_HOME: &str = "home";
pub const KIND_SECTION: &str = "section";
pub const KIND_TAXONOMY: &str = "taxonomy";
pub const KIND_TERM: &str = "term";
pub const KIND_TEMPORARY: &str = "temporary";
pub const KIND_RSS: &str = "rss";
pub const KIND_SITEMAP: &str = "sitemap";
pub const KIND_SITEMAP_INDEX: &str = "sitemapindex";
pub const KIND_ROBOTS_TXT: &str = "robotstxt";
pub const KIND_STATUS_404: &str = "404";

/// Go: `kinds.GetKindMain(s)` (case-insensitive; `taxonomyterm` legacy names mapped).
// Go: resources/kinds/kinds.go:GetKindMain
pub fn get_kind_main(s: &str) -> &'static str {
    todo!()
}

/// Go: `kinds.GetKindAny(s)`.
// Go: resources/kinds/kinds.go:GetKindAny
pub fn get_kind_any(s: &str) -> &'static str {
    todo!()
}

/// Go: `kinds.IsBranch(kind)` — home, section, taxonomy, term.
pub fn is_branch(kind: &str) -> bool {
    matches!(kind, KIND_HOME | KIND_SECTION | KIND_TAXONOMY | KIND_TERM)
}

/// Go: `kinds.IsDeprecatedAndReplacedWith`.
pub fn is_deprecated_and_replaced_with(s: &str) -> &'static str {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/kinds/kinds.go (119 lines; 5/5 funcs executed)
// EX L52-65: init()
// EX L87-89: GetKindMain(s string) string
// EX L92-97: GetKindAny(s string) string
// EX L100-107: IsBranch(kind string) bool
// EX L110-119: IsDeprecatedAndReplacedWith(s string) string
// ---------------------------------------------------------------------------
