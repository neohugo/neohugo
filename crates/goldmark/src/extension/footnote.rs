// Go: github.com/yuin/goldmark@v1.7.12/extension/footnote.go

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use super::ast as east;
use crate::ast::{Ast, NodeId, WalkStatus};
use crate::parser::{
    self, AstTransformer, BlockParser, Context, ContextKey, InlineParser, OptionValue, State,
};
use crate::renderer::html::{self, HtmlRendererOption};
use crate::renderer::{self, NodeRenderer, NodeRendererFuncRegisterer, RendererOption};
use crate::text::{Reader, new_segment};
use crate::util::{self, BufWriter};
use crate::{Extender, Markdown};

/// Holds the document's `*ast.FootnoteList` (a `NodeId`).
static FOOTNOTE_LIST_KEY: LazyLock<ContextKey> = LazyLock::new(parser::new_context_key);
/// Holds the `[]*ast.FootnoteLink` created so far (a `Vec<NodeId>`).
static FOOTNOTE_LINK_LIST_KEY: LazyLock<ContextKey> = LazyLock::new(parser::new_context_key);

struct FootnoteBlockParser;

// Go: extension/footnote.go:NewFootnoteBlockParser
/// NewFootnoteBlockParser returns a new parser.BlockParser that can parse
/// footnotes of the Markdown(PHP Markdown Extra) text.
pub fn new_footnote_block_parser() -> Box<dyn BlockParser> {
    Box::new(FootnoteBlockParser)
}

impl BlockParser for FootnoteBlockParser {
    // Go: extension/footnote.go:footnoteBlockParser.Trigger
    fn trigger(&self) -> Option<&[u8]> {
        Some(b"[")
    }

    // Go: extension/footnote.go:footnoteBlockParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        let (line, segment) = reader.peek_line();
        let line = line.unwrap_or_default();
        let mut pos = pc.block_offset();
        if pos < 0 || line[pos as usize] != b'[' {
            return (None, State::NO_CHILDREN);
        }
        pos += 1;
        if pos > line.len() as i64 - 1 || line[pos as usize] != b'^' {
            return (None, State::NO_CHILDREN);
        }
        let open = pos + 1;
        let closure = util::find_closure(&line[(pos + 1) as usize..], b'[', b']', false, false);
        let closes = pos + 1 + closure;
        let next = closes + 1;
        if closure > -1 {
            if next >= line.len() as i64 || line[next as usize] != b':' {
                return (None, State::NO_CHILDREN);
            }
        } else {
            return (None, State::NO_CHILDREN);
        }
        let padding = segment.padding;
        let label = reader
            .value(new_segment(
                segment.start + open - padding,
                segment.start + closes - padding,
            ))
            .into_owned();
        if util::is_blank(&label) {
            return (None, State::NO_CHILDREN);
        }
        let item = east::new_footnote(ast, label);

        pos = next + 1 - padding;
        if pos >= line.len() as i64 {
            reader.advance(pos);
            return (Some(item), State::NO_CHILDREN);
        }
        reader.advance_and_set_padding(pos, padding);
        (Some(item), State::HAS_CHILDREN)
    }

    // Go: extension/footnote.go:footnoteBlockParser.Continue
    fn continue_<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> State {
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        if util::is_blank(&line) {
            return State::CONTINUE | State::HAS_CHILDREN;
        }
        let lo = reader.line_offset();
        let (childpos, padding) = util::indent_position(&line, lo, 4);
        if childpos < 0 {
            return State::CLOSE;
        }
        reader.advance_and_set_padding(childpos, padding);
        State::CONTINUE | State::HAS_CHILDREN
    }

    // Go: extension/footnote.go:footnoteBlockParser.Close
    fn close<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        _reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) {
        let list = match pc.get_as::<NodeId>(*FOOTNOTE_LIST_KEY) {
            Some(&l) => l,
            None => {
                let l = east::new_footnote_list(ast);
                pc.set_value(*FOOTNOTE_LIST_KEY, l);
                let p = ast.parent(node).expect("footnote has a parent");
                ast.insert_before(p, Some(node), l);
                l
            }
        };
        let p = ast.parent(node).expect("footnote has a parent");
        ast.remove_child(p, node);
        ast.append_child(list, node);
    }

    // Go: extension/footnote.go:footnoteBlockParser.CanInterruptParagraph
    fn can_interrupt_paragraph(&self) -> bool {
        true
    }

    // Go: extension/footnote.go:footnoteBlockParser.CanAcceptIndentedLine
    fn can_accept_indented_line(&self) -> bool {
        false
    }
}

