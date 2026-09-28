//! Port of `hugolib/shortcode.go`.
//!
//! Owner: Wave B task T22 (hugolib-content).

//! Go `hugolib/shortcode.go` (render half; extraction is `shortcode_parse.rs`, T20): rendering
//! with the shortcode template (data = `ShortcodeWithPage`, template execution through
//! `crate::template_exec::execute` with `ExecKind::Shortcode`), `prepareShortcodesForPage` and
//! `expandShortcodeTokens`. For `{{% %}}` shortcodes in markdown, `prepareShortcode` sets
//! `is_in_goldmark` on the context (shortcode.go:327-332) — the ONLY place it is set.
//!
//! The `ShortcodeWithPage` type (shortcode.go:50-188) is in `shortcode_page.rs`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use go_value::Value;
use nh_common::Result;
use nh_common::herrors::{Error, FilePos};
use nh_tpl::template::TplContext;
use nh_tplimpl::category::Category;
use nh_tplimpl::templatestore::{TemplInfo, TemplateQuery};

use crate::hugo_sites::HugoSites;
use crate::page__output::{PageOutput, upgrade_hs};
use crate::shortcode_page::{
    PrerenderedShortcode, ShortcodeRenderFunc, ShortcodeRenderer, ShortcodeWithPage, zero_shortcode,
};
use crate::template_exec::{ExecCall, ExecKind};

pub use crate::shortcode_parse::{
    SHORTCODE_PLACEHOLDER_PREFIX, Shortcode, ShortcodeHandler, ShortcodeInner,
    create_shortcode_placeholder,
};

/// The rendered (or delayed) shortcodes of a page by placeholder (Go
/// `map[string]shortcodeRenderer`). Shared and mutable like Go's map: `.RenderShortcodes` of an
/// included page merges its placeholders into the including page's map
/// (page__content.go:704-722).
pub type ShortcodeRenderers = Arc<Mutex<BTreeMap<String, Arc<dyn ShortcodeRenderer>>>>;

/// Go: `prepareShortcode(ctx, level, s, sc, parent, po, isRenderString)` — a renderer that
/// renders the shortcode when called (the caller may delay it).
// Go: hugolib/shortcode.go:prepareShortcode
pub fn prepare_shortcode(
    level: i64,
    sc: &Arc<Shortcode>,
    parent: Option<Arc<ShortcodeWithPage>>,
    po: &Arc<PageOutput>,
    is_render_string: bool,
) -> Result<Arc<dyn ShortcodeRenderer>> {
    let sc = sc.clone();
    let po = po.clone();

    // Allow the caller to delay the rendering of the shortcode if needed.
    let f = move |ctx: &TplContext| -> Result<(Vec<u8>, bool)> {
        let h = upgrade_hs(&po.hs);
        let p = h.page(po.p);
        let to_parse_err = |err: Error| -> Error {
            let source = p.content.as_ref().map(|c| c.must_source()).unwrap_or(&[]);
            p.parse_error(
                err.wrap(format!(
                    "failed to render shortcode {}",
                    go_strconv::quote(sc.name.as_bytes())
                )),
                source,
                sc.pos,
            )
        };

        let in_goldmark;
        let ctx = if p.meta.page_config.content_media_type.is_markdown() && sc.do_markup {
            // Signal downwards that the content rendered will be
            // parsed and rendered by Goldmark.
            in_goldmark = ctx.in_goldmark();
            &in_goldmark
        } else {
            ctx
        };
        let r = do_render_shortcode(ctx, level, &sc, parent.clone(), &po, is_render_string)
            .map_err(to_parse_err)?;
        let (b, has_variants) = r.render_shortcode(ctx).map_err(to_parse_err)?;
        Ok((b, has_variants))
    };

    Ok(Arc::new(ShortcodeRenderFunc(Arc::new(f))))
}

