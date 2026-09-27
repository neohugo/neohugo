// Go: github.com/yuin/goldmark@v1.7.12/extension/definition_list.go

use std::sync::{Arc, LazyLock};

use super::ast as east;
use crate::ast::{Ast, NodeId, NodeValue, WalkStatus};
use crate::parser::{self, BlockParser, Context, OptionValue, State};
use crate::renderer::html::{self, HtmlOption};
use crate::renderer::{self, NodeRenderer, NodeRendererFuncRegisterer};
use crate::text::Reader;
use crate::util::{self, BufWriter, BytesFilter};
use crate::{Extender, Markdown};

struct DefinitionListParser;

// Go: extension/definition_list.go:NewDefinitionListParser
/// NewDefinitionListParser return a new parser.BlockParser that
/// can parse PHP Markdown Extra Definition lists.
pub fn new_definition_list_parser() -> Box<dyn BlockParser> {
    Box::new(DefinitionListParser)
}

impl BlockParser for DefinitionListParser {
    // Go: extension/definition_list.go:definitionListParser.Trigger
    fn trigger(&self) -> Option<&[u8]> {
        Some(b":")
    }

    // Go: extension/definition_list.go:definitionListParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        parent: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        if ast.custom::<east::DefinitionList>(parent).is_some() {
            return (None, State::NO_CHILDREN);
        }
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        let pos = pc.block_offset();
        let indent = pc.block_indent();
        if pos < 0 || line[pos as usize] != b':' || indent != 0 {
            return (None, State::NO_CHILDREN);
        }

        let last = ast.last_child(parent);
        // need 1 or more spaces after ':'
        let (mut w, _) = util::indent_width(&line[(pos + 1) as usize..], pos + 1);
        if w < 1 {
            return (None, State::NO_CHILDREN);
        }
        if w >= 8 {
            // starts with indented code
            w = 5;
        }
        w += pos + 1; /* 1 = ':' */

        let para = last.filter(|&l| matches!(ast.value(l), NodeValue::Paragraph));
        let list;
        let mut status = State::HAS_CHILDREN;
        if let Some(para) = para {
            let prev = ast.previous_sibling(para);
            match prev.filter(|&p| ast.custom::<east::DefinitionList>(p).is_some()) {
                Some(l) => {
                    // is not first item
                    let d = ast.custom_mut::<east::DefinitionList>(l).unwrap();
                    d.offset = w;
                    d.temporary_paragraph = Some(para);
                    list = l;
                }
                None => {
                    // is first item
                    list = east::new_definition_list(ast, w, Some(para));
                    status = status | State::REQUIRE_PARAGRAPH;
                }
            }
        } else if let Some(l) = last.filter(|&l| ast.custom::<east::DefinitionList>(l).is_some()) {
            // multiple description
            let d = ast.custom_mut::<east::DefinitionList>(l).unwrap();
            d.offset = w;
            d.temporary_paragraph = None;
            list = l;
        } else {
            return (None, State::NO_CHILDREN);
        }

        (Some(list), status)
    }

    // Go: extension/definition_list.go:definitionListParser.Continue
    fn continue_<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> State {
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        if util::is_blank(&line) {
            return State::CONTINUE | State::HAS_CHILDREN;
        }
        let offset = east::must::<east::DefinitionList>(ast, node, "DefinitionList").offset;
        let lo = reader.line_offset();
        let (w, _) = util::indent_width(&line, lo);
        if w < offset {
            return State::CLOSE;
        }
        let lo = reader.line_offset();
        let (pos, padding) = util::indent_position(&line, lo, offset);
        reader.advance_and_set_padding(pos, padding);
        State::CONTINUE | State::HAS_CHILDREN
    }

    // Go: extension/definition_list.go:definitionListParser.Close
    fn close<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
        // nothing to do
    }

    // Go: extension/definition_list.go:definitionListParser.CanInterruptParagraph
    fn can_interrupt_paragraph(&self) -> bool {
        true
    }

    // Go: extension/definition_list.go:definitionListParser.CanAcceptIndentedLine
    fn can_accept_indented_line(&self) -> bool {
        false
    }
}

