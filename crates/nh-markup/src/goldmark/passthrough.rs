//! Port of `markup/goldmark/passthrough/passthrough.go`.
//!
//! STUB (passthrough disabled): `markup.goldmark.extensions.passthrough.enable = true` makes
//! `newMarkdown` fail with `neohugo-rs: the goldmark passthrough extension is not supported`
//! (see `goldmark::convert`). The extension (hugo-goldmark-extensions/passthrough) and its
//! renderer are not ported.
//!
//! Owner: Wave B task T06 (markup).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/passthrough/passthrough.go (166 lines; 0/7 funcs executed)
//   types: (group), passthroughContext
//    L30-35: New(cfg goldmark_config.Passthrough) goldmark.Extender STUB (error in newMarkdown)
//    L44-80: (e *passthroughExtension) Extend(m goldmark.Markdown) STUB
//    L82-85: newHTMLRenderer() renderer.NodeRenderer STUB
//    L87-90: (r *htmlRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer) STUB
//    L92-149: (r *htmlRenderer) renderPassthroughBlock(w util.BufWriter, src []byte, node ast.Node, entering bool) (ast.WalkStatus, error) STUB
//    L160-162: (p *passthroughContext) Type() string STUB
//    L164-166: (p *passthroughContext) Inner() string STUB
// ---------------------------------------------------------------------------