/// Go: `doRenderShortcode(ctx, level, s, sc, parent, po, isRenderString)`.
// Go: hugolib/shortcode.go:doRenderShortcode
pub fn do_render_shortcode(
    ctx: &TplContext,
    level: i64,
    sc: &Arc<Shortcode>,
    parent: Option<Arc<ShortcodeWithPage>>,
    po: &Arc<PageOutput>,
    is_render_string: bool,
) -> Result<PrerenderedShortcode> {
    let h = upgrade_hs(&po.hs);
    let p = h.page(po.p);
    let s = &h.sites[po.site_idx];
    let store = s.deps.get_template_store();

    // Tracks whether this shortcode or any of its children has template variations
    // in other languages or output formats. We are currently only interested in
    // the output formats.
    let mut has_variants = false;

    let tmpl: Arc<TemplInfo>;

    if sc.is_inline {
        if !s.deps.exec_helper().sec().enable_inline_shortcodes {
            return Ok(zero_shortcode());
        }
        let template_path = go_path::path::join(&["_inline_shortcode", &p.meta.path(), &sc.name]);
        if sc.is_closing {
            let templ_str = sc.inner_string();

            match store.text_parse_bytes(&template_path, &templ_str) {
                Ok(t) => tmpl = t,
                Err(err) => {
                    if is_render_string {
                        return Err(p.wrap_error(err));
                    }
                    let filename = p.meta.f.as_ref().map(|f| f.filename().to_string());
                    let fe = nh_common::herrors::new_file_error_from_name(
                        err,
                        filename.as_deref().unwrap_or(""),
                    );
                    let mut pos = fe.pos().cloned().unwrap_or_default();
                    pos.line += p.pos_offset(sc.pos).line_number;
                    return Err(p.wrap_error(fe.at(pos)));
                }
            }
        } else {
            // Re-use of shortcode defined earlier in the same page.
            match store.text_lookup(&template_path) {
                Some(t) => tmpl = t,
                None => {
                    return Err(Error::new(format!(
                        "no earlier definition of shortcode {} found",
                        go_strconv::quote(sc.name.as_bytes())
                    )));
                }
            }
        }
    } else {
        let of_count: Arc<Mutex<BTreeMap<String, i64>>> = Arc::new(Mutex::new(BTreeMap::new()));
        let of_count2 = of_count.clone();
        let include = move |m: &TemplInfo| -> bool {
            *of_count2
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry(m.d().output_format)
                .or_insert(0) += 1;
            true
        };
        let (base, mut layout_descriptor) = po.get_internal_template_base_path_and_descriptor(p);

        // With shortcodes/mymarkdown.md (only), this allows {{% mymarkdown %}} when rendering HTML,
        // but will not resolve any template when doing {{< mymarkdown >}}.
        layout_descriptor.always_allow_plain_text = sc.do_markup;
        let q = TemplateQuery {
            path: base,
            name: sc.name.clone(),
            category: Category::Shortcode,
            desc: layout_descriptor,
            consider: Some(Arc::new(include)),
        };
        let v = store.lookup_shortcode(&q);
        match v {
            Ok(Some(v)) => tmpl = v,
            Ok(None) => return Ok(zero_shortcode()),
            Err(err) => return Err(err),
        }
        has_variants = has_variants || of_count.lock().unwrap_or_else(|e| e.into_inner()).len() > 1;
    }

    let data = Arc::new(ShortcodeWithPage::new(&h, po.p, sc, parent));

    if !sc.inner.is_empty() {
        let mut inner: Vec<u8> = Vec::new();
        for inner_data in &sc.inner {
            match inner_data {
                ShortcodeInner::Text(t) => inner.extend_from_slice(t),
                ShortcodeInner::Shortcode(isc) => {
                    let s = prepare_shortcode(
                        level + 1,
                        isc,
                        Some(data.clone()),
                        po,
                        is_render_string,
                    )?;
                    let (ss, more) = s.render_shortcode_string(ctx)?;
                    has_variants = has_variants || more;
                    inner.extend_from_slice(&ss);
                }
            }
        }

        // Pre Hugo 0.55 this was the behavior even for the outer-most
        // shortcode.
        if sc.do_markup && (level > 0 || sc.config_version() == 1) {
            let b = match p.current_output().content_renderer() {
                Some(cr) => cr.parse_and_render_content(ctx, &inner, false)?,
                None => Default::default(),
            };

            let mut new_inner = b.bytes;

            // If the type is “” (unknown) or “markdown”, we assume the markdown
            // generation has been performed. Given the input: `a line`, markdown
            // specifies the HTML `<p>a line</p>\n`. When dealing with documents as a
            // whole, this is OK. When dealing with an `{{ .Inner }}` block in Hugo,
            // this is not so good. This code does two things:
            //
            // 1.  Check to see if inner has a newline in it. If so, the Inner data is
            //     unchanged.
            // 2   If inner does not have a newline, strip the wrapping <p> block and
            //     the newline.
            match p.meta.page_config.content.markup.as_str() {
                "" | "markdown" => {
                    if !inner.contains(&b'\n') {
                        new_inner = inner_cleanup(new_inner);
                    }
                }
                _ => {}
            }

            // TODO(bep) we may have plain text inner templates.
            let _ = data.inner.set(Value::html(new_inner));
        } else {
            let _ = data.inner.set(Value::html(inner));
        }
    }

    let result = render_shortcode_with_page(&h, po, ctx, &tmpl, &data);

    let mut result = match result {
        Ok(r) => r,
        Err(err) => {
            if sc.is_inline {
                let filename = p.meta.f.as_ref().map(|f| f.filename().to_string());
                let fe = nh_common::herrors::new_file_error_from_name(
                    err,
                    filename.as_deref().unwrap_or(""),
                );
                let mut pos = fe.pos().cloned().unwrap_or_default();
                pos.line += p.pos_offset(sc.pos).line_number;
                return Err(fe.at(pos));
            }
            return Err(err);
        }
    };

    if sc.inner.is_empty() && !sc.indentation.is_empty() {
        let mut b: Vec<u8> = Vec::new();
        let mut i = 0;
        crate::shortcode_page::visit_lines_after_bytes(&result, |line| {
            // The first line is correctly indented.
            if i > 0 {
                b.extend_from_slice(&sc.indentation);
            }
            i += 1;
            b.extend_from_slice(line);
        });

        result = b;
    }

    Ok(PrerenderedShortcode {
        s: result,
        has_variants,
    })
}

