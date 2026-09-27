// Go: github.com/yuin/goldmark@v1.7.12/renderer/html/html.go

//! Renderer that outputs HTMLs.

use std::sync::{Arc, LazyLock};

use go_unicode::{Rune, utf8};

use super::{
    Config as RendererConfig, NodeRenderer, NodeRendererFunc, NodeRendererFuncRegisterer,
    RendererOption,
};
use crate::ast::{self, Ast, AttrValue, AutoLinkType, NodeId, NodeValue, WalkStatus};
use crate::parser::OptionValue;
use crate::util::{self, BufWriter, BytesFilter};

/// A Config struct has configurations for the HTML based renderers.
#[derive(Clone)]
pub struct Config {
    pub writer: Arc<dyn Writer>,
    pub hard_wraps: bool,
    pub east_asian_line_breaks: EastAsianLineBreaks,
    pub xhtml: bool,
    pub unsafe_: bool,
}

// Go: renderer/html/html.go:NewConfig
/// NewConfig returns a new Config with defaults.
pub fn new_config() -> Config {
    Config {
        writer: DEFAULT_WRITER.clone(),
        hard_wraps: false,
        east_asian_line_breaks: EastAsianLineBreaks::None,
        xhtml: false,
        unsafe_: false,
    }
}

impl Default for Config {
    fn default() -> Self {
        new_config()
    }
}

impl Config {
    // Go: renderer/html/html.go:Config.SetOption
    /// SetOption implements renderer.NodeRenderer.SetOption.
    pub fn set_option(&mut self, name: &str, value: &OptionValue) {
        match name {
            OPT_HARD_WRAPS => {
                self.hard_wraps = *value.downcast_ref::<bool>().expect("HardWraps: bool")
            }
            OPT_EAST_ASIAN_LINE_BREAKS => {
                self.east_asian_line_breaks = *value
                    .downcast_ref::<EastAsianLineBreaks>()
                    .expect("EastAsianLineBreaks: EastAsianLineBreaks")
            }
            OPT_XHTML => self.xhtml = *value.downcast_ref::<bool>().expect("XHTML: bool"),
            OPT_UNSAFE => self.unsafe_ = *value.downcast_ref::<bool>().expect("Unsafe: bool"),
            OPT_TEXT_WRITER => {
                self.writer = value
                    .downcast_ref::<Arc<dyn Writer>>()
                    .expect("Writer: html.Writer")
                    .clone()
            }
            _ => {}
        }
    }
}

/// An Option interface sets options for HTML based renderers.
pub trait HtmlOption {
    /// SetHTMLOption sets the HTML config.
    fn set_html_option(&self, c: &mut Config);
}

/// An html.Option usable by reference as both an HTML option and a
/// renderer option. In Go every html.Option is also a renderer.Option and
/// extensions (`WithTableHTMLOptions`, `WithFootnoteHTMLOptions`) call both
/// methods on the same stored option values.
pub trait HtmlRendererOption: HtmlOption + Send + Sync {
    /// renderer.Option.SetConfig without consuming the option.
    fn set_renderer_config(&self, c: &mut RendererConfig);
}

impl<T: HtmlOption + RendererOption + Clone + Send + Sync + 'static> HtmlRendererOption for T {
    fn set_renderer_config(&self, c: &mut RendererConfig) {
        Box::new(self.clone()).set_config(c)
    }
}

/// TextWriter is an option name used in WithWriter.
pub const OPT_TEXT_WRITER: &str = "Writer";

/// Go: `withWriter`.
#[derive(Clone)]
pub struct WithWriter(Arc<dyn Writer>);

impl RendererOption for WithWriter {
    fn set_config(self: Box<Self>, c: &mut RendererConfig) {
        c.options
            .insert(OPT_TEXT_WRITER.to_string(), Arc::new(self.0));
    }
}

impl HtmlOption for WithWriter {
    fn set_html_option(&self, c: &mut Config) {
        c.writer = self.0.clone();
    }
}

// Go: renderer/html/html.go:WithWriter
/// WithWriter is a functional option that allow you to set the given writer to
/// the renderer.
pub fn with_writer(writer: Arc<dyn Writer>) -> WithWriter {
    WithWriter(writer)
}

/// HardWraps is an option name used in WithHardWraps.
pub const OPT_HARD_WRAPS: &str = "HardWraps";

/// Go: `withHardWraps`.
#[derive(Clone)]
pub struct WithHardWraps;

impl RendererOption for WithHardWraps {
    fn set_config(self: Box<Self>, c: &mut RendererConfig) {
        c.options.insert(OPT_HARD_WRAPS.to_string(), Arc::new(true));
    }
}

impl HtmlOption for WithHardWraps {
    fn set_html_option(&self, c: &mut Config) {
        c.hard_wraps = true;
    }
}

