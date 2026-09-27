//! Port of `markup/highlight/highlight.go`.
//!
//! STUB highlighter: fenced code rendering returns an explicit unsupported error
//!
//! Owner: Wave B task T06 (markup).


use nh_common::Result;

use super::config::Config;

/// Go: `highlight.Highlighter`. STUB: the Rust port has no chroma; any fenced code block that
/// reaches the highlighter must fail loudly (explicit unsupported error), never render silently.
pub trait Highlighter: Send + Sync {
    fn highlight(&self, code: &str, lang: &str, opts: &go_value::Value) -> Result<String>;
    fn highlight_code_block(&self, ctx: &go_value::Value, opts: &go_value::Value) -> Result<go_value::Value>;
    fn is_default_code_block_renderer(&self) -> bool {
        true
    }
}

/// Go: `highlight.New(cfg)` — returns the unsupported-highlighter stub.
pub fn new(cfg: Config) -> std::sync::Arc<dyn Highlighter> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/highlight/highlight.go (360 lines; 2/22 funcs executed)
//   types: Highlighter, chromaHighlighter, HighlightResult, preWrapper, startEnd, byteCountFlexiWriter
// EX L49-53: init()
// EX L55-59: New(cfg Config) Highlighter
//    L72-84: (h chromaHighlighter) Highlight(code, lang string, opts any) (string, error)
//    L86-123: (h chromaHighlighter) HighlightCodeBlock(ctx hooks.CodeblockContext, opts any) (HighlightResult, error)
//    L125-142: (h chromaHighlighter) RenderCodeblock(cctx context.Context, w hugio.FlexiWriter, ctx hooks.CodeblockContext) error
//    L144-146: (h chromaHighlighter) IsDefaultCodeBlockRenderer() bool
//    L156-158: (h HighlightResult) Wrapped() template.HTML
//    L161-163: (h HighlightResult) Inner() template.HTML
//    L165-248: highlight(fw hugio.FlexiWriter, code, lang string, attributes []attributes.Attribute, cfg Config) (int, int, error)
//    L250-252: getPreWrapper(language string, writeCounter *byteCountFlexiWriter) *preWrapper
//    L261-270: (p *preWrapper) Start(code bool, styleAttr string) string
//    L272-274: inlineCodeAttrs(lang string) string
//    L276-284: WritePreStart(w io.Writer, language, styleAttr string)
//    L288-291: (p *preWrapper) End(code bool) string
//    L298-300: (s startEnd) Start(code bool, styleAttr string) string
//    L302-304: (s startEnd) End(code bool) string
//    L306-327: writeDivStart(w hugio.FlexiWriter, attrs []attributes.Attribute, wrapperClass string)
//    L329-332: writeDivEnd(w hugio.FlexiWriter)
//    L339-343: (w *byteCountFlexiWriter) Write(p []byte) (int, error)
//    L345-348: (w *byteCountFlexiWriter) WriteByte(c byte) error
//    L350-354: (w *byteCountFlexiWriter) WriteString(s string) (int, error)
//    L356-360: (w *byteCountFlexiWriter) WriteRune(r rune) (int, error)
// ---------------------------------------------------------------------------
