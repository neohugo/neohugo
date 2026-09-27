//! The parser: turns tokens into events.
//!
//! Go: gopkg.in/yaml.v2@v2.4.0 parserc.go. The grammar comments of the Go
//! file are omitted; the functions keep their Go names and control flow.

use crate::scannerc::yaml_parser_fetch_more_tokens;
use crate::yamlh::*;

/// The scalar fields of the token at the head of the queue. (Go's
/// peek_token returns a pointer; the byte fields are taken from the queue
/// with `head_value`/`head_suffix`/`head_prefix` just before the token is
/// skipped.)
#[derive(Clone, Copy, Debug)]
struct Tok {
    typ: TokenType,
    start_mark: Mark,
    end_mark: Mark,
    style: ScalarStyle,
    major: i8,
    minor: i8,
}

// Go: parserc.go:peek_token
fn peek_token(parser: &mut Parser) -> Option<Tok> {
    if parser.token_available || yaml_parser_fetch_more_tokens(parser) {
        let t = &parser.tokens[parser.tokens_head];
        return Some(Tok {
            typ: t.typ,
            start_mark: t.start_mark,
            end_mark: t.end_mark,
            style: t.style,
            major: t.major,
            minor: t.minor,
        });
    }
    None
}

fn head_value(parser: &mut Parser) -> Vec<u8> {
    let h = parser.tokens_head;
    std::mem::take(&mut parser.tokens[h].value)
}

fn head_suffix(parser: &mut Parser) -> Vec<u8> {
    let h = parser.tokens_head;
    std::mem::take(&mut parser.tokens[h].suffix)
}

fn head_prefix(parser: &mut Parser) -> Vec<u8> {
    let h = parser.tokens_head;
    std::mem::take(&mut parser.tokens[h].prefix)
}

// Go: parserc.go:skip_token
fn skip_token(parser: &mut Parser) {
    parser.token_available = false;
    parser.tokens_parsed += 1;
    parser.stream_end_produced = parser.tokens[parser.tokens_head].typ == TokenType::StreamEnd;
    parser.tokens_head += 1;
}

// Go: parserc.go:yaml_parser_parse
//
// Get the next event.
pub(crate) fn yaml_parser_parse(parser: &mut Parser, event: &mut Event) -> bool {
    // Erase the event object.
    *event = Event::default();

    // No events after the end of the stream or error.
    if parser.stream_end_produced
        || parser.error != ErrorType::NoError
        || parser.state == ParserState::End
    {
        return true;
    }

    // Generate the next event.
    yaml_parser_state_machine(parser, event)
}

// Go: parserc.go:yaml_parser_set_parser_error
fn yaml_parser_set_parser_error(parser: &mut Parser, problem: &str, problem_mark: Mark) -> bool {
    parser.error = ErrorType::Parser;
    parser.problem = problem.to_string();
    parser.problem_mark = problem_mark;
    false
}

// Go: parserc.go:yaml_parser_set_parser_error_context
fn yaml_parser_set_parser_error_context(
    parser: &mut Parser,
    context: &str,
    context_mark: Mark,
    problem: &str,
    problem_mark: Mark,
) -> bool {
    parser.error = ErrorType::Parser;
    parser.context = context.to_string();
    parser.context_mark = context_mark;
    parser.problem = problem.to_string();
    parser.problem_mark = problem_mark;
    false
}