/// Go's `regexp.MustCompile(`\A<p>(.*)</p>\n\z`).ReplaceAll(b, "$1")`: the whole input is one
/// paragraph whose content has no newline (`.` matches any byte but `\n`, invalid UTF-8
/// included).
fn inner_cleanup(b: Vec<u8>) -> Vec<u8> {
    const PRE: &[u8] = b"<p>";
    const POST: &[u8] = b"</p>\n";
    if b.len() >= PRE.len() + POST.len() && b.starts_with(PRE) && b.ends_with(POST) {
        let mid = &b[PRE.len()..b.len() - POST.len()];
        if !mid.contains(&b'\n') {
            return mid.to_vec();
        }
    }
    b
}

impl ShortcodeHandler {
    /// Go: `(s *shortcodeHandler) prepareShortcodesForPage(ctx, po, isRenderString)`.
    // Go: hugolib/shortcode.go:prepareShortcodesForPage
    pub fn prepare_shortcodes_for_page(
        &self,
        po: &Arc<PageOutput>,
        is_render_string: bool,
    ) -> Result<ShortcodeRenderers> {
        let mut rendered: BTreeMap<String, Arc<dyn ShortcodeRenderer>> = BTreeMap::new();

        for v in &self.shortcodes {
            let s = prepare_shortcode(0, v, None, po, is_render_string)?;
            rendered.insert(v.placeholder.clone(), s);
        }

        Ok(Arc::new(Mutex::new(rendered)))
    }
}

