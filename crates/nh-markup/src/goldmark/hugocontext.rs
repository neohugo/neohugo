//! Port of `markup/goldmark/hugocontext/hugocontext.go`.
//!
//! Owner: Wave B task T06 (markup).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/hugocontext/hugocontext.go (317 lines; 10/18 funcs executed)
//   types: HugoContext, hugoContextParser, hugoContextRenderer, hugoContextTransformer, hugoContextExtension
// EX L35-37: New(logger loggers.Logger) goldmark.Extender
//    L41-58: Wrap(b []byte, pid uint64) string
//    L73-77: (n *HugoContext) Dump(source []byte, level int)
//    L79-94: (n *HugoContext) parseAttrs(attrBytes []byte)
//    L96-98: (h *HugoContext) Kind() ast.NodeKind
// EX L111-113: (a *hugoContextParser) Trigger() []byte
// EX L115-135: (s *hugoContextParser) Parse(parent ast.Node, reader text.Reader, pc parser.Context) ast.Node
// EX L142-144: (r *hugoContextRenderer) SetOption(name renderer.OptionName, value any)
// EX L146-150: (r *hugoContextRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer)
// EX L152-157: (r *hugoContextRenderer) stripHugoCtx(b []byte) ([]byte, bool)
//    L159-161: (r *hugoContextRenderer) logRawHTMLEmittedWarn(w util.BufWriter)
//    L163-170: (r *hugoContextRenderer) getPage(w util.BufWriter) any
//    L172-174: (r *hugoContextRenderer) isHTMLComment(b []byte) bool
// EX L177-218: (r *hugoContextRenderer) renderHTMLBlock( w util.BufWriter, source []byte, node ast.Node, entering bool, ) (ast.WalkStatus, error)
// EX L220-242: (r *hugoContextRenderer) renderRawHTML( w util.BufWriter, source []byte, node ast.Node, entering bool, ) (ast.WalkStatus, error)
//    L244-260: (r *hugoContextRenderer) handleHugoContext(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// EX L266-293: (a *hugoContextTransformer) Transform(n *ast.Document, reader text.Reader, pc parser.Context)
// EX L299-317: (a *hugoContextExtension) Extend(m goldmark.Markdown)
// ---------------------------------------------------------------------------
