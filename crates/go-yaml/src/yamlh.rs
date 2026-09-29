//! Types and character classes of the libyaml port.
//!
//! Go: gopkg.in/yaml.v2@v2.4.0 yamlh.go (parser half) and yamlprivateh.go.
//! The emitter's types are in `encode.rs`.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// yamlprivateh.go

/// The size of the input raw buffer. Go: input_raw_buffer_size.
pub(crate) const INPUT_RAW_BUFFER_SIZE: usize = 512;

/// The size of the input buffer. Go: input_buffer_size.
pub(crate) const INPUT_BUFFER_SIZE: usize = INPUT_RAW_BUFFER_SIZE * 3;

// Go: yamlprivateh.go:is_alpha
#[inline]
pub(crate) fn is_alpha(b: &[u8], i: usize) -> bool {
    let c = b[i];
    c.is_ascii_digit() || c.is_ascii_uppercase() || c.is_ascii_lowercase() || c == b'_' || c == b'-'
}

// Go: yamlprivateh.go:is_digit
#[inline]
pub(crate) fn is_digit(b: &[u8], i: usize) -> bool {
    b[i].is_ascii_digit()
}

// Go: yamlprivateh.go:as_digit
#[inline]
pub(crate) fn as_digit(b: &[u8], i: usize) -> i64 {
    b[i] as i64 - b'0' as i64
}

// Go: yamlprivateh.go:is_hex
#[inline]
pub(crate) fn is_hex(b: &[u8], i: usize) -> bool {
    let c = b[i];
    c.is_ascii_digit() || (b'A'..=b'F').contains(&c) || (b'a'..=b'f').contains(&c)
}

// Go: yamlprivateh.go:as_hex
#[inline]
pub(crate) fn as_hex(b: &[u8], i: usize) -> i64 {
    let bi = b[i];
    if (b'A'..=b'F').contains(&bi) {
        return bi as i64 - b'A' as i64 + 10;
    }
    if (b'a'..=b'f').contains(&bi) {
        return bi as i64 - b'a' as i64 + 10;
    }
    bi as i64 - b'0' as i64
}

// Go: yamlprivateh.go:is_ascii
#[inline]
pub(crate) fn is_ascii(b: &[u8], i: usize) -> bool {
    b[i] <= 0x7F
}

// Go: yamlprivateh.go:is_printable
#[inline]
pub(crate) fn is_printable(b: &[u8], i: usize) -> bool {
    (b[i] == 0x0A) // . == #x0A
        || (b[i] >= 0x20 && b[i] <= 0x7E) // #x20 <= . <= #x7E
        || (b[i] == 0xC2 && b[i + 1] >= 0xA0) // #0xA0 <= . <= #xD7FF
        || (b[i] > 0xC2 && b[i] < 0xED)
        || (b[i] == 0xED && b[i + 1] < 0xA0)
        || (b[i] == 0xEE)
        || (b[i] == 0xEF // #xE000 <= . <= #xFFFD
            && !(b[i + 1] == 0xBB && b[i + 2] == 0xBF) // && . != #xFEFF
            && !(b[i + 1] == 0xBF && (b[i + 2] == 0xBE || b[i + 2] == 0xBF)))
}

// Go: yamlprivateh.go:is_z
#[inline]
pub(crate) fn is_z(b: &[u8], i: usize) -> bool {
    b[i] == 0x00
}

// Go: yamlprivateh.go:is_bom
//
// Note: like the Go code, this checks the *beginning of the buffer*, not
// position `i` (a libyaml port quirk that is part of the observable
// behaviour).
#[inline]
pub(crate) fn is_bom(b: &[u8], _i: usize) -> bool {
    b[0] == 0xEF && b[1] == 0xBB && b[2] == 0xBF
}

// Go: yamlprivateh.go:is_space
#[inline]
pub(crate) fn is_space(b: &[u8], i: usize) -> bool {
    b[i] == b' '
}

// Go: yamlprivateh.go:is_tab
#[inline]
pub(crate) fn is_tab(b: &[u8], i: usize) -> bool {
    b[i] == b'\t'
}

// Go: yamlprivateh.go:is_blank
#[inline]
pub(crate) fn is_blank(b: &[u8], i: usize) -> bool {
    b[i] == b' ' || b[i] == b'\t'
}

// Go: yamlprivateh.go:is_break
#[inline]
pub(crate) fn is_break(b: &[u8], i: usize) -> bool {
    b[i] == b'\r'
        || b[i] == b'\n'
        || (b[i] == 0xC2 && b[i + 1] == 0x85)
        || (b[i] == 0xE2 && b[i + 1] == 0x80 && b[i + 2] == 0xA8)
        || (b[i] == 0xE2 && b[i + 1] == 0x80 && b[i + 2] == 0xA9)
}

// Go: yamlprivateh.go:is_crlf
#[inline]
pub(crate) fn is_crlf(b: &[u8], i: usize) -> bool {
    b[i] == b'\r' && b[i + 1] == b'\n'
}

