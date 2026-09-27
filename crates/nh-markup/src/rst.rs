//! Port of `markup/rst/convert.go`.
//!
//! STUB
//!
//! Owner: Wave B task T06 (markup).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/rst/convert.go (131 lines; 1/6 funcs executed)
//   types: provider, rstConverter
// EX L35-42: (p provider) New(cfg converter.ProviderConfig) (converter.Provider, error)
//    L49-55: (c *rstConverter) Convert(ctx converter.RenderContext) (converter.ResultRender, error)
//    L57-59: (c *rstConverter) Supports(feature identity.Identity) bool
//    L63-107: (c *rstConverter) getRstContent(src []byte, ctx converter.DocumentContext) ([]byte, error)
//    L111-118: getRstBinaryNameAndPath() (string, string)
//    L121-131: Supports() bool
// ---------------------------------------------------------------------------
