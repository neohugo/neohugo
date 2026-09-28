//! Port of `markup/highlight/highlight.go`.
//!
//! STUB highlighter: fenced code rendering returns an explicit unsupported error
//!
//! Owner: Wave B task T06 (markup).

use std::sync::Arc;

use go_value::{HostCtx, Value};
use nh_common::Result;
use nh_common::herrors::Error;

use super::config::Config;
use crate::converter::hooks::CodeBlockRenderer;

/// The error of every highlighting call (README rule 5: never approximate Chroma).
pub const UNSUPPORTED: &str = "neohugo-rs: highlighting (chroma) is not supported";

/// Go: `highlight.Highlighter`. STUB: the Rust port has no chroma; any fenced code block that
/// reaches the highlighter must fail loudly (explicit unsupported error), never render silently.
pub trait Highlighter: Send + Sync {
    fn highlight(&self, code: &str, lang: &str, opts: &go_value::Value) -> Result<String>;
    fn highlight_code_block(
        &self,
        ctx: &go_value::Value,
        opts: &go_value::Value,
    ) -> Result<go_value::Value>;
    fn is_default_code_block_renderer(&self) -> bool {
        true
    }
    /// Go: the highlighter is also a `hooks.CodeBlockRenderer` (the default one).
    fn as_code_block_renderer(self: Arc<Self>) -> Arc<dyn CodeBlockRenderer>;
}

/// Go: `highlight.chromaHighlighter` (stub).
pub struct ChromaHighlighter {
    pub cfg: Config,
}

/// Go: `highlight.New(cfg)` — returns the unsupported-highlighter stub.
// Go: markup/highlight/highlight.go:New
pub fn new(cfg: Config) -> Arc<dyn Highlighter> {
    Arc::new(ChromaHighlighter { cfg })
}

impl Highlighter for ChromaHighlighter {
    // Go: markup/highlight/highlight.go:Highlight
    fn highlight(&self, _code: &str, _lang: &str, _opts: &Value) -> Result<String> {
        Err(Error::new(UNSUPPORTED))
    }

    // Go: markup/highlight/highlight.go:HighlightCodeBlock
    fn highlight_code_block(&self, _ctx: &Value, _opts: &Value) -> Result<Value> {
        Err(Error::new(UNSUPPORTED))
    }

    // Go: markup/highlight/highlight.go:IsDefaultCodeBlockRenderer
    fn is_default_code_block_renderer(&self) -> bool {
        true
    }

    fn as_code_block_renderer(self: Arc<Self>) -> Arc<dyn CodeBlockRenderer> {
        self
    }
}

impl CodeBlockRenderer for ChromaHighlighter {
    // Go: markup/highlight/highlight.go:RenderCodeblock
    fn render_codeblock(&self, _cctx: HostCtx<'_>, _w: &mut Vec<u8>, _ctx: &Value) -> Result<()> {
        Err(Error::new(UNSUPPORTED))
    }

    fn is_default_code_block_renderer(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/highlight/highlight.go (360 lines; 2/22 funcs executed)
//   types: Highlighter, chromaHighlighter, HighlightResult, preWrapper, startEnd, byteCountFlexiWriter
// OK L49-53: init() (the attribute table lives in internal::attributes)
// OK L55-59: New(cfg Config) Highlighter
//    L72-84: (h chromaHighlighter) Highlight(code, lang string, opts any) (string, error) STUB
//    L86-123: (h chromaHighlighter) HighlightCodeBlock(ctx hooks.CodeblockContext, opts any) (HighlightResult, error) STUB
//    L125-142: (h chromaHighlighter) RenderCodeblock(cctx context.Context, w hugio.FlexiWriter, ctx hooks.CodeblockContext) error STUB
// OK L144-146: (h chromaHighlighter) IsDefaultCodeBlockRenderer() bool
//    L156-158: (h HighlightResult) Wrapped() template.HTML STUB
//    L161-163: (h HighlightResult) Inner() template.HTML STUB
//    L165-248: highlight(fw hugio.FlexiWriter, code, lang string, attributes []attributes.Attribute, cfg Config) (int, int, error) STUB
//    L250-252: getPreWrapper(language string, writeCounter *byteCountFlexiWriter) *preWrapper STUB
//    L261-270: (p *preWrapper) Start(code bool, styleAttr string) string STUB
//    L272-274: inlineCodeAttrs(lang string) string STUB
//    L276-284: WritePreStart(w io.Writer, language, styleAttr string) STUB
//    L288-291: (p *preWrapper) End(code bool) string STUB
//    L298-300: (s startEnd) Start(code bool, styleAttr string) string STUB
//    L302-304: (s startEnd) End(code bool) string STUB
//    L306-327: writeDivStart(w hugio.FlexiWriter, attrs []attributes.Attribute, wrapperClass string) STUB
//    L329-332: writeDivEnd(w hugio.FlexiWriter) STUB
//    L339-343: (w *byteCountFlexiWriter) Write(p []byte) (int, error) STUB
//    L345-348: (w *byteCountFlexiWriter) WriteByte(c byte) error STUB
//    L350-354: (w *byteCountFlexiWriter) WriteString(s string) (int, error) STUB
//    L356-360: (w *byteCountFlexiWriter) WriteRune(r rune) (int, error) STUB
// ---------------------------------------------------------------------------
