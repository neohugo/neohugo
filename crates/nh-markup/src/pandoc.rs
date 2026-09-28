//! Port of `markup/pandoc/convert.go`.
//!
//! STUB: converting returns `neohugo-rs: pandoc is not supported`.
//!
//! Owner: Wave B task T06 (markup).

use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::Error;

use crate::converter::converter::{
    Converter, DocumentContext, Provider, ProviderConfig, RenderContext, ResultRender,
    new_provider as converter_new_provider,
};

/// Go: `pandocConverter` (stub).
struct PandocConverter;

impl Converter for PandocConverter {
    // Go: markup/pandoc/convert.go:Convert
    fn convert(&self, _ctx: &RenderContext<'_>) -> Result<ResultRender> {
        Err(Error::new("neohugo-rs: pandoc is not supported"))
    }
}

/// Go: `pandoc.Provider.New(cfg)`: registering the converter works; converting fails.
// Go: markup/pandoc/convert.go:New
pub fn new_provider(_cfg: &ProviderConfig) -> Result<Arc<dyn Provider>> {
    Ok(converter_new_provider(
        "pandoc",
        Arc::new(|_ctx: DocumentContext| Ok(Arc::new(PandocConverter) as Arc<dyn Converter>)),
    ))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/pandoc/convert.go (89 lines; 1/6 funcs executed)
//   types: provider, pandocConverter
// OK L31-38: (p provider) New(cfg converter.ProviderConfig) (converter.Provider, error)
//    L45-51: (c *pandocConverter) Convert(ctx converter.RenderContext) (converter.ResultRender, error) STUB
//    L53-55: (c *pandocConverter) Supports(feature identity.Identity) bool STUB
//    L58-68: (c *pandocConverter) getPandocContent(src []byte, ctx converter.DocumentContext) ([]byte, error) STUB
//    L72-77: getPandocBinaryName() string STUB
//    L80-89: Supports() bool STUB
// ---------------------------------------------------------------------------