struct FootnoteParser;

// Go: extension/footnote.go:NewFootnoteParser
/// NewFootnoteParser returns a new parser.InlineParser that can parse
/// footnote links of the Markdown(PHP Markdown Extra) text.
pub fn new_footnote_parser() -> Box<dyn InlineParser> {
    Box::new(FootnoteParser)
}

impl InlineParser for FootnoteParser {
    // Go: extension/footnote.go:footnoteParser.Trigger
    fn trigger(&self) -> &[u8] {
        // footnote syntax probably conflict with the image syntax.
        // So we need trigger this parser with '!'.
        b"!["
    }

    // Go: extension/footnote.go:footnoteParser.Parse
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        parent: NodeId,
        block: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> Option<NodeId> {
        let (line, segment) = block.peek_line();
        let line = line.unwrap_or_default();
        let mut pos: usize = 1;
        if !line.is_empty() && line[0] == b'!' {
            pos += 1;
        }
        if pos >= line.len() || line[pos] != b'^' {
            return None;
        }
        pos += 1;
        if pos >= line.len() {
            return None;
        }
        let open = pos;
        let closure = util::find_closure(&line[pos..], b'[', b']', false, false);
        if closure < 0 {
            return None;
        }
        let closes = pos as i64 + closure;
        let value = block
            .value(new_segment(
                segment.start + open as i64,
                segment.start + closes,
            ))
            .into_owned();
        block.advance(closes + 1);

        let list = pc.get_as::<NodeId>(*FOOTNOTE_LIST_KEY).copied()?;
        let mut index = 0;
        let mut def = ast.first_child(list);
        while let Some(d) = def {
            let fd = east::must::<east::Footnote>(ast, d, "Footnote");
            if fd.ref_ == value {
                if fd.index < 0 {
                    let count = {
                        let l = east::must_mut::<east::FootnoteList>(ast, list, "FootnoteList");
                        l.count += 1;
                        l.count
                    };
                    east::must_mut::<east::Footnote>(ast, d, "Footnote").index = count;
                }
                index = east::must::<east::Footnote>(ast, d, "Footnote").index;
                break;
            }
            def = ast.next_sibling(d);
        }
        if index == 0 {
            return None;
        }

        let fnlink = east::new_footnote_link(ast, index);
        pc.compute_if_absent(*FOOTNOTE_LINK_LIST_KEY, || Box::new(Vec::<NodeId>::new()))
            .downcast_mut::<Vec<NodeId>>()
            .expect("footnote link list")
            .push(fnlink);
        if line[0] == b'!' {
            let t = ast.new_text_segment(new_segment(segment.start, segment.start + 1));
            ast.append_child(parent, t);
        }

        Some(fnlink)
    }
}

struct FootnoteASTTransformer;

// Go: extension/footnote.go:NewFootnoteASTTransformer
/// NewFootnoteASTTransformer returns a new parser.ASTTransformer that
/// insert a footnote list to the last of the document.
pub fn new_footnote_ast_transformer() -> Box<dyn AstTransformer> {
    Box::new(FootnoteASTTransformer)
}

