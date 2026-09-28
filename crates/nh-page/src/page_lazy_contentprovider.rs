//! Port of `resources/page/page_lazy_contentprovider.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Go `page.LazyContentProvider`: a content provider that creates the real provider on first
//! use and can be RESET (Go `lcp.init.Reset()`), after which the next use creates a new one.
//! nh-hugolib's `shiftToOutputFormat` (page__init.rs) installs one on the current output of every
//! page of the sites that are NOT rendering, and resets it on each further shift
//! (page.go:719-743). The seeksnack build executes `NewLazyContentProvider`, `Reset`, `Plain`,
//! `ParseAndRenderContent`, `ParseContent` and `RenderContent` through it.
//!
//! Never create the provider while holding the lock (HUGO_LAYER.md §4.8): creation renders
//! nothing itself, but keep the rule uniform (read under lock, create outside, first stored wins).

use std::any::Any;
use std::sync::{Arc, Mutex};

use nh_common::Result;

/// Go: `page.OutputFormatContentProvider` — implemented by nh-hugolib's `PageContentOutput`.
/// Methods take the template context (`HostCtx`) where Go takes `context.Context`. Only the
/// members the build reaches are listed; add the rest of the Go interface as needed.
pub trait OutputFormatContentProvider: Send + Sync {
    /// Go `Plain(ctx)`.
    fn plain(&self, ctx: go_value::HostCtx<'_>) -> Result<go_value::GoString>;
    /// Go `Content(ctx)` (returns `any`: a `template.HTML`).
    fn content(&self, ctx: go_value::HostCtx<'_>) -> Result<go_value::Value>;
    /// Downcast to the concrete provider (nh-hugolib uses it to reach ParseContent/RenderContent).
    fn as_any(&self) -> &dyn Any;
}

/// The factory Go passes to `NewLazyContentProvider`.
pub type ContentProviderFactory =
    Box<dyn Fn() -> Result<Arc<dyn OutputFormatContentProvider>> + Send + Sync>;

/// Go: `page.LazyContentProvider`.
pub struct LazyContentProvider {
    f: ContentProviderFactory,
    /// Go `init *lazy.Init` + `cp`: `None` until first use or after `reset()`.
    cp: Mutex<Option<Arc<dyn OutputFormatContentProvider>>>,
}

impl LazyContentProvider {
    /// Go: `NewLazyContentProvider(f)`.
    // Go: resources/page/page_lazy_contentprovider.go:NewLazyContentProvider
    pub fn new(f: ContentProviderFactory) -> LazyContentProvider {
        LazyContentProvider {
            f,
            cp: Mutex::new(None),
        }
    }

    /// Go: `Reset()`.
    // Go: resources/page/page_lazy_contentprovider.go:Reset
    pub fn reset(&self) {
        *self.cp.lock().unwrap() = None;
    }

    /// The provider, created on first use (Go `lcp.init.Do(ctx)`).
    pub fn provider(&self) -> Result<Arc<dyn OutputFormatContentProvider>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_lazy_contentprovider.go (181 lines; 6/20 funcs executed)
//   types: OutputFormatContentProvider, OutputFormatPageContentProvider, LazyContentProvider
// EX L58-72: NewLazyContentProvider(f func() (OutputFormatContentProvider, error)) *LazyContentProvider
// EX L74-76: (lcp *LazyContentProvider) Reset()
//    L78-81: (lcp *LazyContentProvider) Markup(opts ...any) Markup
//    L83-87: (lcp *LazyContentProvider) TableOfContents(ctx context.Context) template.HTML
//    L89-93: (lcp *LazyContentProvider) Fragments(ctx context.Context) *tableofcontents.Fragments
//    L95-99: (lcp *LazyContentProvider) Content(ctx context.Context) (any, error)
//    L101-104: (lcp *LazyContentProvider) ContentWithoutSummary(ctx context.Context) (template.HTML, error)
// EX L106-110: (lcp *LazyContentProvider) Plain(ctx context.Context) string
//    L112-116: (lcp *LazyContentProvider) PlainWords(ctx context.Context) []string
//    L118-122: (lcp *LazyContentProvider) Summary(ctx context.Context) template.HTML
//    L124-128: (lcp *LazyContentProvider) Truncated(ctx context.Context) bool
//    L130-134: (lcp *LazyContentProvider) FuzzyWordCount(ctx context.Context) int
//    L136-140: (lcp *LazyContentProvider) WordCount(ctx context.Context) int
//    L142-146: (lcp *LazyContentProvider) ReadingTime(ctx context.Context) int
//    L148-152: (lcp *LazyContentProvider) Len(ctx context.Context) int
//    L154-157: (lcp *LazyContentProvider) Render(ctx context.Context, layout ...string) (template.HTML, error)
//    L159-163: (lcp *LazyContentProvider) RenderString(ctx context.Context, args ...any) (template.HTML, error)
// EX L165-169: (lcp *LazyContentProvider) ParseAndRenderContent(ctx context.Context, content []byte, renderTOC bool) (converter.ResultRender, error)
// EX L171-175: (lcp *LazyContentProvider) ParseContent(ctx context.Context, content []byte) (converter.ResultParse, bool, error)
// EX L177-181: (lcp *LazyContentProvider) RenderContent(ctx context.Context, content []byte, doc any) (converter.ResultRender, bool, error)
// ---------------------------------------------------------------------------
