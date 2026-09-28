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
//! Go's semantics, kept here: the provider starts as the nop provider (`NopCPageContentRenderer`);
//! the factory runs once (`lazy.Init`), and a failure leaves the previous provider in place (the
//! methods ignore the init error and delegate to whatever `cp` is); `Reset` only allows the
//! factory to run again, it does not drop the current provider.
//!
//! Never create the provider while holding the lock (HUGO_LAYER.md §4.8): the factory runs
//! between two short critical sections, and the first stored result wins.

use std::any::Any;
use std::sync::{Arc, Mutex};

use go_value::{GoString, HostCtx, Value};
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

/// Go `lazy.Init` state of the provider.
#[derive(Default)]
struct LazyState {
    /// The init ran (Go `sync.Once` done); cleared by `Reset`.
    done: bool,
    /// Go `lcp.cp`; `None` is `NopCPageContentRenderer`.
    cp: Option<Arc<dyn OutputFormatContentProvider>>,
    /// Go `ini.err` (cleared by `Reset`).
    err: Option<nh_common::herrors::Error>,
}

/// Go: `page.LazyContentProvider`.
pub struct LazyContentProvider {
    f: ContentProviderFactory,
    /// Go `init *lazy.Init` + `cp`.
    cp: Mutex<LazyState>,
}