/// Go: `expandShortcodeTokens(ctx, source, tokenHandler)` — replaces the prefixed shortcode
/// tokens with the real content. A token wrapped in `<p>…</p>` replaces the paragraph too
/// (issue #1148); Go checks `(k+4) < len(source)` (not `end+4`) before slicing, reproduced.
// Go: hugolib/shortcode.go:expandShortcodeTokens
pub fn expand_shortcode_tokens(
    source: &[u8],
    token_handler: &mut dyn FnMut(&str) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    let mut source = source.to_vec();
    let mut start = 0usize;

    let pre: &[u8] = SHORTCODE_PLACEHOLDER_PREFIX.as_bytes();
    let post: &[u8] = b"HBHB";
    let p_start: &[u8] = b"<p>";
    let p_end: &[u8] = b"</p>";

    let mut k = go_unicode::strings::index(&source[start..], pre);

    while k != -1 {
        let ku = k as usize;
        let mut j = start + ku;
        let post_idx = go_unicode::strings::index(&source[j..], post);
        if post_idx < 0 {
            // this should never happen, but let the caller decide to panic or not
            return Err(Error::new(
                "illegal state in content; shortcode token missing end delim",
            ));
        }

        let mut end = j + post_idx as usize + 4;
        let key = String::from_utf8_lossy(&source[j..end]).into_owned();
        let new_val = token_handler(&key)?;

        // Issue #1148: Check for wrapping p-tags <p>
        if j >= 3 && &source[j - 3..j] == p_start && (ku + 4) < source.len() {
            // Go slices `source[end:end+4]`, which may reach past `len` into the slice's spare
            // capacity (stale bytes) or panic; the port treats a token at the very end as not
            // wrapped (deviation, see PORTING.md: markdown output never ends inside a `<p>`).
            if end + 4 <= source.len() && &source[end..end + 4] == p_end {
                j -= 3;
                end += 4;
            }
        }

        // This and other cool slice tricks: https://github.com/golang/go/wiki/SliceTricks
        let mut ns = Vec::with_capacity(j + new_val.len() + source.len() - end);
        ns.extend_from_slice(&source[..j]);
        ns.extend_from_slice(&new_val);
        ns.extend_from_slice(&source[end..]);
        source = ns;
        start = j;
        k = go_unicode::strings::index(&source[start..], pre);
    }

    Ok(source)
}

/// Go: `renderShortcodeWithPage(ctx, h, tmpl, data)` — executes the shortcode template
/// through `template_exec::execute` (`ExecKind::Shortcode(name)`, the test seam).
// Go: hugolib/shortcode.go:renderShortcodeWithPage
pub fn render_shortcode_with_page(
    h: &Arc<HugoSites>,
    po: &PageOutput,
    ctx: &TplContext,
    tmpl: &Arc<TemplInfo>,
    data: &Arc<ShortcodeWithPage>,
) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let call = ExecCall {
        page: Some(po.p),
        output_format: po.f.name.clone(),
        kind: ExecKind::Shortcode(data.name.clone()),
        ordinal: 0,
    };
    let v = Value::Object(data.clone());
    crate::template_exec::execute(h, po.site_idx, ctx, tmpl, &mut buffer, &v, &call)
        .map_err(|err| err.wrap("failed to process shortcode"))?;
    Ok(buffer)
}