// Go: parserc.go:yaml_parser_state_machine
//
// State dispatcher.
fn yaml_parser_state_machine(parser: &mut Parser, event: &mut Event) -> bool {
    match parser.state {
        ParserState::StreamStart => yaml_parser_parse_stream_start(parser, event),
        ParserState::ImplicitDocumentStart => yaml_parser_parse_document_start(parser, event, true),
        ParserState::DocumentStart => yaml_parser_parse_document_start(parser, event, false),
        ParserState::DocumentContent => yaml_parser_parse_document_content(parser, event),
        ParserState::DocumentEnd => yaml_parser_parse_document_end(parser, event),
        ParserState::BlockNode => yaml_parser_parse_node(parser, event, true, false),
        ParserState::BlockNodeOrIndentlessSequence => {
            yaml_parser_parse_node(parser, event, true, true)
        }
        ParserState::FlowNode => yaml_parser_parse_node(parser, event, false, false),
        ParserState::BlockSequenceFirstEntry => {
            yaml_parser_parse_block_sequence_entry(parser, event, true)
        }
        ParserState::BlockSequenceEntry => {
            yaml_parser_parse_block_sequence_entry(parser, event, false)
        }
        ParserState::IndentlessSequenceEntry => {
            yaml_parser_parse_indentless_sequence_entry(parser, event)
        }
        ParserState::BlockMappingFirstKey => {
            yaml_parser_parse_block_mapping_key(parser, event, true)
        }
        ParserState::BlockMappingKey => yaml_parser_parse_block_mapping_key(parser, event, false),
        ParserState::BlockMappingValue => yaml_parser_parse_block_mapping_value(parser, event),
        ParserState::FlowSequenceFirstEntry => {
            yaml_parser_parse_flow_sequence_entry(parser, event, true)
        }
        ParserState::FlowSequenceEntry => {
            yaml_parser_parse_flow_sequence_entry(parser, event, false)
        }
        ParserState::FlowSequenceEntryMappingKey => {
            yaml_parser_parse_flow_sequence_entry_mapping_key(parser, event)
        }
        ParserState::FlowSequenceEntryMappingValue => {
            yaml_parser_parse_flow_sequence_entry_mapping_value(parser, event)
        }
        ParserState::FlowSequenceEntryMappingEnd => {
            yaml_parser_parse_flow_sequence_entry_mapping_end(parser, event)
        }
        ParserState::FlowMappingFirstKey => yaml_parser_parse_flow_mapping_key(parser, event, true),
        ParserState::FlowMappingKey => yaml_parser_parse_flow_mapping_key(parser, event, false),
        ParserState::FlowMappingValue => yaml_parser_parse_flow_mapping_value(parser, event, false),
        ParserState::FlowMappingEmptyValue => {
            yaml_parser_parse_flow_mapping_value(parser, event, true)
        }
        // Go: panic("invalid parser state"); unreachable because
        // yaml_parser_parse returns early in the End state.
        ParserState::End => true,
    }
}

/// Pop the parser state stack (Go: `parser.state = parser.states[len-1]`).
fn pop_state(parser: &mut Parser) {
    parser.state = parser.states.pop().unwrap_or(ParserState::End);
}

/// Pop the marks stack.
fn pop_mark(parser: &mut Parser) -> Mark {
    parser.marks.pop().unwrap_or_default()
}

// Go: parserc.go:yaml_parser_parse_stream_start
//
// Parse the production:
// stream   ::= STREAM-START implicit_document? explicit_document* STREAM-END
//              ************
fn yaml_parser_parse_stream_start(parser: &mut Parser, event: &mut Event) -> bool {
    let Some(token) = peek_token(parser) else {
        return false;
    };
    if token.typ != TokenType::StreamStart {
        return yaml_parser_set_parser_error(
            parser,
            "did not find expected <stream-start>",
            token.start_mark,
        );
    }
    parser.state = ParserState::ImplicitDocumentStart;
    *event = Event {
        typ: EventType::StreamStart,
        start_mark: token.start_mark,
        end_mark: token.end_mark,
        ..Default::default()
    };
    skip_token(parser);
    true
}

// Go: parserc.go:yaml_parser_parse_document_start
fn yaml_parser_parse_document_start(
    parser: &mut Parser,
    event: &mut Event,
    implicit: bool,
) -> bool {
    let Some(mut token) = peek_token(parser) else {
        return false;
    };

    // Parse extra document end indicators.
    if !implicit {
        while token.typ == TokenType::DocumentEnd {
            skip_token(parser);
            match peek_token(parser) {
                Some(t) => token = t,
                None => return false,
            }
        }
    }

    if implicit
        && token.typ != TokenType::VersionDirective
        && token.typ != TokenType::TagDirective
        && token.typ != TokenType::DocumentStart
        && token.typ != TokenType::StreamEnd
    {
        // Parse an implicit document.
        if !yaml_parser_process_directives(parser, false) {
            return false;
        }
        parser.states.push(ParserState::DocumentEnd);
        parser.state = ParserState::BlockNode;

        *event = Event {
            typ: EventType::DocumentStart,
            start_mark: token.start_mark,
            end_mark: token.end_mark,
            ..Default::default()
        };
    } else if token.typ != TokenType::StreamEnd {
        // Parse an explicit document.
        let start_mark = token.start_mark;
        if !yaml_parser_process_directives(parser, true) {
            return false;
        }
        let Some(token) = peek_token(parser) else {
            return false;
        };
        if token.typ != TokenType::DocumentStart {
            yaml_parser_set_parser_error(
                parser,
                "did not find expected <document start>",
                token.start_mark,
            );
            return false;
        }
        parser.states.push(ParserState::DocumentEnd);
        parser.state = ParserState::DocumentContent;
        let end_mark = token.end_mark;

        *event = Event {
            typ: EventType::DocumentStart,
            start_mark,
            end_mark,
            implicit: false,
            ..Default::default()
        };
        skip_token(parser);
    } else {
        // Parse the stream end.
        parser.state = ParserState::End;
        *event = Event {
            typ: EventType::StreamEnd,
            start_mark: token.start_mark,
            end_mark: token.end_mark,
            ..Default::default()
        };
        skip_token(parser);
    }

    true
}

