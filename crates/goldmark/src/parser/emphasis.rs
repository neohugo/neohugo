// Go: github.com/yuin/goldmark@v1.7.12/parser/emphasis.go

use std::sync::{Arc, LazyLock};

use super::{Context, Delimiter, DelimiterProcessor, InlineParser, scan_delimiter};
use crate::ast::{Ast, NodeId};
use crate::text::Reader;

struct EmphasisDelimiterProcessor;

impl DelimiterProcessor for EmphasisDelimiterProcessor {
    // Go: parser/emphasis.go:emphasisDelimiterProcessor.IsDelimiter
    fn is_delimiter(&self, b: u8) -> bool {
        b == b'*' || b == b'_'
    }

    // Go: parser/emphasis.go:emphasisDelimiterProcessor.CanOpenCloser
    fn can_open_closer(&self, opener: &Delimiter, closer: &Delimiter) -> bool {
        opener.char == closer.char
    }

    // Go: parser/emphasis.go:emphasisDelimiterProcessor.OnMatch
    fn on_match(&self, ast: &mut Ast, consumes: i64) -> NodeId {
        ast.new_emphasis(consumes)
    }
}

static DEFAULT_EMPHASIS_DELIMITER_PROCESSOR: LazyLock<Arc<dyn DelimiterProcessor>> =
    LazyLock::new(|| Arc::new(EmphasisDelimiterProcessor));

struct EmphasisParser;

// Go: parser/emphasis.go:NewEmphasisParser
/// NewEmphasisParser return a new InlineParser that parses emphasises.
pub fn new_emphasis_parser() -> Box<dyn InlineParser> {
    Box::new(EmphasisParser)
}

impl InlineParser for EmphasisParser {
    fn trigger(&self) -> &[u8] {
        b"*_"
    }

    // Go: parser/emphasis.go:emphasisParser.Parse
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        block: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> Option<NodeId> {
        let before = block.precending_character();
        let (line, segment) = block.peek_line();
        let line = line?;
        let mut node = scan_delimiter(
            &line,
            before,
            1,
            DEFAULT_EMPHASIS_DELIMITER_PROCESSOR.clone(),
        )?;
        node.segment = segment.with_stop(segment.start + node.original_length);
        block.advance(node.original_length);
        let id = ast.new_delimiter(node);
        pc.push_delimiter(ast, id);
        Some(id)
    }
}
