// Go: github.com/yuin/goldmark@v1.7.12/parser/paragraph.go

use super::{BlockParser, Context, State};
use crate::ast::{Ast, NodeId};
use crate::text::Reader;
use crate::util;

struct ParagraphParser;

// Go: parser/paragraph.go:NewParagraphParser
/// NewParagraphParser returns a new BlockParser that
/// parses paragraphs.
pub fn new_paragraph_parser() -> Box<dyn BlockParser> {
    Box::new(ParagraphParser)
}

impl BlockParser for ParagraphParser {
    fn trigger(&self) -> Option<&[u8]> {
        None
    }

    // Go: parser/paragraph.go:paragraphParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        let (_, segment) = reader.peek_line();
        let segment = segment.trim_left_space(reader.source());
        if segment.is_empty() {
            return (None, State::NO_CHILDREN);
        }
        let node = ast.new_paragraph();
        ast.lines_mut(node).append(segment);
        reader.advance_to_eol();
        (Some(node), State::NO_CHILDREN)
    }

    // Go: parser/paragraph.go:paragraphParser.Continue
    fn continue_<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> State {
        let (line, segment) = reader.peek_line();
        if util::is_blank(line.as_deref().unwrap_or(&[])) {
            return State::CLOSE;
        }
        ast.lines_mut(node).append(segment);
        reader.advance_to_eol();
        State::CONTINUE | State::NO_CHILDREN
    }

    // Go: parser/paragraph.go:paragraphParser.Close
    fn close<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
        let source = reader.source();
        let lines = ast.lines_mut(node);
        if lines.len() != 0 {
            // trim leading spaces
            for i in 0..lines.len() {
                let l = lines.at(i);
                lines.set(i, l.trim_left_space(source));
            }

            // trim trailing spaces
            let length = lines.len();
            let last_line = lines.at(length - 1);
            lines.set(length - 1, last_line.trim_right_space(source));
        }
        if lines.len() == 0 {
            let parent = ast.parent(node).expect("paragraph has no parent");
            ast.remove_child(parent, node);
        }
    }

    fn can_interrupt_paragraph(&self) -> bool {
        false
    }

    fn can_accept_indented_line(&self) -> bool {
        false
    }
}