struct DefinitionDescriptionParser;

// Go: extension/definition_list.go:NewDefinitionDescriptionParser
/// NewDefinitionDescriptionParser return a new parser.BlockParser that
/// can parse definition description starts with ':'.
pub fn new_definition_description_parser() -> Box<dyn BlockParser> {
    Box::new(DefinitionDescriptionParser)
}

impl BlockParser for DefinitionDescriptionParser {
    // Go: extension/definition_list.go:definitionDescriptionParser.Trigger
    fn trigger(&self) -> Option<&[u8]> {
        Some(b":")
    }

    // Go: extension/definition_list.go:definitionDescriptionParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        parent: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        let pos = pc.block_offset();
        let indent = pc.block_indent();
        if pos < 0 || line[pos as usize] != b':' || indent != 0 {
            return (None, State::NO_CHILDREN);
        }
        let Some(list) = ast.custom_mut::<east::DefinitionList>(parent) else {
            return (None, State::NO_CHILDREN);
        };
        let para = list.temporary_paragraph.take();
        let list_offset = list.offset;
        let list = parent;
        if let Some(para) = para {
            let lines = ast.lines(para).clone();
            let l = lines.len();
            for i in 0..l {
                let term = east::new_definition_term(ast);
                let segment = lines.at(i);
                let seg = segment.trim_right_space(reader.source());
                ast.lines_mut(term).append(seg);
                ast.append_child(list, term);
            }
            let pp = ast.parent(para).expect("temporary paragraph has a parent");
            ast.remove_child(pp, para);
        }
        let (cpos, padding) =
            util::indent_position(&line[(pos + 1) as usize..], pos + 1, list_offset - pos - 1);
        reader.advance_and_set_padding(cpos + 1, padding);

        (
            Some(east::new_definition_description(ast)),
            State::HAS_CHILDREN,
        )
    }

    // Go: extension/definition_list.go:definitionDescriptionParser.Continue
    fn continue_<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> State {
        // definitionListParser detects end of the description.
        // so this method will never be called.
        State::CONTINUE | State::HAS_CHILDREN
    }

    // Go: extension/definition_list.go:definitionDescriptionParser.Close
    fn close<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
        let is_tight = !ast.has_blank_previous_lines(node);
        east::must_mut::<east::DefinitionDescription>(ast, node, "DefinitionDescription")
            .is_tight = is_tight;
        if is_tight {
            let mut gc = ast.first_child(node);
            while let Some(g) = gc {
                if matches!(ast.value(g), NodeValue::Paragraph) {
                    let text_block = ast.new_text_block();
                    let lines = ast.lines(g).clone();
                    ast.set_lines(text_block, &lines);
                    ast.replace_child(node, g, text_block);
                }
                // Go reads gc.NextSibling() after the replacement: a
                // replaced paragraph is detached, so the loop stops there.
                gc = ast.next_sibling(g);
            }
        }
    }

    // Go: extension/definition_list.go:definitionDescriptionParser.CanInterruptParagraph
    fn can_interrupt_paragraph(&self) -> bool {
        true
    }

    // Go: extension/definition_list.go:definitionDescriptionParser.CanAcceptIndentedLine
    fn can_accept_indented_line(&self) -> bool {
        false
    }
}

/// DefinitionListHTMLRenderer is a renderer.NodeRenderer implementation that
/// renders DefinitionList nodes.
pub struct DefinitionListHTMLRenderer {
    pub config: html::Config,
}

// Go: extension/definition_list.go:NewDefinitionListHTMLRenderer
/// NewDefinitionListHTMLRenderer returns a new DefinitionListHTMLRenderer.
pub fn new_definition_list_html_renderer(opts: Vec<Box<dyn HtmlOption>>) -> Box<dyn NodeRenderer> {
    let mut r = DefinitionListHTMLRenderer {
        config: html::new_config(),
    };
    for opt in opts {
        opt.set_html_option(&mut r.config);
    }
    Box::new(r)
}

