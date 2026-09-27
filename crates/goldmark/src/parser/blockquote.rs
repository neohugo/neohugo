// Go: github.com/yuin/goldmark@v1.7.12/parser/blockquote.go

use super::{BlockParser, Context, State};
use crate::ast::{Ast, NodeId};
use crate::text::Reader;
use crate::util;

struct BlockquoteParser;

// Go: parser/blockquote.go:NewBlockquoteParser
/// NewBlockquoteParser returns a new BlockParser that
/// parses blockquotes.
pub fn new_blockquote_parser() -> Box<dyn BlockParser> {
    Box::new(BlockquoteParser)
}

impl BlockquoteParser {
    // Go: parser/blockquote.go:blockquoteParser.process
    fn process<'a>(&self, reader: &mut dyn Reader<'a>) -> bool {
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        let offset = reader.line_offset();
        let (w, mut pos) = util::indent_width(&line, offset);
        if w > 3 || pos >= line.len() as i64 || line[pos as usize] != b'>' {
            return false;
        }
        pos += 1;
        if pos >= line.len() as i64 || line[pos as usize] == b'\n' {
            reader.advance(pos);
            return true;
        }
        reader.advance(pos);
        if line[pos as usize] == b' ' || line[pos as usize] == b'\t' {
            let mut padding = 0;
            if line[pos as usize] == b'\t' {
                padding = util::tab_width(reader.line_offset()) - 1;
            }
            reader.advance_and_set_padding(1, padding);
        }
        true
    }
}

impl BlockParser for BlockquoteParser {
    fn trigger(&self) -> Option<&[u8]> {
        Some(b">")
    }

    // Go: parser/blockquote.go:blockquoteParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        if self.process(reader) {
            return (Some(ast.new_blockquote()), State::HAS_CHILDREN);
        }
        (None, State::NO_CHILDREN)
    }

    // Go: parser/blockquote.go:blockquoteParser.Continue
    fn continue_<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> State {
        if self.process(reader) {
            return State::CONTINUE | State::HAS_CHILDREN;
        }
        State::CLOSE
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