impl AstTransformer for FootnoteASTTransformer {
    // Go: extension/footnote.go:footnoteASTTransformer.Transform
    fn transform<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        _reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) {
        let list = pc.get_as::<NodeId>(*FOOTNOTE_LIST_KEY).copied();
        let fnlist = pc.get_as::<Vec<NodeId>>(*FOOTNOTE_LINK_LIST_KEY).cloned();

        pc.set(*FOOTNOTE_LIST_KEY, None);
        pc.set(*FOOTNOTE_LINK_LIST_KEY, None);

        let Some(list) = list else {
            return;
        };

        // Go: map[int]int (only looked up, never iterated)
        let mut counter: HashMap<i64, i64> = HashMap::new();
        if let Some(fnlist) = &fnlist {
            for &fnlink in fnlist {
                let idx = east::must::<east::FootnoteLink>(ast, fnlink, "FootnoteLink").index;
                if idx >= 0 {
                    *counter.entry(idx).or_insert(0) += 1;
                }
            }
            let mut ref_counter: HashMap<i64, i64> = HashMap::new();
            for &fnlink in fnlist {
                let l = east::must_mut::<east::FootnoteLink>(ast, fnlink, "FootnoteLink");
                l.ref_count = counter.get(&l.index).copied().unwrap_or(0);
                let rc = ref_counter.entry(l.index).or_insert(0);
                l.ref_index = *rc;
                *rc += 1;
            }
        }
        let mut footnote = ast.first_child(list);
        while let Some(f) = footnote {
            let mut container = f;
            let next = ast.next_sibling(f);
            if let Some(fc) = ast.last_child(container)
                && ast.is_paragraph(Some(fc))
            {
                container = fc;
            }
            let index = east::must::<east::Footnote>(ast, f, "Footnote").index;
            if index < 0 {
                ast.remove_child(list, f);
            } else {
                let ref_count = counter.get(&index).copied().unwrap_or(0);
                let back_link = east::new_footnote_backlink(ast, index);
                {
                    let b = east::must_mut::<east::FootnoteBacklink>(
                        ast,
                        back_link,
                        "FootnoteBacklink",
                    );
                    b.ref_count = ref_count;
                    b.ref_index = 0;
                }
                ast.append_child(container, back_link);
                if ref_count > 1 {
                    for i in 1..ref_count {
                        let back_link = east::new_footnote_backlink(ast, index);
                        {
                            let b = east::must_mut::<east::FootnoteBacklink>(
                                ast,
                                back_link,
                                "FootnoteBacklink",
                            );
                            b.ref_count = ref_count;
                            b.ref_index = i;
                        }
                        ast.append_child(container, back_link);
                    }
                }
            }
            footnote = next;
        }
        ast.sort_children(list, |ast, n1, n2| {
            if east::must::<east::Footnote>(ast, n1, "Footnote").index
                < east::must::<east::Footnote>(ast, n2, "Footnote").index
            {
                return -1;
            }
            1
        });
        if east::must::<east::FootnoteList>(ast, list, "FootnoteList").count <= 0 {
            let p = ast
                .parent(list)
                .expect("invalid memory address or nil pointer dereference");
            ast.remove_child(p, list);
            return;
        }

        ast.append_child(node, list);
    }
}

/// Go: `func(gast.Node) []byte` (IDPrefixFunction); `None` is a nil result.
pub type IDPrefixFunction = Arc<dyn Fn(&Ast, NodeId) -> Option<Vec<u8>> + Send + Sync>;

/// FootnoteConfig holds configuration values for the footnote extension.
///
/// Link* and Backlink* configurations have some variables:
/// Occurrences of “^^” in the string will be replaced by the
/// corresponding footnote number in the HTML output.
/// Occurrences of “%%” will be replaced by a number for the
/// reference (footnotes can have multiple references).
#[derive(Clone)]
pub struct FootnoteConfig {
    pub config: html::Config,

    /// IDPrefix is a prefix for the id attributes generated by footnotes
    /// (`None` is Go's nil).
    pub id_prefix: Option<Vec<u8>>,

    /// IDPrefix is a function that determines the id attribute for given Node.
    pub id_prefix_function: Option<IDPrefixFunction>,

    /// LinkTitle is an optional title attribute for footnote links.
    pub link_title: Vec<u8>,

    /// BacklinkTitle is an optional title attribute for footnote backlinks.
    pub backlink_title: Vec<u8>,

    /// LinkClass is a class for footnote links.
    pub link_class: Vec<u8>,

    /// BacklinkClass is a class for footnote backlinks.
    pub backlink_class: Vec<u8>,

    /// BacklinkHTML is an HTML content for footnote backlinks.
    pub backlink_html: Vec<u8>,
}

// Go: extension/footnote.go:NewFootnoteConfig
/// NewFootnoteConfig returns a new Config with defaults.
pub fn new_footnote_config() -> FootnoteConfig {
    FootnoteConfig {
        config: html::new_config(),
        id_prefix: None,
        id_prefix_function: None,
        link_title: b"".to_vec(),
        backlink_title: b"".to_vec(),
        link_class: b"footnote-ref".to_vec(),
        backlink_class: b"footnote-backref".to_vec(),
        backlink_html: b"&#x21a9;&#xfe0e;".to_vec(),
    }
}