// Go: parserc.go:yaml_parser_parse_document_content
//
// Parse the productions:
// explicit_document    ::= DIRECTIVE* DOCUMENT-START block_node? DOCUMENT-END*
//                                                    ***********
fn yaml_parser_parse_document_content(parser: &mut Parser, event: &mut Event) -> bool {
    let Some(token) = peek_token(parser) else {
        return false;
    };
    if token.typ == TokenType::VersionDirective
        || token.typ == TokenType::TagDirective
        || token.typ == TokenType::DocumentStart
        || token.typ == TokenType::DocumentEnd
        || token.typ == TokenType::StreamEnd
    {
        pop_state(parser);
        return yaml_parser_process_empty_scalar(parser, event, token.start_mark);
    }
    yaml_parser_parse_node(parser, event, true, false)
}

// Go: parserc.go:yaml_parser_parse_document_end
//
// Parse the productions:
// implicit_document    ::= block_node DOCUMENT-END*
//                                     *************
// explicit_document    ::= DIRECTIVE* DOCUMENT-START block_node? DOCUMENT-END*
fn yaml_parser_parse_document_end(parser: &mut Parser, event: &mut Event) -> bool {
    let Some(token) = peek_token(parser) else {
        return false;
    };

    let start_mark = token.start_mark;
    let mut end_mark = token.start_mark;

    let mut implicit = true;
    if token.typ == TokenType::DocumentEnd {
        end_mark = token.end_mark;
        skip_token(parser);
        implicit = false;
    }

    parser.tag_directives.clear();

    parser.state = ParserState::DocumentStart;
    *event = Event {
        typ: EventType::DocumentEnd,
        start_mark,
        end_mark,
        implicit,
        ..Default::default()
    };
    true
}

