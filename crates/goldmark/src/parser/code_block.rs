// Go: github.com/yuin/goldmark@v1.7.12/parser/code_block.go

use super::{BlockParser, Context, State};
use crate::ast::{Ast, NodeId};
use crate::text::{Reader, Segment, new_segment};
use crate::util;

struct CodeBlockParser;

// Go: parser/code_block.go:NewCodeBlockParser
/// NewCodeBlockParser returns a new BlockParser that
/// parses code blocks.
pub fn new_code_block_parser() -> Box<dyn BlockParser> {
    Box::new(CodeBlockParser)
}

impl BlockParser for CodeBlockParser {
    fn trigger(&self) -> Option<&[u8]> {
        None
    }

    // Go: parser/code_block.go:codeBlockParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        let offset = reader.line_offset();
        let (pos, padding) = util::indent_position(&line, offset, 4);
        if pos < 0 || util::is_blank(&line) {
            return (None, State::NO_CHILDREN);
        }
        let node = ast.new_code_block();
        reader.advance_and_set_padding(pos, padding);
        let (_, mut segment) = reader.peek_line();
        // if code block line starts with a tab, keep a tab as it is.
        if segment.padding != 0 {
            preserve_leading_tab_in_code_block(&mut segment, reader, 0);
        }
        segment.force_newline = true;
        ast.lines_mut(node).append(segment);
        reader.advance_to_eol();
        (Some(node), State::NO_CHILDREN)
    }

    // Go: parser/code_block.go:codeBlockParser.Continue
    fn continue_<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> State {
        let (line, segment) = reader.peek_line();
        let line = line.unwrap_or_default();
        if util::is_blank(&line) {
            let s = segment.trim_left_space_width(4, reader.source());
            ast.lines_mut(node).append(s);
            return State::CONTINUE | State::NO_CHILDREN;
        }
        let offset = reader.line_offset();
        let (pos, padding) = util::indent_position(&line, offset, 4);
        if pos < 0 {
            return State::CLOSE;
        }
        reader.advance_and_set_padding(pos, padding);
        let (_, mut segment) = reader.peek_line();

        // if code block line starts with a tab, keep a tab as it is.
        if segment.padding != 0 {
            preserve_leading_tab_in_code_block(&mut segment, reader, 0);
        }

        segment.force_newline = true;
        ast.lines_mut(node).append(segment);
        reader.advance_to_eol();
        State::CONTINUE | State::NO_CHILDREN
    }

    // Go: parser/code_block.go:codeBlockParser.Close
    fn close<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
        // trim trailing blank lines
        let source = reader.source();
        let lines = ast.lines_mut(node);
        let mut length = lines.len() - 1;
        while length >= 0 {
            let line = lines.at(length);
            if util::is_blank(&line.value(source)) {
                length -= 1;
            } else {
                break;
            }
        }
        lines.set_sliced(0, length + 1);
    }

    fn can_interrupt_paragraph(&self) -> bool {
        false
    }

    fn can_accept_indented_line(&self) -> bool {
        true
    }
}

// Go: parser/code_block.go:preserveLeadingTabInCodeBlock
pub(crate) fn preserve_leading_tab_in_code_block<'a>(
    segment: &mut Segment,
    reader: &mut dyn Reader<'a>,
    indent: i64,
) {
    let offset_with_padding = reader.line_offset() + indent;
    let (sl, ss) = reader.position();
    reader.set_position(sl, new_segment(ss.start - 1, ss.stop));
    if offset_with_padding == reader.line_offset() {
        segment.padding = 0;
        segment.start -= 1;
    }
    reader.set_position(sl, ss);
}
