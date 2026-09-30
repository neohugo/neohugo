//! The content phase (C1) of the skeleton: Markdown through `neohugo-markup` without hooks,
//! HTML passed through, then the summary, plain text and counts.
//!
//! **Skeleton (T38).** No shortcodes, no render hooks, no memo cells, one hook variant
//! (`Html`); T34 replaces this module with the real content engine.

use std::sync::Arc;

use neohugo_base::PageId;
use neohugo_config::site::{EmojiPolicy, SiteConfig};
use neohugo_markup::{ExpandedMarkdown, MarkdownOptions, NoHooks, SourceContexts, text};
use neohugo_page::Markup;
use neohugo_view::RenderedContent;

use crate::RenderError;

/// Hugo's summary divider.
const SUMMARY_DIVIDER: &str = "<!--more-->";

/// The Markdown options of a language.
pub(crate) fn markdown_options(site: &SiteConfig) -> MarkdownOptions {
    MarkdownOptions::from_config(&site.markup, site.emoji == EmojiPolicy::Enabled)
}

/// Renders Markdown `md` as page `page`'s (no hooks).
pub(crate) fn markdown(
    md: &str,
    page: PageId,
    file: &Arc<std::path::Path>,
    o: &MarkdownOptions,
) -> Result<neohugo_markup::RenderedMarkdown, RenderError> {
    let contexts = SourceContexts::default();
    neohugo_markup::render(
        &ExpandedMarkdown {
            text: md,
            page,
            contexts: &contexts,
            file,
        },
        o,
        &NoHooks,
        None,
    )
    .map_err(|e| RenderError::Markup {
        file: file.to_path_buf(),
        message: e.to_string(),
    })
}

/// A page's content file, as the content phase reads it.
pub(crate) struct PageSource<'a> {
    /// The text after the front matter.
    pub body: &'a str,
    pub file: Arc<std::path::Path>,
    pub markup: Markup,
    /// Front matter `summary`.
    pub summary: Option<&'a str>,
}

/// The content of page `page` from `src`.
pub(crate) fn render_page(
    page: PageId,
    src: &PageSource<'_>,
    site: &SiteConfig,
) -> Result<RenderedContent, RenderError> {
    let o = markdown_options(site);
    let (html, toc, fragments) = match src.markup {
        Markup::Markdown => {
            let r = markdown(src.body, page, &src.file, &o)?;
            let toc = r.toc.to_html(&o.toc);
            (r.html, toc, r.fragments)
        }
        Markup::Html => (src.body.to_owned(), String::new(), Default::default()),
    };
    let (content, summary, truncated) = match text::split_at_marker(&html, SUMMARY_DIVIDER) {
        Some((summary, rest)) => {
            let truncated = !rest.trim().is_empty();
            let content = format!("{summary}\n{}", rest.trim_end());
            (content, summary, truncated)
        }
        None => match src.summary {
            Some(s) => {
                let r = markdown(s, page, &src.file, &o)?;
                let truncated = true;
                (html, r.html, truncated)
            }
            None => {
                let s = text::auto_summary(&html, site.summary_length, false);
                (html.clone(), s.html, s.truncated)
            }
        },
    };
    // Hugo trims the summary (the layouts decide the white space around it).
    let summary = summary.trim_end().to_owned();
    let plain = text::strip_html(&content);
    let word_count = text::word_count(&plain, false);
    Ok(RenderedContent {
        html: content,
        summary,
        truncated,
        plain,
        word_count,
        fuzzy_word_count: word_count.div_ceil(100) * 100,
        reading_time: word_count.div_ceil(213),
        table_of_contents: toc,
        fragments: Arc::new(fragments),
    })
}
