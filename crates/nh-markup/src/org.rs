//! Port of `markup/org/convert.go`.
//!
//! STUB: converting returns `neohugo-rs: org-mode (go-org) is not supported`.
//!
//! Owner: Wave B task T06 (markup).

use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::Error;

use crate::converter::converter::{
    Converter, DocumentContext, Provider, ProviderConfig, RenderContext, ResultRender,
    new_provider as converter_new_provider,
};

/// Go: `orgConverter` (stub).
struct OrgConverter;

impl Converter for OrgConverter {
    // Go: markup/org/convert.go:Convert
    fn convert(&self, _ctx: &RenderContext<'_>) -> Result<ResultRender> {
        Err(Error::new("neohugo-rs: org-mode (go-org) is not supported"))
    }
}

/// Go: `org.Provider.New(cfg)`: registering the converter works; converting fails.
// Go: markup/org/convert.go:New
pub fn new_provider(_cfg: &ProviderConfig) -> Result<Arc<dyn Provider>> {
    Ok(converter_new_provider(
        "org",
        Arc::new(|_ctx: DocumentContext| Ok(Arc::new(OrgConverter) as Arc<dyn Converter>)),
    ))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/org/convert.go (74 lines; 1/3 funcs executed)
//   types: provide, orgConverter
// OK L33-40: (p provide) New(cfg converter.ProviderConfig) (converter.Provider, error)
//    L47-70: (c *orgConverter) Convert(ctx converter.RenderContext) (converter.ResultRender, error) STUB
//    L72-74: (c *orgConverter) Supports(feature identity.Identity) bool STUB
// ---------------------------------------------------------------------------
