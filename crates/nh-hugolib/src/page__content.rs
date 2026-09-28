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
//!
//! Go's context keys that only hugolib reads travel in `TplContext.host` as a [`HostState`]
//! (the content callback of `setGetContentCallbackInContext`).

use std::sync::Arc;

use go_value::{GoString, HostCtx, SafeKind, SliceType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_page::page_markup::{
    SUMMARY_TYPE_AUTO, SUMMARY_TYPE_FRONT_MATTER, SUMMARY_TYPE_MANUAL, Summary,
    extract_summary_from_html, extract_summary_from_html_with_divider,
};
use nh_parser::pageparser::pagelexer::Config as ParseConfig;
use nh_tpl::template::TplContext;

use crate::hugo_sites::HugoSites;
use crate::page::PageState;
use crate::page__per_output::{PageContentOutput, tpl_ctx};
use crate::shortcode::{ShortcodeRenderers, expand_shortcode_tokens};
use crate::shortcode_page::{ShortcodeRenderer, toc_shortcode_placeholder};

pub use crate::page__content_parse::{
    CachedContent, ContentItem, ContentParseInfo, INTERNAL_SUMMARY_DIVIDER_BASE,
};

/// Go: `pageDependencyScopeGlobal` (identity tracking; kept for the context's shape).
const PAGE_DEPENDENCY_SCOPE_GLOBAL: i64 = 2;

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
    /// Temporary storage of placeholders mapped to their content (Go `contentPlaceholders
    /// map[string]shortcodeRenderer`): shortcodes etc. Some of these will need to be replaced
    /// after any markup is rendered, so they share a common prefix. Shared like Go's map (the
    /// content callback merges into it).
    pub content_placeholders: ShortcodeRenderers,
    /// The parsed document (goldmark AST) for render.
    pub ast_doc: Option<nh_markup::converter::converter::ParsedDoc>,
}

/// Go: `setGetContentCallbackInContext`'s value: `func(*pageContentOutput, contentTableOfContents)`.
pub type ContentCallback =
    Arc<dyn Fn(&Arc<PageContentOutput>, &ContentTableOfContents) + Send + Sync>;

/// The hugolib-only keys of Go's `context.Context`, stored in `TplContext.host`.
#[derive(Clone, Default)]
pub struct HostState {
    /// Go `setGetContentCallbackInContext` (`contextKeyContentCallback`).
    pub content_callback: Option<ContentCallback>,
}

impl HostState {
    /// The state of a context (the zero state when unset).
    pub fn of(ctx: &TplContext) -> HostState {
        ctx.host
            .as_ref()
            .and_then(|h| h.downcast_ref::<HostState>())
            .cloned()
            .unwrap_or_default()
    }

    /// A context with this state (Go `context.WithValue`).
    pub fn set_on(self, ctx: &TplContext) -> TplContext {
        TplContext {
            host: Some(Arc::new(self)),
            ..ctx.clone()
        }
    }
}

/// Go: `(c *cachedContent) getOrCreateScope(scope, pco)` — key `scope + pco.po.f.Name`.
// Go: hugolib/page__content.go:getOrCreateScope
pub fn get_or_create_scope(
    c: &CachedContent,
    scope: &str,
    pco: &Arc<PageContentOutput>,
) -> Arc<CachedContentScope> {
    let key = format!("{}{}", scope, pco.po.f.name);
    c.scopes
        .get_or_create(key, |_| {
            Ok(Arc::new(CachedContentScope {
                scope: scope.to_string(),
                pco: pco.clone(),
            }))
        })
        .expect("create cannot fail")
}

/// `getOrCreateScope` on the page's cached content (a page without parsed content, e.g. one
/// created during assembly, gets a fresh scope: its results are cached by key in the PageMap).
pub(crate) fn get_or_create_scope_for(
    h: &Arc<HugoSites>,
    scope: &str,
    pco: &Arc<PageContentOutput>,
) -> Arc<CachedContentScope> {
    match &h.page(pco.po.p).content {
        Some(c) => get_or_create_scope(c, scope, pco),
        None => Arc::new(CachedContentScope {
            scope: scope.to_string(),
            pco: pco.clone(),
        }),
    }
}

