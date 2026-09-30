//! Shortcode execution: a page body → [`ExpandedSource`] (REWRITE_PLAN.md §3.2).
//!
//! - `{{% %}}` output is spliced into the Markdown; `{{< >}}` output becomes a page-local
//!   placeholder token swapped in after Markdown.
//! - Nested calls are executed first and their output is part of the parent's `inner`. The
//!   inner of the outermost `{{% %}}` is raw; a nested `{{% %}}` has its inner rendered as
//!   Markdown (a single line loses its `<p>`).
//! - `ordinal` counts per nesting level; `parent` is the enclosing call.
//! - A call without inner content whose tag is indented gets the indentation on every further
//!   line of its output.
//! - Includes (`render_shortcodes` in the content phase) come back as inclusion tokens: in
//!   `{{% %}}` output they are replaced by the included source with its placeholders renumbered
//!   into this page's table and a context span for the included page.

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use neohugo_base::PageId;
use neohugo_config::global::InlineShortcodes;
use neohugo_layouts::{ShortcodeMiss, ShortcodeQuery, TemplateName};
use neohugo_markup::SourceContexts;
use neohugo_pageparser::{
    Body, Closing, Delim, InnerUse, Scalar, Segment, ShortcodeArgs, ShortcodeCall,
};
use neohugo_site::Page;
use neohugo_view::views::ShortcodeView;
use neohugo_view::{ContentError, ExpandedSource, RenderScope, RenderStringOptions, SCOPE_KEY};

use crate::session::Session;
use crate::summary::DIVIDER_SOURCE;
use crate::tokens::{self, INCLUSION};

/// The value of a shortcode argument.
fn scalar(s: &Scalar) -> tera::Value {
    match s {
        Scalar::String(v) => tera::Value::from(v.as_str()),
        Scalar::Int(i) => tera::Value::from(*i),
        Scalar::Float(f) => tera::Value::from(*f),
        Scalar::Bool(b) => tera::Value::from(*b),
    }
}

/// `inner` with the call's indentation removed from every line that starts with it (Hugo
/// `.InnerDeindent`).
fn deindent(inner: &str, indentation: &str) -> String {
    if indentation.is_empty() {
        return inner.to_owned();
    }
    inner
        .split_inclusive('\n')
        .map(|line| line.strip_prefix(indentation).unwrap_or(line))
        .collect()
}

/// `out` with `indentation` added before every line after the first.
fn indent(out: &str, indentation: &str) -> String {
    let mut s = String::with_capacity(out.len());
    for (i, line) in out.split_inclusive('\n').enumerate() {
        if i > 0 {
            s.push_str(indentation);
        }
        s.push_str(line);
    }
    s
}

/// Hugo's cleanup of a one-line inner rendered as Markdown: `<p>x</p>\n` → `x`.
fn one_line(html: String) -> String {
    match html
        .strip_prefix("<p>")
        .and_then(|s| s.strip_suffix("</p>\n"))
    {
        Some(inner) if !inner.contains('\n') => inner.to_owned(),
        _ => html,
    }
}

/// One page's shortcode expansion.
pub(crate) struct Expander<'s> {
    pub session: &'s Session,
    pub page: &'s Page,
    /// The content file (the whole text, for positions).
    pub file: &'s Arc<Path>,
    pub text: &'s str,
    pub body_offset: usize,
    /// The shortcode lookup path (the page's key with its type as first segment).
    pub path: &'s neohugo_base::paths::ContentKey,
    pub format: neohugo_base::FormatId,
    /// The scope of the expansion's computation.
    pub scope: &'s RenderScope,
    out: String,
    placeholders: Vec<Arc<str>>,
    contexts: Vec<(Range<usize>, PageId)>,
    /// Inline shortcode templates defined so far (`name.inline`).
    inline: BTreeMap<String, String>,
}

