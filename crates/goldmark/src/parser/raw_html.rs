// Go: github.com/yuin/goldmark@v1.7.12/parser/raw_html.go

use super::regexps;
use super::{Context, InlineParser};
use crate::ast::{Ast, NodeId};
use crate::text::{Reader, RuneStream, new_segment};
use crate::util;

struct RawHTMLParser;

// Go: parser/raw_html.go:NewRawHTMLParser
/// NewRawHTMLParser return a new InlineParser that can parse
/// inline htmls.
pub fn new_raw_html_parser() -> Box<dyn InlineParser> {
    Box::new(RawHTMLParser)
}

const OPEN_PROCESSING_INSTRUCTION: &[u8] = b"<?";
const CLOSE_PROCESSING_INSTRUCTION: &[u8] = b"?>";
const OPEN_CDATA: &[u8] = b"<![CDATA[";
const CLOSE_CDATA: &[u8] = b"]]>";
const CLOSE_DECL: &[u8] = b">";
const EMPTY_COMMENT1: &[u8] = b"<!-->";
const EMPTY_COMMENT2: &[u8] = b"<!--->";
const OPEN_COMMENT: &[u8] = b"<!--";
const CLOSE_COMMENT: &[u8] = b"-->";

impl InlineParser for RawHTMLParser {
    fn trigger(&self) -> &[u8] {
        b"<"
    }

    // Go: parser/raw_html.go:rawHTMLParser.Parse
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        block: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> Option<NodeId> {
        let (line, _) = block.peek_line();
        let line = line.unwrap_or_default();
        if line.len() > 1 && util::is_alpha_numeric(line[1]) {
            return parse_multi_line_regexp(ast, &mut regexps::open_tag, block, pc);
        }
        if line.len() > 2 && line[1] == b'/' && util::is_alpha_numeric(line[2]) {
            return parse_multi_line_regexp(ast, &mut regexps::close_tag, block, pc);
        }
        if line.starts_with(OPEN_COMMENT) {
            return parse_comment(ast, block, pc);
        }
        if line.starts_with(OPEN_PROCESSING_INSTRUCTION) {
            return parse_until(ast, block, CLOSE_PROCESSING_INSTRUCTION, pc);
        }
        if line.len() > 2 && line[1] == b'!' && line[2] >= b'A' && line[2] <= b'Z' {
            return parse_until(ast, block, CLOSE_DECL, pc);
        }
        if line.starts_with(OPEN_CDATA) {
            return parse_until(ast, block, CLOSE_CDATA, pc);
        }
        None
    }
}

// Go: parser/raw_html.go:rawHTMLParser.parseComment
fn parse_comment<'a>(
    ast: &mut Ast,
    block: &mut dyn Reader<'a>,
    _pc: &mut Context,
) -> Option<NodeId> {
    let (saved_line, saved_segment) = block.position();
    let node = ast.new_raw_html();
    let (line, mut segment) = block.peek_line();
    let mut line = line.unwrap_or_default();
    if line.starts_with(EMPTY_COMMENT1) {
        let seg = segment.with_stop(segment.start + EMPTY_COMMENT1.len() as i64);
        ast.raw_html_mut(node).unwrap().segments.append(seg);
        block.advance(EMPTY_COMMENT1.len() as i64);
        return Some(node);
    }
    if line.starts_with(EMPTY_COMMENT2) {
        let seg = segment.with_stop(segment.start + EMPTY_COMMENT2.len() as i64);
        ast.raw_html_mut(node).unwrap().segments.append(seg);
        block.advance(EMPTY_COMMENT2.len() as i64);
        return Some(node);
    }
    let mut offset = OPEN_COMMENT.len() as i64;
    // Go: line = line[offset:] (a reslice; `skip` avoids copying the line)
    let mut skip = offset as usize;
    loop {
        let index = util::index(&line[skip..], CLOSE_COMMENT);
        if index > -1 {
            let seg =
                segment.with_stop(segment.start + offset + index + CLOSE_COMMENT.len() as i64);
            ast.raw_html_mut(node).unwrap().segments.append(seg);
            block.advance(offset + index + CLOSE_COMMENT.len() as i64);
            return Some(node);
        }
        offset = 0;
        ast.raw_html_mut(node).unwrap().segments.append(segment);
        block.advance_line();
        let (l, s) = block.peek_line();
        segment = s;
        match l {
            None => break,
            Some(l) => {
                line = l;
                skip = 0;
            }
        }
    }
    block.set_position(saved_line, saved_segment);
    None
}

// Go: parser/raw_html.go:rawHTMLParser.parseUntil
fn parse_until<'a>(
    ast: &mut Ast,
    block: &mut dyn Reader<'a>,
    closer: &[u8],
    _pc: &mut Context,
) -> Option<NodeId> {
    let (saved_line, saved_segment) = block.position();
    let node = ast.new_raw_html();
    loop {
        let (line, segment) = block.peek_line();
        let Some(line) = line else {
            break;
        };
        let index = util::index(&line, closer);
        if index > -1 {
            let seg = segment.with_stop(segment.start + index + closer.len() as i64);
            ast.raw_html_mut(node).unwrap().segments.append(seg);
            block.advance(index + closer.len() as i64);
            return Some(node);
        }
        ast.raw_html_mut(node).unwrap().segments.append(segment);
        block.advance_line();
    }
    block.set_position(saved_line, saved_segment);
    None
}

// Go: parser/raw_html.go:rawHTMLParser.parseMultiLineRegexp
fn parse_multi_line_regexp<'a>(
    ast: &mut Ast,
    reg: &mut dyn FnMut(&mut RuneStream<'_>) -> Option<i64>,
    block: &mut dyn Reader<'a>,
    _pc: &mut Context,
) -> Option<NodeId> {
    let (sline, ssegment) = block.position();
    if block.match_with(reg) {
        let node = ast.new_raw_html();
        let (eline, esegment) = block.position();
        block.set_position(sline, ssegment);
        loop {
            let (line, segment) = block.peek_line();
            if line.is_none() {
                break;
            }
            let (l, _) = block.position();
            let mut start = segment.start;
            if l == sline {
                start = ssegment.start;
            }
            let mut end = segment.stop;
            if l == eline {
                end = esegment.start;
            }

            ast.raw_html_mut(node)
                .unwrap()
                .segments
                .append(new_segment(start, end));
            if l == eline {
                block.advance(end - start);
                break;
            }
            block.advance_line();
        }
        return Some(node);
    }
    None
}