/// Go: `(pi *contentParseInfo) contentToRender(ctx, source, renderedShortcodes)` — the content
/// to be processed by Goldmark or similar: source ranges, replacements, `{{% %}}` shortcodes
/// rendered in place and the placeholders of the others.
// Go: hugolib/page__content.go:contentToRender
pub fn content_to_render(
    pi: &ContentParseInfo,
    ctx: &TplContext,
    source: &[u8],
    rendered_shortcodes: &ShortcodeRenderers,
) -> Result<(Vec<u8>, bool)> {
    let mut has_variants = false;
    let mut c: Vec<u8> = Vec::with_capacity(source.len() + source.len() / 10);

    for it in &pi.items_step2 {
        match it {
            ContentItem::Source { low, high } => c.extend_from_slice(&source[*low..*high]),
            ContentItem::Replacement(v) => c.extend_from_slice(v),
            ContentItem::Shortcode(v) => {
                if !v.insert_placeholder() {
                    // Insert the rendered shortcode.
                    let rendered_shortcode = rendered_shortcodes
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(&v.placeholder)
                        .cloned();
                    let Some(rendered_shortcode) = rendered_shortcode else {
                        // This should never happen.
                        panic!("rendered shortcode {:?} not found", v.placeholder);
                    };

                    let (b, more) = rendered_shortcode
                        .render_shortcode(ctx)
                        .map_err(|err| err.wrap("failed to render shortcode"))?;
                    has_variants = has_variants || more;
                    c.extend_from_slice(&b);
                } else {
                    // Insert the placeholder so we can insert the content after
                    // markdown processing.
                    c.extend_from_slice(v.placeholder.as_bytes());
                }
            }
        }
    }

    Ok((c, has_variants))
}

/// The page, its parsed content (if any) and its site's PageMap.
struct ScopeCtx<'a> {
    p: &'a PageState,
    cc: Option<&'a Arc<CachedContent>>,
}

impl<'a> ScopeCtx<'a> {
    fn source_key(&self) -> String {
        match self.cc {
            Some(c) => c.pi.source_key.clone(),
            None => self.p.pid.to_string(),
        }
    }

    fn items_empty(&self) -> bool {
        self.cc.is_none_or(|c| c.pi.items_step2.is_empty())
    }

    fn has_summary_divider(&self) -> bool {
        self.cc.is_some_and(|c| c.pi.has_summary_divider)
    }
}

impl CachedContentScope {
    fn h(&self) -> Arc<HugoSites> {
        self.pco.h()
    }

