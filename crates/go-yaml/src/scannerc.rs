//! The scanner: turns the character stream into tokens.
//!
//! Go: gopkg.in/yaml.v2@v2.4.0 scannerc.go. Functions keep their Go names and
//! control flow. Buffer reads use the full backing store of the working
//! buffer (Go reads `buffer[:len]`; Go would panic on a read past `len`,
//! which the v2.4.0 code never does).

use crate::readerc::yaml_parser_update_buffer;
use crate::yamlh::*;

/// Go: `if parser.unread < n && !yaml_parser_update_buffer(parser, n) { return false }`.
macro_rules! cache {
    ($parser:expr, $n:expr) => {
        if $parser.unread < $n && !yaml_parser_update_buffer($parser, $n) {
            return false;
        }
    };
}

// Go: scannerc.go:skip
fn skip(parser: &mut Parser) {
    parser.mark.index += 1;
    parser.mark.column += 1;
    parser.unread -= 1;
    parser.buffer_pos += width(parser.buffer[parser.buffer_pos]);
}

// Go: scannerc.go:skip_line
fn skip_line(parser: &mut Parser) {
    if is_crlf(&parser.buffer, parser.buffer_pos) {
        parser.mark.index += 2;
        parser.mark.column = 0;
        parser.mark.line += 1;
        parser.unread -= 2;
        parser.buffer_pos += 2;
    } else if is_break(&parser.buffer, parser.buffer_pos) {
        parser.mark.index += 1;
        parser.mark.column = 0;
        parser.mark.line += 1;
        parser.unread -= 1;
        parser.buffer_pos += width(parser.buffer[parser.buffer_pos]);
    }
}

// Go: scannerc.go:read
//
// Copy a character to a string buffer and advance pointers.
fn read(parser: &mut Parser, s: &mut Vec<u8>) {
    let w = width(parser.buffer[parser.buffer_pos]);
    // Go: `if w == 0 { panic("invalid character sequence") }` — the reader
    // only stores valid UTF-8, so this cannot happen.
    let w = w.max(1);
    s.extend_from_slice(&parser.buffer[parser.buffer_pos..parser.buffer_pos + w]);
    parser.buffer_pos += w;
    parser.mark.index += 1;
    parser.mark.column += 1;
    parser.unread -= 1;
}

// Go: scannerc.go:read_line
//
// Copy a line break character to a string buffer and advance pointers.
fn read_line(parser: &mut Parser, s: &mut Vec<u8>) {
    let buf = &parser.buffer;
    let pos = parser.buffer_pos;
    if buf[pos] == b'\r' && buf[pos + 1] == b'\n' {
        // CR LF . LF
        s.push(b'\n');
        parser.buffer_pos += 2;
        parser.mark.index += 1;
        parser.unread -= 1;
    } else if buf[pos] == b'\r' || buf[pos] == b'\n' {
        // CR|LF . LF
        s.push(b'\n');
        parser.buffer_pos += 1;
    } else if buf[pos] == 0xC2 && buf[pos + 1] == 0x85 {
        // NEL . LF
        s.push(b'\n');
        parser.buffer_pos += 2;
    } else if buf[pos] == 0xE2
        && buf[pos + 1] == 0x80
        && (buf[pos + 2] == 0xA8 || buf[pos + 2] == 0xA9)
    {
        // LS|PS . LS|PS
        s.extend_from_slice(&buf[pos..pos + 3]);
        parser.buffer_pos += 3;
    } else {
        return;
    }
    parser.mark.index += 1;
    parser.mark.column = 0;
    parser.mark.line += 1;
    parser.unread -= 1;
}

// Go: scannerc.go:yaml_parser_set_scanner_error
fn yaml_parser_set_scanner_error(
    parser: &mut Parser,
    context: &str,
    context_mark: Mark,
    problem: &str,
) -> bool {
    parser.error = ErrorType::Scanner;
    parser.context = context.to_string();
    parser.context_mark = context_mark;
    parser.problem = problem.to_string();
    parser.problem_mark = parser.mark;
    false
}

// Go: scannerc.go:yaml_parser_set_scanner_tag_error
fn yaml_parser_set_scanner_tag_error(
    parser: &mut Parser,
    directive: bool,
    context_mark: Mark,
    problem: &str,
) -> bool {
    let context = if directive {
        "while parsing a %TAG directive"
    } else {
        "while parsing a tag"
    };
    yaml_parser_set_scanner_error(parser, context, context_mark, problem)
}

// Go: scannerc.go:yaml_parser_fetch_more_tokens
//
// Ensure that the tokens queue contains at least one token which can be
// returned to the Parser.
pub(crate) fn yaml_parser_fetch_more_tokens(parser: &mut Parser) -> bool {
    // While we need more tokens to fetch, do it.
    loop {
        if parser.tokens_head != parser.tokens.len() {
            // If queue is non-empty, check if any potential simple key may
            // occupy the head position.
            match parser.simple_keys_by_tok.get(&parser.tokens_parsed) {
                None => break,
                Some(&head_tok_idx) => {
                    let (valid, ok) = yaml_simple_key_is_valid(parser, head_tok_idx);
                    if !ok {
                        return false;
                    } else if !valid {
                        break;
                    }
                }
            }
        }
        // Fetch the next token.
        if !yaml_parser_fetch_next_token(parser) {
            return false;
        }
    }

    parser.token_available = true;
    true
}

// Go: scannerc.go:yaml_parser_fetch_next_token
//
// The dispatcher for token fetchers.
fn yaml_parser_fetch_next_token(parser: &mut Parser) -> bool {
    // Ensure that the buffer is initialized.
    cache!(parser, 1);

    // Check if we just started scanning.  Fetch STREAM-START then.
    if !parser.stream_start_produced {
        return yaml_parser_fetch_stream_start(parser);
    }

    // Eat whitespaces and comments until we reach the next token.
    if !yaml_parser_scan_to_next_token(parser) {
        return false;
    }

    // Check the indentation level against the current column.
    let column = parser.mark.column;
    if !yaml_parser_unroll_indent(parser, column) {
        return false;
    }

    // Ensure that the buffer contains at least 4 characters.  4 is the length
    // of the longest indicators ('--- ' and '... ').
    cache!(parser, 4);

    let pos = parser.buffer_pos;

    // Is it the end of the stream?
    if is_z(&parser.buffer, pos) {
        return yaml_parser_fetch_stream_end(parser);
    }

    // Is it a directive?
    if parser.mark.column == 0 && parser.buffer[pos] == b'%' {
        return yaml_parser_fetch_directive(parser);
    }

    let buf = &parser.buffer;

    // Is it the document start indicator?
    if parser.mark.column == 0
        && buf[pos] == b'-'
        && buf[pos + 1] == b'-'
        && buf[pos + 2] == b'-'
        && is_blankz(buf, pos + 3)
    {
        return yaml_parser_fetch_document_indicator(parser, TokenType::DocumentStart);
    }

    // Is it the document end indicator?
    if parser.mark.column == 0
        && buf[pos] == b'.'
        && buf[pos + 1] == b'.'
        && buf[pos + 2] == b'.'
        && is_blankz(buf, pos + 3)
    {
        return yaml_parser_fetch_document_indicator(parser, TokenType::DocumentEnd);
    }

    let c = buf[pos];

    // Is it the flow sequence start indicator?
    if c == b'[' {
        return yaml_parser_fetch_flow_collection_start(parser, TokenType::FlowSequenceStart);
    }

    // Is it the flow mapping start indicator?
    if c == b'{' {
        return yaml_parser_fetch_flow_collection_start(parser, TokenType::FlowMappingStart);
    }

    // Is it the flow sequence end indicator?
    if c == b']' {
        return yaml_parser_fetch_flow_collection_end(parser, TokenType::FlowSequenceEnd);
    }

    // Is it the flow mapping end indicator?
    if c == b'}' {
        return yaml_parser_fetch_flow_collection_end(parser, TokenType::FlowMappingEnd);
    }

    // Is it the flow entry indicator?
    if c == b',' {
        return yaml_parser_fetch_flow_entry(parser);
    }

    // Is it the block entry indicator?
    if c == b'-' && is_blankz(buf, pos + 1) {
        return yaml_parser_fetch_block_entry(parser);
    }

    // Is it the key indicator?
    if c == b'?' && (parser.flow_level > 0 || is_blankz(buf, pos + 1)) {
        return yaml_parser_fetch_key(parser);
    }

    // Is it the value indicator?
    if c == b':' && (parser.flow_level > 0 || is_blankz(buf, pos + 1)) {
        return yaml_parser_fetch_value(parser);
    }

    // Is it an alias?
    if c == b'*' {
        return yaml_parser_fetch_anchor(parser, TokenType::Alias);
    }

    // Is it an anchor?
    if c == b'&' {
        return yaml_parser_fetch_anchor(parser, TokenType::Anchor);
    }

    // Is it a tag?
    if c == b'!' {
        return yaml_parser_fetch_tag(parser);
    }

    // Is it a literal scalar?
    if c == b'|' && parser.flow_level == 0 {
        return yaml_parser_fetch_block_scalar(parser, true);
    }

    // Is it a folded scalar?
    if c == b'>' && parser.flow_level == 0 {
        return yaml_parser_fetch_block_scalar(parser, false);
    }

    // Is it a single-quoted scalar?
    if c == b'\'' {
        return yaml_parser_fetch_flow_scalar(parser, true);
    }

    // Is it a double-quoted scalar?
    if c == b'"' {
        return yaml_parser_fetch_flow_scalar(parser, false);
    }

    // Is it a plain scalar?
    //
    // A plain scalar may start with any non-blank characters except
    //
    //      '-', '?', ':', ',', '[', ']', '{', '}',
    //      '#', '&', '*', '!', '|', '>', '\'', '\"',
    //      '%', '@', '`'.
    //
    // In the block context (and, for the '-' indicator, in the flow context
    // too), it may also start with the characters
    //
    //      '-', '?', ':'
    //
    // if it is followed by a non-space character.
    //
    // The last rule is more restrictive than the specification requires.
    if !(is_blankz(buf, pos)
        || c == b'-'
        || c == b'?'
        || c == b':'
        || c == b','
        || c == b'['
        || c == b']'
        || c == b'{'
        || c == b'}'
        || c == b'#'
        || c == b'&'
        || c == b'*'
        || c == b'!'
        || c == b'|'
        || c == b'>'
        || c == b'\''
        || c == b'"'
        || c == b'%'
        || c == b'@'
        || c == b'`')
        || (c == b'-' && !is_blank(buf, pos + 1))
        || (parser.flow_level == 0 && (c == b'?' || c == b':') && !is_blankz(buf, pos + 1))
    {
        return yaml_parser_fetch_plain_scalar(parser);
    }

    // If we don't determine the token type so far, it is an error.
    let mark = parser.mark;
    yaml_parser_set_scanner_error(
        parser,
        "while scanning for the next token",
        mark,
        "found character that cannot start any token",
    )
}

