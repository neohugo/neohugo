//! Port of `markup/goldmark/internal/extensions/attributes/attributes.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `markup/goldmark/internal/extensions/attributes`: the AST transformer that assigns
//! auto heading IDs (`generateAutoID` via `render.TextPlain` + `util.ResolveEntityNames`, last line
//! of multi-line setext headings) and handles `{#id .class}` attribute blocks.

use std::sync::LazyLock;

use goldmark::ast::{self, Ast, AttrValue, CustomNode, NodeId, NodeKind, NodeType, WalkStatus};
use goldmark::parser::{self, BlockParser, Context, State};
use goldmark::text::Reader;
use goldmark::util;
use goldmark::{Extender, Markdown};

use crate::goldmark::goldmark_config::Parser as ParserConfig;
use crate::goldmark::internal::render;

// This extension is based on/inspired by https://github.com/mdigger/goldmark-attributes
// MIT License
// Copyright (c) 2019 Dmitry Sedykh

static KIND_ATTRIBUTES_BLOCK: LazyLock<NodeKind> =
    LazyLock::new(|| ast::new_node_kind("AttributesBlock"));
const ATTR_NAME_ID: &[u8] = b"id";

// Go: markup/goldmark/internal/extensions/attributes/attributes.go:New
pub fn new(cfg: ParserConfig) -> Box<dyn Extender> {
    Box::new(AttrExtension { cfg })
}

struct AttrExtension {
    cfg: ParserConfig,
}

impl Extender for AttrExtension {
    // Go: markup/goldmark/internal/extensions/attributes/attributes.go:Extend
    fn extend(&self, m: &mut Markdown) {
        if self.cfg.attribute.block {
            m.parser()
                .add_options(vec![parser::with_block_parsers(vec![util::prioritized(
                    Box::new(AttrParser) as Box<dyn BlockParser>,
                    100,
                )])]);
        }
        m.parser()
            .add_options(vec![parser::with_ast_transformers(vec![
                util::prioritized(
                    Box::new(Transformer {
                        cfg: self.cfg.clone(),
                    }) as Box<dyn parser::AstTransformer>,
                    100,
                ),
            ])]);
    }
}

struct AttrParser;

/// Go: `attributesBlock` (an `ast.BaseBlock` holding the parsed attributes).
#[derive(Debug)]
struct AttributesBlock;

impl CustomNode for AttributesBlock {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl BlockParser for AttrParser {
    // Go: markup/goldmark/internal/extensions/attributes/attributes.go:Trigger
    fn trigger(&self) -> Option<&[u8]> {
        Some(b"{")
    }

    // Go: markup/goldmark/internal/extensions/attributes/attributes.go:Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        if let Some(attrs) = parser::parse_attributes(reader) {
            // add attributes
            let node = ast.new_custom_node(
                *KIND_ATTRIBUTES_BLOCK,
                NodeType::Block,
                Box::new(AttributesBlock),
            );
            for attr in attrs {
                ast.set_attribute(node, &attr.name, attr.value.clone());
            }
            return (Some(node), State::NO_CHILDREN);
        }
        (None, State::REQUIRE_PARAGRAPH)
    }

    // Go: markup/goldmark/internal/extensions/attributes/attributes.go:Continue
    fn continue_<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> State {
        State::CLOSE
    }

    // Go: markup/goldmark/internal/extensions/attributes/attributes.go:Close
    fn close<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
    }

    // Go: markup/goldmark/internal/extensions/attributes/attributes.go:CanInterruptParagraph
    fn can_interrupt_paragraph(&self) -> bool {
        true
    }

    // Go: markup/goldmark/internal/extensions/attributes/attributes.go:CanAcceptIndentedLine
    fn can_accept_indented_line(&self) -> bool {
        false
    }
}

struct Transformer {
    cfg: ParserConfig,
}

impl Transformer {
    // Go: markup/goldmark/internal/extensions/attributes/attributes.go:isFragmentNode
    fn is_fragment_node(&self, ast: &Ast, n: NodeId) -> bool {
        let k = ast.kind(n);
        k == *goldmark::extension::ast::KIND_DEFINITION_TERM || k == ast::KIND_HEADING
    }

    // Go: markup/goldmark/internal/extensions/attributes/attributes.go:generateAutoID
    fn generate_auto_id<'a>(
        &self,
        ast: &mut Ast,
        n: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) {
        let mut text: Vec<u8> = Vec::new();
        if ast.kind(n) == ast::KIND_HEADING {
            if self.cfg.auto_heading_id {
                text = text_heading_id(ast, n, reader);
            }
        } else if ast.kind(n) == *goldmark::extension::ast::KIND_DEFINITION_TERM
            && self.cfg.auto_definition_term_id
        {
            text = render::text_plain(ast, n, reader.source());
        }

        if !text.is_empty() {
            let heading_id = pc.ids().generate(&text, ast.kind(n));
            ast.set_attribute(n, ATTR_NAME_ID, AttrValue::Bytes(heading_id));
        }
    }
}