const OPT_FOOTNOTE_ID_PREFIX: &str = "FootnoteIDPrefix";
const OPT_FOOTNOTE_ID_PREFIX_FUNCTION: &str = "FootnoteIDPrefixFunction";
const OPT_FOOTNOTE_LINK_TITLE: &str = "FootnoteLinkTitle";
const OPT_FOOTNOTE_BACKLINK_TITLE: &str = "FootnoteBacklinkTitle";
const OPT_FOOTNOTE_LINK_CLASS: &str = "FootnoteLinkClass";
const OPT_FOOTNOTE_BACKLINK_CLASS: &str = "FootnoteBacklinkClass";
const OPT_FOOTNOTE_BACKLINK_HTML: &str = "FootnoteBacklinkHTML";

fn bytes_option(value: &OptionValue) -> Vec<u8> {
    value
        .downcast_ref::<Vec<u8>>()
        .expect("interface conversion: not []uint8")
        .clone()
}

impl FootnoteConfig {
    // Go: extension/footnote.go:FootnoteConfig.SetOption
    /// SetOption implements renderer.SetOptioner.
    pub fn set_option(&mut self, name: &str, value: &OptionValue) {
        match name {
            OPT_FOOTNOTE_ID_PREFIX_FUNCTION => {
                self.id_prefix_function = Some(
                    value
                        .downcast_ref::<IDPrefixFunction>()
                        .expect("interface conversion: not func(ast.Node) []uint8")
                        .clone(),
                )
            }
            OPT_FOOTNOTE_ID_PREFIX => self.id_prefix = Some(bytes_option(value)),
            OPT_FOOTNOTE_LINK_TITLE => self.link_title = bytes_option(value),
            OPT_FOOTNOTE_BACKLINK_TITLE => self.backlink_title = bytes_option(value),
            OPT_FOOTNOTE_LINK_CLASS => self.link_class = bytes_option(value),
            OPT_FOOTNOTE_BACKLINK_CLASS => self.backlink_class = bytes_option(value),
            OPT_FOOTNOTE_BACKLINK_HTML => self.backlink_html = bytes_option(value),
            _ => self.config.set_option(name, value),
        }
    }
}

/// FootnoteOption interface is a functional option interface for the extension.
#[derive(Clone)]
pub enum FootnoteOption {
    /// Go: WithFootnoteHTMLOptions.
    HtmlOptions(Vec<Arc<dyn HtmlRendererOption>>),
    /// Go: WithFootnoteIDPrefix.
    IDPrefix(Vec<u8>),
    /// Go: WithFootnoteIDPrefixFunction.
    IDPrefixFunction(IDPrefixFunction),
    /// Go: WithFootnoteLinkTitle.
    LinkTitle(Vec<u8>),
    /// Go: WithFootnoteBacklinkTitle.
    BacklinkTitle(Vec<u8>),
    /// Go: WithFootnoteLinkClass.
    LinkClass(Vec<u8>),
    /// Go: WithFootnoteBacklinkClass.
    BacklinkClass(Vec<u8>),
    /// Go: WithFootnoteBacklinkHTML.
    BacklinkHTML(Vec<u8>),
}

impl FootnoteOption {
    /// SetFootnoteOption sets given option to the extension.
    pub fn set_footnote_option(&self, c: &mut FootnoteConfig) {
        match self {
            FootnoteOption::HtmlOptions(v) => {
                for o in v {
                    o.set_html_option(&mut c.config);
                }
            }
            FootnoteOption::IDPrefix(v) => c.id_prefix = Some(v.clone()),
            FootnoteOption::IDPrefixFunction(f) => c.id_prefix_function = Some(f.clone()),
            FootnoteOption::LinkTitle(v) => c.link_title = v.clone(),
            FootnoteOption::BacklinkTitle(v) => c.backlink_title = v.clone(),
            FootnoteOption::LinkClass(v) => c.link_class = v.clone(),
            FootnoteOption::BacklinkClass(v) => c.backlink_class = v.clone(),
            FootnoteOption::BacklinkHTML(v) => c.backlink_html = v.clone(),
        }
    }
}

