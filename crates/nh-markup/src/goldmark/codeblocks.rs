//! Port of `markup/goldmark/codeblocks/render.go`.
//!
//! Owner: Wave B task T06 (markup).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/codeblocks/render.go (179 lines; 4/9 funcs executed)
//   types: (group), codeBlockContext
// EX L41-43: New() goldmark.Extender
// EX L45-49: (e *codeBlocksExtension) Extend(m goldmark.Markdown)
// EX L51-54: newHTMLRenderer() renderer.NodeRenderer
// EX L56-58: (r *htmlRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer)
//    L60-123: (r *htmlRenderer) renderCodeBlock(w util.BufWriter, src []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
//    L133-135: (c *codeBlockContext) Type() string
//    L137-139: (c *codeBlockContext) Inner() string
//    L141-145: getLang(node *ast.FencedCodeBlock, src []byte) string
//    L147-179: getAttributes(node *ast.FencedCodeBlock, infostr []byte) ([]ast.Attribute, string, error)
// ---------------------------------------------------------------------------