// Go: scannerc.go:yaml_simple_key_is_valid
fn yaml_simple_key_is_valid(parser: &mut Parser, idx: usize) -> (bool, bool) {
    let simple_key = parser.simple_keys[idx];
    if !simple_key.possible {
        return (false, true);
    }

    // The 1.2 specification says:
    //
    //     "If the ? indicator is omitted, parsing needs to see past the
    //     implicit key to recognize it as such. To limit the amount of
    //     lookahead required, the “:” indicator must appear at most 1024
    //     Unicode characters beyond the start of the key. In addition, the key
    //     is restricted to a single line."
    //
    if simple_key.mark.line < parser.mark.line || simple_key.mark.index + 1024 < parser.mark.index {
        // Check if the potential simple key to be removed is required.
        if simple_key.required {
            return (
                false,
                yaml_parser_set_scanner_error(
                    parser,
                    "while scanning a simple key",
                    simple_key.mark,
                    "could not find expected ':'",
                ),
            );
        }
        parser.simple_keys[idx].possible = false;
        return (false, true);
    }
    (true, true)
}

// Go: scannerc.go:yaml_parser_save_simple_key
//
// Check if a simple key may start at the current position and add it if
// needed.
fn yaml_parser_save_simple_key(parser: &mut Parser) -> bool {
    // A simple key is required at the current position if the scanner is in
    // the block context and the current column coincides with the indentation
    // level.
    let required = parser.flow_level == 0 && parser.indent == parser.mark.column;

    //
    // If the current position may start a simple key, save it.
    //
    if parser.simple_key_allowed {
        let simple_key = SimpleKey {
            possible: true,
            required,
            token_number: parser.tokens_parsed + (parser.tokens.len() - parser.tokens_head) as i64,
            mark: parser.mark,
        };

        if !yaml_parser_remove_simple_key(parser) {
            return false;
        }
        let last = parser.simple_keys.len() - 1;
        parser.simple_keys[last] = simple_key;
        parser
            .simple_keys_by_tok
            .insert(simple_key.token_number, last);
    }
    true
}

// Go: scannerc.go:yaml_parser_remove_simple_key
//
// Remove a potential simple key at the current flow level.
fn yaml_parser_remove_simple_key(parser: &mut Parser) -> bool {
    let i = parser.simple_keys.len() - 1;
    if parser.simple_keys[i].possible {
        // If the key is required, it is an error.
        if parser.simple_keys[i].required {
            let mark = parser.simple_keys[i].mark;
            return yaml_parser_set_scanner_error(
                parser,
                "while scanning a simple key",
                mark,
                "could not find expected ':'",
            );
        }
        // Remove the key from the stack.
        parser.simple_keys[i].possible = false;
        let tn = parser.simple_keys[i].token_number;
        parser.simple_keys_by_tok.remove(&tn);
    }
    true
}

/// max_flow_level limits the flow_level. Go: max_flow_level.
const MAX_FLOW_LEVEL: i64 = 10000;

// Go: scannerc.go:yaml_parser_increase_flow_level
//
// Increase the flow level and resize the simple key list if needed.
fn yaml_parser_increase_flow_level(parser: &mut Parser) -> bool {
    // Reset the simple key on the next level.
    parser.simple_keys.push(SimpleKey {
        possible: false,
        required: false,
        token_number: parser.tokens_parsed + (parser.tokens.len() - parser.tokens_head) as i64,
        mark: parser.mark,
    });

    // Increase the flow level.
    parser.flow_level += 1;
    if parser.flow_level > MAX_FLOW_LEVEL {
        let mark = parser.simple_keys[parser.simple_keys.len() - 1].mark;
        return yaml_parser_set_scanner_error(
            parser,
            "while increasing flow level",
            mark,
            &format!("exceeded max depth of {MAX_FLOW_LEVEL}"),
        );
    }
    true
}

// Go: scannerc.go:yaml_parser_decrease_flow_level
fn yaml_parser_decrease_flow_level(parser: &mut Parser) -> bool {
    if parser.flow_level > 0 {
        parser.flow_level -= 1;
        let last = parser.simple_keys.len() - 1;
        let tn = parser.simple_keys[last].token_number;
        parser.simple_keys_by_tok.remove(&tn);
        parser.simple_keys.truncate(last);
    }
    true
}

/// max_indents limits the indents stack size. Go: max_indents.
const MAX_INDENTS: usize = 10000;

// Go: scannerc.go:yaml_parser_roll_indent
//
// Push the current indentation level to the stack and set the new level
// the current column is greater than the indentation level.  In this case,
// append or insert the specified token into the token queue.
fn yaml_parser_roll_indent(
    parser: &mut Parser,
    column: i64,
    number: i64,
    typ: TokenType,
    mark: Mark,
) -> bool {
    // In the flow context, do nothing.
    if parser.flow_level > 0 {
        return true;
    }

    if parser.indent < column {
        // Push the current indentation level to the stack and set the new
        // indentation level.
        parser.indents.push(parser.indent);
        parser.indent = column;
        if parser.indents.len() > MAX_INDENTS {
            let m = parser.simple_keys[parser.simple_keys.len() - 1].mark;
            return yaml_parser_set_scanner_error(
                parser,
                "while increasing indent level",
                m,
                &format!("exceeded max depth of {MAX_INDENTS}"),
            );
        }

        // Create a token and insert it into the queue.
        let token = Token {
            typ,
            start_mark: mark,
            end_mark: mark,
            ..Default::default()
        };
        let mut number = number;
        if number > -1 {
            number -= parser.tokens_parsed;
        }
        yaml_insert_token(parser, number, token);
    }
    true
}

// Go: scannerc.go:yaml_parser_unroll_indent
//
// Pop indentation levels from the indents stack until the current level
// becomes less or equal to the column.  For each indentation level, append
// the BLOCK-END token.
fn yaml_parser_unroll_indent(parser: &mut Parser, column: i64) -> bool {
    // In the flow context, do nothing.
    if parser.flow_level > 0 {
        return true;
    }

    // Loop through the indentation levels in the stack.
    while parser.indent > column {
        // Create a token and append it to the queue.
        let token = Token {
            typ: TokenType::BlockEnd,
            start_mark: parser.mark,
            end_mark: parser.mark,
            ..Default::default()
        };
        yaml_insert_token(parser, -1, token);

        // Pop the indentation level.
        parser.indent = parser.indents.pop().unwrap_or(-1);
    }
    true
}