impl RendererOption for FootnoteOption {
    fn set_config(self: Box<Self>, c: &mut renderer::Config) {
        let (name, value): (&str, OptionValue) = match *self {
            FootnoteOption::HtmlOptions(v) => {
                for o in v {
                    o.set_renderer_config(c);
                }
                return;
            }
            FootnoteOption::IDPrefix(v) => (OPT_FOOTNOTE_ID_PREFIX, Arc::new(v)),
            FootnoteOption::IDPrefixFunction(f) => (OPT_FOOTNOTE_ID_PREFIX_FUNCTION, Arc::new(f)),
            FootnoteOption::LinkTitle(v) => (OPT_FOOTNOTE_LINK_TITLE, Arc::new(v)),
            FootnoteOption::BacklinkTitle(v) => (OPT_FOOTNOTE_BACKLINK_TITLE, Arc::new(v)),
            FootnoteOption::LinkClass(v) => (OPT_FOOTNOTE_LINK_CLASS, Arc::new(v)),
            FootnoteOption::BacklinkClass(v) => (OPT_FOOTNOTE_BACKLINK_CLASS, Arc::new(v)),
            FootnoteOption::BacklinkHTML(v) => (OPT_FOOTNOTE_BACKLINK_HTML, Arc::new(v)),
        };
        c.options.insert(name.to_string(), value);
    }
}

// Go: extension/footnote.go:WithFootnoteHTMLOptions
/// WithFootnoteHTMLOptions is functional option that wraps goldmark HTMLRenderer options.
pub fn with_footnote_html_options(opts: Vec<Arc<dyn HtmlRendererOption>>) -> FootnoteOption {
    FootnoteOption::HtmlOptions(opts)
}

// Go: extension/footnote.go:WithFootnoteIDPrefix
/// WithFootnoteIDPrefix is a functional option that is a prefix for the id attributes generated by footnotes.
pub fn with_footnote_id_prefix(a: impl AsRef<[u8]>) -> FootnoteOption {
    FootnoteOption::IDPrefix(a.as_ref().to_vec())
}

// Go: extension/footnote.go:WithFootnoteIDPrefixFunction
/// WithFootnoteIDPrefixFunction is a functional option that is a prefix for the id attributes generated by footnotes.
pub fn with_footnote_id_prefix_function(a: IDPrefixFunction) -> FootnoteOption {
    FootnoteOption::IDPrefixFunction(a)
}

// Go: extension/footnote.go:WithFootnoteLinkTitle
/// WithFootnoteLinkTitle is a functional option that is an optional title attribute for footnote links.
pub fn with_footnote_link_title(a: impl AsRef<[u8]>) -> FootnoteOption {
    FootnoteOption::LinkTitle(a.as_ref().to_vec())
}

// Go: extension/footnote.go:WithFootnoteBacklinkTitle
/// WithFootnoteBacklinkTitle is a functional option that is an optional title attribute for footnote backlinks.
pub fn with_footnote_backlink_title(a: impl AsRef<[u8]>) -> FootnoteOption {
    FootnoteOption::BacklinkTitle(a.as_ref().to_vec())
}

// Go: extension/footnote.go:WithFootnoteLinkClass
/// WithFootnoteLinkClass is a functional option that is a class for footnote links.
pub fn with_footnote_link_class(a: impl AsRef<[u8]>) -> FootnoteOption {
    FootnoteOption::LinkClass(a.as_ref().to_vec())
}

// Go: extension/footnote.go:WithFootnoteBacklinkClass
/// WithFootnoteBacklinkClass is a functional option that is a class for footnote backlinks.
pub fn with_footnote_backlink_class(a: impl AsRef<[u8]>) -> FootnoteOption {
    FootnoteOption::BacklinkClass(a.as_ref().to_vec())
}

// Go: extension/footnote.go:WithFootnoteBacklinkHTML
/// WithFootnoteBacklinkHTML is an HTML content for footnote backlinks.
pub fn with_footnote_backlink_html(a: impl AsRef<[u8]>) -> FootnoteOption {
    FootnoteOption::BacklinkHTML(a.as_ref().to_vec())
}

/// FootnoteHTMLRenderer is a renderer.NodeRenderer implementation that
/// renders FootnoteLink nodes.
pub struct FootnoteHTMLRenderer {
    pub footnote_config: FootnoteConfig,
}

