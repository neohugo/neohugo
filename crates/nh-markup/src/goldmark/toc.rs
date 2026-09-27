//! Port of `markup/goldmark/toc.go`.
//!
//! Owner: Wave B task T06 (markup).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/toc.go (141 lines; 3/3 funcs executed)
//   types: tocTransformer, tocExtension
// EX L42-121: (t *tocTransformer) Transform(n *ast.Document, reader text.Reader, pc parser.Context)
// EX L127-131: newTocExtension(options []renderer.Option) goldmark.Extender
// EX L133-141: (e *tocExtension) Extend(m goldmark.Markdown)
// ---------------------------------------------------------------------------