// Go: parserc.go:yaml_parser_parse_node
fn yaml_parser_parse_node(
    parser: &mut Parser,
    event: &mut Event,
    block: bool,
    indentless_sequence: bool,
) -> bool {
    let Some(mut token) = peek_token(parser) else {
        return false;
    };

    if token.typ == TokenType::Alias {
        pop_state(parser);
        *event = Event {
            typ: EventType::Alias,
            start_mark: token.start_mark,
            end_mark: token.end_mark,
            anchor: head_value(parser),
            ..Default::default()
        };
        skip_token(parser);
        return true;
    }

    let mut start_mark = token.start_mark;
    let mut end_mark = token.start_mark;

    let mut tag_token = false;
    let mut tag_handle: Vec<u8> = Vec::new();
    let mut tag_suffix: Vec<u8> = Vec::new();
    let mut anchor: Vec<u8> = Vec::new();
    let mut tag_mark = Mark::default();
    if token.typ == TokenType::Anchor {
        anchor = head_value(parser);
        start_mark = token.start_mark;
        end_mark = token.end_mark;
        skip_token(parser);
        match peek_token(parser) {
            Some(t) => token = t,
            None => return false,
        }
        if token.typ == TokenType::Tag {
            tag_token = true;
            tag_handle = head_value(parser);
            tag_suffix = head_suffix(parser);
            tag_mark = token.start_mark;
            end_mark = token.end_mark;
            skip_token(parser);
            match peek_token(parser) {
                Some(t) => token = t,
                None => return false,
            }
        }
    } else if token.typ == TokenType::Tag {
        tag_token = true;
        tag_handle = head_value(parser);
        tag_suffix = head_suffix(parser);
        start_mark = token.start_mark;
        tag_mark = token.start_mark;
        end_mark = token.end_mark;
        skip_token(parser);
        match peek_token(parser) {
            Some(t) => token = t,
            None => return false,
        }
        if token.typ == TokenType::Anchor {
            anchor = head_value(parser);
            end_mark = token.end_mark;
            skip_token(parser);
            match peek_token(parser) {
                Some(t) => token = t,
                None => return false,
            }
        }
    }

    let mut tag: Vec<u8> = Vec::new();
    if tag_token {
        if tag_handle.is_empty() {
            tag = tag_suffix;
        } else {
            for td in &parser.tag_directives {
                if td.handle == tag_handle {
                    tag = td.prefix.clone();
                    tag.extend_from_slice(&tag_suffix);
                    break;
                }
            }
            if tag.is_empty() {
                yaml_parser_set_parser_error_context(
                    parser,
                    "while parsing a node",
                    start_mark,
                    "found undefined tag handle",
                    tag_mark,
                );
                return false;
            }
        }
    }

    let implicit = tag.is_empty();
    if indentless_sequence && token.typ == TokenType::BlockEntry {
        end_mark = token.end_mark;
        parser.state = ParserState::IndentlessSequenceEntry;
        *event = Event {
            typ: EventType::SequenceStart,
            start_mark,
            end_mark,
            anchor,
            tag,
            implicit,
            ..Default::default()
        };
        return true;
    }
    if token.typ == TokenType::Scalar {
        let mut plain_implicit = false;
        let mut quoted_implicit = false;
        end_mark = token.end_mark;
        if (tag.is_empty() && token.style == ScalarStyle::Plain)
            || (tag.len() == 1 && tag[0] == b'!')
        {
            plain_implicit = true;
        } else if tag.is_empty() {
            quoted_implicit = true;
        }
        pop_state(parser);

        *event = Event {
            typ: EventType::Scalar,
            start_mark,
            end_mark,
            anchor,
            tag,
            value: head_value(parser),
            implicit: plain_implicit,
            quoted_implicit,
        };
        skip_token(parser);
        return true;
    }
    if token.typ == TokenType::FlowSequenceStart {
        // [Go] Some of the events below can be merged as they differ only on style.
        end_mark = token.end_mark;
        parser.state = ParserState::FlowSequenceFirstEntry;
        *event = Event {
            typ: EventType::SequenceStart,
            start_mark,
            end_mark,
            anchor,
            tag,
            implicit,
            ..Default::default()
        };
        return true;
    }
    if token.typ == TokenType::FlowMappingStart {
        end_mark = token.end_mark;
        parser.state = ParserState::FlowMappingFirstKey;
        *event = Event {
            typ: EventType::MappingStart,
            start_mark,
            end_mark,
            anchor,
            tag,
            implicit,
            ..Default::default()
        };
        return true;
    }
    if block && token.typ == TokenType::BlockSequenceStart {
        end_mark = token.end_mark;
        parser.state = ParserState::BlockSequenceFirstEntry;
        *event = Event {
            typ: EventType::SequenceStart,
            start_mark,
            end_mark,
            anchor,
            tag,
            implicit,
            ..Default::default()
        };
        return true;
    }
    if block && token.typ == TokenType::BlockMappingStart {
        end_mark = token.end_mark;
        parser.state = ParserState::BlockMappingFirstKey;
        *event = Event {
            typ: EventType::MappingStart,
            start_mark,
            end_mark,
            anchor,
            tag,
            implicit,
            ..Default::default()
        };
        return true;
    }
    if !anchor.is_empty() || !tag.is_empty() {
        pop_state(parser);

        *event = Event {
            typ: EventType::Scalar,
            start_mark,
            end_mark,
            anchor,
            tag,
            implicit,
            quoted_implicit: false,
            ..Default::default()
        };
        return true;
    }

    let context = if block {
        "while parsing a block node"
    } else {
        "while parsing a flow node"
    };
    yaml_parser_set_parser_error_context(
        parser,
        context,
        start_mark,
        "did not find expected node content",
        token.start_mark,
    );
    false
}