// Go: extension/footnote.go:NewFootnoteHTMLRenderer
/// NewFootnoteHTMLRenderer returns a new FootnoteHTMLRenderer.
pub fn new_footnote_html_renderer(opts: &[FootnoteOption]) -> Box<dyn NodeRenderer> {
    let mut r = FootnoteHTMLRenderer {
        footnote_config: new_footnote_config(),
    };
    for opt in opts {
        opt.set_footnote_option(&mut r.footnote_config);
    }
    Box::new(r)
}

impl NodeRenderer for FootnoteHTMLRenderer {
    // Go: extension/footnote.go:FootnoteHTMLRenderer.RegisterFuncs
    fn register_funcs(self: Arc<Self>, reg: &mut dyn NodeRendererFuncRegisterer) {
        reg.register(
            *east::KIND_FOOTNOTE_LINK,
            html::bind(&self, FootnoteHTMLRenderer::render_footnote_link),
        );
        reg.register(
            *east::KIND_FOOTNOTE_BACKLINK,
            html::bind(&self, FootnoteHTMLRenderer::render_footnote_backlink),
        );
        reg.register(
            *east::KIND_FOOTNOTE,
            html::bind(&self, FootnoteHTMLRenderer::render_footnote),
        );
        reg.register(
            *east::KIND_FOOTNOTE_LIST,
            html::bind(&self, FootnoteHTMLRenderer::render_footnote_list),
        );
    }

    // Go: FootnoteConfig.SetOption (promoted)
    fn set_option(&mut self, name: &str, value: &OptionValue) {
        self.footnote_config.set_option(name, value);
    }
}

type R = Result<WalkStatus, crate::Error>;