impl parser::AstTransformer for Transformer {
    // Go: markup/goldmark/internal/extensions/attributes/attributes.go:Transform
    fn transform<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) {
        let mut attributes: Vec<NodeId> = Vec::new();
        let mut solitary_attribute_nodes: Vec<NodeId> = Vec::new();
        let _ = ast::walk(ast, node, &mut |ast, node, entering| {
            if !entering {
                return Ok(WalkStatus::Continue);
            }

            if self.is_fragment_node(ast, node) {
                match ast.attribute(node, ATTR_NAME_ID) {
                    None => self.generate_auto_id(ast, node, reader, pc),
                    Some(id) => {
                        // Go: `id.([]byte)` panics for any other value type.
                        let id = match id {
                            AttrValue::Bytes(b) => b.clone(),
                            other => panic!(
                                "interface conversion: interface {{}} is {}, not []uint8",
                                other.go_type_name()
                            ),
                        };
                        pc.ids().put(&id);
                    }
                }
            }

            if self.cfg.attribute.block && ast.kind(node) == *KIND_ATTRIBUTES_BLOCK {
                // Attributes for fenced code blocks are handled in their own extension,
                // but note that we currently only support code block attributes when
                // CodeFences=true.
                if let Some(prev) = ast.previous_sibling(node)
                    && ast.kind(prev) != ast::KIND_FENCED_CODE_BLOCK
                    && !ast.has_blank_previous_lines(node)
                {
                    attributes.push(node);
                    return Ok(WalkStatus::SkipChildren);
                } else {
                    solitary_attribute_nodes.push(node);
                }
            }

            Ok(WalkStatus::Continue)
        });

        for attr in attributes {
            if let Some(prev) = ast.previous_sibling(attr)
                && ast.typ(prev) == NodeType::Block
            {
                let list: Vec<ast::Attribute> =
                    ast.attributes(attr).map(|a| a.to_vec()).unwrap_or_default();
                for a in list {
                    if ast.attribute(prev, &a.name).is_none() {
                        ast.set_attribute(prev, &a.name, a.value.clone());
                    }
                }
            }
            // remove attributes node
            let parent = ast.parent(attr).expect("attributes block parent");
            ast.remove_child(parent, attr);
        }

        // Remove any solitary attribute nodes.
        for n in solitary_attribute_nodes {
            let parent = ast.parent(n).expect("attributes block parent");
            ast.remove_child(parent, n);
        }
    }
}

/// Markdown settext headers can have multiple lines, use the last line for the ID.
// Go: markup/goldmark/internal/extensions/attributes/attributes.go:textHeadingID
fn text_heading_id<'a>(ast: &Ast, n: NodeId, reader: &mut dyn Reader<'a>) -> Vec<u8> {
    let text = render::text_plain(ast, n, reader.source());
    if ast.lines(n).len() > 1 {
        // For multiline headings, Goldmark's extension for headings returns the last line.
        // We have a slightly different approach, but in most cases the end result should be the
        // same. Instead of looking at the text segments in Lines (see #13405 for issues with
        // that), we split the text above and use the last line.
        return match text.iter().rposition(|&c| c == b'\n') {
            Some(i) => text[i + 1..].to_vec(),
            None => text,
        };
    }
    text
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/internal/extensions/attributes/attributes.go (204 lines; 6/14 funcs executed)
//   types: attrExtension, attrParser, attributesBlock, transformer
// OK L27-29: New(cfg goldmark_config.Parser) goldmark.Extender
// OK L35-47: (a *attrExtension) Extend(m goldmark.Markdown)
// OK L51-53: (a *attrParser) CanAcceptIndentedLine() bool
// OK L55-57: (a *attrParser) CanInterruptParagraph() bool
// OK L59-60: (a *attrParser) Close(node ast.Node, reader text.Reader, pc parser.Context)
// OK L62-64: (a *attrParser) Continue(node ast.Node, reader text.Reader, pc parser.Context) parser.State
// OK L66-78: (a *attrParser) Open(parent ast.Node, reader text.Reader, pc parser.Context) (ast.Node, parser.State)
// OK L80-82: (a *attrParser) Trigger() []byte
//    L88-99: (a *attributesBlock) Dump(source []byte, level int) (debug only; not ported)
// OK L101-103: (a *attributesBlock) Kind() ast.NodeKind
// OK L109-116: (a *transformer) isFragmentNode(n ast.Node) bool
// OK L118-169: (a *transformer) Transform(node *ast.Document, reader text.Reader, pc parser.Context)
// OK L171-188: (a *transformer) generateAutoID(n ast.Node, reader text.Reader, pc parser.Context)
// OK L191-204: textHeadingID(n *ast.Heading, reader text.Reader) []byte
// ---------------------------------------------------------------------------
