// Go: github.com/yuin/goldmark@v1.7.12/parser/fcode_block.go

use std::sync::LazyLock;

use super::code_block::preserve_leading_tab_in_code_block;
use super::{BlockParser, Context, ContextKey, State, new_context_key};
use crate::ast::{Ast, NodeId};
use crate::text::{Reader, new_segment, new_segment_padding};
use crate::util;

struct FencedCodeBlockParser;

// Go: parser/fcode_block.go:NewFencedCodeBlockParser
/// NewFencedCodeBlockParser returns a new BlockParser that
/// parses fenced code blocks.
pub fn new_fenced_code_block_parser() -> Box<dyn BlockParser> {
    Box::new(FencedCodeBlockParser)
}

struct FenceData {
    char: u8,
    indent: i64,
    length: i64,
    node: NodeId,
}

static FENCED_CODE_BLOCK_INFO_KEY: LazyLock<ContextKey> = LazyLock::new(new_context_key);

impl BlockParser for FencedCodeBlockParser {
    fn trigger(&self) -> Option<&[u8]> {
        Some(b"~`")
    }

    // Go: parser/fcode_block.go:fencedCodeBlockParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        let (line, segment) = reader.peek_line();
        let line = line.unwrap_or_default();
        let pos = pc.block_offset();
        if pos < 0 || (line[pos as usize] != b'`' && line[pos as usize] != b'~') {
            return (None, State::NO_CHILDREN);
        }
        let findent = pos;
        let fence_char = line[pos as usize];
        let mut i = pos as usize;
        while i < line.len() && line[i] == fence_char {
            i += 1;
        }
        let o_fence_length = i as i64 - pos;
        if o_fence_length < 3 {
            return (None, State::NO_CHILDREN);
        }
        let mut info: Option<NodeId> = None;
        if i + 1 < line.len() {
            let rest = &line[i..];
            let left = util::trim_left_space_length(rest);
            let right = util::trim_right_space_length(rest);
            if left < rest.len() - right {
                let info_start = segment.start - segment.padding + i as i64 + left as i64;
                let info_stop = segment.stop - right as i64;
                let value = &rest[left..rest.len() - right];
                if fence_char == b'`' && value.contains(&b'`') {
                    return (None, State::NO_CHILDREN);
                } else if info_start != info_stop {
                    info = Some(ast.new_text_segment(new_segment(info_start, info_stop)));
                }
            }
        }
        let node = ast.new_fenced_code_block(info);
        pc.set_value(
            *FENCED_CODE_BLOCK_INFO_KEY,
            FenceData {
                char: fence_char,
                indent: findent,
                length: o_fence_length,
                node,
            },
        );
        (Some(node), State::NO_CHILDREN)
    }

    // Go: parser/fcode_block.go:fencedCodeBlockParser.Continue
    fn continue_<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> State {
        let (line, segment) = reader.peek_line();
        let line = line.unwrap_or_default();
        let (fchar, findent, flength) = {
            let fdata = pc
                .get_as::<FenceData>(*FENCED_CODE_BLOCK_INFO_KEY)
                .expect("interface conversion: interface {} is nil, not *parser.fenceData");
            (fdata.char, fdata.indent, fdata.length)
        };

        let offset = reader.line_offset();
        let (w, pos) = util::indent_width(&line, offset);
        if w < 4 {
            let mut i = pos as usize;
            while i < line.len() && line[i] == fchar {
                i += 1;
            }
            let length = i as i64 - pos;
            if length >= flength && util::is_blank(&line[i..]) {
                let mut newline = 1;
                if line[line.len() - 1] != b'\n' {
                    newline = 0;
                }
                reader.advance(segment.stop - segment.start - newline + segment.padding);
                return State::CLOSE;
            }
        }
        let offset = reader.line_offset();
        let (mut pos, mut padding) =
            util::indent_position_padding(&line, offset, segment.padding, findent);
        if pos < 0 {
            pos = util::first_non_space_position(&line);
            if pos < 0 {
                pos = 0;
            }
            padding = 0;
        }
        let mut seg = new_segment_padding(segment.start + pos, segment.stop, padding);
        // if code block line starts with a tab, keep a tab as it is.
        if padding != 0 {
            preserve_leading_tab_in_code_block(&mut seg, reader, findent);
        }
        seg.force_newline = true; // EOF as newline
        ast.lines_mut(node).append(seg);
        reader.advance_and_set_padding(segment.stop - segment.start - pos - 1, padding);
        State::CONTINUE | State::NO_CHILDREN
    }

    // Go: parser/fcode_block.go:fencedCodeBlockParser.Close
    fn close<'a>(
        &self,
        _ast: &mut Ast,
        node: NodeId,
        _reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) {
        let fnode = pc
            .get_as::<FenceData>(*FENCED_CODE_BLOCK_INFO_KEY)
            .expect("interface conversion: interface {} is nil, not *parser.fenceData")
            .node;
        if fnode == node {
            pc.set(*FENCED_CODE_BLOCK_INFO_KEY, None);
        }
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }

    fn can_accept_indented_line(&self) -> bool {
        false
    }
}