impl NodeRenderer for DefinitionListHTMLRenderer {
    // Go: extension/definition_list.go:DefinitionListHTMLRenderer.RegisterFuncs
    fn register_funcs(self: Arc<Self>, reg: &mut dyn NodeRendererFuncRegisterer) {
        reg.register(
            *east::KIND_DEFINITION_LIST,
            html::bind(&self, DefinitionListHTMLRenderer::render_definition_list),
        );
        reg.register(
            *east::KIND_DEFINITION_TERM,
            html::bind(&self, DefinitionListHTMLRenderer::render_definition_term),
        );
        reg.register(
            *east::KIND_DEFINITION_DESCRIPTION,
            html::bind(
                &self,
                DefinitionListHTMLRenderer::render_definition_description,
            ),
        );
    }

    // Go: the embedded html.Config's SetOption
    fn set_option(&mut self, name: &str, value: &OptionValue) {
        self.config.set_option(name, value);
    }
}

/// DefinitionListAttributeFilter defines attribute names which dl elements can have.
pub static DEFINITION_LIST_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| html::GLOBAL_ATTRIBUTE_FILTER.clone());

/// DefinitionTermAttributeFilter defines attribute names which dd elements can have.
pub static DEFINITION_TERM_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| html::GLOBAL_ATTRIBUTE_FILTER.clone());

/// DefinitionDescriptionAttributeFilter defines attribute names which dd elements can have.
pub static DEFINITION_DESCRIPTION_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| html::GLOBAL_ATTRIBUTE_FILTER.clone());

type R = Result<WalkStatus, crate::Error>;

impl DefinitionListHTMLRenderer {
    // Go: extension/definition_list.go:DefinitionListHTMLRenderer.renderDefinitionList
    fn render_definition_list(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            if ast.attributes(n).is_some() {
                w.write_string("<dl");
                html::render_attributes(w, ast, n, Some(&DEFINITION_LIST_ATTRIBUTE_FILTER));
                w.write_string(">\n");
            } else {
                w.write_string("<dl>\n");
            }
        } else {
            w.write_string("</dl>\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: extension/definition_list.go:DefinitionListHTMLRenderer.renderDefinitionTerm
    fn render_definition_term(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            if ast.attributes(n).is_some() {
                w.write_string("<dt");
                html::render_attributes(w, ast, n, Some(&DEFINITION_TERM_ATTRIBUTE_FILTER));
                w.write_byte(b'>');
            } else {
                w.write_string("<dt>");
            }
        } else {
            w.write_string("</dt>\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: extension/definition_list.go:DefinitionListHTMLRenderer.renderDefinitionDescription
    fn render_definition_description(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            let n = east::must::<east::DefinitionDescription>(ast, node, "DefinitionDescription");
            w.write_string("<dd");
            if ast.attributes(node).is_some() {
                html::render_attributes(
                    w,
                    ast,
                    node,
                    Some(&DEFINITION_DESCRIPTION_ATTRIBUTE_FILTER),
                );
            }
            if n.is_tight {
                w.write_string(">");
            } else {
                w.write_string(">\n");
            }
        } else {
            w.write_string("</dd>\n");
        }
        Ok(WalkStatus::Continue)
    }
}

/// Go: `type definitionList struct{}`.
pub struct DefinitionListExt;

// Go: extension/definition_list.go:DefinitionList
/// DefinitionList is an extension that allow you to use PHP Markdown Extra Definition lists.
pub fn definition_list() -> Box<dyn Extender> {
    Box::new(DefinitionListExt)
}

impl Extender for DefinitionListExt {
    // Go: extension/definition_list.go:definitionList.Extend
    fn extend(&self, m: &mut Markdown) {
        m.parser().add_options(vec![parser::with_block_parsers(vec![
            util::prioritized(new_definition_list_parser(), 101),
            util::prioritized(new_definition_description_parser(), 102),
        ])]);
        m.renderer()
            .add_options(vec![renderer::with_node_renderers(vec![
                util::prioritized(new_definition_list_html_renderer(Vec::new()), 500),
            ])]);
    }
}