// Go: renderer/html/html.go:WithHardWraps
/// WithHardWraps is a functional option that indicates whether softline breaks
/// should be rendered as '<br>'.
pub fn with_hard_wraps() -> WithHardWraps {
    WithHardWraps
}

/// EastAsianLineBreaks is an option name used in WithEastAsianLineBreaks.
pub const OPT_EAST_ASIAN_LINE_BREAKS: &str = "EastAsianLineBreaks";

/// A EastAsianLineBreaks is a style of east asian line breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EastAsianLineBreaks {
    /// EastAsianLineBreaksNone renders line breaks as it is.
    None = 0,
    /// EastAsianLineBreaksSimple follows east_asian_line_breaks in Pandoc.
    Simple = 1,
    /// EastAsianLineBreaksCSS3Draft follows CSS text level3 "Segment Break Transformation Rules" with some enhancements.
    Css3Draft = 2,
}

impl EastAsianLineBreaks {
    // Go: renderer/html/html.go:EastAsianLineBreaks.softLineBreak
    fn soft_line_break(self, this_last_rune: Rune, sibling_first_rune: Rune) -> bool {
        match self {
            EastAsianLineBreaks::None => false,
            EastAsianLineBreaks::Simple => {
                !(util::is_east_asian_wide_rune(this_last_rune)
                    && util::is_east_asian_wide_rune(sibling_first_rune))
            }
            EastAsianLineBreaks::Css3Draft => east_asian_line_breaks_css3_draft_soft_line_break(
                this_last_rune,
                sibling_first_rune,
            ),
        }
    }
}

// Go: renderer/html/html.go:eastAsianLineBreaksCSS3DraftSoftLineBreak
fn east_asian_line_breaks_css3_draft_soft_line_break(
    this_last_rune: Rune,
    sibling_first_rune: Rune,
) -> bool {
    // Implements CSS text level3 Segment Break Transformation Rules with some enhancements.
    // References:
    //   - https://www.w3.org/TR/2020/WD-css-text-3-20200429/#line-break-transform
    //   - https://github.com/w3c/csswg-drafts/issues/5086

    // Rule1:
    //   If the character immediately before or immediately after the segment break is
    //   the zero-width space character (U+200B), then the break is removed, leaving behind the zero-width space.
    if this_last_rune == 0x200B || sibling_first_rune == 0x200B {
        return false;
    }

    // Rule2:
    //   Otherwise, if the East Asian Width property of both the character before and after the segment break is
    //   F, W, or H (not A), and neither side is Hangul, then the segment break is removed.
    let this_w = util::east_asian_width(this_last_rune);
    let sibling_w = util::east_asian_width(sibling_first_rune);
    if (this_w == "F" || this_w == "W" || this_w == "H")
        && (sibling_w == "F" || sibling_w == "W" || sibling_w == "H")
    {
        return go_unicode::is(go_unicode::HANGUL, this_last_rune)
            || go_unicode::is(go_unicode::HANGUL, sibling_first_rune);
    }

    // Rule3:
    //   Otherwise, if either the character before or after the segment break belongs to
    //   the space-discarding character set and it is a Unicode Punctuation (P*) or U+3000,
    //   then the segment break is removed.
    if util::is_space_discarding_unicode_rune(this_last_rune)
        || go_unicode::is_punct(this_last_rune)
        || this_last_rune == 0x3000
        || util::is_space_discarding_unicode_rune(sibling_first_rune)
        || go_unicode::is_punct(sibling_first_rune)
        || sibling_first_rune == 0x3000
    {
        return false;
    }

    // Rule4:
    //   Otherwise, the segment break is converted to a space (U+0020).
    true
}

/// Go: `withEastAsianLineBreaks`.
#[derive(Clone)]
pub struct WithEastAsianLineBreaks(EastAsianLineBreaks);

impl RendererOption for WithEastAsianLineBreaks {
    fn set_config(self: Box<Self>, c: &mut RendererConfig) {
        c.options
            .insert(OPT_EAST_ASIAN_LINE_BREAKS.to_string(), Arc::new(self.0));
    }
}

impl HtmlOption for WithEastAsianLineBreaks {
    fn set_html_option(&self, c: &mut Config) {
        c.east_asian_line_breaks = self.0;
    }
}

// Go: renderer/html/html.go:WithEastAsianLineBreaks
/// WithEastAsianLineBreaks is a functional option that indicates whether softline breaks
/// between east asian wide characters should be ignored.
pub fn with_east_asian_line_breaks(e: EastAsianLineBreaks) -> WithEastAsianLineBreaks {
    WithEastAsianLineBreaks(e)
}

/// XHTML is an option name used in WithXHTML.
pub const OPT_XHTML: &str = "XHTML";

/// Go: `withXHTML`.
#[derive(Clone)]
pub struct WithXHTML;

