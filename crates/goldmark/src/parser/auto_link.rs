// Go: github.com/yuin/goldmark@v1.7.12/parser/auto_link.go

use super::{Context, InlineParser};
use crate::ast::{Ast, AutoLinkType, NodeId, Text};
use crate::text::{Reader, new_segment};
use crate::util;

struct AutoLinkParser;

// Go: parser/auto_link.go:NewAutoLinkParser
/// NewAutoLinkParser returns a new InlineParser that parses autolinks
/// surrounded by '<' and '>' .
pub fn new_auto_link_parser() -> Box<dyn InlineParser> {
    Box::new(AutoLinkParser)
}

impl InlineParser for AutoLinkParser {
    fn trigger(&self) -> &[u8] {
        b"<"
    }

    // Go: parser/auto_link.go:autoLinkParser.Parse
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        block: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> Option<NodeId> {
        let (line, segment) = block.peek_line();
        let line = line.unwrap_or_default();
        let mut stop = util::find_email_index(&line[1..]);
        let mut typ = AutoLinkType::Email;
        if stop < 0 {
            stop = util::find_url_index(&line[1..]);
            typ = AutoLinkType::Url;
        }
        if stop < 0 {
            return None;
        }
        stop += 1;
        if stop >= line.len() as i64 || line[stop as usize] != b'>' {
            return None;
        }
        let value = Text {
            segment: new_segment(segment.start + 1, segment.start + stop),
            flags: 0,
        };
        block.advance(stop + 1);
        Some(ast.new_auto_link(typ, value))
    }
}
