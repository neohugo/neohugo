// Go: github.com/yuin/goldmark@v1.7.12/parser/thematic_break.go

use super::{BlockParser, Context, State};
use crate::ast::{Ast, NodeId};
use crate::text::Reader;
use crate::util;

struct ThematicBreakParser;

// Go: parser/thematic_break.go:NewThematicBreakParser
/// NewThematicBreakParser returns a new BlockParser that
/// parses thematic breaks.
pub fn new_thematic_break_parser() -> Box<dyn BlockParser> {
    Box::new(ThematicBreakParser)
}

// Go: parser/thematic_break.go:isThematicBreak
pub(crate) fn is_thematic_break(line: &[u8], offset: i64) -> bool {
    let (w, pos) = util::indent_width(line, offset);
    if w > 3 {
        return false;
    }
    let mut mark: u8 = 0;
    let mut count = 0;
    for &c in &line[pos as usize..] {
        if util::is_space(c) {
            continue;
        }
        if mark == 0 {
            mark = c;
            count = 1;
            if mark == b'*' || mark == b'-' || mark == b'_' {
                continue;
            }
            return false;
        }
        if c != mark {
            return false;
        }
        count += 1;
    }
    count > 2
}

impl BlockParser for ThematicBreakParser {
    fn trigger(&self) -> Option<&[u8]> {
        Some(b"-*_")
    }

    // Go: parser/thematic_break.go:thematicBreakPraser.Open
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
        if is_thematic_break(&line, offset) {
            reader.advance_to_eol();
            return (Some(ast.new_thematic_break()), State::NO_CHILDREN);
        }
        (None, State::NO_CHILDREN)
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
