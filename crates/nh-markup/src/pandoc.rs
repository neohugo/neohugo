//! Port of `markup/pandoc/convert.go`.
//!
//! STUB
//!
//! Owner: Wave B task T06 (markup).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/pandoc/convert.go (89 lines; 1/6 funcs executed)
//   types: provider, pandocConverter
// EX L31-38: (p provider) New(cfg converter.ProviderConfig) (converter.Provider, error)
//    L45-51: (c *pandocConverter) Convert(ctx converter.RenderContext) (converter.ResultRender, error)
//    L53-55: (c *pandocConverter) Supports(feature identity.Identity) bool
//    L58-68: (c *pandocConverter) getPandocContent(src []byte, ctx converter.DocumentContext) ([]byte, error)
//    L72-77: getPandocBinaryName() string
//    L80-89: Supports() bool
// ---------------------------------------------------------------------------
