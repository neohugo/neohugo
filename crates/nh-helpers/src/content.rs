//! Port of `helpers/content.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).


use std::sync::Arc;

use nh_common::Result;
use nh_config::config_provider::AllProvider;
use nh_markup::converter::converter::Provider;
use nh_markup::markup::ConverterProvider;

/// Go: `helpers.ContentSpec`.
#[derive(Clone)]
pub struct ContentSpec {
    pub converters: Arc<dyn ConverterProvider>,
    pub cfg: Arc<dyn AllProvider>,
}

impl ContentSpec {
    /// Go: `helpers.NewContentSpec(cfg, logger, contentFs, ex)`.
    // Go: helpers/content.go:NewContentSpec
    pub fn new(cfg: Arc<dyn AllProvider>, exec: Arc<nh_config::hexec::Exec>) -> Result<Arc<ContentSpec>> {
        todo!()
    }

    /// Go: `ContentSpec.ResolveMarkup(in)`.
    // Go: helpers/content.go:ResolveMarkup
    pub fn resolve_markup(&self, input: &str) -> String { todo!() }

    // Go: helpers/content.go:SanitizeAnchorName
    pub fn sanitize_anchor_name(&self, s: &str) -> String { todo!() }

    /// Go: `ContentSpec.TrimShortHTML(input, markup)` — strips a single wrapping `<p>`..`</p>`
    /// when `bytes.Count(input, "<p>") == 1` (markdown only).
    // Go: helpers/content.go:TrimShortHTML
    pub fn trim_short_html(&self, input: &[u8], markup: &str) -> Vec<u8> { todo!() }
}

/// Go: `helpers.TotalWords(s)` (count of non-space runs).
// Go: helpers/content.go:TotalWords
pub fn total_words(s: &[u8]) -> i64 { todo!() }

/// Go: `helpers.BytesToHTML` -> `template.HTML`.
pub fn bytes_to_html(b: &[u8]) -> go_value::Value {
    go_value::Value::html(b.to_vec())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/content.go (186 lines; 4/8 funcs executed)
//   types: ContentSpec
// EX L48-82: NewContentSpec(cfg config.AllProvider, logger loggers.Logger, contentFs afero.Fs, ex *hexec.Exec) (*ContentSpec, error)
//    L85-87: stripEmptyNav(in []byte) []byte
// EX L90-92: BytesToHTML(b []byte) template.HTML
//    L95-128: ExtractTOC(content []byte) (newcontent []byte, toc []byte)
//    L130-132: (c *ContentSpec) SanitizeAnchorName(s string) string
//    L134-146: (c *ContentSpec) ResolveMarkup(in string) string
// EX L151-162: TotalWords(s string) int
// EX L167-186: (c *ContentSpec) TrimShortHTML(input []byte, markup string) []byte
// ---------------------------------------------------------------------------