impl RendererOption for WithXHTML {
    fn set_config(self: Box<Self>, c: &mut RendererConfig) {
        c.options.insert(OPT_XHTML.to_string(), Arc::new(true));
    }
}

impl HtmlOption for WithXHTML {
    fn set_html_option(&self, c: &mut Config) {
        c.xhtml = true;
    }
}

// Go: renderer/html/html.go:WithXHTML
/// WithXHTML is a functional option indicates that nodes should be rendered in
/// xhtml instead of HTML5.
pub fn with_xhtml() -> WithXHTML {
    WithXHTML
}

/// Unsafe is an option name used in WithUnsafe.
pub const OPT_UNSAFE: &str = "Unsafe";

/// Go: `withUnsafe`.
#[derive(Clone)]
pub struct WithUnsafe;

impl RendererOption for WithUnsafe {
    fn set_config(self: Box<Self>, c: &mut RendererConfig) {
        c.options.insert(OPT_UNSAFE.to_string(), Arc::new(true));
    }
}

impl HtmlOption for WithUnsafe {
    fn set_html_option(&self, c: &mut Config) {
        c.unsafe_ = true;
    }
}

// Go: renderer/html/html.go:WithUnsafe
/// WithUnsafe is a functional option that renders dangerous contents
/// (raw htmls and potentially dangerous links) as it is.
pub fn with_unsafe() -> WithUnsafe {
    WithUnsafe
}

/// A Renderer struct is an implementation of renderer.NodeRenderer that renders
/// nodes as (X)HTML.
pub struct Renderer {
    pub config: Config,
}

// Go: renderer/html/html.go:NewRenderer
/// NewRenderer returns a new Renderer with given options.
pub fn new_renderer(opts: Vec<Box<dyn HtmlOption>>) -> Box<dyn NodeRenderer> {
    let mut r = Renderer {
        config: new_config(),
    };
    for opt in opts {
        opt.set_html_option(&mut r.config);
    }
    Box::new(r)
}

type RenderMethod = fn(
    &Renderer,
    &mut dyn BufWriter,
    &[u8],
    &Ast,
    NodeId,
    bool,
) -> Result<WalkStatus, crate::Error>;

/// Wraps a renderer method as a NodeRendererFunc bound to `r`.
pub fn bind<R: Send + Sync + 'static>(
    r: &Arc<R>,
    f: fn(&R, &mut dyn BufWriter, &[u8], &Ast, NodeId, bool) -> Result<WalkStatus, crate::Error>,
) -> NodeRendererFunc {
    let r = r.clone();
    Arc::new(move |w, source, ast, n, entering| f(&r, w, source, ast, n, entering))
}

impl NodeRenderer for Renderer {
    // Go: renderer/html/html.go:Renderer.RegisterFuncs
    fn register_funcs(self: Arc<Self>, reg: &mut dyn NodeRendererFuncRegisterer) {
        let m = |f: RenderMethod| bind(&self, f);
        // blocks

        reg.register(ast::KIND_DOCUMENT, m(Renderer::render_document));
        reg.register(ast::KIND_HEADING, m(Renderer::render_heading));
        reg.register(ast::KIND_BLOCKQUOTE, m(Renderer::render_blockquote));
        reg.register(ast::KIND_CODE_BLOCK, m(Renderer::render_code_block));
        reg.register(
            ast::KIND_FENCED_CODE_BLOCK,
            m(Renderer::render_fenced_code_block),
        );
        reg.register(ast::KIND_HTML_BLOCK, m(Renderer::render_html_block));
        reg.register(ast::KIND_LIST, m(Renderer::render_list));
        reg.register(ast::KIND_LIST_ITEM, m(Renderer::render_list_item));
        reg.register(ast::KIND_PARAGRAPH, m(Renderer::render_paragraph));
        reg.register(ast::KIND_TEXT_BLOCK, m(Renderer::render_text_block));
        reg.register(ast::KIND_THEMATIC_BREAK, m(Renderer::render_thematic_break));

        // inlines

        reg.register(ast::KIND_AUTO_LINK, m(Renderer::render_auto_link));
        reg.register(ast::KIND_CODE_SPAN, m(Renderer::render_code_span));
        reg.register(ast::KIND_EMPHASIS, m(Renderer::render_emphasis));
        reg.register(ast::KIND_IMAGE, m(Renderer::render_image));
        reg.register(ast::KIND_LINK, m(Renderer::render_link));
        reg.register(ast::KIND_RAW_HTML, m(Renderer::render_raw_html));
        reg.register(ast::KIND_TEXT, m(Renderer::render_text));
        reg.register(ast::KIND_STRING, m(Renderer::render_string));
    }

    fn set_option(&mut self, name: &str, value: &OptionValue) {
        self.config.set_option(name, value);
    }
}