    fn scope_ctx<'a>(&self, h: &'a Arc<HugoSites>) -> ScopeCtx<'a> {
        let p = h.page(self.pco.po.p);
        ScopeCtx {
            p,
            cc: p.content.as_ref(),
        }
    }

    /// Go: `keyScope(ctx)` — markup scope + the content output's format name.
    // Go: hugolib/page__content.go:keyScope
    fn key_scope(&self, ctx: &TplContext) -> String {
        format!("{}{}", ctx.markup_scope, self.pco.po.f.name)
    }

    /// Go: `contentRendered(ctx)`.
    // Go: hugolib/page__content.go:contentRendered
    pub fn content_rendered(&self, ctx: &TplContext) -> Result<ContentSummary> {
        let h = self.h();
        let sc = self.scope_ctx(&h);
        let cp = &self.pco;
        let mut ctx = ctx.clone();
        ctx.dependency_scope = PAGE_DEPENDENCY_SCOPE_GLOBAL;
        let ctx = &ctx;
        let key = format!("{}/{}", sc.source_key(), self.key_scope(ctx));
        let pm = &h.sites[sc.p.site_idx].page_map;

        let v = pm.cache_content_rendered.get_or_create(key, |_| {
            cp.content_rendered
                .store(true, std::sync::atomic::Ordering::SeqCst);
            let po = &cp.po;
            let p = sc.p;

            let ct = self.content_to_c(ctx)?;

            let rs = (|| -> Result<ContentSummary> {
                let mut result = ContentSummary::default();

                if sc.items_empty() {
                    // Nothing to do.
                    return Ok(result);
                }

                let mut b: Vec<u8> = if let Some(doc) = &ct.ast_doc {
                    // The content is parsed, but not rendered.
                    let r = match po.content_renderer() {
                        Some(cr) => cr.render_content(ctx, &ct.content_to_render, doc)?,
                        None => None,
                    };
                    let Some(r) = r else {
                        return Err(Error::new(
                            "invalid state: astDoc is set but RenderContent returned false",
                        ));
                    };
                    r.bytes
                } else {
                    // Copy the content to be rendered.
                    ct.content_to_render.clone()
                };

                // There are one or more replacement tokens to be replaced.
                let mut has_shortcode_variants = false;
                let toc_placeholder = toc_shortcode_placeholder();
                let mut token_handler = |token: &str| -> Result<Vec<u8>> {
                    if token == toc_placeholder {
                        return Ok(ct.table_of_contents_html.clone());
                    }
                    let renderer = ct
                        .content_placeholders
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(token)
                        .cloned();
                    if let Some(renderer) = renderer {
                        let (repl, more) = renderer.render_shortcode(ctx)?;
                        has_shortcode_variants = has_shortcode_variants || more;
                        return Ok(repl);
                    }
                    // This should never happen.
                    let n = ct
                        .content_placeholders
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .len();
                    panic!("unknown shortcode token {token:?} (number of tokens: {n})");
                };

                b = expand_shortcode_tokens(&b, &mut token_handler)?;
                if has_shortcode_variants {
                    p.incr_page_output_template_variation();
                }

                let mt = &p.meta.page_config.content_media_type;
                if sc.has_summary_divider() {
                    let summarized = extract_summary_from_html_with_divider(
                        mt,
                        &b,
                        INTERNAL_SUMMARY_DIVIDER_BASE.as_bytes(),
                    );
                    result.summary = Summary {
                        text: GoString::from(summarized.summary()),
                        type_: SUMMARY_TYPE_MANUAL.to_string(),
                        truncated: summarized.truncated(),
                    };
                    result.content_without_summary = summarized.content_without_summary();
                    result.content = summarized.content();
                } else {
                    result.content = b;
                }

                if !sc.has_summary_divider() && p.meta.page_config.summary.is_empty() {
                    let num_words = h.sites[p.site_idx].conf.root.summary_length;
                    let is_cjk_language = p.meta.page_config.is_cjk_language;
                    let summary =
                        extract_summary_from_html(mt, &result.content, num_words, is_cjk_language);
                    result.summary = Summary {
                        text: GoString::from(summary.summary()),
                        type_: SUMMARY_TYPE_AUTO.to_string(),
                        truncated: summary.truncated(),
                    };
                    result.content_without_summary = summary.content_without_summary();
                }
                Ok(result)
            })();

            let mut rs = rs.map_err(|err| cp.po_page_wrap_error(err))?;

            if rs.summary.is_zero() {
                let b = match cp.po.content_renderer() {
                    Some(cr) => cr.parse_and_render_content(
                        ctx,
                        p.meta.page_config.summary.as_bytes(),
                        false,
                    )?,
                    None => Default::default(),
                };
                let html = h.sites[p.site_idx]
                    .deps
                    .content_spec()
                    .trim_short_html(&b.bytes, &p.meta.page_config.content.markup);
                rs.summary = Summary {
                    text: GoString::from(html),
                    type_: SUMMARY_TYPE_FRONT_MATTER.to_string(),
                    truncated: false,
                };
                rs.content_without_summary = rs.content.clone();
            }

            Ok(Arc::new(Ok(rs)))
        });

        match v {
            Ok(v) => match &*v {
                Ok(v) => Ok(v.clone()),
                Err(err) => Err(err.clone()),
            },
            Err(err) => Err(sc.p.wrap_error(err)),
        }
    }

    /// Go: `mustContentToC(ctx)`.
    // Go: hugolib/page__content.go:mustContentToC
    pub fn must_content_to_c(&self, ctx: &TplContext) -> ContentTableOfContents {
        match self.content_to_c(ctx) {
            Ok(ct) => ct,
            Err(err) => panic!("{}", err.message()),
        }
    }

    /// Go: `contentToC(ctx)` — shortcodes prepared, the content to render (markdown shortcodes
    /// rendered in place), and for markup converters that can parse separately (Goldmark) the
    /// parsed document and its table of contents.
    // Go: hugolib/page__content.go:contentToC
    pub fn content_to_c(&self, ctx: &TplContext) -> Result<ContentTableOfContents> {
        let h = self.h();
        let sc = self.scope_ctx(&h);
        let cp = &self.pco;
        let key = format!("{}/{}", sc.source_key(), self.key_scope(ctx));
        let pm = &h.sites[sc.p.site_idx].page_map;

        let v = pm.cache_content_toc.get_or_create(key, |_| {
            let source: &[u8] = match sc.cc {
                Some(c) => c.pi.content_source(),
                None => &[],
            };

            let mut ct = ContentTableOfContents::default();
            cp.init_render_hooks_internal();
            let po = &cp.po;
            let p = sc.p;
            ct.content_placeholders = match sc.cc {
                Some(c) => c.shortcode_state.prepare_shortcodes_for_page(po, false)?,
                None => Default::default(),
            };

            // Callback called from below (e.g. in .RenderShortcodes)
            let placeholders = ct.content_placeholders.clone();
            let cp1 = cp.clone();
            let ctx_callback: ContentCallback = Arc::new(
                move |cp2: &Arc<PageContentOutput>, ct2: &ContentTableOfContents| {
                    let h = cp1.h();
                    cp1.other_outputs
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(h.page(cp2.po.p).pid, cp2.clone());

                    // Merge content placeholders
                    let other: Vec<(String, Arc<dyn ShortcodeRenderer>)> = ct2
                        .content_placeholders
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    placeholders
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .extend(other);

                    // Transfer shortcode names so HasShortcode works for shortcodes from
                    // included pages: not ported (the handler is immutable after capture; see
                    // PORTING.md).
                    if h.page(cp2.po.p)
                        .page_output_template_variations_state
                        .load(std::sync::atomic::Ordering::SeqCst)
                        > 0
                    {
                        h.page(cp1.po.p).incr_page_output_template_variation();
                    }
                },
            );

            let ctx = &HostState {
                content_callback: Some(ctx_callback),
            }
            .set_on(ctx);

            let (content_to_render, has_variants) = match sc.cc {
                Some(c) => content_to_render(&c.pi, ctx, source, &ct.content_placeholders)?,
                None => (Vec::new(), false),
            };
            ct.content_to_render = content_to_render;

            if has_variants {
                p.incr_page_output_template_variation();
            }

            let is_html = p.meta.page_config.content_media_type.is_html();

            if !is_html {
                let toc_cfg = h.sites[p.site_idx]
                    .deps
                    .content_spec()
                    .converters
                    .get_markup_config()
                    .table_of_contents
                    .clone();
                let create_and_set_toc =
                    |ct: &mut ContentTableOfContents,
                     toc: Option<Arc<nh_markup::tableofcontents::Fragments>>| {
                        ct.table_of_contents_html = match &toc {
                            Some(t) => {
                                t.to_html(toc_cfg.start_level, toc_cfg.end_level, toc_cfg.ordered)
                            }
                            None => Vec::new(),
                        };
                        ct.table_of_contents = toc;
                    };

                // If the converter supports doing the parsing separately, we do that.
                let parse_result = match po.content_renderer() {
                    Some(cr) => cr.parse_content(ctx, &ct.content_to_render)?,
                    None => None,
                };
                if let Some(parse_result) = parse_result {
                    // This is Goldmark.
                    // Store away the parse result for later use.
                    create_and_set_toc(&mut ct, parse_result.table_of_contents.clone());

                    ct.ast_doc = Some(parse_result.doc);
                } else {
                    // This is Asciidoctor etc.
                    let r = match po.content_renderer() {
                        Some(cr) => {
                            cr.parse_and_render_content(ctx, &ct.content_to_render, true)?
                        }
                        None => Default::default(),
                    };

                    ct.content_to_render = r.bytes.clone();

                    if let Some(toc) = r.table_of_contents {
                        create_and_set_toc(&mut ct, Some(toc));
                    } else {
                        let (tmp_content, tmp_table_of_contents) =
                            nh_helpers::content::extract_toc(&ct.content_to_render);
                        ct.table_of_contents_html = tmp_table_of_contents.unwrap_or_default();
                        ct.table_of_contents = Some(nh_markup::tableofcontents::empty());
                        ct.content_to_render = tmp_content;
                    }
                }
            }

            Ok(Arc::new(Ok(ct)))
        })?;

        match &*v {
            Ok(v) => Ok(v.clone()),
            Err(err) => Err(err.clone()),
        }
    }

    /// Go: `contentPlain(ctx)`.
    // Go: hugolib/page__content.go:contentPlain
    pub fn content_plain(&self, ctx: &TplContext) -> Result<ContentPlainPlainWords> {
        let h = self.h();
        let sc = self.scope_ctx(&h);
        let key = format!("{}/{}", sc.source_key(), self.key_scope(ctx));
        let pm = &h.sites[sc.p.site_idx].page_map;
        let timeout = h.sites[sc.p.site_idx]
            .conf
            .c
            .as_ref()
            .map(|c| c.timeout)
            .unwrap_or_default();

        let v = pm
            .cache_content_plain
            .get_or_create_with_timeout(key, timeout, |_| {
                let mut result = ContentPlainPlainWords::default();

                let rendered = self.content_rendered(ctx)?;

                result.plain = nh_tpl::template::strip_html(&rendered.content);
                result.plain_words = go_unicode::strings::fields(&result.plain)
                    .into_iter()
                    .map(|w| w.to_vec())
                    .collect();

                let is_cjk_language = sc.p.meta.page_config.is_cjk_language;

                if is_cjk_language {
                    result.word_count = 0;
                    for word in &result.plain_words {
                        let rune_count = go_unicode::utf8::rune_count(word) as i64;
                        if word.len() as i64 == rune_count {
                            result.word_count += 1;
                        } else {
                            result.word_count += rune_count;
                        }
                    }
                } else {
                    result.word_count = nh_helpers::content::total_words(&result.plain);
                }

                // TODO(bep) is set in a test. Fix that.
                if result.fuzzy_word_count == 0 {
                    result.fuzzy_word_count = (result.word_count + 100) / 100 * 100;
                }

                if is_cjk_language {
                    result.reading_time = (result.word_count + 500) / 501;
                } else {
                    result.reading_time = (result.word_count + 212) / 213;
                }

                Ok(Arc::new(Ok(result)))
            });

        match v {
            Ok(v) => match &*v {
                Ok(v) => Ok(v.clone()),
                Err(err) => Err(err.clone()),
            },
            Err(err) => {
                if nh_common::herrors::is_timeout_error(&err) {
                    return Err(err.wrap(
                        "timed out rendering the page content. Extend the `timeout` limit in your Hugo config file",
                    ));
                }
                Err(err)
            }
        }
    }

    /// Go: `prepareContext(ctx)` — the markup scope is recursive: an outer non-zero scope is
    /// kept, else this scope's is set.
    // Go: hugolib/page__content.go:prepareContext
    pub fn prepare_context(&self, ctx: &TplContext) -> TplContext {
        // A regular page's shortcode etc. may be rendered by e.g. the home page,
        // so we need to track any changes to this content's page (no identity tracking).

        // The markup scope is recursive, so if already set to a non zero value, preserve that value.
        let s = &ctx.markup_scope;
        if !s.is_empty() || *s == self.scope {
            return ctx.clone();
        }
        ctx.with_markup_scope(&self.scope)
    }

    /// Go: `Content(ctx)`.
    // Go: hugolib/page__content.go:Content
    pub fn content(&self, ctx: HostCtx<'_>) -> Result<Value> {
        let ctx = self.prepare_context(&tpl_ctx(ctx));
        let cr = self.content_rendered(&ctx)?;
        Ok(Value::html(cr.content))
    }

    /// Go: `ContentWithoutSummary(ctx)`.
    // Go: hugolib/page__content.go:ContentWithoutSummary
    pub fn content_without_summary(&self, ctx: HostCtx<'_>) -> Result<Value> {
        let ctx = self.prepare_context(&tpl_ctx(ctx));
        let cr = self.content_rendered(&ctx)?;
        Ok(Value::html(cr.content_without_summary))
    }

    /// Go: `Summary(ctx)`.
    // Go: hugolib/page__content.go:Summary
    pub fn summary(&self, ctx: HostCtx<'_>) -> Result<Summary> {
        let ctx = self.prepare_context(&tpl_ctx(ctx));
        let rendered = self.content_rendered(&ctx)?;
        Ok(rendered.summary)
    }

    /// Go: `RenderString(ctx, args...)`.
    // Go: hugolib/page__content.go:RenderString
    pub fn render_string(&self, ctx: HostCtx<'_>, args: &[Value]) -> Result<Value> {
        let ctx = &self.prepare_context(&tpl_ctx(ctx));

        if args.is_empty() || args.len() > 2 {
            return Err(Error::new("want 1 or 2 arguments"));
        }

        let pco = &self.pco;
        let h = self.h();
        let p = h.page(pco.po.p);

        // Go `defaultRenderStringOpts`.
        let mut display = "inline".to_string();
        let mut opts_markup = String::new();
        let mut sidx = 1;

        if args.len() == 1 {
            sidx = 0;
        } else {
            let Value::Map(m) = &args[0] else {
                return Err(Error::new("first argument must be a map"));
            };
            // mapstructure.WeakDecode into renderStringOpts{Display, Markup}: case-insensitive
            // field names, weakly typed strings.
            for (k, v) in m.entries.iter() {
                let k = go_unicode::strings::to_lower_str(
                    std::str::from_utf8(k.as_bytes()).unwrap_or_default(),
                )
                .into_owned();
                let sv = || -> Result<String> {
                    let s = nh_common::cast::caste::to_string_e(v).map_err(|err| {
                        Error::new(format!("failed to decode options: {}", err.message()))
                    })?;
                    Ok(String::from_utf8_lossy(s.as_bytes()).into_owned())
                };
                match k.as_str() {
                    "display" => display = sv()?,
                    "markup" => opts_markup = sv()?,
                    _ => {}
                }
            }
            if !opts_markup.is_empty() {
                opts_markup = nh_markup::markup::resolve_markup(&opts_markup);
            }
        }

        let content_to_renderv = &args[sidx];

        if let Value::Object(o) = content_to_renderv
            && o.type_name() == "hstring.HTML"
        {
            // This content is already rendered, this is potentially
            // a infinite recursion.
            return Err(Error::new(
                "text is already rendered, repeating it may cause infinite recursion",
            ));
        }

        let content_to_render = nh_common::cast::caste::to_string_e(content_to_renderv)?;
        let content_to_render: &[u8] = content_to_render.as_bytes();

        pco.init_render_hooks_internal();

        let mut conv = p.get_content_converter(&h, &pco.po);

        if !opts_markup.is_empty() && opts_markup != p.meta.page_config.content_media_type.sub_type
        {
            conv = match crate::page__per_output::new_content_converter(&h, p, &opts_markup) {
                Ok(c) => c,
                Err((_, err)) => return Err(p.wrap_error(err)),
            };
        }

        let mut rendered: Vec<u8>;

        if nh_parser::pageparser::pageparser::has_shortcode(content_to_render) {
            // String contains a shortcode.
            let items = nh_parser::pageparser::pageparser::parse_bytes(
                content_to_render,
                ParseConfig {
                    no_front_matter: true,
                    no_summary_divider: true,
                },
            )?;
            let mut parse_info = ContentParseInfo {
                pid: p.pid,
                source_key: String::new(),
                open_source: None,
                source: Arc::new(content_to_render.to_vec()),
                front_matter: None,
                has_summary_divider: false,
                pos_main_content: 0,
                has_non_markdown_shortcode: false,
                items_step1: items,
                items_step2: Vec::new(),
            };

            let site = &h.sites[p.site_idx];
            let mut s = crate::shortcode_parse::ShortcodeHandler::new(
                &p.path_or_title(),
                site.deps.exec_helper().sec().enable_inline_shortcodes,
            );
            crate::page__content_parse::map_items_after_front_matter(
                &mut parse_info,
                &mut s,
                site.deps.get_template_store(),
            )?;

            let placeholders = s.prepare_shortcodes_for_page(&pco.po, true)?;

            let (content_to_render, has_variants) =
                content_to_render_fn(&parse_info, ctx, content_to_render, &placeholders)?;
            if has_variants {
                p.incr_page_output_template_variation();
            }
            let b = pco
                .render_content_with_converter(ctx, &*conv, &content_to_render, false)
                .map_err(|err| p.wrap_error(err))?;
            rendered = b.bytes;

            if parse_info.has_non_markdown_shortcode {
                let mut has_shortcode_variants = false;
                let toc_placeholder = toc_shortcode_placeholder();

                let mut token_handler = |token: &str| -> Result<Vec<u8>> {
                    if token == toc_placeholder {
                        let toc = self.content_to_c(ctx)?;
                        // The Page's TableOfContents was accessed in a shortcode.
                        return Ok(toc.table_of_contents_html);
                    }
                    let renderer = placeholders
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(token)
                        .cloned();
                    if let Some(renderer) = renderer {
                        let (repl, more) = renderer.render_shortcode(ctx)?;
                        has_shortcode_variants = has_shortcode_variants || more;
                        return Ok(repl);
                    }
                    // This should not happen.
                    Err(Error::new(format!("unknown shortcode token {token:?}")))
                };

                rendered = expand_shortcode_tokens(&rendered, &mut token_handler)?;
                if has_shortcode_variants {
                    p.incr_page_output_template_variation();
                }
            }

            // We need a consolidated view in $page.HasShortcode (transferNames: not ported,
            // see PORTING.md).
        } else {
            let c = pco
                .render_content_with_converter(ctx, &*conv, content_to_render, false)
                .map_err(|err| p.wrap_error(err))?;

            rendered = c.bytes;
        }

        if display == "inline" {
            let mut markup = p.meta.page_config.content.markup.clone();
            let cs = h.sites[p.site_idx].deps.content_spec();
            if !opts_markup.is_empty() {
                markup = cs.resolve_markup(&opts_markup);
            }
            rendered = cs.trim_short_html(&rendered, &markup);
        }

        Ok(Value::html(rendered))
    }

    /// Go: `RenderShortcodes(ctx)`.
    // Go: hugolib/page__content.go:RenderShortcodes
    pub fn render_shortcodes(&self, ctx: HostCtx<'_>) -> Result<Value> {
        let ctx = &self.prepare_context(&tpl_ctx(ctx));

        let pco = &self.pco;
        let h = self.h();
        let sc = self.scope_ctx(&h);
        let p = sc.p;

        let source: &[u8] = match sc.cc {
            Some(c) => c.pi.content_source(),
            None => &[],
        };
        let ct = self.content_to_c(ctx)?;

        let mut has_variants = false;
        let cb = HostState::of(ctx).content_callback;
        let insert_placeholders = cb.is_some();
        let mut cc: Vec<u8> = Vec::with_capacity(source.len() + source.len() / 10);
        if let Some(c) = sc.cc {
            for it in &c.pi.items_step2 {
                match it {
                    ContentItem::Source { low, high } => cc.extend_from_slice(&source[*low..*high]),
                    ContentItem::Replacement(_) => {
                        // Ignore.
                    }
                    ContentItem::Shortcode(v) => {
                        if !insert_placeholders || !v.insert_placeholder() {
                            // Insert the rendered shortcode.
                            let rendered_shortcode = ct
                                .content_placeholders
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .get(&v.placeholder)
                                .cloned();
                            let Some(rendered_shortcode) = rendered_shortcode else {
                                // This should never happen.
                                panic!("rendered shortcode {:?} not found", v.placeholder);
                            };

                            let (b, more) = rendered_shortcode
                                .render_shortcode(ctx)
                                .map_err(|err| err.wrap("failed to render shortcode"))?;
                            has_variants = has_variants || more;
                            cc.extend_from_slice(&b);
                        } else {
                            // Insert the placeholder so we can insert the content after
                            // markdown processing.
                            cc.extend_from_slice(v.placeholder.as_bytes());
                        }
                    }
                }
            }
        }

        if has_variants {
            p.incr_page_output_template_variation();
        }

        if let Some(cb) = cb {
            cb(pco, &ct);
        }

        if ctx.is_in_goldmark {
            // This content will be parsed and rendered by Goldmark.
            // Wrap it in a special Hugo markup to assign the correct Page from
            // the stack.
            return Ok(Value::html(nh_markup::goldmark::hugocontext::wrap(
                &cc, p.pid,
            )));
        }

        Ok(Value::html(cc))
    }

    /// Go: `Plain(ctx)`.
    // Go: hugolib/page__content.go:Plain
    pub fn plain(&self, ctx: HostCtx<'_>) -> GoString {
        let ctx = self.prepare_context(&tpl_ctx(ctx));
        GoString::from(self.must_content_plain(&ctx).plain)
    }

    /// Go: `PlainWords(ctx)`.
    // Go: hugolib/page__content.go:PlainWords
    pub fn plain_words(&self, ctx: HostCtx<'_>) -> Vec<GoString> {
        let ctx = self.prepare_context(&tpl_ctx(ctx));
        self.must_content_plain(&ctx)
            .plain_words
            .into_iter()
            .map(GoString::from)
            .collect()
    }

    /// `PlainWords` as a `[]string` template value.
    pub fn plain_words_value(&self, ctx: HostCtx<'_>) -> Value {
        Value::list(
            SliceType::String,
            self.plain_words(ctx)
                .into_iter()
                .map(Value::string)
                .collect(),
        )
    }

    /// Go: `WordCount(ctx)`.
    // Go: hugolib/page__content.go:WordCount
    pub fn word_count(&self, ctx: HostCtx<'_>) -> i64 {
        let ctx = self.prepare_context(&tpl_ctx(ctx));
        self.must_content_plain(&ctx).word_count
    }

    /// Go: `FuzzyWordCount(ctx)`.
    // Go: hugolib/page__content.go:FuzzyWordCount
    pub fn fuzzy_word_count(&self, ctx: HostCtx<'_>) -> i64 {
        let ctx = self.prepare_context(&tpl_ctx(ctx));
        self.must_content_plain(&ctx).fuzzy_word_count
    }

    /// Go: `ReadingTime(ctx)`.
    // Go: hugolib/page__content.go:ReadingTime
    pub fn reading_time(&self, ctx: HostCtx<'_>) -> i64 {
        let ctx = self.prepare_context(&tpl_ctx(ctx));
        self.must_content_plain(&ctx).reading_time
    }

    /// Go: `Len(ctx)` — the length in bytes of the rendered content.
    // Go: hugolib/page__content.go:Len
    pub fn len(&self, ctx: HostCtx<'_>) -> i64 {
        let ctx = self.prepare_context(&tpl_ctx(ctx));
        self.must_content_rendered(&ctx).content.len() as i64
    }

    /// Go: `Fragments(ctx)` (`None` = nil).
    // Go: hugolib/page__content.go:Fragments
    pub fn fragments(
        &self,
        ctx: HostCtx<'_>,
    ) -> Option<Arc<nh_markup::tableofcontents::Fragments>> {
        let ctx = self.prepare_context(&tpl_ctx(ctx));
        self.must_content_to_c(&ctx).table_of_contents
    }

    /// Go: `fragmentsHTML(ctx)` — the table of contents (`template.HTML`).
    // Go: hugolib/page__content.go:fragmentsHTML
    pub fn fragments_html(&self, ctx: HostCtx<'_>) -> Value {
        let ctx = self.prepare_context(&tpl_ctx(ctx));
        Value::html(self.must_content_to_c(&ctx).table_of_contents_html)
    }

    /// Go: `mustContentPlain(ctx)` — an error fails the build (`pco.fail`), zero result.
    // Go: hugolib/page__content.go:mustContentPlain
    fn must_content_plain(&self, ctx: &TplContext) -> ContentPlainPlainWords {
        match self.content_plain(ctx) {
            Ok(r) => r,
            Err(err) => {
                self.pco.fail(err);
                ContentPlainPlainWords::default()
            }
        }
    }

    /// Go: `mustContentRendered(ctx)`.
    // Go: hugolib/page__content.go:mustContentRendered
    fn must_content_rendered(&self, ctx: &TplContext) -> ContentSummary {
        match self.content_rendered(ctx) {
            Ok(r) => r,
            Err(err) => {
                self.pco.fail(err);
                ContentSummary::default()
            }
        }
    }
}