// Go: parserc.go:yaml_parser_parse_block_sequence_entry
//
// Parse the productions:
// block_sequence ::= BLOCK-SEQUENCE-START (BLOCK-ENTRY block_node?)* BLOCK-END
//                    ********************  *********** *             *********
fn yaml_parser_parse_block_sequence_entry(
    parser: &mut Parser,
    event: &mut Event,
    first: bool,
) -> bool {
    if first {
        let Some(token) = peek_token(parser) else {
            return false;
        };
        parser.marks.push(token.start_mark);
        skip_token(parser);
    }

    let Some(mut token) = peek_token(parser) else {
        return false;
    };

    if token.typ == TokenType::BlockEntry {
        let mark = token.end_mark;
        skip_token(parser);
        match peek_token(parser) {
            Some(t) => token = t,
            None => return false,
        }
        if token.typ != TokenType::BlockEntry && token.typ != TokenType::BlockEnd {
            parser.states.push(ParserState::BlockSequenceEntry);
            return yaml_parser_parse_node(parser, event, true, false);
        } else {
            parser.state = ParserState::BlockSequenceEntry;
            return yaml_parser_process_empty_scalar(parser, event, mark);
        }
    }
    if token.typ == TokenType::BlockEnd {
        pop_state(parser);
        pop_mark(parser);

        *event = Event {
            typ: EventType::SequenceEnd,
            start_mark: token.start_mark,
            end_mark: token.end_mark,
            ..Default::default()
        };

        skip_token(parser);
        return true;
    }

    let context_mark = pop_mark(parser);
    yaml_parser_set_parser_error_context(
        parser,
        "while parsing a block collection",
        context_mark,
        "did not find expected '-' indicator",
        token.start_mark,
    )
}

// Go: parserc.go:yaml_parser_parse_indentless_sequence_entry
//
// Parse the productions:
// indentless_sequence  ::= (BLOCK-ENTRY block_node?)+
//                           *********** *
fn yaml_parser_parse_indentless_sequence_entry(parser: &mut Parser, event: &mut Event) -> bool {
    let Some(mut token) = peek_token(parser) else {
        return false;
    };

    if token.typ == TokenType::BlockEntry {
        let mark = token.end_mark;
        skip_token(parser);
        match peek_token(parser) {
            Some(t) => token = t,
            None => return false,
        }
        if token.typ != TokenType::BlockEntry
            && token.typ != TokenType::Key
            && token.typ != TokenType::Value
            && token.typ != TokenType::BlockEnd
        {
            parser.states.push(ParserState::IndentlessSequenceEntry);
            return yaml_parser_parse_node(parser, event, true, false);
        }
        parser.state = ParserState::IndentlessSequenceEntry;
        return yaml_parser_process_empty_scalar(parser, event, mark);
    }
    pop_state(parser);

    *event = Event {
        typ: EventType::SequenceEnd,
        start_mark: token.start_mark,
        end_mark: token.start_mark, // [Go] Shouldn't this be token.end_mark?
        ..Default::default()
    };
    true
}

// Go: parserc.go:yaml_parser_parse_block_mapping_key
//
// Parse the productions:
// block_mapping        ::= BLOCK-MAPPING_START
//                          *******************
//                          ((KEY block_node_or_indentless_sequence?)?
//                            *** *
//                          (VALUE block_node_or_indentless_sequence?)?)*
//
//                          BLOCK-END
//                          *********
fn yaml_parser_parse_block_mapping_key(
    parser: &mut Parser,
    event: &mut Event,
    first: bool,
) -> bool {
    if first {
        let Some(token) = peek_token(parser) else {
            return false;
        };
        parser.marks.push(token.start_mark);
        skip_token(parser);
    }

    let Some(mut token) = peek_token(parser) else {
        return false;
    };

    if token.typ == TokenType::Key {
        let mark = token.end_mark;
        skip_token(parser);
        match peek_token(parser) {
            Some(t) => token = t,
            None => return false,
        }
        if token.typ != TokenType::Key
            && token.typ != TokenType::Value
            && token.typ != TokenType::BlockEnd
        {
            parser.states.push(ParserState::BlockMappingValue);
            return yaml_parser_parse_node(parser, event, true, true);
        } else {
            parser.state = ParserState::BlockMappingValue;
            return yaml_parser_process_empty_scalar(parser, event, mark);
        }
    } else if token.typ == TokenType::BlockEnd {
        pop_state(parser);
        pop_mark(parser);
        *event = Event {
            typ: EventType::MappingEnd,
            start_mark: token.start_mark,
            end_mark: token.end_mark,
            ..Default::default()
        };
        skip_token(parser);
        return true;
    }

    let context_mark = pop_mark(parser);
    yaml_parser_set_parser_error_context(
        parser,
        "while parsing a block mapping",
        context_mark,
        "did not find expected key",
        token.start_mark,
    )
}