/// GlobalAttributeFilter defines attribute names which any elements can have.
pub static GLOBAL_ATTRIBUTE_FILTER: LazyLock<BytesFilter> = LazyLock::new(|| {
    util::new_bytes_filter_string(
        "accesskey,autocapitalize,autofocus,class,contenteditable,dir,draggable,enterkeyhint,hidden,id,inert,inputmode,is,itemid,itemprop,itemref,itemscope,itemtype,lang,part,role,slot,spellcheck,style,tabindex,title,translate",
    )
});

/// HeadingAttributeFilter defines attribute names which heading elements can have.
pub static HEADING_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| GLOBAL_ATTRIBUTE_FILTER.clone());

/// BlockquoteAttributeFilter defines attribute names which blockquote elements can have.
pub static BLOCKQUOTE_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| GLOBAL_ATTRIBUTE_FILTER.extend_string("cite"));

/// ListAttributeFilter defines attribute names which list elements can have.
pub static LIST_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| GLOBAL_ATTRIBUTE_FILTER.extend_string("start,reversed,type"));

/// ListItemAttributeFilter defines attribute names which list item elements can have.
pub static LIST_ITEM_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| GLOBAL_ATTRIBUTE_FILTER.extend_string("value"));

/// ParagraphAttributeFilter defines attribute names which paragraph elements can have.
pub static PARAGRAPH_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| GLOBAL_ATTRIBUTE_FILTER.clone());

/// ThematicAttributeFilter defines attribute names which hr elements can have.
pub static THEMATIC_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| GLOBAL_ATTRIBUTE_FILTER.extend_string("align,color,noshade,size,width"));

/// LinkAttributeFilter defines attribute names which link elements can have.
pub static LINK_ATTRIBUTE_FILTER: LazyLock<BytesFilter> = LazyLock::new(|| {
    GLOBAL_ATTRIBUTE_FILTER
        .extend_string("download,hreflang,media,ping,referrerpolicy,rel,shape,target")
});

/// CodeAttributeFilter defines attribute names which code elements can have.
pub static CODE_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| GLOBAL_ATTRIBUTE_FILTER.clone());

/// EmphasisAttributeFilter defines attribute names which emphasis elements can have.
pub static EMPHASIS_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| GLOBAL_ATTRIBUTE_FILTER.clone());

/// ImageAttributeFilter defines attribute names which image elements can have.
pub static IMAGE_ATTRIBUTE_FILTER: LazyLock<BytesFilter> = LazyLock::new(|| {
    GLOBAL_ATTRIBUTE_FILTER.extend_string(
        "align,border,crossorigin,decoding,height,importance,intrinsicsize,ismap,loading,referrerpolicy,sizes,srcset,usemap,width",
    )
});

type R = Result<WalkStatus, crate::Error>;

impl Renderer {
    // Go: renderer/html/html.go:Renderer.writeLines
    fn write_lines(&self, w: &mut dyn BufWriter, source: &[u8], ast: &Ast, n: NodeId) {
        let lines = ast.lines(n);
        for i in 0..lines.len() {
            let line = lines.at(i);
            self.config.writer.raw_write(w, &line.value(source));
        }
    }

