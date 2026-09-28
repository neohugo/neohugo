//! Port of `markup/rst/convert.go`.
//!
//! STUB: converting returns `neohugo-rs: reStructuredText (rst2html) is not supported`.
//!
//! Owner: Wave B task T06 (markup).

use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::Error;

use crate::converter::converter::{
    Converter, DocumentContext, Provider, ProviderConfig, RenderContext, ResultRender,
    new_provider as converter_new_provider,
};

/// Go: `rstConverter` (stub).
struct RstConverter;

impl Converter for RstConverter {
    // Go: markup/rst/convert.go:Convert
    fn convert(&self, _ctx: &RenderContext<'_>) -> Result<ResultRender> {
        Err(Error::new(
            "neohugo-rs: reStructuredText (rst2html) is not supported",
        ))
    }
}

/// Go: `rst.Provider.New(cfg)`: registering the converter works; converting fails.
// Go: markup/rst/convert.go:New
pub fn new_provider(_cfg: &ProviderConfig) -> Result<Arc<dyn Provider>> {
    Ok(converter_new_provider(
        "rst",
        Arc::new(|_ctx: DocumentContext| Ok(Arc::new(RstConverter) as Arc<dyn Converter>)),
    ))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/rst/convert.go (131 lines; 1/6 funcs executed)
//   types: provider, rstConverter
// OK L35-42: (p provider) New(cfg converter.ProviderConfig) (converter.Provider, error)
//    L49-55: (c *rstConverter) Convert(ctx converter.RenderContext) (converter.ResultRender, error) STUB
//    L57-59: (c *rstConverter) Supports(feature identity.Identity) bool STUB
//    L63-107: (c *rstConverter) getRstContent(src []byte, ctx converter.DocumentContext) ([]byte, error) STUB
//    L111-118: getRstBinaryNameAndPath() (string, string) STUB
//    L121-131: Supports() bool STUB
// ---------------------------------------------------------------------------