// Go: parserc.go:yaml_parser_parse_block_mapping_value
//
// Parse the productions:
// block_mapping        ::= BLOCK-MAPPING_START
//
//                          ((KEY block_node_or_indentless_sequence?)?
//
//                          (VALUE block_node_or_indentless_sequence?)?)*
//                           ***** *
//                          BLOCK-END
fn yaml_parser_parse_block_mapping_value(parser: &mut Parser, event: &mut Event) -> bool {
    let Some(mut token) = peek_token(parser) else {
        return false;
    };
    if token.typ == TokenType::Value {
        let mark = token.end_mark;
        skip_token(parser);
        match peek_token(parser) {
            Some(t) => token = t,
            None => return false,
        }
        if token.typ != TokenType::Key
            && token.typ != TokenType::Value
            && token.typ != TokenType::BlockEnd
        {
            parser.states.push(ParserState::BlockMappingKey);
            return yaml_parser_parse_node(parser, event, true, true);
        }
        parser.state = ParserState::BlockMappingKey;
        return yaml_parser_process_empty_scalar(parser, event, mark);
    }
    parser.state = ParserState::BlockMappingKey;
    yaml_parser_process_empty_scalar(parser, event, token.start_mark)
}

// Go: parserc.go:yaml_parser_parse_flow_sequence_entry
//
// Parse the productions:
// flow_sequence        ::= FLOW-SEQUENCE-START
//                          *******************
//                          (flow_sequence_entry FLOW-ENTRY)*
//                           *                   **********
//                          flow_sequence_entry?
//                          *
//                          FLOW-SEQUENCE-END
//                          *****************
// flow_sequence_entry  ::= flow_node | KEY flow_node? (VALUE flow_node?)?
//                          *
fn yaml_parser_parse_flow_sequence_entry(
    parser: &mut Parser,
    event: &mut Event,
    first: bool,
) -> bool {
    if first {
        let Some(token) = peek_token(parser) else {
            return false;
        };
        parser.marks.push(token.start_mark);
        skip_token(parser);
    }
    let Some(mut token) = peek_token(parser) else {
        return false;
    };
    if token.typ != TokenType::FlowSequenceEnd {
        if !first {
            if token.typ == TokenType::FlowEntry {
                skip_token(parser);
                match peek_token(parser) {
                    Some(t) => token = t,
                    None => return false,
                }
            } else {
                let context_mark = pop_mark(parser);
                return yaml_parser_set_parser_error_context(
                    parser,
                    "while parsing a flow sequence",
                    context_mark,
                    "did not find expected ',' or ']'",
                    token.start_mark,
                );
            }
        }

        if token.typ == TokenType::Key {
            parser.state = ParserState::FlowSequenceEntryMappingKey;
            *event = Event {
                typ: EventType::MappingStart,
                start_mark: token.start_mark,
                end_mark: token.end_mark,
                implicit: true,
                ..Default::default()
            };
            skip_token(parser);
            return true;
        } else if token.typ != TokenType::FlowSequenceEnd {
            parser.states.push(ParserState::FlowSequenceEntry);
            return yaml_parser_parse_node(parser, event, false, false);
        }
    }

    pop_state(parser);
    pop_mark(parser);

    *event = Event {
        typ: EventType::SequenceEnd,
        start_mark: token.start_mark,
        end_mark: token.end_mark,
        ..Default::default()
    };

    skip_token(parser);
    true
}

// Go: parserc.go:yaml_parser_parse_flow_sequence_entry_mapping_key
//
// Parse the productions:
// flow_sequence_entry  ::= flow_node | KEY flow_node? (VALUE flow_node?)?
//                                      *** *
fn yaml_parser_parse_flow_sequence_entry_mapping_key(
    parser: &mut Parser,
    event: &mut Event,
) -> bool {
    let Some(token) = peek_token(parser) else {
        return false;
    };
    if token.typ != TokenType::Value
        && token.typ != TokenType::FlowEntry
        && token.typ != TokenType::FlowSequenceEnd
    {
        parser
            .states
            .push(ParserState::FlowSequenceEntryMappingValue);
        return yaml_parser_parse_node(parser, event, false, false);
    }
    let mark = token.end_mark;
    skip_token(parser);
    parser.state = ParserState::FlowSequenceEntryMappingValue;
    yaml_parser_process_empty_scalar(parser, event, mark)
}

