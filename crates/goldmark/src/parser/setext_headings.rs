// Go: github.com/yuin/goldmark@v1.7.12/parser/setext_headings.go

use std::sync::LazyLock;

use super::atx_heading::{attr_bytes, generate_auto_heading_id, parse_last_line_attributes};
use super::{
    BlockParser, Context, ContextKey, HeadingConfig, HeadingOption, State, new_context_key,
};
use crate::ast::{Ast, NodeId};
use crate::text::Reader;
use crate::util;

static TEMPORARY_PARAGRAPH_KEY: LazyLock<ContextKey> = LazyLock::new(new_context_key);

struct SetextHeadingParser {
    config: HeadingConfig,
}

// Go: parser/setext_headings.go:matchesSetextHeadingBar
pub(crate) fn matches_setext_heading_bar(line: &[u8]) -> Option<u8> {
    let mut start = 0;
    let mut end = line.len();
    let space = util::trim_left_length(line, b" ");
    if space > 3 {
        return None;
    }
    start += space;
    let level1 = util::trim_left_length(&line[start..end], b"=");
    let mut c = b'=';
    let mut level2 = 0;
    if level1 == 0 {
        level2 = util::trim_left_length(&line[start..end], b"-");
        c = b'-';
    }
    // Go: line[end-1] panics on an empty line.
    if util::is_space(line[end - 1]) {
        end -= util::trim_right_space_length(&line[start..end]);
    }
    if !((level1 > 0 && start + level1 == end) || (level2 > 0 && start + level2 == end)) {
        return None;
    }
    Some(c)
}

// Go: parser/setext_headings.go:NewSetextHeadingParser
/// NewSetextHeadingParser return a new BlockParser that can parse Setext headings.
pub fn new_setext_heading_parser(opts: Vec<Box<dyn HeadingOption>>) -> Box<dyn BlockParser> {
    let mut p = SetextHeadingParser {
        config: HeadingConfig::default(),
    };
    for o in opts {
        o.set_heading_option(&mut p.config);
    }
    Box::new(p)
}

impl BlockParser for SetextHeadingParser {
    fn trigger(&self) -> Option<&[u8]> {
        Some(b"-=")
    }

    // Go: parser/setext_headings.go:setextHeadingParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        parent: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        let Some(last) = pc.last_opened_block().map(|b| b.node) else {
            return (None, State::NO_CHILDREN);
        };
        if !ast.is_paragraph(Some(last)) || ast.parent(last) != Some(parent) {
            return (None, State::NO_CHILDREN);
        }
        let (line, segment) = reader.peek_line();
        let line = line.unwrap_or_default();
        let Some(c) = matches_setext_heading_bar(&line) else {
            return (None, State::NO_CHILDREN);
        };
        let mut level = 1;
        if c == b'-' {
            level = 2;
        }
        let node = ast.new_heading(level);
        ast.lines_mut(node).append(segment);
        pc.set_value(*TEMPORARY_PARAGRAPH_KEY, last);
        (Some(node), State::NO_CHILDREN | State::REQUIRE_PARAGRAPH)
    }

    fn continue_<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> State {
        State::CLOSE
    }

    // Go: parser/setext_headings.go:setextHeadingParser.Close
    fn close<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) {
        let heading = node;
        let mut segment = ast.lines(node).at(0);
        ast.lines_mut(heading).clear();
        let tmp = *pc
            .get_as::<NodeId>(*TEMPORARY_PARAGRAPH_KEY)
            .expect("interface conversion: interface {} is nil, not *ast.Paragraph");
        pc.set(*TEMPORARY_PARAGRAPH_KEY, None);
        if ast.lines(tmp).len() == 0 {
            let next = ast.next_sibling(heading);
            segment = segment.trim_left_space(reader.source());
            let hp = ast.parent(heading).expect("heading has no parent");
            if next.is_none() || !ast.is_paragraph(next) {
                let para = ast.new_paragraph();
                ast.lines_mut(para).append(segment);
                ast.insert_after(hp, heading, para);
            } else {
                ast.lines_mut(next.unwrap()).unshift(segment);
            }
            ast.remove_child(hp, heading);
        } else {
            let lines = ast.lines(tmp).clone();
            ast.set_lines(heading, &lines);
            let blank = ast.has_blank_previous_lines(tmp);
            ast.set_blank_previous_lines(heading, blank);
            if let Some(tp) = ast.parent(tmp) {
                ast.remove_child(tp, tmp);
            }
        }

        if self.config.attribute {
            parse_last_line_attributes(ast, node, reader, pc);
        }

        if self.config.auto_heading_id {
            match ast.attribute_string(node, "id") {
                None => generate_auto_heading_id(ast, heading, reader, pc),
                Some(id) => {
                    let id = attr_bytes(id);
                    pc.ids().put(&id);
                }
            }
        }
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }

    fn can_accept_indented_line(&self) -> bool {
        false
    }

    fn set_option(&mut self, name: &str, _value: &super::OptionValue) {
        self.config.set_option(name);
    }
}