/// `content_to_render` under another name (inside `render_string`, where a local shadows it).
fn content_to_render_fn(
    pi: &ContentParseInfo,
    ctx: &TplContext,
    source: &[u8],
    rendered_shortcodes: &ShortcodeRenderers,
) -> Result<(Vec<u8>, bool)> {
    content_to_render(pi, ctx, source, rendered_shortcodes)
}

impl PageContentOutput {
    /// `p.wrapError(err)` of this output's page.
    pub(crate) fn po_page_wrap_error(&self, err: Error) -> Error {
        let h = self.h();
        h.page(self.po.p).wrap_error(err)
    }
}

/// `template.HTML` of bytes (Go `helpers.BytesToHTML`).
pub fn html_value(b: Vec<u8>) -> Value {
    Value::Safe(SafeKind::Html, GoString::from(b))
}

/// Go: `(c *cachedContent) version(cp)` — `StaleVersion() + cp.contentRenderedVersion`
/// (always the render version: no stale tracking in a one-shot build).
// Go: hugolib/page__content.go:version
pub fn version(cp: &PageContentOutput) -> u32 {
    cp.content_rendered_version
        .load(std::sync::atomic::Ordering::SeqCst)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__content.go (1191 lines; 23/39 funcs executed) — parse half in page__content_parse.rs (T20)
//   types: contentTableOfContents, contentSummary, contentPlainPlainWords, contextKey, cachedContentScope
// OK L171-181: (c *cachedContent) getOrCreateScope(scope string, pco *pageContentOutput) *cachedContentScope
// OK L227-264: (pi *contentParseInfo) contentToRender(ctx context.Context, source []byte, renderedShortcodes map[string]shortcodeRenderer) ([]byte, bool, error)
// OK L522-524: (c *cachedContentScope) keyScope(ctx context.Context) string
// OK L526-660: (c *cachedContentScope) contentRendered(ctx context.Context) (contentSummary, error)
// OK L662-668: (c *cachedContentScope) mustContentToC(ctx context.Context) contentTableOfContents
// OK L678-791: (c *cachedContentScope) contentToC(ctx context.Context) (contentTableOfContents, error)
// OK L793-796: (c *cachedContent) version(cp *pageContentOutput) uint32
// OK L798-858: (c *cachedContentScope) contentPlain(ctx context.Context) (contentPlainPlainWords, error)
// OK L866-876: (c *cachedContentScope) prepareContext(ctx context.Context) context.Context
// OK L878-880: (c *cachedContentScope) Render(ctx context.Context) (page.Content, error)
// OK L882-889: (c *cachedContentScope) Content(ctx context.Context) (template.HTML, error)
// OK L891-898: (c *cachedContentScope) ContentWithoutSummary(ctx context.Context) (template.HTML, error)
// OK L900-904: (c *cachedContentScope) Summary(ctx context.Context) (page.Summary, error)
// OK L906-1059: (c *cachedContentScope) RenderString(ctx context.Context, args ...any) (template.HTML, error)
// OK L1061-1131: (c *cachedContentScope) RenderShortcodes(ctx context.Context) (template.HTML, error)
// OK L1133-1136: (c *cachedContentScope) Plain(ctx context.Context) string
// OK L1138-1141: (c *cachedContentScope) PlainWords(ctx context.Context) []string
// OK L1143-1146: (c *cachedContentScope) WordCount(ctx context.Context) int
// OK L1148-1151: (c *cachedContentScope) FuzzyWordCount(ctx context.Context) int
// OK L1153-1156: (c *cachedContentScope) ReadingTime(ctx context.Context) int
// OK L1158-1161: (c *cachedContentScope) Len(ctx context.Context) int
// OK L1163-1170: (c *cachedContentScope) Fragments(ctx context.Context) *tableofcontents.Fragments
// OK L1172-1175: (c *cachedContentScope) fragmentsHTML(ctx context.Context) template.HTML
// OK L1177-1183: (c *cachedContentScope) mustContentPlain(ctx context.Context) contentPlainPlainWords
// OK L1185-1191: (c *cachedContentScope) mustContentRendered(ctx context.Context) contentSummary
// ---------------------------------------------------------------------------
