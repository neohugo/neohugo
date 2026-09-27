//! Port of `resources/page/page_matcher.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


use go_value::{Map, Value};
use nh_common::maps::ordered::Ordered;
use nh_common::Result;
use nh_config::namespace::ConfigNamespace;

/// Go: `page.PageMatcher` (cascade `_target` / `target`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PageMatcher {
    /// A Glob pattern matching the content path below /content.
    pub path: String,
    /// A Glob pattern matching the Page's Kind(s).
    pub kind: String,
    /// A Glob pattern matching the Page's language.
    pub lang: String,
    /// A Glob pattern matching the Page's Environment.
    pub environment: String,
}

/// Go: `page.PageMatcherParamsConfig`.
#[derive(Clone, Debug)]
pub struct PageMatcherParamsConfig {
    pub params: Map,
    pub fields: Map,
    pub target: PageMatcher,
}

/// Compiled cascade (Go `*maps.Ordered[PageMatcher, PageMatcherParamsConfig]`).
pub type Cascade = Ordered<PageMatcher, PageMatcherParamsConfig>;

/// Go: `page.DecodeCascadeConfig(logger, handleLegacyFormat, in)`.
// Go: resources/page/page_matcher.go:DecodeCascadeConfig
pub fn decode_cascade_config(handle_legacy_format: bool, input: &Value) -> Result<ConfigNamespace<Vec<PageMatcherParamsConfig>, Cascade>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_matcher.go (238 lines; 1/8 funcs executed)
//   types: PageMatcher, PageMatcherParamsConfig
//    L50-85: (m PageMatcher) Matches(p Page) bool
//    L96-100: isGlobWithExtension(s string) bool
//    L102-106: CheckCascadePattern(logger loggers.Logger, m PageMatcher)
// EX L108-164: DecodeCascadeConfig(logger loggers.Logger, handleLegacyFormat bool, in any) (*config.ConfigNamespace[[]PageMatcherParamsConfig, *maps.Ordered[PageM...
//    L167-173: DecodeCascade(logger loggers.Logger, handleLegacyFormat bool, in any) (*maps.Ordered[PageMatcher, PageMatcherParamsConfig], error)
//    L175-203: mapToPageMatcherParamsConfig(m map[string]any) (PageMatcherParamsConfig, error)
//    L206-223: decodePageMatcher(m any, v *PageMatcher) error
//    L234-238: (p *PageMatcherParamsConfig) init() error
// ---------------------------------------------------------------------------
