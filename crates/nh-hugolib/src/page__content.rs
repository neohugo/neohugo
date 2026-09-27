//! Port of `hugolib/page__content.go` (render half).
//!
//! Owner: Wave B task T22 (hugolib-content).


//! Go `hugolib/page__content.go` (render half; parsing is `page__content_parse.rs`, T20):
//! `contentToRender` (shortcode placeholders `HAHAHUGOSHORTCODE<pid>s<n>HBHB`), markup conversion
//! (parse once for TOC, render per output), `expandShortcodeTokens` (incl. the `<p>TOKEN</p>`
//! unwrap and the `(k+4)` bounds bug), summary, `.Plain` (tpl::strip_html), word counts,
//! `RenderString`, `RenderShortcodes`.
//!
//! Caching (HUGO_LAYER.md §7.4, §4.8): a scope object per (markup scope + output format name) in
//! `CachedContent.scopes`; the rendered results in the page's SITE `PageMap` partitions
//! (`cache_content_rendered` / `cache_content_plain` / `cache_content_toc`) keyed
//! `sourceKey + "/" + markupScope(ctx) + outputFormat.Name`. Never render while holding a lock:
//! rendering re-enters templates, other pages' content and this page's other scopes.
//!
//! `RenderShortcodes`: when the context has `is_in_goldmark` (set ONLY for `{{% %}}` shortcodes,
//! shortcode.go:331), the result is wrapped with `hugocontext.Wrap(content, pid)`
//! (page__content.go:1119-1124).

use std::collections::BTreeMap;
use std::sync::Arc;

use nh_common::Result;

use crate::page__per_output::PageContentOutput;

pub use crate::page__content_parse::{CachedContent, ContentItem, ContentParseInfo};

/// Go: `cachedContentScope` — `cachedContent` seen through one `pageContentOutput` and one
/// markup scope. Holds no result caches itself (see the module docs).
pub struct CachedContentScope {
    pub scope: String,
    /// Go `pco`: the content output that created this scope (first creator wins).
    pub pco: Arc<PageContentOutput>,
}

/// Go: `contentSummary`.
#[derive(Clone, Debug, Default)]
pub struct ContentSummary {
    pub content: Vec<u8>,
    pub content_without_summary: Vec<u8>,
    pub summary: nh_page::page_markup::Summary,
}

/// Go: `contentPlainPlainWords`.
#[derive(Clone, Debug, Default)]
pub struct ContentPlainPlainWords {
    pub plain: Vec<u8>,
    pub plain_words: Vec<Vec<u8>>,
    pub word_count: i64,
    pub fuzzy_word_count: i64,
    pub reading_time: i64,
}

/// Go: `contentTableOfContents`.
#[derive(Clone, Default)]
pub struct ContentTableOfContents {
    pub content_to_render: Vec<u8>,
    pub table_of_contents: Option<Arc<nh_markup::tableofcontents::Fragments>>,
    pub table_of_contents_html: Vec<u8>,
    /// Placeholder -> rendered shortcode output.
    pub content_placeholders: BTreeMap<String, Vec<u8>>,
    /// The parsed document (goldmark AST) for render.
    pub ast_doc: Option<nh_markup::converter::converter::ParsedDoc>,
}

/// Go: `(c *cachedContent) getOrCreateScope(scope, pco)` — key `scope + pco.po.f.Name`.
// Go: hugolib/page__content.go:getOrCreateScope
pub fn get_or_create_scope(c: &CachedContent, scope: &str, pco: &Arc<PageContentOutput>) -> Arc<CachedContentScope> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__content.go (1191 lines; 23/39 funcs executed) — parse half in page__content_parse.rs (T20)
//   types: contentTableOfContents, contentSummary, contentPlainPlainWords, contextKey, cachedContentScope
// EX L171-181: (c *cachedContent) getOrCreateScope(scope string, pco *pageContentOutput) *cachedContentScope
// EX L227-264: (pi *contentParseInfo) contentToRender(ctx context.Context, source []byte, renderedShortcodes map[string]shortcodeRenderer) ([]byte, bool, error)
// EX L522-524: (c *cachedContentScope) keyScope(ctx context.Context) string
// EX L526-660: (c *cachedContentScope) contentRendered(ctx context.Context) (contentSummary, error)
//    L662-668: (c *cachedContentScope) mustContentToC(ctx context.Context) contentTableOfContents
// EX L678-791: (c *cachedContentScope) contentToC(ctx context.Context) (contentTableOfContents, error)
// EX L793-796: (c *cachedContent) version(cp *pageContentOutput) uint32
// EX L798-858: (c *cachedContentScope) contentPlain(ctx context.Context) (contentPlainPlainWords, error)
// EX L866-876: (c *cachedContentScope) prepareContext(ctx context.Context) context.Context
// EX L878-880: (c *cachedContentScope) Render(ctx context.Context) (page.Content, error)
// EX L882-889: (c *cachedContentScope) Content(ctx context.Context) (template.HTML, error)
//    L891-898: (c *cachedContentScope) ContentWithoutSummary(ctx context.Context) (template.HTML, error)
//    L900-904: (c *cachedContentScope) Summary(ctx context.Context) (page.Summary, error)
// EX L906-1059: (c *cachedContentScope) RenderString(ctx context.Context, args ...any) (template.HTML, error)
//    L1061-1131: (c *cachedContentScope) RenderShortcodes(ctx context.Context) (template.HTML, error)
// EX L1133-1136: (c *cachedContentScope) Plain(ctx context.Context) string
//    L1138-1141: (c *cachedContentScope) PlainWords(ctx context.Context) []string
//    L1143-1146: (c *cachedContentScope) WordCount(ctx context.Context) int
//    L1148-1151: (c *cachedContentScope) FuzzyWordCount(ctx context.Context) int
//    L1153-1156: (c *cachedContentScope) ReadingTime(ctx context.Context) int
//    L1158-1161: (c *cachedContentScope) Len(ctx context.Context) int
//    L1163-1170: (c *cachedContentScope) Fragments(ctx context.Context) *tableofcontents.Fragments
//    L1172-1175: (c *cachedContentScope) fragmentsHTML(ctx context.Context) template.HTML
// EX L1177-1183: (c *cachedContentScope) mustContentPlain(ctx context.Context) contentPlainPlainWords
//    L1185-1191: (c *cachedContentScope) mustContentRendered(ctx context.Context) contentSummary
// ---------------------------------------------------------------------------
