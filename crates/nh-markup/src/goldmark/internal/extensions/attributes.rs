//! Port of `markup/goldmark/internal/extensions/attributes/attributes.go`.
//!
//! Owner: Wave B task T06 (markup).


//! Go `markup/goldmark/internal/extensions/attributes`: the AST transformer that assigns
//! auto heading IDs (`generateAutoID` via `render.TextPlain` + `util.ResolveEntityNames`, last line
//! of multi-line setext headings) and handles `{#id .class}` attribute blocks.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/internal/extensions/attributes/attributes.go (204 lines; 6/14 funcs executed)
//   types: attrExtension, attrParser, attributesBlock, transformer
// EX L27-29: New(cfg goldmark_config.Parser) goldmark.Extender
// EX L35-47: (a *attrExtension) Extend(m goldmark.Markdown)
//    L51-53: (a *attrParser) CanAcceptIndentedLine() bool
//    L55-57: (a *attrParser) CanInterruptParagraph() bool
//    L59-60: (a *attrParser) Close(node ast.Node, reader text.Reader, pc parser.Context)
//    L62-64: (a *attrParser) Continue(node ast.Node, reader text.Reader, pc parser.Context) parser.State
//    L66-78: (a *attrParser) Open(parent ast.Node, reader text.Reader, pc parser.Context) (ast.Node, parser.State)
//    L80-82: (a *attrParser) Trigger() []byte
//    L88-99: (a *attributesBlock) Dump(source []byte, level int)
//    L101-103: (a *attributesBlock) Kind() ast.NodeKind
// EX L109-116: (a *transformer) isFragmentNode(n ast.Node) bool
// EX L118-169: (a *transformer) Transform(node *ast.Document, reader text.Reader, pc parser.Context)
// EX L171-188: (a *transformer) generateAutoID(n ast.Node, reader text.Reader, pc parser.Context)
// EX L191-204: textHeadingID(n *ast.Heading, reader text.Reader) []byte
// ---------------------------------------------------------------------------
