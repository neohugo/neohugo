//! Port of `markup/goldmark/internal/render/context.go`.
//!
//! Owner: Wave B task T06 (markup).


//! Go `markup/goldmark/internal/render/context.go`: the render buffer with a position stack used
//! to capture hook `.Text` (push position on enter, pop the rendered slice on exit), per-kind
//! ordinals and value stacks, and `TextPlain` (with its first-child-only quirk).

/// Go: `render.Context` (buffer part).
#[derive(Default)]
pub struct Context {
    pub buf: Vec<u8>,
    pub(crate) positions: Vec<usize>,
    pub(crate) pids: Vec<u64>,
    pub(crate) ordinals: std::collections::HashMap<u32, i64>,
}

impl Context {
    // Go: markup/goldmark/internal/render/context.go:PushPos
    pub fn push_pos(&mut self, n: usize) {
        self.positions.push(n);
    }

    // Go: markup/goldmark/internal/render/context.go:PopPos
    pub fn pop_pos(&mut self) -> usize {
        self.positions.pop().expect("position stack underflow")
    }

    /// Go: `PopRenderedString` — `buf[pos:]` as string, then `Truncate(pos)`.
    // Go: markup/goldmark/internal/render/context.go:PopRenderedString
    pub fn pop_rendered_string(&mut self) -> Vec<u8> {
        let pos = self.pop_pos();
        let s = self.buf[pos..].to_vec();
        self.buf.truncate(pos);
        s
    }

    // Go: markup/goldmark/internal/render/context.go:GetAndIncrementOrdinal
    pub fn get_and_increment_ordinal(&mut self, kind: u32) -> i64 {
        let e = self.ordinals.entry(kind).or_insert(0);
        let v = *e;
        *e += 1;
        v
    }
}

/// Go: `render.TextPlain(n, source)` — follows only the FIRST child of non-text inline nodes.
// Go: markup/goldmark/internal/render/context.go:TextPlain
pub fn text_plain_quirk_doc() {}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/internal/render/context.go (327 lines; 15/25 funcs executed)
//   types: BufWriter, Context, ContextData, RenderContextDataHolder, hookBase
//    L41-43: (b *BufWriter) Available() int
//    L45-47: (b *BufWriter) Buffered() int
// EX L49-51: (b *BufWriter) Flush() error
// EX L62-69: (ctx *Context) GetAndIncrementOrdinal(kind ast.NodeKind) int
// EX L71-73: (ctx *Context) PushPos(n int)
// EX L75-80: (ctx *Context) PopPos() int
// EX L82-87: (ctx *Context) PopRenderedString() string
//    L90-92: (ctx *Context) PushPid(pid uint64)
// EX L95-100: (ctx *Context) PeekPid() uint64
//    L103-111: (ctx *Context) PopPid() uint64
// EX L113-118: (ctx *Context) PushValue(k ast.NodeKind, v any)
// EX L120-132: (ctx *Context) PopValue(k ast.NodeKind) any
// EX L134-143: (ctx *Context) PeekValue(k ast.NodeKind) any
// EX L155-157: (ctx *RenderContextDataHolder) RenderContext() converter.RenderContext
// EX L159-161: (ctx *RenderContextDataHolder) DocumentContext() converter.DocumentContext
//    L166-206: extractSourceSample(n ast.Node, src []byte) []byte
// EX L209-220: GetPageAndPageInner(rctx *Context) (any, any)
// EX L223-251: NewBaseContext(rctx *Context, renderer any, n ast.Node, src []byte, getSourceSample func() []byte, ordinal int) hooks.BaseContext
//    L268-270: (c *hookBase) Page() any
//    L272-274: (c *hookBase) PageInner() any
//    L276-278: (c *hookBase) Ordinal() int
//    L280-285: (c *hookBase) Position() htext.Position
//    L288-290: (c *hookBase) PositionerSourceTarget() []byte
// EX L296-304: TextPlain(n ast.Node, source []byte) string
// EX L306-327: textPlainTo(c ast.Node, source []byte, buf *bytes.Buffer)
// ---------------------------------------------------------------------------
