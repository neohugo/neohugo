// Go: github.com/yuin/goldmark@v1.7.12/parser/code_span.go

use super::{Context, InlineParser};
use crate::ast::{Ast, NodeId};
use crate::text::Reader;

struct CodeSpanParser;

// Go: parser/code_span.go:NewCodeSpanParser
/// NewCodeSpanParser return a new InlineParser that parses inline codes
/// surrounded by '`' .
pub fn new_code_span_parser() -> Box<dyn InlineParser> {
    Box::new(CodeSpanParser)
}

impl InlineParser for CodeSpanParser {
    fn trigger(&self) -> &[u8] {
        b"`"
    }

    // Go: parser/code_span.go:codeSpanParser.Parse
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        block: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> Option<NodeId> {
        let (line, start_segment) = block.peek_line();
        let line = line.unwrap_or_default();
        let mut opener = 0usize;
        while opener < line.len() && line[opener] == b'`' {
            opener += 1;
        }
        block.advance(opener as i64);
        let (l, pos) = block.position();
        let node = ast.new_code_span();
        'outer: loop {
            let (line, mut segment) = block.peek_line();
            let Some(line) = line else {
                block.set_position(l, pos);
                return Some(ast.new_text_segment(
                    start_segment.with_stop(start_segment.start + opener as i64),
                ));
            };
            let mut i = 0usize;
            while i < line.len() {
                let c = line[i];
                if c == b'`' {
                    let oldi = i;
                    while i < line.len() && line[i] == b'`' {
                        i += 1;
                    }
                    let closure = i - oldi;
                    if closure == opener && (i >= line.len() || line[i] != b'`') {
                        segment = segment.with_stop(segment.start + (i - closure) as i64);
                        if !segment.is_empty() {
                            let t = ast.new_raw_text_segment(segment);
                            ast.append_child(node, t);
                        }
                        block.advance(i as i64);
                        break 'outer;
                    }
                }
                i += 1;
            }
            let t = ast.new_raw_text_segment(segment);
            ast.append_child(node, t);
            block.advance_line();
        }
        // end:
        let source = block.source();
        if !ast.code_span_is_blank(node, source) {
            // trim first halfspace and last halfspace
            let first = ast.first_child(node).expect("code span child");
            let last = ast.last_child(node).expect("code span child");
            let segment = ast.text_node(first).expect("code span Text").segment;
            let mut should_trimmed = true;
            if !(!segment.is_empty() && is_space_or_newline(source[segment.start as usize])) {
                should_trimmed = false;
            }
            let segment = ast.text_node(last).expect("code span Text").segment;
            if !(!segment.is_empty() && is_space_or_newline(source[(segment.stop - 1) as usize])) {
                should_trimmed = false;
            }
            if should_trimmed {
                let t = ast.text_node_mut(first).unwrap();
                let segment = t.segment;
                t.segment = segment.with_start(segment.start + 1);
                let t = ast.text_node_mut(last).unwrap();
                let segment = t.segment;
                t.segment = segment.with_stop(segment.stop - 1);
            }
        }
        Some(node)
    }
}

// Go: parser/code_span.go:isSpaceOrNewline
fn is_space_or_newline(c: u8) -> bool {
    c == b' ' || c == b'\n'
}
