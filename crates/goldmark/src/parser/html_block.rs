// Go: github.com/yuin/goldmark@v1.7.12/parser/html_block.go

use super::regexps;
use super::{BlockParser, Context, State};
use crate::ast::{Ast, HTMLBlockType, NodeId};
use crate::text::Reader;
use crate::util;

static ALLOWED_BLOCK_TAGS: [&str; 63] = [
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "meta",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "search",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
];

fn allowed_block_tag(name: &[u8]) -> bool {
    ALLOWED_BLOCK_TAGS.iter().any(|t| t.as_bytes() == name)
}

const HTML_BLOCK_TYPE2_CLOSE: &[u8] = b"-->";
const HTML_BLOCK_TYPE3_CLOSE: &[u8] = b"?>";
const HTML_BLOCK_TYPE4_CLOSE: &[u8] = b">";
const HTML_BLOCK_TYPE5_CLOSE: &[u8] = b"]]>";

struct HTMLBlockParser;

// Go: parser/html_block.go:NewHTMLBlockParser
/// NewHTMLBlockParser return a new BlockParser that can parse html
/// blocks.
pub fn new_html_block_parser() -> Box<dyn BlockParser> {
    Box::new(HTMLBlockParser)
}

/// Go `strings.ToLower` of a tag name matched by `[a-zA-Z]+[a-zA-Z0-9\-]*`
/// (ASCII only).
fn to_lower(b: &[u8]) -> Vec<u8> {
    go_unicode::bytes::to_lower(b)
}

impl BlockParser for HTMLBlockParser {
    fn trigger(&self) -> Option<&[u8]> {
        Some(b"<")
    }

    // Go: parser/html_block.go:htmlBlockParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        let mut node: Option<NodeId> = None;
        let (line, segment) = reader.peek_line();
        let line = line.unwrap_or_default();
        let last = pc.last_opened_block().map(|b| b.node);
        let pos = pc.block_offset();
        if pos < 0 || line[pos as usize] != b'<' {
            return (None, State::NO_CHILDREN);
        }

        if regexps::html_block_type1_open(&line) {
            node = Some(ast.new_html_block(HTMLBlockType::Type1));
        } else if regexps::html_block_type2_open(&line) {
            node = Some(ast.new_html_block(HTMLBlockType::Type2));
        } else if regexps::html_block_type3_open(&line) {
            node = Some(ast.new_html_block(HTMLBlockType::Type3));
        } else if regexps::html_block_type4_open(&line) {
            node = Some(ast.new_html_block(HTMLBlockType::Type4));
        } else if regexps::html_block_type5_open(&line) {
            node = Some(ast.new_html_block(HTMLBlockType::Type5));
        } else if let Some(m) = regexps::html_block_type7(&line) {
            let is_close_tag = m.is_close_tag;
            let has_attr = m.has_attr;
            let tag_name = to_lower(&line[m.tag.0..m.tag.1]);
            if allowed_block_tag(&tag_name) {
                node = Some(ast.new_html_block(HTMLBlockType::Type6));
            } else if tag_name != b"script"
                && tag_name != b"style"
                && tag_name != b"pre"
                && !ast.is_paragraph(last)
                && !(is_close_tag && has_attr)
            {
                // type 7 can not interrupt paragraph
                node = Some(ast.new_html_block(HTMLBlockType::Type7));
            }
        }
        if node.is_none()
            && let Some((s, e)) = regexps::html_block_type6(&line)
        {
            let tag_name = &line[s..e];
            if allowed_block_tag(&to_lower(tag_name)) {
                node = Some(ast.new_html_block(HTMLBlockType::Type6));
            }
        }
        if let Some(node) = node {
            reader.advance_to_eol();
            ast.lines_mut(node).append(segment);
            return (Some(node), State::NO_CHILDREN);
        }
        (None, State::NO_CHILDREN)
    }

    // Go: parser/html_block.go:htmlBlockParser.Continue
    fn continue_<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> State {
        let html_block = *ast.html_block(node).unwrap();
        let (line, segment) = reader.peek_line();
        let line = line.unwrap_or_default();
        let source = reader.source();
        let lines_len = ast.lines(node).len();

        match html_block.html_block_type {
            HTMLBlockType::Type1 => {
                if lines_len == 1 {
                    let first_line = ast.lines(node).at(0);
                    if regexps::html_block_type1_close(&first_line.value(source)) {
                        return State::CLOSE;
                    }
                }
                if regexps::html_block_type1_close(&line) {
                    ast.html_block_mut(node).unwrap().closure_line = segment;
                    reader.advance_to_eol();
                    return State::CLOSE;
                }
            }
            HTMLBlockType::Type2
            | HTMLBlockType::Type3
            | HTMLBlockType::Type4
            | HTMLBlockType::Type5 => {
                let closure_pattern = match html_block.html_block_type {
                    HTMLBlockType::Type2 => HTML_BLOCK_TYPE2_CLOSE,
                    HTMLBlockType::Type3 => HTML_BLOCK_TYPE3_CLOSE,
                    HTMLBlockType::Type4 => HTML_BLOCK_TYPE4_CLOSE,
                    _ => HTML_BLOCK_TYPE5_CLOSE,
                };

                if lines_len == 1 {
                    let first_line = ast.lines(node).at(0);
                    if util::index(&first_line.value(source), closure_pattern) >= 0 {
                        return State::CLOSE;
                    }
                }
                if util::index(&line, closure_pattern) >= 0 {
                    ast.html_block_mut(node).unwrap().closure_line = segment;
                    reader.advance_to_eol();
                    return State::CLOSE;
                }
            }
            HTMLBlockType::Type6 | HTMLBlockType::Type7 => {
                if util::is_blank(&line) {
                    return State::CLOSE;
                }
            }
        }
        ast.lines_mut(node).append(segment);
        reader.advance_to_eol();
        State::CONTINUE | State::NO_CHILDREN
    }

    fn close<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
        // nothing to do
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }

    fn can_accept_indented_line(&self) -> bool {
        false
    }
}