impl FootnoteHTMLRenderer {
    // Go: extension/footnote.go:FootnoteHTMLRenderer.renderFootnoteLink
    fn render_footnote_link(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            let n = east::must::<east::FootnoteLink>(ast, node, "FootnoteLink");
            let is = go_strconv::itoa(n.index);
            w.write_string("<sup id=\"");
            w.write(&self.id_prefix(ast, node));
            w.write_string("fnref");
            if n.ref_index > 0 {
                w.write_string(&go_strconv::itoa(n.ref_index));
            }
            w.write_byte(b':');
            w.write_string(&is);
            w.write_string("\"><a href=\"#");
            w.write(&self.id_prefix(ast, node));
            w.write_string("fn:");
            w.write_string(&is);
            w.write_string("\" class=\"");
            w.write(&apply_footnote_template(
                &self.footnote_config.link_class,
                n.index,
                n.ref_count,
            ));
            if !self.footnote_config.link_title.is_empty() {
                w.write_string("\" title=\"");
                w.write(&util::escape_html(&apply_footnote_template(
                    &self.footnote_config.link_title,
                    n.index,
                    n.ref_count,
                )));
            }
            w.write_string("\" role=\"doc-noteref\">");

            w.write_string(&is);
            w.write_string("</a></sup>");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: extension/footnote.go:FootnoteHTMLRenderer.renderFootnoteBacklink
    fn render_footnote_backlink(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            let n = east::must::<east::FootnoteBacklink>(ast, node, "FootnoteBacklink");
            let is = go_strconv::itoa(n.index);
            w.write_string("&#160;<a href=\"#");
            w.write(&self.id_prefix(ast, node));
            w.write_string("fnref");
            if n.ref_index > 0 {
                w.write_string(&go_strconv::itoa(n.ref_index));
            }
            w.write_byte(b':');
            w.write_string(&is);
            w.write_string("\" class=\"");
            w.write(&apply_footnote_template(
                &self.footnote_config.backlink_class,
                n.index,
                n.ref_count,
            ));
            if !self.footnote_config.backlink_title.is_empty() {
                w.write_string("\" title=\"");
                w.write(&util::escape_html(&apply_footnote_template(
                    &self.footnote_config.backlink_title,
                    n.index,
                    n.ref_count,
                )));
            }
            w.write_string("\" role=\"doc-backlink\">");
            w.write(&apply_footnote_template(
                &self.footnote_config.backlink_html,
                n.index,
                n.ref_count,
            ));
            w.write_string("</a>");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: extension/footnote.go:FootnoteHTMLRenderer.renderFootnote
    fn render_footnote(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = east::must::<east::Footnote>(ast, node, "Footnote");
        let is = go_strconv::itoa(n.index);
        if entering {
            w.write_string("<li id=\"");
            w.write(&self.id_prefix(ast, node));
            w.write_string("fn:");
            w.write_string(&is);
            w.write_string("\"");
            if ast.attributes(node).is_some() {
                html::render_attributes(w, ast, node, Some(&html::LIST_ITEM_ATTRIBUTE_FILTER));
            }
            w.write_string(">\n");
        } else {
            w.write_string("</li>\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: extension/footnote.go:FootnoteHTMLRenderer.renderFootnoteList
    fn render_footnote_list(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            w.write_string("<div class=\"footnotes\" role=\"doc-endnotes\"");
            if ast.attributes(node).is_some() {
                html::render_attributes(w, ast, node, Some(&html::GLOBAL_ATTRIBUTE_FILTER));
            }
            w.write_byte(b'>');
            if self.footnote_config.config.xhtml {
                w.write_string("\n<hr />\n");
            } else {
                w.write_string("\n<hr>\n");
            }
            w.write_string("<ol>\n");
        } else {
            w.write_string("</ol>\n");
            w.write_string("</div>\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: extension/footnote.go:FootnoteHTMLRenderer.idPrefix
    fn id_prefix(&self, ast: &Ast, node: NodeId) -> Vec<u8> {
        if let Some(p) = &self.footnote_config.id_prefix {
            return p.clone();
        }
        if let Some(f) = &self.footnote_config.id_prefix_function {
            return f(ast, node).unwrap_or_default();
        }
        b"".to_vec()
    }
}

// Go: extension/footnote.go:applyFootnoteTemplate
fn apply_footnote_template(b: &[u8], index: i64, ref_count: i64) -> Vec<u8> {
    let mut fast = true;
    for (i, &c) in b.iter().enumerate() {
        if i != 0 {
            if b[i - 1] == b'^' && c == b'^' {
                fast = false;
                break;
            }
            if b[i - 1] == b'%' && c == b'%' {
                fast = false;
                break;
            }
        }
    }
    if fast {
        return b.to_vec();
    }
    let is = go_strconv::itoa(index);
    let rs = go_strconv::itoa(ref_count);
    let ret = go_unicode::bytes::replace_all(b, b"^^", is.as_bytes());
    go_unicode::bytes::replace_all(&ret, b"%%", rs.as_bytes())
}

/// Go: `type footnote struct{ options []FootnoteOption }`.
pub struct FootnoteExt {
    options: Vec<FootnoteOption>,
}

// Go: extension/footnote.go:Footnote
/// Footnote is an extension that allow you to use PHP Markdown Extra Footnotes.
pub fn footnote() -> Box<dyn Extender> {
    Box::new(FootnoteExt {
        options: Vec::new(),
    })
}

// Go: extension/footnote.go:NewFootnote
/// NewFootnote returns a new extension with given options.
pub fn new_footnote(opts: Vec<FootnoteOption>) -> Box<dyn Extender> {
    Box::new(FootnoteExt { options: opts })
}

impl Extender for FootnoteExt {
    // Go: extension/footnote.go:footnote.Extend
    fn extend(&self, m: &mut Markdown) {
        m.parser().add_options(vec![
            parser::with_block_parsers(vec![util::prioritized(new_footnote_block_parser(), 999)]),
            parser::with_inline_parsers(vec![util::prioritized(new_footnote_parser(), 101)]),
            parser::with_ast_transformers(vec![util::prioritized(
                new_footnote_ast_transformer(),
                999,
            )]),
        ]);
        m.renderer()
            .add_options(vec![renderer::with_node_renderers(vec![
                util::prioritized(new_footnote_html_renderer(&self.options), 500),
            ])]);
    }
}

#[cfg(test)]
mod tests {
    use super::apply_footnote_template;

    #[test]
    fn template() {
        assert_eq!(apply_footnote_template(b"a^b%c", 1, 2), b"a^b%c");
        assert_eq!(apply_footnote_template(b"t-%%-^^", 3, 2), b"t-2-3");
        assert_eq!(apply_footnote_template(b"^^^", 7, 1), b"7^");
        assert_eq!(apply_footnote_template(b"%%%%", 7, 12), b"1212");
    }
}