// Go: parserc.go:yaml_parser_parse_flow_sequence_entry_mapping_value
//
// Parse the productions:
// flow_sequence_entry  ::= flow_node | KEY flow_node? (VALUE flow_node?)?
//                                                      ***** *
fn yaml_parser_parse_flow_sequence_entry_mapping_value(
    parser: &mut Parser,
    event: &mut Event,
) -> bool {
    let Some(token) = peek_token(parser) else {
        return false;
    };
    if token.typ == TokenType::Value {
        skip_token(parser);
        let Some(token) = peek_token(parser) else {
            return false;
        };
        if token.typ != TokenType::FlowEntry && token.typ != TokenType::FlowSequenceEnd {
            parser.states.push(ParserState::FlowSequenceEntryMappingEnd);
            return yaml_parser_parse_node(parser, event, false, false);
        }
    }
    // Note: like Go, this uses the token peeked *before* the VALUE token
    // was skipped (the inner `token` shadows only inside the if).
    parser.state = ParserState::FlowSequenceEntryMappingEnd;
    yaml_parser_process_empty_scalar(parser, event, token.start_mark)
}

// Go: parserc.go:yaml_parser_parse_flow_sequence_entry_mapping_end
//
// Parse the productions:
// flow_sequence_entry  ::= flow_node | KEY flow_node? (VALUE flow_node?)?
//                                                                      *
fn yaml_parser_parse_flow_sequence_entry_mapping_end(
    parser: &mut Parser,
    event: &mut Event,
) -> bool {
    let Some(token) = peek_token(parser) else {
        return false;
    };
    parser.state = ParserState::FlowSequenceEntry;
    *event = Event {
        typ: EventType::MappingEnd,
        start_mark: token.start_mark,
        end_mark: token.start_mark, // [Go] Shouldn't this be end_mark?
        ..Default::default()
    };
    true
}

// Go: parserc.go:yaml_parser_parse_flow_mapping_key
//
// Parse the productions:
// flow_mapping         ::= FLOW-MAPPING-START
//                          ******************
//                          (flow_mapping_entry FLOW-ENTRY)*
//                           *                  **********
//                          flow_mapping_entry?
//                          ******************
//                          FLOW-MAPPING-END
//                          ****************
// flow_mapping_entry   ::= flow_node | KEY flow_node? (VALUE flow_node?)?
//                          *           *** *
fn yaml_parser_parse_flow_mapping_key(parser: &mut Parser, event: &mut Event, first: bool) -> bool {
    if first {
        let Some(token) = peek_token(parser) else {
            return false;
        };
        parser.marks.push(token.start_mark);
        skip_token(parser);
    }

    let Some(mut token) = peek_token(parser) else {
        return false;
    };

    if token.typ != TokenType::FlowMappingEnd {
        if !first {
            if token.typ == TokenType::FlowEntry {
                skip_token(parser);
                match peek_token(parser) {
                    Some(t) => token = t,
                    None => return false,
                }
            } else {
                let context_mark = pop_mark(parser);
                return yaml_parser_set_parser_error_context(
                    parser,
                    "while parsing a flow mapping",
                    context_mark,
                    "did not find expected ',' or '}'",
                    token.start_mark,
                );
            }
        }

        if token.typ == TokenType::Key {
            skip_token(parser);
            match peek_token(parser) {
                Some(t) => token = t,
                None => return false,
            }
            if token.typ != TokenType::Value
                && token.typ != TokenType::FlowEntry
                && token.typ != TokenType::FlowMappingEnd
            {
                parser.states.push(ParserState::FlowMappingValue);
                return yaml_parser_parse_node(parser, event, false, false);
            } else {
                parser.state = ParserState::FlowMappingValue;
                return yaml_parser_process_empty_scalar(parser, event, token.start_mark);
            }
        } else if token.typ != TokenType::FlowMappingEnd {
            parser.states.push(ParserState::FlowMappingEmptyValue);
            return yaml_parser_parse_node(parser, event, false, false);
        }
    }

    pop_state(parser);
    pop_mark(parser);
    *event = Event {
        typ: EventType::MappingEnd,
        start_mark: token.start_mark,
        end_mark: token.end_mark,
        ..Default::default()
    };
    skip_token(parser);
    true
}