impl<'s> Expander<'s> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        session: &'s Session,
        page: &'s Page,
        file: &'s Arc<Path>,
        text: &'s str,
        body_offset: usize,
        path: &'s neohugo_base::paths::ContentKey,
        format: neohugo_base::FormatId,
        scope: &'s RenderScope,
    ) -> Self {
        Self {
            session,
            page,
            file,
            text,
            body_offset,
            path,
            format,
            scope,
            out: String::new(),
            placeholders: Vec::new(),
            contexts: Vec::new(),
            inline: BTreeMap::new(),
        }
    }

    fn position(&self, span: &Range<usize>) -> String {
        let (line, col) = neohugo_pageparser::line_col(self.text, self.body_offset + span.start);
        format!("{}:{line}:{col}", self.file.display())
    }

    fn query(&self, name: &'s str, markdown: bool) -> ShortcodeQuery<'s> {
        ShortcodeQuery {
            name,
            path: self.path,
            kind: Some(self.page.kind),
            lang: Some(self.page.lang),
            format: self.format,
            markdown,
        }
    }

    /// Whether shortcode `name` uses its inner content.
    pub(crate) fn inner_use(&self, name: &str) -> InnerUse {
        let t = self.session.templates();
        let q = |markdown| ShortcodeQuery {
            name,
            path: self.path,
            kind: Some(self.page.kind),
            lang: Some(self.page.lang),
            format: self.format,
            markdown,
        };
        match t
            .shortcode_query(&q(false))
            .or_else(|_| t.shortcode_query(&q(true)))
        {
            Ok(tpl) => {
                if t.uses_variable(&tpl, "inner") || t.uses_variable(&tpl, "inner_deindent") {
                    InnerUse::Required
                } else {
                    InnerUse::Unused
                }
            }
            Err(ShortcodeMiss::NotFound(_)) => InnerUse::UnknownShortcode,
            Err(ShortcodeMiss::Incompatible { .. }) => InnerUse::Unused,
        }
    }

    /// Expands `body` (the page's body, parsed).
    pub(crate) fn run(mut self, body: &Body<'_>) -> Result<ExpandedSource, ContentError> {
        for (i, seg) in body.segments.iter().enumerate() {
            if body.summary_divider == Some(i) {
                self.out.push_str(DIVIDER_SOURCE);
            }
            match seg {
                Segment::Text(t) => self.out.push_str(t),
                Segment::Shortcode(call) => {
                    let output = self.call(call, None)?;
                    match call.delim {
                        Delim::Markdown => self.splice(&output),
                        Delim::Html => {
                            let text = self.session.inclusions().resolve_all(&output);
                            self.out
                                .push_str(&tokens::placeholder(self.placeholders.len()));
                            self.placeholders.push(Arc::from(text));
                        }
                    }
                }
            }
        }
        if body.summary_divider == Some(body.segments.len()) {
            self.out.push_str(DIVIDER_SOURCE);
        }
        Ok(ExpandedSource {
            markdown: self.out,
            placeholders: self.placeholders,
            contexts: SourceContexts(self.contexts),
        })
    }

    /// Appends `{{% %}}` output, replacing inclusion tokens by the included sources.
    fn splice(&mut self, output: &str) {
        let found = tokens::find(output, INCLUSION);
        let mut last = 0;
        for (r, n) in found {
            let Some((q, src)) = self.session.inclusions().take(n) else {
                continue;
            };
            self.out.push_str(&output[last..r.start]);
            let start = self.out.len();
            let offset = self.placeholders.len();
            self.out.push_str(&tokens::renumber(&src.markdown, offset));
            self.placeholders.extend(src.placeholders.iter().cloned());
            for (span, p) in &src.contexts.0 {
                self.contexts
                    .push((span.start + start..span.end + start, *p));
            }
            self.contexts.push((start..self.out.len(), q));
            last = r.end;
        }
        self.out.push_str(&output[last..]);
    }

    /// The output of `call` (nested calls executed into its inner content).
    fn call(
        &mut self,
        call: &ShortcodeCall<'_>,
        parent: Option<&ShortcodeView>,
    ) -> Result<String, ContentError> {
        let position = self.position(&call.span);
        let (args, params, named) = match &call.args {
            ShortcodeArgs::None => (
                Vec::new(),
                tera::Value::from(Vec::<tera::Value>::new()),
                false,
            ),
            ShortcodeArgs::Positional(list) => {
                let args: Vec<tera::Value> = list.iter().map(scalar).collect();
                (args.clone(), tera::Value::from(args), false)
            }
            ShortcodeArgs::Named(pairs) => {
                let mut m = tera::value::Map::new();
                for (k, v) in pairs {
                    m.insert(tera::value::Key::String(k.as_str().into()), scalar(v));
                }
                (Vec::new(), tera::Value::from(m), true)
            }
        };
        let view = ShortcodeView {
            name: call.name.to_owned(),
            args,
            params,
            is_named_params: named,
            ordinal: u32::try_from(call.ordinal).unwrap_or(u32::MAX),
            parent: parent.map(|p| Box::new(p.clone())),
            // Go's `.Position` prints as `"file:line:col"` (common/text.Position).
            position: format!("\"{position}\""),
        };
        let mut inner = String::new();
        let mut has_inner = false;
        if let Closing::Closed { inner: segs, .. } = &call.closing {
            has_inner = !segs.is_empty();
            if !call.inline {
                for seg in segs {
                    match seg {
                        Segment::Text(t) => inner.push_str(t),
                        Segment::Shortcode(nested) => {
                            let out = self.call(nested, Some(&view))?;
                            inner.push_str(&self.session.inclusions().resolve_all(&out));
                        }
                    }
                }
                if call.delim == Delim::Markdown && parent.is_some() {
                    let raw = !inner.contains('\n');
                    let html = self
                        .session
                        .markdown_in_scope(
                            &inner,
                            RenderStringOptions {
                                display_block: true,
                            },
                            self.scope,
                        )
                        .map_err(|e| wrap(&position, &e))?;
                    inner = if raw { one_line(html) } else { html };
                }
            }
        }
        let deindented = deindent(&inner, call.indentation);
        let ctx = self.context(&view, &inner, &deindented);
        let out = if call.inline {
            self.inline(call, &ctx, &position)?
        } else {
            let tpl = self
                .session
                .templates()
                .shortcode_query(&self.query(call.name, call.delim == Delim::Markdown))
                .map_err(|e| ContentError::Render(format!("{position}: {e}")))?;
            self.render(&tpl, &ctx, &position)?
        };
        Ok(if !has_inner && !call.indentation.is_empty() {
            indent(&out, call.indentation)
        } else {
            out
        })
    }

    /// An inline shortcode: its inner content is its template (Tera syntax), defined by its
    /// first closed call and reused by self-closed ones.
    fn inline(
        &mut self,
        call: &ShortcodeCall<'_>,
        ctx: &tera::Context,
        position: &str,
    ) -> Result<String, ContentError> {
        if self.session.model().config.security.inline_shortcodes == InlineShortcodes::Disabled {
            return Ok(String::new());
        }
        let body = match &call.closing {
            Closing::Closed { span, .. } => {
                let body = self.text[self.body_offset + span.start..self.body_offset + span.end]
                    .to_owned();
                self.inline.insert(call.name.to_owned(), body.clone());
                body
            }
            _ => self.inline.get(call.name).cloned().ok_or_else(|| {
                ContentError::Render(format!(
                    "{position}: inline shortcode {:?} used before it is defined",
                    call.name
                ))
            })?,
        };
        self.session
            .templates()
            .tera()
            .render_str(&body, ctx, true)
            .map_err(|e| ContentError::Render(format!("{position}: {}", error_chain(&e))))
    }

    fn context(&self, view: &ShortcodeView, inner: &str, deindented: &str) -> tera::Context {
        let s = self.session;
        let generation = s.views().generation(self.scope.phase, self.scope.variant);
        let mut ctx = tera::Context::new();
        ctx.insert_value("page", generation.full(self.page.id));
        ctx.insert_value("site", generation.sites[self.page.lang].clone());
        ctx.insert_value("hugo", s.hugo().clone());
        ctx.insert("lang", &s.model().config.sites[self.page.lang].language.key);
        ctx.insert_value("shortcode", tera::Value::from_serializable(view));
        ctx.insert_value("inner", tera::Value::safe_string(inner));
        ctx.insert_value("inner_deindent", tera::Value::safe_string(deindented));
        ctx.insert_value(SCOPE_KEY, self.scope.child().to_value());
        ctx
    }

    fn render(
        &self,
        tpl: &TemplateName,
        ctx: &tera::Context,
        position: &str,
    ) -> Result<String, ContentError> {
        if self.scope.child().too_deep() {
            return Err(ContentError::TooDeep {
                limit: neohugo_view::MAX_DEPTH,
            });
        }
        self.session
            .templates()
            .tera()
            .render(tpl.as_str(), ctx)
            .map_err(|e| {
                ContentError::Render(format!(
                    "{position}: shortcode template {tpl}: {}",
                    error_chain(&e)
                ))
            })
    }
}

/// A content error at a shortcode position.
fn wrap(position: &str, e: &ContentError) -> ContentError {
    match e {
        ContentError::Cycle(m) => ContentError::Cycle(format!("{position}: {m}")),
        _ => ContentError::Render(format!("{position}: {e}")),
    }
}

/// A Tera error with its causes.
pub(crate) fn error_chain(e: &tera::Error) -> String {
    let mut s = e.to_string();
    let mut src = std::error::Error::source(e);
    while let Some(c) = src {
        let t = c.to_string();
        if !s.contains(&t) {
            s.push_str(": ");
            s.push_str(&t);
        }
        src = c.source();
    }
    s
}
