//! Port of `markup/converter/converter.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `markup/converter`: the markup-converter API between page content rendering (nh-hugolib) and
//! the markdown engine glue (goldmark).

use std::any::Any;
use std::sync::Arc;

use go_value::Value;
use nh_common::Result;
use nh_config::config_provider::AllProvider;
use nh_config::hexec::Exec;

use super::hooks::GetRendererFunc;
use crate::highlight::highlight::Highlighter;
use crate::markup_config::Config as MarkupConfig;
use crate::tableofcontents::Fragments;

/// Go: `converter.ProviderConfig`.
#[derive(Clone)]
pub struct ProviderConfig {
    pub conf: Arc<dyn AllProvider>,
    pub exec: Arc<Exec>,
    /// Go's `highlight.Highlighter` (nil = `NewConverterProvider` creates one).
    pub highlighter: Option<Arc<dyn Highlighter>>,
    /// Go's `Logger` (`None` = no logging).
    pub logger: Option<Arc<nh_common::loggers::Logger>>,
}

impl ProviderConfig {
    /// Go: `ProviderConfig.MarkupConfig()` = `Conf.GetConfigSection("markup")`.
    pub fn markup_config(&self) -> Arc<MarkupConfig> {
        nh_config::config_provider::config_section::<MarkupConfig>(&*self.conf, "markup")
    }
}

/// Go: `converter.Provider`.
pub trait Provider: Send + Sync {
    fn new_converter(&self, ctx: DocumentContext) -> Result<Arc<dyn Converter>>;
    fn name(&self) -> &str;
}

/// Go: `converter.DocumentContext`.
#[derive(Clone)]
pub struct DocumentContext {
    /// The page (Go `any`): a `page.Page` template value.
    pub document: Value,
    /// Go `DocumentLookup func(uint64) any` — hugocontext pid -> page (for `.PageInner`).
    pub document_lookup: Option<Arc<dyn Fn(u64) -> Value + Send + Sync>>,
    pub document_id: String,
    pub document_name: String,
    pub filename: String,
}

/// Go: `converter.RenderContext`.
pub struct RenderContext<'a> {
    /// Go `Ctx context.Context`: the template context (a `nh_tpl::TplContext`) as `HostCtx`.
    pub ctx: go_value::HostCtx<'a>,
    pub src: &'a [u8],
    pub render_toc: bool,
    pub get_renderer: Option<GetRendererFunc>,
}

/// Go: `converter.Converter`.
pub trait Converter: Send + Sync {
    fn convert(&self, ctx: &RenderContext<'_>) -> Result<ResultRender>;
    /// Go: `converter.ParseRenderer` (goldmark only): parse once (TOC), render later.
    fn as_parse_renderer(&self) -> Option<&dyn ParseRenderer> {
        None
    }
    /// Go: `converter.AnchorNameSanitizer`.
    fn sanitize_anchor_name(&self, _s: &str) -> Option<String> {
        None
    }
}

/// Go: `converter.ParseRenderer`.
pub trait ParseRenderer: Send + Sync {
    fn parse(&self, ctx: &RenderContext<'_>) -> Result<ResultParse>;
    fn render(&self, ctx: &RenderContext<'_>, doc: &ParsedDoc) -> Result<ResultRender>;
}

/// The parsed document (Go `Doc() any`: the goldmark AST).
pub type ParsedDoc = Arc<dyn Any + Send + Sync>;

/// Go: `converter.ResultParse`.
#[derive(Clone)]
pub struct ResultParse {
    pub doc: ParsedDoc,
    pub table_of_contents: Option<Arc<Fragments>>,
}

/// Go: `converter.ResultRender` (+ optional `TableOfContentsProvider`).
#[derive(Clone, Default)]
pub struct ResultRender {
    pub bytes: Vec<u8>,
    pub table_of_contents: Option<Arc<Fragments>>,
}

/// Go: `converter.NopConverter`.
pub struct NopConverter;

impl Converter for NopConverter {
    fn convert(&self, _ctx: &RenderContext<'_>) -> Result<ResultRender> {
        Ok(ResultRender::default())
    }
}

/// Go: `converter.NewProvider(name, create)`.
// Go: markup/converter/converter.go:NewProvider
pub fn new_provider(
    name: &str,
    create: Arc<dyn Fn(DocumentContext) -> Result<Arc<dyn Converter>> + Send + Sync>,
) -> Arc<dyn Provider> {
    Arc::new(NewConverter {
        name: name.to_string(),
        create,
    })
}

/// Go: `converter.newConverter`.
struct NewConverter {
    name: String,
    create: Arc<dyn Fn(DocumentContext) -> Result<Arc<dyn Converter>> + Send + Sync>,
}

impl Provider for NewConverter {
    // Go: markup/converter/converter.go:New
    fn new_converter(&self, ctx: DocumentContext) -> Result<Arc<dyn Converter>> {
        (self.create)(ctx)
    }

    // Go: markup/converter/converter.go:Name
    fn name(&self) -> &str {
        &self.name
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/converter/converter.go (158 lines; 4/7 funcs executed)
//   types: ProviderConfig, ProviderProvider, Provider, newConverter, nopConverter, Converter, ParseRenderer,
//          ResultRender, ResultParse, DocumentInfo, TableOfContentsProvider, AnchorNameSanitizer, Bytes,
//          DocumentContext, RenderContext
// OK L40-42: (p ProviderConfig) MarkupConfig() markup_config.Config
// OK L56-61: NewProvider(name string, create func(ctx DocumentContext) (Converter, error)) Provider
// OK L68-70: (n newConverter) New(ctx DocumentContext) (Converter, error)
// OK L72-74: (n newConverter) Name() string
// OK L80-82: (nopConverter) Convert(ctx RenderContext) (ResultRender, error)
// OK L84-86: (nopConverter) Supports(feature identity.Identity) bool
// OK L132-134: (b Bytes) Bytes() []byte (ResultRender.bytes)
// ---------------------------------------------------------------------------