// Go: scannerc.go:yaml_parser_fetch_stream_start
//
// Initialize the scanner and produce the STREAM-START token.
fn yaml_parser_fetch_stream_start(parser: &mut Parser) -> bool {
    // Set the initial indentation.
    parser.indent = -1;

    // Initialize the simple key stack.
    parser.simple_keys.push(SimpleKey::default());

    parser.simple_keys_by_tok = Default::default();

    // A simple key is allowed at the beginning of the stream.
    parser.simple_key_allowed = true;

    // We have started.
    parser.stream_start_produced = true;

    // Create the STREAM-START token and append it to the queue.
    let token = Token {
        typ: TokenType::StreamStart,
        start_mark: parser.mark,
        end_mark: parser.mark,
        encoding: parser.encoding,
        ..Default::default()
    };
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_stream_end
//
// Produce the STREAM-END token and shut down the scanner.
fn yaml_parser_fetch_stream_end(parser: &mut Parser) -> bool {
    // Force new line.
    if parser.mark.column != 0 {
        parser.mark.column = 0;
        parser.mark.line += 1;
    }

    // Reset the indentation level.
    if !yaml_parser_unroll_indent(parser, -1) {
        return false;
    }

    // Reset simple keys.
    if !yaml_parser_remove_simple_key(parser) {
        return false;
    }

    parser.simple_key_allowed = false;

    // Create the STREAM-END token and append it to the queue.
    let token = Token {
        typ: TokenType::StreamEnd,
        start_mark: parser.mark,
        end_mark: parser.mark,
        ..Default::default()
    };
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_directive
//
// Produce a VERSION-DIRECTIVE or TAG-DIRECTIVE token.
fn yaml_parser_fetch_directive(parser: &mut Parser) -> bool {
    // Reset the indentation level.
    if !yaml_parser_unroll_indent(parser, -1) {
        return false;
    }

    // Reset simple keys.
    if !yaml_parser_remove_simple_key(parser) {
        return false;
    }

    parser.simple_key_allowed = false;

    // Create the YAML-DIRECTIVE or TAG-DIRECTIVE token.
    let mut token = Token::default();
    if !yaml_parser_scan_directive(parser, &mut token) {
        return false;
    }
    // Append the token to the queue.
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_document_indicator
//
// Produce the DOCUMENT-START or DOCUMENT-END token.
fn yaml_parser_fetch_document_indicator(parser: &mut Parser, typ: TokenType) -> bool {
    // Reset the indentation level.
    if !yaml_parser_unroll_indent(parser, -1) {
        return false;
    }

    // Reset simple keys.
    if !yaml_parser_remove_simple_key(parser) {
        return false;
    }

    parser.simple_key_allowed = false;

    // Consume the token.
    let start_mark = parser.mark;

    skip(parser);
    skip(parser);
    skip(parser);

    let end_mark = parser.mark;

    // Create the DOCUMENT-START or DOCUMENT-END token.
    let token = Token {
        typ,
        start_mark,
        end_mark,
        ..Default::default()
    };
    // Append the token to the queue.
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_flow_collection_start
//
// Produce the FLOW-SEQUENCE-START or FLOW-MAPPING-START token.
fn yaml_parser_fetch_flow_collection_start(parser: &mut Parser, typ: TokenType) -> bool {
    // The indicators '[' and '{' may start a simple key.
    if !yaml_parser_save_simple_key(parser) {
        return false;
    }

    // Increase the flow level.
    if !yaml_parser_increase_flow_level(parser) {
        return false;
    }

    // A simple key may follow the indicators '[' and '{'.
    parser.simple_key_allowed = true;

    // Consume the token.
    let start_mark = parser.mark;
    skip(parser);
    let end_mark = parser.mark;

    // Create the FLOW-SEQUENCE-START of FLOW-MAPPING-START token.
    let token = Token {
        typ,
        start_mark,
        end_mark,
        ..Default::default()
    };
    // Append the token to the queue.
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_flow_collection_end
//
// Produce the FLOW-SEQUENCE-END or FLOW-MAPPING-END token.
fn yaml_parser_fetch_flow_collection_end(parser: &mut Parser, typ: TokenType) -> bool {
    // Reset any potential simple key on the current flow level.
    if !yaml_parser_remove_simple_key(parser) {
        return false;
    }

    // Decrease the flow level.
    if !yaml_parser_decrease_flow_level(parser) {
        return false;
    }

    // No simple keys after the indicators ']' and '}'.
    parser.simple_key_allowed = false;

    // Consume the token.

    let start_mark = parser.mark;
    skip(parser);
    let end_mark = parser.mark;

    // Create the FLOW-SEQUENCE-END of FLOW-MAPPING-END token.
    let token = Token {
        typ,
        start_mark,
        end_mark,
        ..Default::default()
    };
    // Append the token to the queue.
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_flow_entry
//
// Produce the FLOW-ENTRY token.
fn yaml_parser_fetch_flow_entry(parser: &mut Parser) -> bool {
    // Reset any potential simple keys on the current flow level.
    if !yaml_parser_remove_simple_key(parser) {
        return false;
    }

    // Simple keys are allowed after ','.
    parser.simple_key_allowed = true;

    // Consume the token.
    let start_mark = parser.mark;
    skip(parser);
    let end_mark = parser.mark;

    // Create the FLOW-ENTRY token and append it to the queue.
    let token = Token {
        typ: TokenType::FlowEntry,
        start_mark,
        end_mark,
        ..Default::default()
    };
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_block_entry
//
// Produce the BLOCK-ENTRY token.
fn yaml_parser_fetch_block_entry(parser: &mut Parser) -> bool {
    // Check if the scanner is in the block context.
    if parser.flow_level == 0 {
        // Check if we are allowed to start a new entry.
        if !parser.simple_key_allowed {
            let mark = parser.mark;
            return yaml_parser_set_scanner_error(
                parser,
                "",
                mark,
                "block sequence entries are not allowed in this context",
            );
        }
        // Add the BLOCK-SEQUENCE-START token if needed.
        let (column, mark) = (parser.mark.column, parser.mark);
        if !yaml_parser_roll_indent(parser, column, -1, TokenType::BlockSequenceStart, mark) {
            return false;
        }
    } else {
        // It is an error for the '-' indicator to occur in the flow context,
        // but we let the Parser detect and report about it because the Parser
        // is able to point to the context.
    }

    // Reset any potential simple keys on the current flow level.
    if !yaml_parser_remove_simple_key(parser) {
        return false;
    }

    // Simple keys are allowed after '-'.
    parser.simple_key_allowed = true;

    // Consume the token.
    let start_mark = parser.mark;
    skip(parser);
    let end_mark = parser.mark;

    // Create the BLOCK-ENTRY token and append it to the queue.
    let token = Token {
        typ: TokenType::BlockEntry,
        start_mark,
        end_mark,
        ..Default::default()
    };
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_key
//
// Produce the KEY token.
fn yaml_parser_fetch_key(parser: &mut Parser) -> bool {
    // In the block context, additional checks are required.
    if parser.flow_level == 0 {
        // Check if we are allowed to start a new key (not nessesary simple).
        if !parser.simple_key_allowed {
            let mark = parser.mark;
            return yaml_parser_set_scanner_error(
                parser,
                "",
                mark,
                "mapping keys are not allowed in this context",
            );
        }
        // Add the BLOCK-MAPPING-START token if needed.
        let (column, mark) = (parser.mark.column, parser.mark);
        if !yaml_parser_roll_indent(parser, column, -1, TokenType::BlockMappingStart, mark) {
            return false;
        }
    }

    // Reset any potential simple keys on the current flow level.
    if !yaml_parser_remove_simple_key(parser) {
        return false;
    }

    // Simple keys are allowed after '?' in the block context.
    parser.simple_key_allowed = parser.flow_level == 0;

    // Consume the token.
    let start_mark = parser.mark;
    skip(parser);
    let end_mark = parser.mark;

    // Create the KEY token and append it to the queue.
    let token = Token {
        typ: TokenType::Key,
        start_mark,
        end_mark,
        ..Default::default()
    };
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_value
//
// Produce the VALUE token.
fn yaml_parser_fetch_value(parser: &mut Parser) -> bool {
    let sk_idx = parser.simple_keys.len() - 1;

    // Have we found a simple key?
    let (valid, ok) = yaml_simple_key_is_valid(parser, sk_idx);
    if !ok {
        return false;
    } else if valid {
        let simple_key = parser.simple_keys[sk_idx];

        // Create the KEY token and insert it into the queue.
        let token = Token {
            typ: TokenType::Key,
            start_mark: simple_key.mark,
            end_mark: simple_key.mark,
            ..Default::default()
        };
        let pos = simple_key.token_number - parser.tokens_parsed;
        yaml_insert_token(parser, pos, token);

        // In the block context, we may need to add the BLOCK-MAPPING-START token.
        if !yaml_parser_roll_indent(
            parser,
            simple_key.mark.column,
            simple_key.token_number,
            TokenType::BlockMappingStart,
            simple_key.mark,
        ) {
            return false;
        }

        // Remove the simple key.
        parser.simple_keys[sk_idx].possible = false;
        parser.simple_keys_by_tok.remove(&simple_key.token_number);

        // A simple key cannot follow another simple key.
        parser.simple_key_allowed = false;
    } else {
        // The ':' indicator follows a complex key.

        // In the block context, extra checks are required.
        if parser.flow_level == 0 {
            // Check if we are allowed to start a complex value.
            if !parser.simple_key_allowed {
                let mark = parser.mark;
                return yaml_parser_set_scanner_error(
                    parser,
                    "",
                    mark,
                    "mapping values are not allowed in this context",
                );
            }

            // Add the BLOCK-MAPPING-START token if needed.
            let (column, mark) = (parser.mark.column, parser.mark);
            if !yaml_parser_roll_indent(parser, column, -1, TokenType::BlockMappingStart, mark) {
                return false;
            }
        }

        // Simple keys after ':' are allowed in the block context.
        parser.simple_key_allowed = parser.flow_level == 0;
    }

    // Consume the token.
    let start_mark = parser.mark;
    skip(parser);
    let end_mark = parser.mark;

    // Create the VALUE token and append it to the queue.
    let token = Token {
        typ: TokenType::Value,
        start_mark,
        end_mark,
        ..Default::default()
    };
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_anchor
//
// Produce the ALIAS or ANCHOR token.
fn yaml_parser_fetch_anchor(parser: &mut Parser, typ: TokenType) -> bool {
    // An anchor or an alias could be a simple key.
    if !yaml_parser_save_simple_key(parser) {
        return false;
    }

    // A simple key cannot follow an anchor or an alias.
    parser.simple_key_allowed = false;

    // Create the ALIAS or ANCHOR token and append it to the queue.
    let mut token = Token::default();
    if !yaml_parser_scan_anchor(parser, &mut token, typ) {
        return false;
    }
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_tag
//
// Produce the TAG token.
fn yaml_parser_fetch_tag(parser: &mut Parser) -> bool {
    // A tag could be a simple key.
    if !yaml_parser_save_simple_key(parser) {
        return false;
    }

    // A simple key cannot follow a tag.
    parser.simple_key_allowed = false;

    // Create the TAG token and append it to the queue.
    let mut token = Token::default();
    if !yaml_parser_scan_tag(parser, &mut token) {
        return false;
    }
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_block_scalar
//
// Produce the SCALAR(...,literal) or SCALAR(...,folded) tokens.
fn yaml_parser_fetch_block_scalar(parser: &mut Parser, literal: bool) -> bool {
    // Remove any potential simple keys.
    if !yaml_parser_remove_simple_key(parser) {
        return false;
    }

    // A simple key may follow a block scalar.
    parser.simple_key_allowed = true;

    // Create the SCALAR token and append it to the queue.
    let mut token = Token::default();
    if !yaml_parser_scan_block_scalar(parser, &mut token, literal) {
        return false;
    }
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_flow_scalar
//
// Produce the SCALAR(...,single-quoted) or SCALAR(...,double-quoted) tokens.
fn yaml_parser_fetch_flow_scalar(parser: &mut Parser, single: bool) -> bool {
    // A plain scalar could be a simple key.
    if !yaml_parser_save_simple_key(parser) {
        return false;
    }

    // A simple key cannot follow a flow scalar.
    parser.simple_key_allowed = false;

    // Create the SCALAR token and append it to the queue.
    let mut token = Token::default();
    if !yaml_parser_scan_flow_scalar(parser, &mut token, single) {
        return false;
    }
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_fetch_plain_scalar
//
// Produce the SCALAR(...,plain) token.
fn yaml_parser_fetch_plain_scalar(parser: &mut Parser) -> bool {
    // A plain scalar could be a simple key.
    if !yaml_parser_save_simple_key(parser) {
        return false;
    }

    // A simple key cannot follow a flow scalar.
    parser.simple_key_allowed = false;

    // Create the SCALAR token and append it to the queue.
    let mut token = Token::default();
    if !yaml_parser_scan_plain_scalar(parser, &mut token) {
        return false;
    }
    yaml_insert_token(parser, -1, token);
    true
}

// Go: scannerc.go:yaml_parser_scan_to_next_token
//
// Eat whitespaces and comments until the next token is found.
fn yaml_parser_scan_to_next_token(parser: &mut Parser) -> bool {
    // Until the next token is not found.
    loop {
        // Allow the BOM mark to start a line.
        cache!(parser, 1);
        if parser.mark.column == 0 && is_bom(&parser.buffer, parser.buffer_pos) {
            skip(parser);
        }

        // Eat whitespaces.
        // Tabs are allowed:
        //  - in the flow context
        //  - in the block context, but not at the beginning of the line or
        //  after '-', '?', or ':' (complex value).
        cache!(parser, 1);

        while parser.buffer[parser.buffer_pos] == b' '
            || ((parser.flow_level > 0 || !parser.simple_key_allowed)
                && parser.buffer[parser.buffer_pos] == b'\t')
        {
            skip(parser);
            cache!(parser, 1);
        }

        // Eat a comment until a line break.
        if parser.buffer[parser.buffer_pos] == b'#' {
            while !is_breakz(&parser.buffer, parser.buffer_pos) {
                skip(parser);
                cache!(parser, 1);
            }
        }

        // If it is a line break, eat it.
        if is_break(&parser.buffer, parser.buffer_pos) {
            cache!(parser, 2);
            skip_line(parser);

            // In the block context, a new line may start a simple key.
            if parser.flow_level == 0 {
                parser.simple_key_allowed = true;
            }
        } else {
            break; // We have found a token.
        }
    }

    true
}

// Go: scannerc.go:yaml_parser_scan_directive
//
// Scan a YAML-DIRECTIVE or TAG-DIRECTIVE token.
//
// Scope:
//      %YAML    1.1    # a comment \n
//      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//      %TAG    !yaml!  tag:yaml.org,2002:  \n
//      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
fn yaml_parser_scan_directive(parser: &mut Parser, token: &mut Token) -> bool {
    // Eat '%'.
    let start_mark = parser.mark;
    skip(parser);

    // Scan the directive name.
    let mut name = Vec::new();
    if !yaml_parser_scan_directive_name(parser, start_mark, &mut name) {
        return false;
    }

    // Is it a YAML directive?
    if name == b"YAML" {
        // Scan the VERSION directive value.
        let (mut major, mut minor) = (0i8, 0i8);
        if !yaml_parser_scan_version_directive_value(parser, start_mark, &mut major, &mut minor) {
            return false;
        }
        let end_mark = parser.mark;

        // Create a VERSION-DIRECTIVE token.
        *token = Token {
            typ: TokenType::VersionDirective,
            start_mark,
            end_mark,
            major,
            minor,
            ..Default::default()
        };

        // Is it a TAG directive?
    } else if name == b"TAG" {
        // Scan the TAG directive value.
        let (mut handle, mut prefix) = (Vec::new(), Vec::new());
        if !yaml_parser_scan_tag_directive_value(parser, start_mark, &mut handle, &mut prefix) {
            return false;
        }
        let end_mark = parser.mark;

        // Create a TAG-DIRECTIVE token.
        *token = Token {
            typ: TokenType::TagDirective,
            start_mark,
            end_mark,
            value: handle,
            prefix,
            ..Default::default()
        };

        // Unknown directive.
    } else {
        yaml_parser_set_scanner_error(
            parser,
            "while scanning a directive",
            start_mark,
            "found unknown directive name",
        );
        return false;
    }

    // Eat the rest of the line including any comments.
    cache!(parser, 1);

    while is_blank(&parser.buffer, parser.buffer_pos) {
        skip(parser);
        cache!(parser, 1);
    }

    if parser.buffer[parser.buffer_pos] == b'#' {
        while !is_breakz(&parser.buffer, parser.buffer_pos) {
            skip(parser);
            cache!(parser, 1);
        }
    }

    // Check if we are at the end of the line.
    if !is_breakz(&parser.buffer, parser.buffer_pos) {
        yaml_parser_set_scanner_error(
            parser,
            "while scanning a directive",
            start_mark,
            "did not find expected comment or line break",
        );
        return false;
    }

    // Eat a line break.
    if is_break(&parser.buffer, parser.buffer_pos) {
        cache!(parser, 2);
        skip_line(parser);
    }

    true
}

// Go: scannerc.go:yaml_parser_scan_directive_name
//
// Scan the directive name.
//
// Scope:
//      %YAML   1.1     # a comment \n
//       ^^^^
//      %TAG    !yaml!  tag:yaml.org,2002:  \n
//       ^^^
fn yaml_parser_scan_directive_name(
    parser: &mut Parser,
    start_mark: Mark,
    name: &mut Vec<u8>,
) -> bool {
    // Consume the directive name.
    cache!(parser, 1);

    let mut s = Vec::new();
    while is_alpha(&parser.buffer, parser.buffer_pos) {
        read(parser, &mut s);
        cache!(parser, 1);
    }

    // Check if the name is empty.
    if s.is_empty() {
        yaml_parser_set_scanner_error(
            parser,
            "while scanning a directive",
            start_mark,
            "could not find expected directive name",
        );
        return false;
    }

    // Check for an blank character after the name.
    if !is_blankz(&parser.buffer, parser.buffer_pos) {
        yaml_parser_set_scanner_error(
            parser,
            "while scanning a directive",
            start_mark,
            "found unexpected non-alphabetical character",
        );
        return false;
    }
    *name = s;
    true
}

// Go: scannerc.go:yaml_parser_scan_version_directive_value
//
// Scan the value of VERSION-DIRECTIVE.
//
// Scope:
//      %YAML   1.1     # a comment \n
//           ^^^^^^
fn yaml_parser_scan_version_directive_value(
    parser: &mut Parser,
    start_mark: Mark,
    major: &mut i8,
    minor: &mut i8,
) -> bool {
    // Eat whitespaces.
    cache!(parser, 1);
    while is_blank(&parser.buffer, parser.buffer_pos) {
        skip(parser);
        cache!(parser, 1);
    }

    // Consume the major version number.
    if !yaml_parser_scan_version_directive_number(parser, start_mark, major) {
        return false;
    }

    // Eat '.'.
    if parser.buffer[parser.buffer_pos] != b'.' {
        return yaml_parser_set_scanner_error(
            parser,
            "while scanning a %YAML directive",
            start_mark,
            "did not find expected digit or '.' character",
        );
    }

    skip(parser);

    // Consume the minor version number.
    if !yaml_parser_scan_version_directive_number(parser, start_mark, minor) {
        return false;
    }
    true
}

const MAX_NUMBER_LENGTH: i8 = 2;

// Go: scannerc.go:yaml_parser_scan_version_directive_number
//
// Scan the version number of VERSION-DIRECTIVE.
//
// Scope:
//      %YAML   1.1     # a comment \n
//              ^
//      %YAML   1.1     # a comment \n
//                ^
fn yaml_parser_scan_version_directive_number(
    parser: &mut Parser,
    start_mark: Mark,
    number: &mut i8,
) -> bool {
    // Repeat while the next character is digit.
    cache!(parser, 1);
    let mut value: i8 = 0;
    let mut length: i8 = 0;
    while is_digit(&parser.buffer, parser.buffer_pos) {
        // Check if the number is too long.
        length += 1;
        if length > MAX_NUMBER_LENGTH {
            return yaml_parser_set_scanner_error(
                parser,
                "while scanning a %YAML directive",
                start_mark,
                "found extremely long version number",
            );
        }
        value = value
            .wrapping_mul(10)
            .wrapping_add(as_digit(&parser.buffer, parser.buffer_pos) as i8);
        skip(parser);
        cache!(parser, 1);
    }

    // Check if the number was present.
    if length == 0 {
        return yaml_parser_set_scanner_error(
            parser,
            "while scanning a %YAML directive",
            start_mark,
            "did not find expected version number",
        );
    }
    *number = value;
    true
}

// Go: scannerc.go:yaml_parser_scan_tag_directive_value
//
// Scan the value of a TAG-DIRECTIVE token.
//
// Scope:
//      %TAG    !yaml!  tag:yaml.org,2002:  \n
//          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
fn yaml_parser_scan_tag_directive_value(
    parser: &mut Parser,
    start_mark: Mark,
    handle: &mut Vec<u8>,
    prefix: &mut Vec<u8>,
) -> bool {
    let mut handle_value = Vec::new();
    let mut prefix_value = Vec::new();

    // Eat whitespaces.
    cache!(parser, 1);

    while is_blank(&parser.buffer, parser.buffer_pos) {
        skip(parser);
        cache!(parser, 1);
    }

    // Scan a handle.
    if !yaml_parser_scan_tag_handle(parser, true, start_mark, &mut handle_value) {
        return false;
    }

    // Expect a whitespace.
    cache!(parser, 1);
    if !is_blank(&parser.buffer, parser.buffer_pos) {
        yaml_parser_set_scanner_error(
            parser,
            "while scanning a %TAG directive",
            start_mark,
            "did not find expected whitespace",
        );
        return false;
    }

    // Eat whitespaces.
    while is_blank(&parser.buffer, parser.buffer_pos) {
        skip(parser);
        cache!(parser, 1);
    }

    // Scan a prefix.
    if !yaml_parser_scan_tag_uri(parser, true, None, start_mark, &mut prefix_value) {
        return false;
    }

    // Expect a whitespace or line break.
    cache!(parser, 1);
    if !is_blankz(&parser.buffer, parser.buffer_pos) {
        yaml_parser_set_scanner_error(
            parser,
            "while scanning a %TAG directive",
            start_mark,
            "did not find expected whitespace or line break",
        );
        return false;
    }

    *handle = handle_value;
    *prefix = prefix_value;
    true
}

// Go: scannerc.go:yaml_parser_scan_anchor
fn yaml_parser_scan_anchor(parser: &mut Parser, token: &mut Token, typ: TokenType) -> bool {
    let mut s = Vec::new();

    // Eat the indicator character.
    let start_mark = parser.mark;
    skip(parser);

    // Consume the value.
    cache!(parser, 1);

    while is_alpha(&parser.buffer, parser.buffer_pos) {
        read(parser, &mut s);
        cache!(parser, 1);
    }

    let end_mark = parser.mark;

    /*
     * Check if length of the anchor is greater than 0 and it is followed by
     * a whitespace character or one of the indicators:
     *
     *      '?', ':', ',', ']', '}', '%', '@', '`'.
     */

    let c = parser.buffer[parser.buffer_pos];
    if s.is_empty()
        || !(is_blankz(&parser.buffer, parser.buffer_pos)
            || c == b'?'
            || c == b':'
            || c == b','
            || c == b']'
            || c == b'}'
            || c == b'%'
            || c == b'@'
            || c == b'`')
    {
        let context = if typ == TokenType::Anchor {
            "while scanning an anchor"
        } else {
            "while scanning an alias"
        };
        yaml_parser_set_scanner_error(
            parser,
            context,
            start_mark,
            "did not find expected alphabetic or numeric character",
        );
        return false;
    }

    // Create a token.
    *token = Token {
        typ,
        start_mark,
        end_mark,
        value: s,
        ..Default::default()
    };

    true
}

// Go: scannerc.go:yaml_parser_scan_tag
//
// Scan a TAG token.
fn yaml_parser_scan_tag(parser: &mut Parser, token: &mut Token) -> bool {
    let mut handle: Vec<u8> = Vec::new();
    let mut suffix: Vec<u8> = Vec::new();

    let start_mark = parser.mark;

    // Check if the tag is in the canonical form.
    cache!(parser, 2);

    if parser.buffer[parser.buffer_pos + 1] == b'<' {
        // Keep the handle as ''

        // Eat '!<'
        skip(parser);
        skip(parser);

        // Consume the tag value.
        if !yaml_parser_scan_tag_uri(parser, false, None, start_mark, &mut suffix) {
            return false;
        }

        // Check for '>' and eat it.
        if parser.buffer[parser.buffer_pos] != b'>' {
            yaml_parser_set_scanner_error(
                parser,
                "while scanning a tag",
                start_mark,
                "did not find the expected '>'",
            );
            return false;
        }

        skip(parser);
    } else {
        // The tag has either the '!suffix' or the '!handle!suffix' form.

        // First, try to scan a handle.
        if !yaml_parser_scan_tag_handle(parser, false, start_mark, &mut handle) {
            return false;
        }

        // Check if it is, indeed, handle.
        if handle[0] == b'!' && handle.len() > 1 && handle[handle.len() - 1] == b'!' {
            // Scan the suffix now.
            if !yaml_parser_scan_tag_uri(parser, false, None, start_mark, &mut suffix) {
                return false;
            }
        } else {
            // It wasn't a handle after all.  Scan the rest of the tag.
            let head = handle.clone();
            if !yaml_parser_scan_tag_uri(parser, false, Some(&head), start_mark, &mut suffix) {
                return false;
            }

            // Set the handle to '!'.
            handle = vec![b'!'];

            // A special case: the '!' tag.  Set the handle to '' and the
            // suffix to '!'.
            if suffix.is_empty() {
                std::mem::swap(&mut handle, &mut suffix);
            }
        }
    }

    // Check the character which ends the tag.
    cache!(parser, 1);
    if !is_blankz(&parser.buffer, parser.buffer_pos) {
        yaml_parser_set_scanner_error(
            parser,
            "while scanning a tag",
            start_mark,
            "did not find expected whitespace or line break",
        );
        return false;
    }

    let end_mark = parser.mark;

    // Create a token.
    *token = Token {
        typ: TokenType::Tag,
        start_mark,
        end_mark,
        value: handle,
        suffix,
        ..Default::default()
    };
    true
}

// Go: scannerc.go:yaml_parser_scan_tag_handle
//
// Scan a tag handle.
fn yaml_parser_scan_tag_handle(
    parser: &mut Parser,
    directive: bool,
    start_mark: Mark,
    handle: &mut Vec<u8>,
) -> bool {
    // Check the initial '!' character.
    cache!(parser, 1);
    if parser.buffer[parser.buffer_pos] != b'!' {
        yaml_parser_set_scanner_tag_error(
            parser,
            directive,
            start_mark,
            "did not find expected '!'",
        );
        return false;
    }

    let mut s = Vec::new();

    // Copy the '!' character.
    read(parser, &mut s);

    // Copy all subsequent alphabetical and numerical characters.
    cache!(parser, 1);
    while is_alpha(&parser.buffer, parser.buffer_pos) {
        read(parser, &mut s);
        cache!(parser, 1);
    }

    // Check if the trailing character is '!' and copy it.
    if parser.buffer[parser.buffer_pos] == b'!' {
        read(parser, &mut s);
    } else {
        // It's either the '!' tag or not really a tag handle.  If it's a %TAG
        // directive, it's an error.  If it's a tag token, it must be a part of URI.
        if directive && s != b"!" {
            yaml_parser_set_scanner_tag_error(
                parser,
                directive,
                start_mark,
                "did not find expected '!'",
            );
            return false;
        }
    }

    *handle = s;
    true
}

// Go: scannerc.go:yaml_parser_scan_tag_uri
//
// Scan a tag.
fn yaml_parser_scan_tag_uri(
    parser: &mut Parser,
    directive: bool,
    head: Option<&[u8]>,
    start_mark: Mark,
    uri: &mut Vec<u8>,
) -> bool {
    let head = head.unwrap_or(&[]);
    let mut s = Vec::new();
    let mut has_tag = !head.is_empty();

    // Copy the head if needed.
    //
    // Note that we don't copy the leading '!' character.
    if head.len() > 1 {
        s.extend_from_slice(&head[1..]);
    }

    // Scan the tag.
    cache!(parser, 1);

    // The set of characters that may appear in URI is as follows:
    //
    //      '0'-'9', 'A'-'Z', 'a'-'z', '_', '-', ';', '/', '?', ':', '@', '&',
    //      '=', '+', '$', ',', '.', '!', '~', '*', '\'', '(', ')', '[', ']',
    //      '%'.
    loop {
        let c = parser.buffer[parser.buffer_pos];
        if !(is_alpha(&parser.buffer, parser.buffer_pos)
            || c == b';'
            || c == b'/'
            || c == b'?'
            || c == b':'
            || c == b'@'
            || c == b'&'
            || c == b'='
            || c == b'+'
            || c == b'$'
            || c == b','
            || c == b'.'
            || c == b'!'
            || c == b'~'
            || c == b'*'
            || c == b'\''
            || c == b'('
            || c == b')'
            || c == b'['
            || c == b']'
            || c == b'%')
        {
            break;
        }
        // Check if it is a URI-escape sequence.
        if c == b'%' {
            if !yaml_parser_scan_uri_escapes(parser, directive, start_mark, &mut s) {
                return false;
            }
        } else {
            read(parser, &mut s);
        }
        cache!(parser, 1);
        has_tag = true;
    }

    if !has_tag {
        yaml_parser_set_scanner_tag_error(
            parser,
            directive,
            start_mark,
            "did not find expected tag URI",
        );
        return false;
    }
    *uri = s;
    true
}

// Go: scannerc.go:yaml_parser_scan_uri_escapes
//
// Decode an URI-escape sequence corresponding to a single UTF-8 character.
fn yaml_parser_scan_uri_escapes(
    parser: &mut Parser,
    directive: bool,
    start_mark: Mark,
    s: &mut Vec<u8>,
) -> bool {
    // Decode the required number of characters.
    let mut w: i64 = 1024;
    while w > 0 {
        // Check for a URI-escaped octet.
        cache!(parser, 3);

        let pos = parser.buffer_pos;
        if !(parser.buffer[pos] == b'%'
            && is_hex(&parser.buffer, pos + 1)
            && is_hex(&parser.buffer, pos + 2))
        {
            return yaml_parser_set_scanner_tag_error(
                parser,
                directive,
                start_mark,
                "did not find URI escaped octet",
            );
        }

        // Get the octet.
        let octet =
            ((as_hex(&parser.buffer, pos + 1) << 4) + as_hex(&parser.buffer, pos + 2)) as u8;

        // If it is the leading octet, determine the length of the UTF-8 sequence.
        if w == 1024 {
            w = width(octet) as i64;
            if w == 0 {
                return yaml_parser_set_scanner_tag_error(
                    parser,
                    directive,
                    start_mark,
                    "found an incorrect leading UTF-8 octet",
                );
            }
        } else {
            // Check if the trailing octet is correct.
            if octet & 0xC0 != 0x80 {
                return yaml_parser_set_scanner_tag_error(
                    parser,
                    directive,
                    start_mark,
                    "found an incorrect trailing UTF-8 octet",
                );
            }
        }

        // Copy the octet and move the pointers.
        s.push(octet);
        skip(parser);
        skip(parser);
        skip(parser);
        w -= 1;
    }
    true
}

// Go: scannerc.go:yaml_parser_scan_block_scalar
//
// Scan a block scalar.
fn yaml_parser_scan_block_scalar(parser: &mut Parser, token: &mut Token, literal: bool) -> bool {
    // Eat the indicator '|' or '>'.
    let start_mark = parser.mark;
    skip(parser);

    // Scan the additional block scalar indicators.
    cache!(parser, 1);

    // Check for a chomping indicator.
    let mut chomping: i64 = 0;
    let mut increment: i64 = 0;
    if parser.buffer[parser.buffer_pos] == b'+' || parser.buffer[parser.buffer_pos] == b'-' {
        // Set the chomping method and eat the indicator.
        chomping = if parser.buffer[parser.buffer_pos] == b'+' {
            1
        } else {
            -1
        };
        skip(parser);

        // Check for an indentation indicator.
        cache!(parser, 1);
        if is_digit(&parser.buffer, parser.buffer_pos) {
            // Check that the indentation is greater than 0.
            if parser.buffer[parser.buffer_pos] == b'0' {
                yaml_parser_set_scanner_error(
                    parser,
                    "while scanning a block scalar",
                    start_mark,
                    "found an indentation indicator equal to 0",
                );
                return false;
            }

            // Get the indentation level and eat the indicator.
            increment = as_digit(&parser.buffer, parser.buffer_pos);
            skip(parser);
        }
    } else if is_digit(&parser.buffer, parser.buffer_pos) {
        // Do the same as above, but in the opposite order.

        if parser.buffer[parser.buffer_pos] == b'0' {
            yaml_parser_set_scanner_error(
                parser,
                "while scanning a block scalar",
                start_mark,
                "found an indentation indicator equal to 0",
            );
            return false;
        }
        increment = as_digit(&parser.buffer, parser.buffer_pos);
        skip(parser);

        cache!(parser, 1);
        if parser.buffer[parser.buffer_pos] == b'+' || parser.buffer[parser.buffer_pos] == b'-' {
            chomping = if parser.buffer[parser.buffer_pos] == b'+' {
                1
            } else {
                -1
            };
            skip(parser);
        }
    }

    // Eat whitespaces and comments to the end of the line.
    cache!(parser, 1);
    while is_blank(&parser.buffer, parser.buffer_pos) {
        skip(parser);
        cache!(parser, 1);
    }
    if parser.buffer[parser.buffer_pos] == b'#' {
        while !is_breakz(&parser.buffer, parser.buffer_pos) {
            skip(parser);
            cache!(parser, 1);
        }
    }

    // Check if we are at the end of the line.
    if !is_breakz(&parser.buffer, parser.buffer_pos) {
        yaml_parser_set_scanner_error(
            parser,
            "while scanning a block scalar",
            start_mark,
            "did not find expected comment or line break",
        );
        return false;
    }

    // Eat a line break.
    if is_break(&parser.buffer, parser.buffer_pos) {
        cache!(parser, 2);
        skip_line(parser);
    }

    let mut end_mark = parser.mark;

    // Set the indentation level if it was specified.
    let mut indent: i64 = 0;
    if increment > 0 {
        if parser.indent >= 0 {
            indent = parser.indent + increment;
        } else {
            indent = increment;
        }
    }

    // Scan the leading line breaks and determine the indentation level if needed.
    let mut s: Vec<u8> = Vec::new();
    let mut leading_break: Vec<u8> = Vec::new();
    let mut trailing_breaks: Vec<u8> = Vec::new();
    if !yaml_parser_scan_block_scalar_breaks(
        parser,
        &mut indent,
        &mut trailing_breaks,
        start_mark,
        &mut end_mark,
    ) {
        return false;
    }

    // Scan the block scalar content.
    cache!(parser, 1);
    let mut leading_blank = false;
    let mut trailing_blank: bool;
    while parser.mark.column == indent && !is_z(&parser.buffer, parser.buffer_pos) {
        // We are at the beginning of a non-empty line.

        // Is it a trailing whitespace?
        trailing_blank = is_blank(&parser.buffer, parser.buffer_pos);

        // Check if we need to fold the leading line break.
        if !literal
            && !leading_blank
            && !trailing_blank
            && !leading_break.is_empty()
            && leading_break[0] == b'\n'
        {
            // Do we need to join the lines by space?
            if trailing_breaks.is_empty() {
                s.push(b' ');
            }
        } else {
            s.extend_from_slice(&leading_break);
        }
        leading_break.clear();

        // Append the remaining line breaks.
        s.extend_from_slice(&trailing_breaks);
        trailing_breaks.clear();

        // Is it a leading whitespace?
        leading_blank = is_blank(&parser.buffer, parser.buffer_pos);

        // Consume the current line.
        while !is_breakz(&parser.buffer, parser.buffer_pos) {
            read(parser, &mut s);
            cache!(parser, 1);
        }

        // Consume the line break.
        cache!(parser, 2);

        read_line(parser, &mut leading_break);

        // Eat the following indentation spaces and line breaks.
        if !yaml_parser_scan_block_scalar_breaks(
            parser,
            &mut indent,
            &mut trailing_breaks,
            start_mark,
            &mut end_mark,
        ) {
            return false;
        }
    }

    // Chomp the tail.
    if chomping != -1 {
        s.extend_from_slice(&leading_break);
    }
    if chomping == 1 {
        s.extend_from_slice(&trailing_breaks);
    }

    // Create a token.
    *token = Token {
        typ: TokenType::Scalar,
        start_mark,
        end_mark,
        value: s,
        style: if literal {
            ScalarStyle::Literal
        } else {
            ScalarStyle::Folded
        },
        ..Default::default()
    };
    true
}

// Go: scannerc.go:yaml_parser_scan_block_scalar_breaks
//
// Scan indentation spaces and line breaks for a block scalar.  Determine the
// indentation level if needed.
fn yaml_parser_scan_block_scalar_breaks(
    parser: &mut Parser,
    indent: &mut i64,
    breaks: &mut Vec<u8>,
    start_mark: Mark,
    end_mark: &mut Mark,
) -> bool {
    *end_mark = parser.mark;

    // Eat the indentation spaces and line breaks.
    let mut max_indent: i64 = 0;
    loop {
        // Eat the indentation spaces.
        cache!(parser, 1);
        while (*indent == 0 || parser.mark.column < *indent)
            && is_space(&parser.buffer, parser.buffer_pos)
        {
            skip(parser);
            cache!(parser, 1);
        }
        if parser.mark.column > max_indent {
            max_indent = parser.mark.column;
        }

        // Check for a tab character messing the indentation.
        if (*indent == 0 || parser.mark.column < *indent)
            && is_tab(&parser.buffer, parser.buffer_pos)
        {
            return yaml_parser_set_scanner_error(
                parser,
                "while scanning a block scalar",
                start_mark,
                "found a tab character where an indentation space is expected",
            );
        }

        // Have we found a non-empty line?
        if !is_break(&parser.buffer, parser.buffer_pos) {
            break;
        }

        // Consume the line break.
        cache!(parser, 2);
        // [Go] Should really be returning breaks instead.
        read_line(parser, breaks);
        *end_mark = parser.mark;
    }

    // Determine the indentation level if needed.
    if *indent == 0 {
        *indent = max_indent;
        if *indent < parser.indent + 1 {
            *indent = parser.indent + 1;
        }
        if *indent < 1 {
            *indent = 1;
        }
    }
    true
}

// Go: scannerc.go:yaml_parser_scan_flow_scalar
//
// Scan a quoted scalar.
fn yaml_parser_scan_flow_scalar(parser: &mut Parser, token: &mut Token, single: bool) -> bool {
    // Eat the left quote.
    let start_mark = parser.mark;
    skip(parser);

    // Consume the content of the quoted scalar.
    let mut s: Vec<u8> = Vec::new();
    let mut leading_break: Vec<u8> = Vec::new();
    let mut trailing_breaks: Vec<u8> = Vec::new();
    let mut whitespaces: Vec<u8> = Vec::new();
    loop {
        // Check that there are no document indicators at the beginning of the line.
        cache!(parser, 4);

        let pos = parser.buffer_pos;
        let b = &parser.buffer;
        if parser.mark.column == 0
            && ((b[pos] == b'-' && b[pos + 1] == b'-' && b[pos + 2] == b'-')
                || (b[pos] == b'.' && b[pos + 1] == b'.' && b[pos + 2] == b'.'))
            && is_blankz(b, pos + 3)
        {
            yaml_parser_set_scanner_error(
                parser,
                "while scanning a quoted scalar",
                start_mark,
                "found unexpected document indicator",
            );
            return false;
        }

        // Check for EOF.
        if is_z(&parser.buffer, parser.buffer_pos) {
            yaml_parser_set_scanner_error(
                parser,
                "while scanning a quoted scalar",
                start_mark,
                "found unexpected end of stream",
            );
            return false;
        }

        // Consume non-blank characters.
        let mut leading_blanks = false;
        while !is_blankz(&parser.buffer, parser.buffer_pos) {
            let pos = parser.buffer_pos;
            let c = parser.buffer[pos];
            if single && c == b'\'' && parser.buffer[pos + 1] == b'\'' {
                // Is is an escaped single quote.
                s.push(b'\'');
                skip(parser);
                skip(parser);
            } else if single && c == b'\'' {
                // It is a right single quote.
                break;
            } else if !single && c == b'"' {
                // It is a right double quote.
                break;
            } else if !single && c == b'\\' && is_break(&parser.buffer, pos + 1) {
                // It is an escaped line break.
                cache!(parser, 3);
                skip(parser);
                skip_line(parser);
                leading_blanks = true;
                break;
            } else if !single && c == b'\\' {
                // It is an escape sequence.
                let mut code_length: usize = 0;

                // Check the escape character.
                match parser.buffer[pos + 1] {
                    b'0' => s.push(0),
                    b'a' => s.push(0x07),
                    b'b' => s.push(0x08),
                    b't' | b'\t' => s.push(0x09),
                    b'n' => s.push(0x0A),
                    b'v' => s.push(0x0B),
                    b'f' => s.push(0x0C),
                    b'r' => s.push(0x0D),
                    b'e' => s.push(0x1B),
                    b' ' => s.push(0x20),
                    b'"' => s.push(b'"'),
                    b'\'' => s.push(b'\''),
                    b'\\' => s.push(b'\\'),
                    // NEL (#x85)
                    b'N' => s.extend_from_slice(b"\xC2\x85"),
                    // #xA0
                    b'_' => s.extend_from_slice(b"\xC2\xA0"),
                    // LS (#x2028)
                    b'L' => s.extend_from_slice(b"\xE2\x80\xA8"),
                    // PS (#x2029)
                    b'P' => s.extend_from_slice(b"\xE2\x80\xA9"),
                    b'x' => code_length = 2,
                    b'u' => code_length = 4,
                    b'U' => code_length = 8,
                    _ => {
                        yaml_parser_set_scanner_error(
                            parser,
                            "while parsing a quoted scalar",
                            start_mark,
                            "found unknown escape character",
                        );
                        return false;
                    }
                }

                skip(parser);
                skip(parser);

                // Consume an arbitrary escape code.
                if code_length > 0 {
                    let mut value: i64 = 0;

                    // Scan the character value.
                    cache!(parser, code_length as i64);
                    for k in 0..code_length {
                        if !is_hex(&parser.buffer, parser.buffer_pos + k) {
                            yaml_parser_set_scanner_error(
                                parser,
                                "while parsing a quoted scalar",
                                start_mark,
                                "did not find expected hexdecimal number",
                            );
                            return false;
                        }
                        value = (value << 4) + as_hex(&parser.buffer, parser.buffer_pos + k);
                    }

                    // Check the value and write the character.
                    if (0xD800..=0xDFFF).contains(&value) || value > 0x10FFFF {
                        yaml_parser_set_scanner_error(
                            parser,
                            "while parsing a quoted scalar",
                            start_mark,
                            "found invalid Unicode character escape code",
                        );
                        return false;
                    }
                    if value <= 0x7F {
                        s.push(value as u8);
                    } else if value <= 0x7FF {
                        s.push((0xC0 + (value >> 6)) as u8);
                        s.push((0x80 + (value & 0x3F)) as u8);
                    } else if value <= 0xFFFF {
                        s.push((0xE0 + (value >> 12)) as u8);
                        s.push((0x80 + ((value >> 6) & 0x3F)) as u8);
                        s.push((0x80 + (value & 0x3F)) as u8);
                    } else {
                        s.push((0xF0 + (value >> 18)) as u8);
                        s.push((0x80 + ((value >> 12) & 0x3F)) as u8);
                        s.push((0x80 + ((value >> 6) & 0x3F)) as u8);
                        s.push((0x80 + (value & 0x3F)) as u8);
                    }

                    // Advance the pointer.
                    for _ in 0..code_length {
                        skip(parser);
                    }
                }
            } else {
                // It is a non-escaped non-blank character.
                read(parser, &mut s);
            }
            cache!(parser, 2);
        }

        cache!(parser, 1);

        // Check if we are at the end of the scalar.
        if single {
            if parser.buffer[parser.buffer_pos] == b'\'' {
                break;
            }
        } else if parser.buffer[parser.buffer_pos] == b'"' {
            break;
        }

        // Consume blank characters.
        while is_blank(&parser.buffer, parser.buffer_pos)
            || is_break(&parser.buffer, parser.buffer_pos)
        {
            if is_blank(&parser.buffer, parser.buffer_pos) {
                // Consume a space or a tab character.
                if !leading_blanks {
                    read(parser, &mut whitespaces);
                } else {
                    skip(parser);
                }
            } else {
                cache!(parser, 2);

                // Check if it is a first line break.
                if !leading_blanks {
                    whitespaces.clear();
                    read_line(parser, &mut leading_break);
                    leading_blanks = true;
                } else {
                    read_line(parser, &mut trailing_breaks);
                }
            }
            cache!(parser, 1);
        }

        // Join the whitespaces or fold line breaks.
        if leading_blanks {
            // Do we need to fold line breaks?
            if !leading_break.is_empty() && leading_break[0] == b'\n' {
                if trailing_breaks.is_empty() {
                    s.push(b' ');
                } else {
                    s.extend_from_slice(&trailing_breaks);
                }
            } else {
                s.extend_from_slice(&leading_break);
                s.extend_from_slice(&trailing_breaks);
            }
            trailing_breaks.clear();
            leading_break.clear();
        } else {
            s.extend_from_slice(&whitespaces);
            whitespaces.clear();
        }
    }

    // Eat the right quote.
    skip(parser);
    let end_mark = parser.mark;

    // Create a token.
    *token = Token {
        typ: TokenType::Scalar,
        start_mark,
        end_mark,
        value: s,
        style: if single {
            ScalarStyle::SingleQuoted
        } else {
            ScalarStyle::DoubleQuoted
        },
        ..Default::default()
    };
    true
}

// Go: scannerc.go:yaml_parser_scan_plain_scalar
//
// Scan a plain scalar.
fn yaml_parser_scan_plain_scalar(parser: &mut Parser, token: &mut Token) -> bool {
    let mut s: Vec<u8> = Vec::new();
    let mut leading_break: Vec<u8> = Vec::new();
    let mut trailing_breaks: Vec<u8> = Vec::new();
    let mut whitespaces: Vec<u8> = Vec::new();
    let mut leading_blanks = false;
    let indent = parser.indent + 1;

    let start_mark = parser.mark;
    let mut end_mark = parser.mark;

    // Consume the content of the plain scalar.
    loop {
        // Check for a document indicator.
        cache!(parser, 4);
        let pos = parser.buffer_pos;
        let b = &parser.buffer;
        if parser.mark.column == 0
            && ((b[pos] == b'-' && b[pos + 1] == b'-' && b[pos + 2] == b'-')
                || (b[pos] == b'.' && b[pos + 1] == b'.' && b[pos + 2] == b'.'))
            && is_blankz(b, pos + 3)
        {
            break;
        }

        // Check for a comment.
        if parser.buffer[parser.buffer_pos] == b'#' {
            break;
        }

        // Consume non-blank characters.
        while !is_blankz(&parser.buffer, parser.buffer_pos) {
            let pos = parser.buffer_pos;
            let c = parser.buffer[pos];
            // Check for indicators that may end a plain scalar.
            if (c == b':' && is_blankz(&parser.buffer, pos + 1))
                || (parser.flow_level > 0
                    && (c == b',' || c == b'?' || c == b'[' || c == b']' || c == b'{' || c == b'}'))
            {
                break;
            }

            // Check if we need to join whitespaces and breaks.
            if leading_blanks || !whitespaces.is_empty() {
                if leading_blanks {
                    // Do we need to fold line breaks?
                    if leading_break.first() == Some(&b'\n') {
                        if trailing_breaks.is_empty() {
                            s.push(b' ');
                        } else {
                            s.extend_from_slice(&trailing_breaks);
                        }
                    } else {
                        s.extend_from_slice(&leading_break);
                        s.extend_from_slice(&trailing_breaks);
                    }
                    trailing_breaks.clear();
                    leading_break.clear();
                    leading_blanks = false;
                } else {
                    s.extend_from_slice(&whitespaces);
                    whitespaces.clear();
                }
            }

            // Copy the character.
            read(parser, &mut s);

            end_mark = parser.mark;
            cache!(parser, 2);
        }

        // Is it the end?
        if !(is_blank(&parser.buffer, parser.buffer_pos)
            || is_break(&parser.buffer, parser.buffer_pos))
        {
            break;
        }

        // Consume blank characters.
        cache!(parser, 1);

        while is_blank(&parser.buffer, parser.buffer_pos)
            || is_break(&parser.buffer, parser.buffer_pos)
        {
            if is_blank(&parser.buffer, parser.buffer_pos) {
                // Check for tab characters that abuse indentation.
                if leading_blanks
                    && parser.mark.column < indent
                    && is_tab(&parser.buffer, parser.buffer_pos)
                {
                    yaml_parser_set_scanner_error(
                        parser,
                        "while scanning a plain scalar",
                        start_mark,
                        "found a tab character that violates indentation",
                    );
                    return false;
                }

                // Consume a space or a tab character.
                if !leading_blanks {
                    read(parser, &mut whitespaces);
                } else {
                    skip(parser);
                }
            } else {
                cache!(parser, 2);

                // Check if it is a first line break.
                if !leading_blanks {
                    whitespaces.clear();
                    read_line(parser, &mut leading_break);
                    leading_blanks = true;
                } else {
                    read_line(parser, &mut trailing_breaks);
                }
            }
            cache!(parser, 1);
        }

        // Check indentation level.
        if parser.flow_level == 0 && parser.mark.column < indent {
            break;
        }
    }

    // Create a token.
    *token = Token {
        typ: TokenType::Scalar,
        start_mark,
        end_mark,
        value: s,
        style: ScalarStyle::Plain,
        ..Default::default()
    };

    // Note that we change the 'simple_key_allowed' flag.
    if leading_blanks {
        parser.simple_key_allowed = true;
    }
    true
}
