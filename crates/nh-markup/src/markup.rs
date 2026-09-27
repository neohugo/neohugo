//! Port of `markup/markup.go`.
//!
//! Owner: Wave B task T06 (markup).


//! Go `markup.ConverterProvider`: registry of markup converters by name/extension
//! (goldmark + stubs for asciidocext, pandoc, rst, org, html).

use std::collections::BTreeMap;
use std::sync::Arc;

use nh_common::Result;

use crate::converter::converter::{Provider, ProviderConfig};
use crate::highlight::highlight::Highlighter;
use crate::markup_config::Config as MarkupConfig;

/// Go: `markup.ConverterProvider`.
pub trait ConverterProvider: Send + Sync {
    /// Lookup by name or file extension (`md`, `markdown`, `goldmark`, `html`, ...).
    fn get(&self, name: &str) -> Option<Arc<dyn Provider>>;
    fn is_goldmark(&self, name: &str) -> bool;
    fn get_markup_config(&self) -> Arc<MarkupConfig>;
    fn get_highlighter(&self) -> Arc<dyn Highlighter>;
}

/// Go: `markup.converterRegistry`.
pub struct ConverterRegistry {
    pub converters: BTreeMap<String, Arc<dyn Provider>>,
    pub config: ProviderConfig,
}

/// Go: `markup.NewConverterProvider(cfg)`.
// Go: markup/markup.go:NewConverterProvider
pub fn new_converter_provider(cfg: ProviderConfig) -> Result<Arc<dyn ConverterProvider>> {
    todo!()
}

/// Go: `markup.ResolveMarkup(s)` — normalises markup names (`md` -> `markdown`, `goldmark`...).
// Go: markup/markup.go:ResolveMarkup
pub fn resolve_markup(s: &str) -> String {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/markup.go (152 lines; 5/7 funcs executed)
//   types: ConverterProvider, converterRegistry
// EX L36-98: NewConverterProvider(cfg converter.ProviderConfig) (ConverterProvider, error)
//    L118-121: (r *converterRegistry) IsGoldmark(name string) bool
// EX L123-125: (r *converterRegistry) Get(name string) converter.Provider
//    L127-129: (r *converterRegistry) GetHighlighter() highlight.Highlighter
// EX L131-133: (r *converterRegistry) GetMarkupConfig() markup_config.Config
// EX L135-139: addConverter(m map[string]converter.Provider, c converter.Provider, aliases ...string)
// EX L142-152: ResolveMarkup(s string) string
// ---------------------------------------------------------------------------
