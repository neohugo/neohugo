//! Port of `markup/goldmark/images/transform.go`.
//!
//! Owner: Wave B task T06 (markup).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/images/transform.go (76 lines; 3/3 funcs executed)
//   types: (group), Transformer
// EX L24-26: New(wrapStandAloneImageWithinParagraph bool) goldmark.Extender
// EX L28-34: (e *imagesExtension) Extend(m goldmark.Markdown)
// EX L41-76: (t *Transformer) Transform(doc *ast.Document, reader text.Reader, pctx parser.Context)
// ---------------------------------------------------------------------------