// Go: yamlprivateh.go:is_breakz
#[inline]
pub(crate) fn is_breakz(b: &[u8], i: usize) -> bool {
    is_break(b, i) || b[i] == 0
}

// Go: yamlprivateh.go:is_spacez
#[inline]
#[allow(dead_code)]
pub(crate) fn is_spacez(b: &[u8], i: usize) -> bool {
    b[i] == b' ' || is_breakz(b, i)
}

// Go: yamlprivateh.go:is_blankz
#[inline]
pub(crate) fn is_blankz(b: &[u8], i: usize) -> bool {
    b[i] == b' ' || b[i] == b'\t' || is_breakz(b, i)
}

// Go: yamlprivateh.go:width
#[inline]
pub(crate) fn width(b: u8) -> usize {
    if b & 0x80 == 0x00 {
        return 1;
    }
    if b & 0xE0 == 0xC0 {
        return 2;
    }
    if b & 0xF0 == 0xE0 {
        return 3;
    }
    if b & 0xF8 == 0xF0 {
        return 4;
    }
    0
}

// ---------------------------------------------------------------------------
// yamlh.go

/// Go: yaml_encoding_t.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum Encoding {
    #[default]
    Any,
    Utf8,
    Utf16Le,
    Utf16Be,
}

/// Go: yaml_error_type_t.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum ErrorType {
    #[default]
    NoError,
    #[allow(dead_code)]
    Memory,
    Reader,
    Scanner,
    Parser,
}

/// Go: yaml_mark_t.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) struct Mark {
    pub index: i64,
    pub line: i64,
    pub column: i64,
}

/// Go: yaml_scalar_style_t.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum ScalarStyle {
    #[default]
    Any,
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

/// Go: yaml_token_type_t.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum TokenType {
    #[default]
    NoToken,
    StreamStart,
    StreamEnd,
    VersionDirective,
    TagDirective,
    DocumentStart,
    DocumentEnd,
    BlockSequenceStart,
    BlockMappingStart,
    BlockEnd,
    FlowSequenceStart,
    FlowSequenceEnd,
    FlowMappingStart,
    FlowMappingEnd,
    BlockEntry,
    FlowEntry,
    Key,
    Value,
    Alias,
    Anchor,
    Tag,
    Scalar,
}

/// Go: yaml_token_t.
#[derive(Clone, Debug, Default)]
pub(crate) struct Token {
    pub typ: TokenType,
    pub start_mark: Mark,
    pub end_mark: Mark,
    #[allow(dead_code)]
    pub encoding: Encoding,
    /// alias/anchor/scalar value, tag handle, tag directive handle.
    pub value: Vec<u8>,
    /// tag suffix.
    pub suffix: Vec<u8>,
    /// tag directive prefix.
    pub prefix: Vec<u8>,
    pub style: ScalarStyle,
    pub major: i8,
    pub minor: i8,
}

/// Go: yaml_event_type_t.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum EventType {
    #[default]
    NoEvent,
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    Alias,
    Scalar,
    SequenceStart,
    SequenceEnd,
    MappingStart,
    MappingEnd,
}

impl EventType {
    /// Go: yamlh.go eventStrings / yaml_event_type_t.String.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            EventType::NoEvent => "none",
            EventType::StreamStart => "stream start",
            EventType::StreamEnd => "stream end",
            EventType::DocumentStart => "document start",
            EventType::DocumentEnd => "document end",
            EventType::Alias => "alias",
            EventType::Scalar => "scalar",
            EventType::SequenceStart => "sequence start",
            EventType::SequenceEnd => "sequence end",
            EventType::MappingStart => "mapping start",
            EventType::MappingEnd => "mapping end",
        }
    }
}

/// Go: yaml_tag_directive_t.
#[derive(Clone, Debug, Default)]
pub(crate) struct TagDirective {
    pub handle: Vec<u8>,
    pub prefix: Vec<u8>,
}

/// Go: yaml_event_t (the fields the decoder uses).
#[derive(Clone, Debug, Default)]
pub(crate) struct Event {
    pub typ: EventType,
    pub start_mark: Mark,
    #[allow(dead_code)]
    pub end_mark: Mark,
    pub anchor: Vec<u8>,
    pub tag: Vec<u8>,
    pub value: Vec<u8>,
    /// plain_implicit for scalars; implicit for the others.
    pub implicit: bool,
    #[allow(dead_code)]
    pub quoted_implicit: bool,
}