// Go: parserc.go:yaml_parser_parse_flow_mapping_value
//
// Parse the productions:
// flow_mapping_entry   ::= flow_node | KEY flow_node? (VALUE flow_node?)?
//                                   *                  ***** *
fn yaml_parser_parse_flow_mapping_value(
    parser: &mut Parser,
    event: &mut Event,
    empty: bool,
) -> bool {
    let Some(mut token) = peek_token(parser) else {
        return false;
    };
    if empty {
        parser.state = ParserState::FlowMappingKey;
        return yaml_parser_process_empty_scalar(parser, event, token.start_mark);
    }
    if token.typ == TokenType::Value {
        skip_token(parser);
        match peek_token(parser) {
            Some(t) => token = t,
            None => return false,
        }
        if token.typ != TokenType::FlowEntry && token.typ != TokenType::FlowMappingEnd {
            parser.states.push(ParserState::FlowMappingKey);
            return yaml_parser_parse_node(parser, event, false, false);
        }
    }
    parser.state = ParserState::FlowMappingKey;
    yaml_parser_process_empty_scalar(parser, event, token.start_mark)
}

// Go: parserc.go:yaml_parser_process_empty_scalar
//
// Generate an empty scalar event.
fn yaml_parser_process_empty_scalar(_parser: &mut Parser, event: &mut Event, mark: Mark) -> bool {
    *event = Event {
        typ: EventType::Scalar,
        start_mark: mark,
        end_mark: mark,
        value: Vec::new(), // Empty
        implicit: true,
        ..Default::default()
    };
    true
}

// Go: parserc.go:default_tag_directives
fn default_tag_directives() -> [TagDirective; 2] {
    [
        TagDirective {
            handle: b"!".to_vec(),
            prefix: b"!".to_vec(),
        },
        TagDirective {
            handle: b"!!".to_vec(),
            prefix: b"tag:yaml.org,2002:".to_vec(),
        },
    ]
}

// Go: parserc.go:yaml_parser_process_directives
//
// Parse directives. The version directive and the tag directive list are
// not returned: the decoder does not use them (Go passes nil refs for the
// implicit document and ignores them for explicit ones).
fn yaml_parser_process_directives(parser: &mut Parser, _explicit: bool) -> bool {
    let mut version_directive: Option<(i8, i8)> = None;

    let Some(mut token) = peek_token(parser) else {
        return false;
    };

    while token.typ == TokenType::VersionDirective || token.typ == TokenType::TagDirective {
        if token.typ == TokenType::VersionDirective {
            if version_directive.is_some() {
                yaml_parser_set_parser_error(
                    parser,
                    "found duplicate %YAML directive",
                    token.start_mark,
                );
                return false;
            }
            if token.major != 1 || token.minor != 1 {
                yaml_parser_set_parser_error(
                    parser,
                    "found incompatible YAML document",
                    token.start_mark,
                );
                return false;
            }
            version_directive = Some((token.major, token.minor));
        } else if token.typ == TokenType::TagDirective {
            let value = TagDirective {
                handle: head_value(parser),
                prefix: head_prefix(parser),
            };
            if !yaml_parser_append_tag_directive(parser, value, false, token.start_mark) {
                return false;
            }
        }

        skip_token(parser);
        match peek_token(parser) {
            Some(t) => token = t,
            None => return false,
        }
    }

    for td in default_tag_directives() {
        if !yaml_parser_append_tag_directive(parser, td, true, token.start_mark) {
            return false;
        }
    }

    true
}

// Go: parserc.go:yaml_parser_append_tag_directive
//
// Append a tag directive to the directives stack.
fn yaml_parser_append_tag_directive(
    parser: &mut Parser,
    value: TagDirective,
    allow_duplicates: bool,
    mark: Mark,
) -> bool {
    for td in &parser.tag_directives {
        if value.handle == td.handle {
            if allow_duplicates {
                return true;
            }
            return yaml_parser_set_parser_error(parser, "found duplicate %TAG directive", mark);
        }
    }

    parser.tag_directives.push(value);
    true
}