    // Go: renderer/html/html.go:Renderer.renderDocument
    pub fn render_document(
        &self,
        _w: &mut dyn BufWriter,
        _source: &[u8],
        _ast: &Ast,
        _n: NodeId,
        _entering: bool,
    ) -> R {
        // nothing to do
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderHeading
    pub fn render_heading(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = ast.heading(node).expect("not a Heading");
        if entering {
            w.write_string("<h");
            w.write_byte(b"0123456"[n.level as usize]);
            if ast.attributes(node).is_some() {
                render_attributes(w, ast, node, Some(&HEADING_ATTRIBUTE_FILTER));
            }
            w.write_byte(b'>');
        } else {
            w.write_string("</h");
            w.write_byte(b"0123456"[n.level as usize]);
            w.write_string(">\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderBlockquote
    pub fn render_blockquote(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            if ast.attributes(n).is_some() {
                w.write_string("<blockquote");
                render_attributes(w, ast, n, Some(&BLOCKQUOTE_ATTRIBUTE_FILTER));
                w.write_byte(b'>');
            } else {
                w.write_string("<blockquote>\n");
            }
        } else {
            w.write_string("</blockquote>\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderCodeBlock
    pub fn render_code_block(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            w.write_string("<pre><code>");
            self.write_lines(w, source, ast, n);
        } else {
            w.write_string("</code></pre>\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderFencedCodeBlock
    pub fn render_fenced_code_block(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            w.write_string("<pre><code");
            if let Some(language) = ast.fenced_code_block_language(n, source) {
                w.write_string(" class=\"language-");
                self.config.writer.write(w, &language);
                w.write_string("\"");
            }
            w.write_byte(b'>');
            self.write_lines(w, source, ast, n);
        } else {
            w.write_string("</code></pre>\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderHTMLBlock
    pub fn render_html_block(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = ast.html_block(node).expect("not an HTMLBlock");
        if entering {
            if self.config.unsafe_ {
                let lines = ast.lines(node);
                for i in 0..lines.len() {
                    let line = lines.at(i);
                    self.config.writer.secure_write(w, &line.value(source));
                }
            } else {
                w.write_string("<!-- raw HTML omitted -->\n");
            }
        } else if n.has_closure() {
            if self.config.unsafe_ {
                let closure = n.closure_line;
                self.config.writer.secure_write(w, &closure.value(source));
            } else {
                w.write_string("<!-- raw HTML omitted -->\n");
            }
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderList
    pub fn render_list(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = ast.list(node).expect("not a List");
        let mut tag = "ul";
        if n.is_ordered() {
            tag = "ol";
        }
        if entering {
            w.write_byte(b'<');
            w.write_string(tag);
            if n.is_ordered() && n.start != 1 {
                w.write_string(&format!(" start=\"{}\"", go_strconv::itoa(n.start)));
            }
            if ast.attributes(node).is_some() {
                render_attributes(w, ast, node, Some(&LIST_ATTRIBUTE_FILTER));
            }
            w.write_string(">\n");
        } else {
            w.write_string("</");
            w.write_string(tag);
            w.write_string(">\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderListItem
    pub fn render_list_item(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            if ast.attributes(n).is_some() {
                w.write_string("<li");
                render_attributes(w, ast, n, Some(&LIST_ITEM_ATTRIBUTE_FILTER));
                w.write_byte(b'>');
            } else {
                w.write_string("<li>");
            }
            if let Some(fc) = ast.first_child(n)
                && !matches!(ast.value(fc), NodeValue::TextBlock)
            {
                w.write_byte(b'\n');
            }
        } else {
            w.write_string("</li>\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderParagraph
    pub fn render_paragraph(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            if ast.attributes(n).is_some() {
                w.write_string("<p");
                render_attributes(w, ast, n, Some(&PARAGRAPH_ATTRIBUTE_FILTER));
                w.write_byte(b'>');
            } else {
                w.write_string("<p>");
            }
        } else {
            w.write_string("</p>\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderTextBlock
    pub fn render_text_block(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if !entering && ast.next_sibling(n).is_some() && ast.first_child(n).is_some() {
            w.write_byte(b'\n');
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderThematicBreak
    pub fn render_thematic_break(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if !entering {
            return Ok(WalkStatus::Continue);
        }
        w.write_string("<hr");
        if ast.attributes(n).is_some() {
            render_attributes(w, ast, n, Some(&THEMATIC_ATTRIBUTE_FILTER));
        }
        if self.config.xhtml {
            w.write_string(" />\n");
        } else {
            w.write_string(">\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderAutoLink
    pub fn render_auto_link(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = ast.auto_link(node).expect("not an AutoLink");
        if !entering {
            return Ok(WalkStatus::Continue);
        }
        w.write_string("<a href=\"");
        let url = ast.auto_link_url(n, source);
        let label = ast.auto_link_label(n, source);
        if n.auto_link_type == AutoLinkType::Email
            && !go_unicode::bytes::to_lower(&url).starts_with(b"mailto:")
        {
            w.write_string("mailto:");
        }
        w.write(&util::escape_html(&util::url_escape(&url, false)));
        if ast.attributes(node).is_some() {
            w.write_byte(b'"');
            render_attributes(w, ast, node, Some(&LINK_ATTRIBUTE_FILTER));
            w.write_byte(b'>');
        } else {
            w.write_string("\">");
        }
        w.write(&util::escape_html(&label));
        w.write_string("</a>");
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderCodeSpan
    pub fn render_code_span(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            if ast.attributes(n).is_some() {
                w.write_string("<code");
                render_attributes(w, ast, n, Some(&CODE_ATTRIBUTE_FILTER));
                w.write_byte(b'>');
            } else {
                w.write_string("<code>");
            }
            let mut c = ast.first_child(n);
            while let Some(cid) = c {
                let segment = ast
                    .text_node(cid)
                    .expect("interface conversion: not *ast.Text")
                    .segment;
                let value = segment.value(source);
                if value.ends_with(b"\n") {
                    self.config.writer.raw_write(w, &value[..value.len() - 1]);
                    self.config.writer.raw_write(w, b" ");
                } else {
                    self.config.writer.raw_write(w, &value);
                }
                c = ast.next_sibling(cid);
            }
            return Ok(WalkStatus::SkipChildren);
        }
        w.write_string("</code>");
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderEmphasis
    pub fn render_emphasis(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = ast.emphasis(node).expect("not an Emphasis");
        let mut tag = "em";
        if n.level == 2 {
            tag = "strong";
        }
        if entering {
            w.write_byte(b'<');
            w.write_string(tag);
            if ast.attributes(node).is_some() {
                render_attributes(w, ast, node, Some(&EMPHASIS_ATTRIBUTE_FILTER));
            }
            w.write_byte(b'>');
        } else {
            w.write_string("</");
            w.write_string(tag);
            w.write_byte(b'>');
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderLink
    pub fn render_link(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = ast.link(node).expect("not a Link");
        if entering {
            w.write_string("<a href=\"");
            if self.config.unsafe_ || !is_dangerous_url(&n.destination) {
                w.write(&util::escape_html(&util::url_escape(&n.destination, true)));
            }
            w.write_byte(b'"');
            if let Some(title) = &n.title {
                w.write_string(" title=\"");
                self.config.writer.write(w, title);
                w.write_byte(b'"');
            }
            if ast.attributes(node).is_some() {
                render_attributes(w, ast, node, Some(&LINK_ATTRIBUTE_FILTER));
            }
            w.write_byte(b'>');
        } else {
            w.write_string("</a>");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderImage
    pub fn render_image(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if !entering {
            return Ok(WalkStatus::Continue);
        }
        let n = ast.image(node).expect("not an Image");
        w.write_string("<img src=\"");
        if self.config.unsafe_ || !is_dangerous_url(&n.destination) {
            w.write(&util::escape_html(&util::url_escape(&n.destination, true)));
        }
        w.write_string("\" alt=\"");
        self.render_texts(w, source, ast, node);
        w.write_byte(b'"');
        if let Some(title) = &n.title {
            w.write_string(" title=\"");
            self.config.writer.write(w, title);
            w.write_byte(b'"');
        }
        if ast.attributes(node).is_some() {
            render_attributes(w, ast, node, Some(&IMAGE_ATTRIBUTE_FILTER));
        }
        if self.config.xhtml {
            w.write_string(" />");
        } else {
            w.write_string(">");
        }
        Ok(WalkStatus::SkipChildren)
    }

    // Go: renderer/html/html.go:Renderer.renderRawHTML
    pub fn render_raw_html(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if !entering {
            return Ok(WalkStatus::SkipChildren);
        }
        if self.config.unsafe_ {
            let n = ast.raw_html(node).expect("not a RawHTML");
            let l = n.segments.len();
            for i in 0..l {
                let segment = n.segments.at(i);
                w.write(&segment.value(source));
            }
            return Ok(WalkStatus::SkipChildren);
        }
        w.write_string("<!-- raw HTML omitted -->");
        Ok(WalkStatus::SkipChildren)
    }

    // Go: renderer/html/html.go:Renderer.renderText
    pub fn render_text(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if !entering {
            return Ok(WalkStatus::Continue);
        }
        let n = ast.text_node(node).expect("not a Text");
        let segment = n.segment;
        if n.is_raw() {
            self.config.writer.raw_write(w, &segment.value(source));
        } else {
            let value = segment.value(source);
            self.config.writer.write(w, &value);
            if n.hard_line_break() || (n.soft_line_break() && self.config.hard_wraps) {
                if self.config.xhtml {
                    w.write_string("<br />\n");
                } else {
                    w.write_string("<br>\n");
                }
            } else if n.soft_line_break() {
                if self.config.east_asian_line_breaks != EastAsianLineBreaks::None
                    && !value.is_empty()
                {
                    if let Some(sibling) = ast.next_sibling(node)
                        && ast.kind(sibling) == ast::KIND_TEXT
                    {
                        let sibling_text = ast.text_node(sibling).unwrap().value(source);
                        if !sibling_text.is_empty() {
                            let this_last_rune = util::to_rune(&value, value.len() - 1);
                            let (sibling_first_rune, _) = utf8::decode_rune(&sibling_text);
                            if self
                                .config
                                .east_asian_line_breaks
                                .soft_line_break(this_last_rune, sibling_first_rune)
                            {
                                w.write_byte(b'\n');
                            }
                        }
                    }
                } else {
                    w.write_byte(b'\n');
                }
            }
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderString
    pub fn render_string(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if !entering {
            return Ok(WalkStatus::Continue);
        }
        let n = ast.string_node(node).expect("not a String");
        if n.is_code() {
            w.write(&n.value);
        } else if n.is_raw() {
            self.config.writer.raw_write(w, &n.value);
        } else {
            self.config.writer.write(w, &n.value);
        }
        Ok(WalkStatus::Continue)
    }

    // Go: renderer/html/html.go:Renderer.renderTexts (depth-first over the
    // descendants; an explicit stack instead of recursion)
    fn render_texts(&self, w: &mut dyn BufWriter, source: &[u8], ast: &Ast, n: NodeId) {
        let mut stack: Vec<Option<NodeId>> = vec![ast.first_child(n)];
        while let Some(top) = stack.last_mut() {
            let Some(cid) = *top else {
                stack.pop();
                continue;
            };
            *top = ast.next_sibling(cid);
            match ast.value(cid) {
                NodeValue::String(_) => {
                    let _ = self.render_string(w, source, ast, cid, true);
                }
                NodeValue::Text(_) => {
                    let _ = self.render_text(w, source, ast, cid, true);
                }
                _ => stack.push(ast.first_child(cid)),
            }
        }
    }
}

const DATA_PREFIX: &[u8] = b"data-";

// Go: renderer/html/html.go:RenderAttributes
/// RenderAttributes renders given node's attributes.
/// You can specify attribute names to render by the filter.
/// If filter is nil, RenderAttributes renders all attributes.
pub fn render_attributes(
    w: &mut dyn BufWriter,
    ast: &Ast,
    node: NodeId,
    filter: Option<&BytesFilter>,
) {
    let Some(attrs) = ast.attributes(node) else {
        return;
    };
    render_attribute_list(w, attrs, filter);
}

/// RenderAttributes over an explicit attribute list (the body of Go's
/// `RenderAttributes`; used where Go mutates a node's attributes right
/// before rendering them, e.g. the table cell alignment style).
pub fn render_attribute_list(
    w: &mut dyn BufWriter,
    attrs: &[crate::ast::Attribute],
    filter: Option<&BytesFilter>,
) {
    for attr in attrs {
        if let Some(filter) = filter
            && !filter.contains(&attr.name)
            && !attr.name.starts_with(DATA_PREFIX)
        {
            continue;
        }
        w.write_string(" ");
        w.write(&attr.name);
        w.write_string("=\"");
        // TODO: convert numeric values to strings
        let value: &[u8] = match &attr.value {
            AttrValue::Bytes(b) => b,
            AttrValue::String(s) => s,
            _ => &[],
        };
        w.write(&util::escape_html(value));
        w.write_byte(b'"');
    }
}

/// A Writer interface writes textual contents to a writer.
pub trait Writer: Send + Sync {
    /// Write writes the given source to writer with resolving references and unescaping
    /// backslash escaped characters.
    fn write(&self, writer: &mut dyn BufWriter, source: &[u8]);

    /// RawWrite writes the given source to writer without resolving references and
    /// unescaping backslash escaped characters.
    fn raw_write(&self, writer: &mut dyn BufWriter, source: &[u8]);

    /// SecureWrite writes the given source to writer with replacing insecure characters.
    fn secure_write(&self, writer: &mut dyn BufWriter, source: &[u8]);
}

const REPLACEMENT_CHARACTER: &[u8] = "\u{fffd}".as_bytes();

/// A WriterConfig struct has configurations for the HTML based writers.
#[derive(Debug, Clone, Copy, Default)]
pub struct WriterConfig {
    /// EscapedSpace is an option that indicates that a '\' escaped half-space(0x20) should not be rendered.
    pub escaped_space: bool,
}

/// A WriterOption interface sets options for HTML based writers.
pub type WriterOption = Box<dyn Fn(&mut WriterConfig)>;

// Go: renderer/html/html.go:WithEscapedSpace
/// WithEscapedSpace is a WriterOption indicates that a '\' escaped half-space(0x20) should not be rendered.
pub fn with_escaped_space() -> WriterOption {
    Box::new(|c: &mut WriterConfig| c.escaped_space = true)
}

/// Go: `defaultWriter`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultWriter {
    pub config: WriterConfig,
}

// Go: renderer/html/html.go:NewWriter
/// NewWriter returns a new Writer.
pub fn new_writer(opts: Vec<WriterOption>) -> Arc<dyn Writer> {
    let mut w = DefaultWriter::default();
    for opt in opts {
        opt(&mut w.config);
    }
    Arc::new(w)
}

// Go: renderer/html/html.go:escapeRune
fn escape_rune(writer: &mut dyn BufWriter, r: Rune) {
    if r < 256
        && let Some(v) = util::escape_html_byte(r as u8)
    {
        writer.write(v);
        return;
    }
    writer.write_rune(util::to_valid_rune(r));
}

impl Writer for DefaultWriter {
    // Go: renderer/html/html.go:defaultWriter.SecureWrite
    fn secure_write(&self, writer: &mut dyn BufWriter, source: &[u8]) {
        let mut n = 0;
        let l = source.len();
        for i in 0..l {
            if source[i] == 0 {
                writer.write(&source[i - n..i]);
                n = 0;
                writer.write(REPLACEMENT_CHARACTER);
                continue;
            }
            n += 1;
        }
        if n != 0 {
            writer.write(&source[l - n..]);
        }
    }

    // Go: renderer/html/html.go:defaultWriter.RawWrite
    fn raw_write(&self, writer: &mut dyn BufWriter, source: &[u8]) {
        let mut n = 0;
        let l = source.len();
        for i in 0..l {
            if let Some(v) = util::escape_html_byte(source[i]) {
                writer.write(&source[i - n..i]);
                n = 0;
                writer.write(v);
                continue;
            }
            n += 1;
        }
        if n != 0 {
            writer.write(&source[l - n..]);
        }
    }

    // Go: renderer/html/html.go:defaultWriter.Write
    fn write(&self, writer: &mut dyn BufWriter, source: &[u8]) {
        let mut escaped = false;
        let mut ok;
        let limit = source.len();
        let mut n = 0;
        let mut i = 0;
        while i < limit {
            let c = source[i];
            if escaped {
                if util::is_punct(c) {
                    self.raw_write(writer, &source[n..i - 1]);
                    n = i;
                    escaped = false;
                    i += 1;
                    continue;
                }
                if self.config.escaped_space && c == b' ' {
                    self.raw_write(writer, &source[n..i - 1]);
                    n = i + 1;
                    escaped = false;
                    i += 1;
                    continue;
                }
            }
            if c == 0 {
                self.raw_write(writer, &source[n..i]);
                self.raw_write(writer, REPLACEMENT_CHARACTER);
                n = i + 1;
                escaped = false;
                i += 1;
                continue;
            }
            if c == b'&' {
                let pos = i;
                let next = i + 1;
                if next < limit && source[next] == b'#' {
                    let nnext = next + 1;
                    if nnext < limit {
                        let nc = source[nnext];
                        // code point like #x22;
                        if nnext < limit && nc == b'x' || nc == b'X' {
                            let start = nnext + 1;
                            (i, ok) =
                                util::read_while(source, [start, limit], util::is_hex_decimal);
                            if ok && i < limit && source[i] == b';' && i - start < 7 {
                                let v = util::parse_uint32_lossy(&source[start..i], 16);
                                self.raw_write(writer, &source[n..pos]);
                                n = i + 1;
                                escape_rune(writer, v as Rune);
                                i += 1;
                                continue;
                            }
                            // code point like #1234;
                        } else if nc.is_ascii_digit() {
                            let start = nnext;
                            (i, ok) = util::read_while(source, [start, limit], util::is_numeric);
                            if ok && i < limit && i - start < 8 && source[i] == b';' {
                                let v = util::parse_uint32_lossy(&source[start..i], 10);
                                self.raw_write(writer, &source[n..pos]);
                                n = i + 1;
                                escape_rune(writer, v as Rune);
                                i += 1;
                                continue;
                            }
                        }
                    }
                } else {
                    let start = next;
                    (i, ok) = util::read_while(source, [start, limit], util::is_alpha_numeric);
                    // entity reference
                    if ok && i < limit && source[i] == b';' {
                        let name = &source[start..i];
                        if let Some(entity) = util::lookup_html5_entity_by_name(name) {
                            self.raw_write(writer, &source[n..pos]);
                            n = i + 1;
                            self.raw_write(writer, entity.characters);
                            i += 1;
                            continue;
                        }
                    }
                }
                i = next - 1;
            }
            if c == b'\\' {
                escaped = true;
                i += 1;
                continue;
            }
            escaped = false;
            i += 1;
        }
        self.raw_write(writer, &source[n..]);
    }
}

/// DefaultWriter is a default instance of the Writer.
pub static DEFAULT_WRITER: LazyLock<Arc<dyn Writer>> = LazyLock::new(|| new_writer(Vec::new()));

const B_DATA_IMAGE: &[u8] = b"data:image/";
const B_PNG: &[u8] = b"png;";
const B_GIF: &[u8] = b"gif;";
const B_JPEG: &[u8] = b"jpeg;";
const B_WEBP: &[u8] = b"webp;";
const B_SVG: &[u8] = b"svg+xml;";
const B_JS: &[u8] = b"javascript:";
const B_VB: &[u8] = b"vbscript:";
const B_FILE: &[u8] = b"file:";
const B_DATA: &[u8] = b"data:";

// Go: renderer/html/html.go:hasPrefix
fn has_prefix(s: &[u8], prefix: &[u8]) -> bool {
    s.len() >= prefix.len()
        && go_unicode::bytes::to_lower(&s[0..prefix.len()]) == go_unicode::bytes::to_lower(prefix)
}

// Go: renderer/html/html.go:IsDangerousURL
/// IsDangerousURL returns true if the given url seems a potentially dangerous url,
/// otherwise false.
pub fn is_dangerous_url(url: &[u8]) -> bool {
    if has_prefix(url, B_DATA_IMAGE) && url.len() >= 11 {
        let v = &url[11..];
        if has_prefix(v, B_PNG)
            || has_prefix(v, B_GIF)
            || has_prefix(v, B_JPEG)
            || has_prefix(v, B_WEBP)
            || has_prefix(v, B_SVG)
        {
            return false;
        }
        return true;
    }
    has_prefix(url, B_JS)
        || has_prefix(url, B_VB)
        || has_prefix(url, B_FILE)
        || has_prefix(url, B_DATA)
}