impl LazyContentProvider {
    /// Go: `NewLazyContentProvider(f)`.
    // Go: resources/page/page_lazy_contentprovider.go:NewLazyContentProvider
    pub fn new(f: ContentProviderFactory) -> LazyContentProvider {
        LazyContentProvider {
            f,
            cp: Mutex::new(LazyState::default()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, LazyState> {
        self.cp.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Go: `Reset()` — the next use runs the factory again.
    // Go: resources/page/page_lazy_contentprovider.go:Reset
    pub fn reset(&self) {
        let mut st = self.lock();
        st.done = false;
        st.err = None;
    }

    /// Go: `lcp.init.Do(ctx)` — runs the factory unless it already ran since the last reset;
    /// returns the init error (Go's callers ignore it).
    pub fn init_do(&self) -> Result<()> {
        {
            let st = self.lock();
            if st.done {
                return match &st.err {
                    Some(e) => Err(e.clone()),
                    None => Ok(()),
                };
            }
        }
        // Created outside the lock (HUGO_LAYER.md §4.8).
        let r = (self.f)();
        let mut st = self.lock();
        if st.done {
            // Another caller finished first: its result wins.
            return match &st.err {
                Some(e) => Err(e.clone()),
                None => Ok(()),
            };
        }
        st.done = true;
        match r {
            Ok(cp) => {
                st.cp = Some(cp);
                Ok(())
            }
            Err(e) => {
                st.err = Some(e.clone());
                Err(e)
            }
        }
    }

    /// The current provider after `init.Do` (`None` is the nop provider).
    pub fn current(&self) -> Option<Arc<dyn OutputFormatContentProvider>> {
        let _ = self.init_do();
        self.lock().cp.clone()
    }

    /// The provider, created on first use (Go `lcp.init.Do(ctx)`). An init error is returned
    /// when there is no provider (Go would use the nop provider).
    pub fn provider(&self) -> Result<Arc<dyn OutputFormatContentProvider>> {
        let r = self.init_do();
        match (self.lock().cp.clone(), r) {
            (Some(cp), _) => Ok(cp),
            (None, Err(e)) => Err(e),
            (None, Ok(())) => Err(nh_common::herrors::Error::new(
                "neohugo-rs: lazy content provider has no provider",
            )),
        }
    }

    /// Go: `lcp.Plain(ctx)` (the nop provider's `""`; an error gives `""` too, as Go's
    /// `Plain` has no error result).
    // Go: resources/page/page_lazy_contentprovider.go:Plain
    pub fn plain(&self, ctx: HostCtx<'_>) -> Result<GoString> {
        match self.current() {
            Some(cp) => cp.plain(ctx),
            None => Ok(GoString::empty()),
        }
    }

    /// Go: `lcp.Content(ctx)` (the nop provider's `""`).
    // Go: resources/page/page_lazy_contentprovider.go:Content
    pub fn content(&self, ctx: HostCtx<'_>) -> Result<Value> {
        match self.current() {
            Some(cp) => cp.content(ctx),
            None => Ok(Value::string("")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct P(usize);

    impl OutputFormatContentProvider for P {
        fn plain(&self, _ctx: HostCtx<'_>) -> Result<GoString> {
            Ok(GoString::from(format!("plain{}", self.0)))
        }
        fn content(&self, _ctx: HostCtx<'_>) -> Result<Value> {
            Ok(Value::html(format!("content{}", self.0)))
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn lcp(fail_from: usize) -> (Arc<AtomicUsize>, LazyContentProvider) {
        let n = Arc::new(AtomicUsize::new(0));
        let n2 = n.clone();
        let f: ContentProviderFactory = Box::new(move || {
            let i = n2.fetch_add(1, Ordering::SeqCst) + 1;
            if i >= fail_from {
                return Err(nh_common::herrors::Error::new(format!("fail{i}")));
            }
            Ok(Arc::new(P(i)) as Arc<dyn OutputFormatContentProvider>)
        });
        (n, LazyContentProvider::new(f))
    }

    /// Created once, shared by every method, recreated after `Reset`.
    #[test]
    fn once_and_reset() {
        let (n, l) = lcp(usize::MAX);
        assert_eq!(l.plain(&()).unwrap().as_bytes(), b"plain1");
        assert!(
            matches!(l.content(&()).unwrap(), Value::Safe(_, s) if s.as_bytes() == b"content1")
        );
        assert_eq!(n.load(Ordering::SeqCst), 1);
        l.reset();
        assert_eq!(n.load(Ordering::SeqCst), 1);
        assert_eq!(l.plain(&()).unwrap().as_bytes(), b"plain2");
        assert_eq!(n.load(Ordering::SeqCst), 2);
    }

    /// A failing factory leaves the nop provider (or the previous one after a reset), and is not
    /// retried until the next reset (Go `lazy.Init`).
    #[test]
    fn failures() {
        let (n, l) = lcp(1);
        assert_eq!(l.plain(&()).unwrap().as_bytes(), b"");
        assert!(l.init_do().is_err());
        assert!(l.provider().is_err());
        assert_eq!(n.load(Ordering::SeqCst), 1);

        let (n, l) = lcp(2);
        assert_eq!(l.plain(&()).unwrap().as_bytes(), b"plain1");
        l.reset();
        assert_eq!(l.plain(&()).unwrap().as_bytes(), b"plain1");
        assert_eq!(n.load(Ordering::SeqCst), 2);
        assert_eq!(
            l.provider().unwrap().plain(&()).unwrap().as_bytes(),
            b"plain1"
        );
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_lazy_contentprovider.go (181 lines; 6/20 funcs executed)
//   types: OutputFormatContentProvider, OutputFormatPageContentProvider, LazyContentProvider
// The methods other than Plain/Content are `init_do()` + a call on `current()` (the provider's
// concrete type, reached with `as_any`, or the nop page's result when `None`); nh-hugolib
// (T22/T23) wires them, see PORTING.md.
// OK L58-72: NewLazyContentProvider(f func() (OutputFormatContentProvider, error)) *LazyContentProvider
// OK L74-76: (lcp *LazyContentProvider) Reset()
// OK L78-81: (lcp *LazyContentProvider) Markup(opts ...any) Markup   (init_do + current)
// OK L83-87: (lcp *LazyContentProvider) TableOfContents(ctx context.Context) template.HTML   (init_do + current)
// OK L89-93: (lcp *LazyContentProvider) Fragments(ctx context.Context) *tableofcontents.Fragments   (init_do + current)
// OK L95-99: (lcp *LazyContentProvider) Content(ctx context.Context) (any, error)
// OK L101-104: (lcp *LazyContentProvider) ContentWithoutSummary(ctx context.Context) (template.HTML, error)   (init_do + current)
// OK L106-110: (lcp *LazyContentProvider) Plain(ctx context.Context) string
// OK L112-116: (lcp *LazyContentProvider) PlainWords(ctx context.Context) []string   (init_do + current)
// OK L118-122: (lcp *LazyContentProvider) Summary(ctx context.Context) template.HTML   (init_do + current)
// OK L124-128: (lcp *LazyContentProvider) Truncated(ctx context.Context) bool   (init_do + current)
// OK L130-134: (lcp *LazyContentProvider) FuzzyWordCount(ctx context.Context) int   (init_do + current)
// OK L136-140: (lcp *LazyContentProvider) WordCount(ctx context.Context) int   (init_do + current)
// OK L142-146: (lcp *LazyContentProvider) ReadingTime(ctx context.Context) int   (init_do + current)
// OK L148-152: (lcp *LazyContentProvider) Len(ctx context.Context) int   (init_do + current)
// OK L154-157: (lcp *LazyContentProvider) Render(ctx context.Context, layout ...string) (template.HTML, error)   (init_do + current)
// OK L159-163: (lcp *LazyContentProvider) RenderString(ctx context.Context, args ...any) (template.HTML, error)   (init_do + current)
// OK L165-169: (lcp *LazyContentProvider) ParseAndRenderContent(ctx context.Context, content []byte, renderTOC bool) (converter.ResultRender, error)   (init_do + current)
// OK L171-175: (lcp *LazyContentProvider) ParseContent(ctx context.Context, content []byte) (converter.ResultParse, bool, error)   (init_do + current)
// OK L177-181: (lcp *LazyContentProvider) RenderContent(ctx context.Context, content []byte, doc any) (converter.ResultRender, bool, error)   (init_do + current)
// ---------------------------------------------------------------------------