/// Go's `herrors.FileError` position of a parse error (`p.parseError`).
pub(crate) fn file_pos(filename: &str, pos: &nh_common::text::Position) -> FilePos {
    FilePos {
        filename: filename.to_string(),
        line: pos.line_number,
        column: pos.column_number,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: hugolib/shortcode_test.go:TestReplaceShortcodeTokens
    #[test]
    fn replace_shortcode_tokens() {
        let long = "BC".repeat(100);
        let cases: Vec<(&str, Vec<(&str, &str)>, Option<String>)> = vec![
            (
                "Hello HAHAHUGOSHORTCODE-1HBHB.",
                vec![("HAHAHUGOSHORTCODE-1HBHB", "World")],
                Some("Hello World.".into()),
            ),
            (
                "Hello HAHAHUGOSHORTCODE-1@}@.",
                vec![("HAHAHUGOSHORTCODE-1HBHB", "World")],
                None,
            ),
            (
                "HAHAHUGOSHORTCODE2-1HBHB",
                vec![("HAHAHUGOSHORTCODE2-1HBHB", "World")],
                Some("World".into()),
            ),
            ("Hello World!", vec![], Some("Hello World!".into())),
            (
                "!HAHAHUGOSHORTCODE-1HBHB",
                vec![("HAHAHUGOSHORTCODE-1HBHB", "World")],
                Some("!World".into()),
            ),
            (
                "HAHAHUGOSHORTCODE-1HBHB!",
                vec![("HAHAHUGOSHORTCODE-1HBHB", "World")],
                Some("World!".into()),
            ),
            (
                "!HAHAHUGOSHORTCODE-1HBHB!",
                vec![("HAHAHUGOSHORTCODE-1HBHB", "World")],
                Some("!World!".into()),
            ),
            (
                "_{_PREFIX-1HBHB",
                vec![("HAHAHUGOSHORTCODE-1HBHB", "World")],
                Some("_{_PREFIX-1HBHB".into()),
            ),
            (
                "Hello HAHAHUGOSHORTCODE-1HBHB.",
                vec![(
                    "HAHAHUGOSHORTCODE-1HBHB",
                    "To You My Old Friend Who Told Me This Fantastic Story",
                )],
                Some("Hello To You My Old Friend Who Told Me This Fantastic Story.".into()),
            ),
            (
                "A HAHAHUGOSHORTCODE-1HBHB asdf HAHAHUGOSHORTCODE-2HBHB.",
                vec![
                    ("HAHAHUGOSHORTCODE-1HBHB", "v1"),
                    ("HAHAHUGOSHORTCODE-2HBHB", "v2"),
                ],
                Some("A v1 asdf v2.".into()),
            ),
            (
                "Hello HAHAHUGOSHORTCODE2-1HBHB. Go HAHAHUGOSHORTCODE2-2HBHB, Go, Go HAHAHUGOSHORTCODE2-3HBHB Go Go!.",
                vec![
                    ("HAHAHUGOSHORTCODE2-1HBHB", "Europe"),
                    ("HAHAHUGOSHORTCODE2-2HBHB", "Jonny"),
                    ("HAHAHUGOSHORTCODE2-3HBHB", "Johnny"),
                ],
                Some("Hello Europe. Go Jonny, Go, Go Johnny Go Go!.".into()),
            ),
            (
                "A HAHAHUGOSHORTCODE-2HBHB HAHAHUGOSHORTCODE-1HBHB.",
                vec![
                    ("HAHAHUGOSHORTCODE-1HBHB", "A"),
                    ("HAHAHUGOSHORTCODE-2HBHB", "B"),
                ],
                Some("A B A.".into()),
            ),
            (
                "A HAHAHUGOSHORTCODE-1HBHB HAHAHUGOSHORTCODE-2",
                vec![("HAHAHUGOSHORTCODE-1HBHB", "A")],
                None,
            ),
            (
                "A HAHAHUGOSHORTCODE-1HBHB but not the second.",
                vec![
                    ("HAHAHUGOSHORTCODE-1HBHB", "A"),
                    ("HAHAHUGOSHORTCODE-2HBHB", "B"),
                ],
                Some("A A but not the second.".into()),
            ),
            (
                "An HAHAHUGOSHORTCODE-1HBHB.",
                vec![
                    ("HAHAHUGOSHORTCODE-1HBHB", "A"),
                    ("HAHAHUGOSHORTCODE-2HBHB", "B"),
                ],
                Some("An A.".into()),
            ),
            (
                "An HAHAHUGOSHORTCODE-1HBHB HAHAHUGOSHORTCODE-2HBHB.",
                vec![
                    ("HAHAHUGOSHORTCODE-1HBHB", "A"),
                    ("HAHAHUGOSHORTCODE-2HBHB", "B"),
                ],
                Some("An A B.".into()),
            ),
            (
                "A HAHAHUGOSHORTCODE-1HBHB HAHAHUGOSHORTCODE-2HBHB HAHAHUGOSHORTCODE-3HBHB HAHAHUGOSHORTCODE-1HBHB HAHAHUGOSHORTCODE-3HBHB.",
                vec![
                    ("HAHAHUGOSHORTCODE-1HBHB", "A"),
                    ("HAHAHUGOSHORTCODE-2HBHB", "B"),
                    ("HAHAHUGOSHORTCODE-3HBHB", "C"),
                ],
                Some("A A B C A C.".into()),
            ),
            // Issue #1148 remove p-tags 10 =>
            (
                "Hello <p>HAHAHUGOSHORTCODE-1HBHB</p>. END.",
                vec![("HAHAHUGOSHORTCODE-1HBHB", "World")],
                Some("Hello World. END.".into()),
            ),
            (
                "Hello <p>HAHAHUGOSHORTCODE-1HBHB</p>. <p>HAHAHUGOSHORTCODE-2HBHB</p> END.",
                vec![
                    ("HAHAHUGOSHORTCODE-1HBHB", "World"),
                    ("HAHAHUGOSHORTCODE-2HBHB", "THE"),
                ],
                Some("Hello World. THE END.".into()),
            ),
            (
                "Hello <p>HAHAHUGOSHORTCODE-1HBHB. END</p>.",
                vec![("HAHAHUGOSHORTCODE-1HBHB", "World")],
                Some("Hello <p>World. END</p>.".into()),
            ),
            (
                "<p>Hello HAHAHUGOSHORTCODE-1HBHB</p>. END.",
                vec![("HAHAHUGOSHORTCODE-1HBHB", "World")],
                Some("<p>Hello World</p>. END.".into()),
            ),
            (
                "Hello <p>HAHAHUGOSHORTCODE-1HBHB12",
                vec![("HAHAHUGOSHORTCODE-1HBHB", "World")],
                Some("Hello <p>World12".into()),
            ),
            (
                "Hello HAHAHUGOSHORTCODE-1HBHB. HAHAHUGOSHORTCODE-1HBHB-HAHAHUGOSHORTCODE-1HBHB HAHAHUGOSHORTCODE-1HBHB HAHAHUGOSHORTCODE-1HBHB HAHAHUGOSHORTCODE-1HBHB END",
                vec![("HAHAHUGOSHORTCODE-1HBHB", long.as_str())],
                Some(format!(
                    "Hello {long}. {long}-{long} {long} {long} {long} END"
                )),
            ),
        ];
        for (i, (input, replacements, expect)) in cases.into_iter().enumerate() {
            let m: std::collections::HashMap<&str, &str> = replacements.into_iter().collect();
            let mut handler = |token: &str| -> Result<Vec<u8>> {
                Ok(m.get(token).copied().unwrap_or("").as_bytes().to_vec())
            };
            let res = expand_shortcode_tokens(input.as_bytes(), &mut handler);
            match expect {
                None => assert!(res.is_err(), "[{i}] expected an error"),
                Some(want) => {
                    assert_eq!(String::from_utf8(res.unwrap()).unwrap(), want, "[{i}]");
                }
            }
        }
    }

    #[test]
    fn inner_cleanup_matches_go_regexp() {
        assert_eq!(inner_cleanup(b"<p>a line</p>\n".to_vec()), b"a line");
        assert_eq!(
            inner_cleanup(b"<p>a\nline</p>\n".to_vec()),
            b"<p>a\nline</p>\n"
        );
        assert_eq!(
            inner_cleanup(b"<p>a</p>\n<p>b</p>\n".to_vec()),
            b"<p>a</p>\n<p>b</p>\n"
        );
        assert_eq!(inner_cleanup(b"<p>a</p>".to_vec()), b"<p>a</p>");
        assert_eq!(inner_cleanup(b"<p></p>\n".to_vec()), b"");
        assert_eq!(inner_cleanup(b"<p>\xff</p>\n".to_vec()), b"\xff");
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/shortcode.go (794 lines; 12/26 funcs executed) — parse half in shortcode_parse.rs (T20)
//   types: ShortcodeWithPage
// OK L79-97: (scp *ShortcodeWithPage) InnerDeindent() template.HTML
// OK L101-108: (scp *ShortcodeWithPage) Position() text.Position
// OK L111-113: (scp *ShortcodeWithPage) Site() page.Site
// OK L117-119: (scp *ShortcodeWithPage) Ref(args map[string]any) (string, error)
// OK L123-125: (scp *ShortcodeWithPage) RelRef(args map[string]any) (string, error)
// OK L128-133: (scp *ShortcodeWithPage) Store() *maps.Scratch
// OK L138-140: (scp *ShortcodeWithPage) Scratch() *maps.Scratch
// OK L143-183: (scp *ShortcodeWithPage) Get(key any) any
// OK L186-188: (scp *ShortcodeWithPage) Unwrapv() any
// OK L311-346: prepareShortcode( ctx context.Context, level int, s *Site, sc *shortcode, parent *ShortcodeWithPage, po *pageOutput, isRenderString bool, ) (shortc...
// OK L348-524: doRenderShortcode( ctx context.Context, level int, s *Site, sc *shortcode, parent *ShortcodeWithPage, po *pageOutput, isRenderString bool, ) (short...
// OK L547-560: (s *shortcodeHandler) prepareShortcodesForPage(ctx context.Context, po *pageOutput, isRenderString bool) (map[string]shortcodeRenderer, error)
// OK L738-783: expandShortcodeTokens( ctx context.Context, source []byte, tokenHandler func(ctx context.Context, token string) ([]byte, error), ) ([]byte, error)
// OK L785-794: renderShortcodeWithPage(ctx context.Context, h *tplimpl.TemplateStore, tmpl *tplimpl.TemplInfo, data *ShortcodeWithPage) (string, error)
// ---------------------------------------------------------------------------