// Tags. Go: yamlh.go constants.
pub(crate) const NULL_TAG: &str = "tag:yaml.org,2002:null";
pub(crate) const BOOL_TAG: &str = "tag:yaml.org,2002:bool";
pub(crate) const STR_TAG: &str = "tag:yaml.org,2002:str";
pub(crate) const INT_TAG: &str = "tag:yaml.org,2002:int";
pub(crate) const FLOAT_TAG: &str = "tag:yaml.org,2002:float";
pub(crate) const TIMESTAMP_TAG: &str = "tag:yaml.org,2002:timestamp";
pub(crate) const SEQ_TAG: &str = "tag:yaml.org,2002:seq";
pub(crate) const MAP_TAG: &str = "tag:yaml.org,2002:map";
pub(crate) const BINARY_TAG: &str = "tag:yaml.org,2002:binary";
pub(crate) const MERGE_TAG: &str = "tag:yaml.org,2002:merge";

/// Go: yaml_simple_key_t.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SimpleKey {
    pub possible: bool,
    pub required: bool,
    pub token_number: i64,
    pub mark: Mark,
}

/// Go: yaml_parser_state_t.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum ParserState {
    #[default]
    StreamStart,
    ImplicitDocumentStart,
    DocumentStart,
    DocumentContent,
    DocumentEnd,
    BlockNode,
    #[allow(dead_code)] // never entered, as in libyaml
    BlockNodeOrIndentlessSequence,
    #[allow(dead_code)] // never entered, as in libyaml
    FlowNode,
    BlockSequenceFirstEntry,
    BlockSequenceEntry,
    IndentlessSequenceEntry,
    BlockMappingFirstKey,
    BlockMappingKey,
    BlockMappingValue,
    FlowSequenceFirstEntry,
    FlowSequenceEntry,
    FlowSequenceEntryMappingKey,
    FlowSequenceEntryMappingValue,
    FlowSequenceEntryMappingEnd,
    FlowMappingFirstKey,
    FlowMappingKey,
    FlowMappingValue,
    FlowMappingEmptyValue,
    End,
}

/// Go: yaml_parser_t (reader, scanner and parser state).
///
/// Go slices with `len`/`cap` semantics are modelled with a fixed-size
/// `Vec` plus an explicit length (`buffer_len`, `raw_len`), so that buffer
/// positions (which leak into behaviour through `is_bom`) match Go exactly.
#[derive(Default)]
pub(crate) struct Parser {
    // Error handling
    pub error: ErrorType,
    pub problem: String,
    #[allow(dead_code)]
    pub problem_offset: i64,
    #[allow(dead_code)]
    pub problem_value: i64,
    pub problem_mark: Mark,
    #[allow(dead_code)]
    pub context: String,
    pub context_mark: Mark,

    // Reader stuff
    pub input: Vec<u8>,
    pub input_pos: usize,
    pub eof: bool,

    /// Backing store of Go's `buffer` (cap = INPUT_BUFFER_SIZE).
    pub buffer: Vec<u8>,
    /// Go's `len(buffer)`.
    pub buffer_len: usize,
    pub buffer_pos: usize,
    pub unread: i64,

    /// Backing store of Go's `raw_buffer` (cap = INPUT_RAW_BUFFER_SIZE).
    pub raw_buffer: Vec<u8>,
    /// Go's `len(raw_buffer)`.
    pub raw_len: usize,
    pub raw_buffer_pos: usize,

    pub encoding: Encoding,

    pub offset: i64,
    pub mark: Mark,

    // Scanner stuff
    pub stream_start_produced: bool,
    pub stream_end_produced: bool,

    pub flow_level: i64,

    pub tokens: Vec<Token>,
    pub tokens_head: usize,
    pub tokens_parsed: i64,
    pub token_available: bool,

    pub indent: i64,
    pub indents: Vec<i64>,

    pub simple_key_allowed: bool,
    pub simple_keys: Vec<SimpleKey>,
    pub simple_keys_by_tok: HashMap<i64, usize>,

    // Parser stuff
    pub state: ParserState,
    pub states: Vec<ParserState>,
    pub marks: Vec<Mark>,
    pub tag_directives: Vec<TagDirective>,
}

impl Parser {
    // Go: apic.go:yaml_parser_initialize + yaml_parser_set_input_string
    pub(crate) fn new(input: Vec<u8>) -> Parser {
        Parser {
            raw_buffer: vec![0; INPUT_RAW_BUFFER_SIZE],
            buffer: vec![0; INPUT_BUFFER_SIZE],
            input,
            ..Default::default()
        }
    }
}

// Go: apic.go:yaml_insert_token
pub(crate) fn yaml_insert_token(parser: &mut Parser, pos: i64, token: Token) {
    // Check if we can move the queue at the beginning of the buffer. (The
    // Go code only compacts when len == cap; compaction never changes the
    // logical queue, so we compact whenever the head is past the middle.)
    if parser.tokens_head > 0 && parser.tokens_head * 2 >= parser.tokens.len() {
        parser.tokens.drain(..parser.tokens_head);
        parser.tokens_head = 0;
    }
    if pos < 0 {
        parser.tokens.push(token);
        return;
    }
    let at = parser.tokens_head + pos as usize;
    parser.tokens.insert(at, token);
}
