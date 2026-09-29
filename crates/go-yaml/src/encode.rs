//! A port of `gopkg.in/yaml.v2` v2.4.0 **encoding** (`yaml.Marshal`): `encode.go` (the
//! encoder over reflection), `sorter.go` (map key order), `emitterc.go` (the libyaml
//! emitter), the emitter half of `apic.go`/`writerc.go`, and `encodeBase64`/`isBase60Float`.
//!
//! Added for neohugo's `parser.InterfaceToConfig(YAML)` (`transform.Remarshal`, `hugo config`).
//! The input is a [`Node`] tree: the caller maps its Go values onto the kinds the encoder
//! distinguishes (`reflect.Kind` plus `time.Time`). Anchors, aliases, explicit tags other than
//! `!!binary`, flow style, struct values, `Marshaler`/`TextMarshaler` dispatch and the non-UTF-8
//! stream encodings are not reachable from a `Node` (the caller resolves `TextMarshaler`s into
//! strings), so those emitter paths are ported only as far as the state machine needs them.

use crate::Error;
use crate::resolve::resolve;
use crate::yamlh::{
    BINARY_TAG, STR_TAG, is_alpha, is_ascii, is_blank, is_blankz, is_bom, is_break, is_printable,
    is_space, width,
};

/// A value for [`marshal`], by the kind yaml.v2's encoder dispatches on.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// `nil` (a nil interface or pointer): `null`.
    Nil,
    Bool(bool),
    /// Any signed integer kind (`intv`).
    Int(i64),
    /// Any unsigned integer kind (`uintv`).
    Uint(u64),
    /// A float; `true` for `float32` (the precision of `FormatFloat`).
    Float(f64, bool),
    /// A string kind (`stringv`; bytes, may be invalid UTF-8).
    Str(Vec<u8>),
    /// A `time.Time`, already formatted with `time.RFC3339Nano` (`timev`).
    Time(Vec<u8>),
    /// A map with string keys (`mapv`); the encoder sorts the keys (`keyList`).
    Map(Vec<(Vec<u8>, Node)>),
    /// A slice or array (`slicev`).
    Seq(Vec<Node>),
}

/// Go: `yaml.Marshal(in)`.
// Go: yaml.go:Marshal
pub fn marshal(n: &Node) -> Result<Vec<u8>, Error> {
    let mut e = Encoder::new();
    e.marshal_doc(n)?;
    e.finish()?;
    Ok(e.emitter.out)
}

// ---------------------------------------------------------------------------
// encode.go

struct Encoder {
    emitter: Emitter,
    do_init: bool,
}

impl Encoder {
    // Go: encode.go:newEncoder
    fn new() -> Encoder {
        let emitter = Emitter {
            // yaml_emitter_set_unicode(&e.emitter, true)
            unicode: true,
            ..Emitter::default()
        };
        Encoder {
            emitter,
            do_init: false,
        }
    }

    // Go: encode.go:(*encoder).init
    fn init(&mut self) -> Result<(), Error> {
        if self.do_init {
            return Ok(());
        }
        self.emit(Event::new(EventType::StreamStart))?;
        self.do_init = true;
        Ok(())
    }

    // Go: encode.go:(*encoder).finish
    fn finish(&mut self) -> Result<(), Error> {
        self.emitter.open_ended = false;
        self.emit(Event::new(EventType::StreamEnd))
    }

    // Go: encode.go:(*encoder).emit, (*encoder).must
    fn emit(&mut self, ev: Event) -> Result<(), Error> {
        if !self.emitter.emit(ev) {
            let mut msg = self.emitter.problem.clone();
            if msg.is_empty() {
                msg = "unknown problem generating YAML content".to_string();
            }
            return Err(Error::fatal(format!("yaml: {msg}")));
        }
        Ok(())
    }

    // Go: encode.go:(*encoder).marshalDoc
    fn marshal_doc(&mut self, n: &Node) -> Result<(), Error> {
        self.init()?;
        let mut ev = Event::new(EventType::DocumentStart);
        ev.implicit = true;
        self.emit(ev)?;
        self.marshal(n)?;
        let mut ev = Event::new(EventType::DocumentEnd);
        ev.implicit = true;
        self.emit(ev)
    }

    // Go: encode.go:(*encoder).marshal
    fn marshal(&mut self, n: &Node) -> Result<(), Error> {
        match n {
            Node::Nil => self.nilv(),
            Node::Map(entries) => self.mapv(entries),
            Node::Seq(items) => self.slicev(items),
            Node::Str(s) => self.stringv(s),
            Node::Int(i) => self.emit_scalar(i.to_string().as_bytes(), b"", Style::Plain),
            Node::Uint(u) => self.emit_scalar(u.to_string().as_bytes(), b"", Style::Plain),
            Node::Float(f, f32) => self.floatv(*f, *f32),
            Node::Bool(b) => {
                let s: &[u8] = if *b { b"true" } else { b"false" };
                self.emit_scalar(s, b"", Style::Plain)
            }
            Node::Time(s) => self.emit_scalar(s, b"", Style::Plain),
        }
    }

    // Go: encode.go:(*encoder).mapv
    fn mapv(&mut self, entries: &[(Vec<u8>, Node)]) -> Result<(), Error> {
        // mappingv: implicit, block style (e.flow is never set here).
        let mut ev = Event::new(EventType::MappingStart);
        ev.implicit = true;
        ev.style = Style::Block;
        self.emit(ev)?;
        let mut keys: Vec<&(Vec<u8>, Node)> = entries.iter().collect();
        let runes: Vec<Vec<go_unicode::Rune>> = keys
            .iter()
            .map(|k| go_unicode::utf8::to_runes(&k.0))
            .collect();
        // sort.Sort(keyList): pdqsort over the keys (the order is not a strict weak order in
        // general, so Go's algorithm decides ties).
        let mut idx: Vec<usize> = (0..keys.len()).collect();
        go_sort::sort_by(&mut idx, |&a, &b| key_less(&runes[a], &runes[b]));
        keys = idx.iter().map(|&i| &entries[i]).collect();
        for k in keys {
            self.stringv(&k.0)?;
            self.marshal(&k.1)?;
        }
        self.emit(Event::new(EventType::MappingEnd))
    }

    // Go: encode.go:(*encoder).slicev
    fn slicev(&mut self, items: &[Node]) -> Result<(), Error> {
        let mut ev = Event::new(EventType::SequenceStart);
        ev.implicit = true;
        ev.style = Style::Block;
        self.emit(ev)?;
        for n in items {
            self.marshal(n)?;
        }
        self.emit(Event::new(EventType::SequenceEnd))
    }

    // Go: encode.go:(*encoder).stringv
    fn stringv(&mut self, s: &[u8]) -> Result<(), Error> {
        let mut tag: &[u8] = b"";
        let mut s = s.to_vec();
        let mut can_use_plain = true;
        if !go_unicode::utf8::valid(&s) {
            // It can't be encoded directly as YAML so use a binary tag
            // and encode it as base64.
            tag = BINARY_TAG.as_bytes();
            s = encode_base64(&s);
        } else {
            // Check to see if it would resolve to a specific
            // tag when encoded unquoted. If it doesn't,
            // there's no need to quote it.
            let rtag = match resolve("", &s) {
                Ok((t, _)) => t,
                Err(_) => String::new(),
            };
            can_use_plain = rtag == STR_TAG && !is_base60_float(&s);
        }
        let style = if s.contains(&b'\n') {
            Style::Literal
        } else if can_use_plain {
            Style::Plain
        } else {
            Style::DoubleQuoted
        };
        self.emit_scalar(&s, tag, style)
    }

    // Go: encode.go:(*encoder).floatv
    fn floatv(&mut self, f: f64, f32: bool) -> Result<(), Error> {
        // Issue #352: When formatting, use the precision of the underlying value
        let precision = if f32 { 32 } else { 64 };
        let mut s = go_strconv::format_float(f, b'g', -1, precision);
        match s.as_str() {
            "+Inf" => s = ".inf".into(),
            "-Inf" => s = "-.inf".into(),
            "NaN" => s = ".nan".into(),
            _ => {}
        }
        self.emit_scalar(s.as_bytes(), b"", Style::Plain)
    }

    // Go: encode.go:(*encoder).nilv
    fn nilv(&mut self) -> Result<(), Error> {
        self.emit_scalar(b"null", b"", Style::Plain)
    }

    // Go: encode.go:(*encoder).emitScalar
    fn emit_scalar(&mut self, value: &[u8], tag: &[u8], style: Style) -> Result<(), Error> {
        let implicit = tag.is_empty();
        let mut ev = Event::new(EventType::Scalar);
        ev.tag = tag.to_vec();
        ev.value = value.to_vec();
        ev.implicit = implicit;
        ev.quoted_implicit = implicit;
        ev.style = style;
        self.emit(ev)
    }
}

/// isBase60Float returns whether s is in base 60 notation as defined in YAML 1.1.
// Go: encode.go:isBase60Float
fn is_base60_float(s: &[u8]) -> bool {
    // Fast path.
    if s.is_empty() {
        return false;
    }
    let c = s[0];
    if !(c == b'+' || c == b'-' || c.is_ascii_digit()) || !s.contains(&b':') {
        return false;
    }
    // Do the full match.
    base60float_match(s)
}

/// `^[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+(?:\.[0-9_]*)?$`
fn base60float_match(s: &[u8]) -> bool {
    let mut i = 0;
    if i < s.len() && (s[i] == b'-' || s[i] == b'+') {
        i += 1;
    }
    if i >= s.len() || !s[i].is_ascii_digit() {
        return false;
    }
    i += 1;
    while i < s.len() && (s[i].is_ascii_digit() || s[i] == b'_') {
        i += 1;
    }
    // (?::[0-5]?[0-9])+ : each group is ':' then one or two digits, the first of two in 0-5.
    // Backtracking: a two-digit group "d1d2" can also be read as "d1" followed by something
    // else; the only thing that can follow a group is ':', '.', or the end, and a digit is none
    // of them, so the greedy reading is the only one.
    let mut groups = 0;
    while i < s.len() && s[i] == b':' {
        let j = i + 1;
        if j < s.len()
            && (b'0'..=b'5').contains(&s[j])
            && j + 1 < s.len()
            && s[j + 1].is_ascii_digit()
        {
            i = j + 2;
        } else if j < s.len() && s[j].is_ascii_digit() {
            i = j + 1;
        } else {
            return false;
        }
        groups += 1;
    }
    if groups == 0 {
        return false;
    }
    if i < s.len() && s[i] == b'.' {
        i += 1;
        while i < s.len() && (s[i].is_ascii_digit() || s[i] == b'_') {
            i += 1;
        }
    }
    i == s.len()
}

/// Go: `base64.StdEncoding` with yaml.v2's 70-column line breaks.
// Go: resolve.go:encodeBase64
fn encode_base64(s: &[u8]) -> Vec<u8> {
    const LINE_LEN: usize = 70;
    const ENC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut inb = Vec::with_capacity(s.len().div_ceil(3) * 4);
    for chunk in s.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let v = (b0 << 16) | (b1 << 8) | b2;
        inb.push(ENC[(v >> 18) as usize & 63]);
        inb.push(ENC[(v >> 12) as usize & 63]);
        inb.push(if chunk.len() > 1 {
            ENC[(v >> 6) as usize & 63]
        } else {
            b'='
        });
        inb.push(if chunk.len() > 2 {
            ENC[v as usize & 63]
        } else {
            b'='
        });
    }
    let enc_len = inb.len();
    let lines = enc_len / LINE_LEN + 1;
    let mut out = Vec::with_capacity(enc_len + lines);
    let mut i = 0;
    while i < enc_len {
        let j = (i + LINE_LEN).min(enc_len);
        out.extend_from_slice(&inb[i..j]);
        if lines > 1 {
            out.push(b'\n');
        }
        i += LINE_LEN;
    }
    out
}

// ---------------------------------------------------------------------------
// sorter.go

/// Go: `keyList.Less` for two string keys (as `[]rune`).
// Go: sorter.go:(keyList).Less
fn key_less(ar: &[go_unicode::Rune], br: &[go_unicode::Rune]) -> bool {
    use go_unicode::{is_digit, is_letter};
    let mut i = 0;
    while i < ar.len() && i < br.len() {
        if ar[i] == br[i] {
            i += 1;
            continue;
        }
        let al = is_letter(ar[i]);
        let bl = is_letter(br[i]);
        if al && bl {
            return ar[i] < br[i];
        }
        if al || bl {
            return bl;
        }
        let (mut an, mut bn): (i64, i64) = (0, 0);
        if ar[i] == '0' as i32 || br[i] == '0' as i32 {
            let mut j = i as isize - 1;
            while j >= 0 && is_digit(ar[j as usize]) {
                if ar[j as usize] != '0' as i32 {
                    an = 1;
                    bn = 1;
                    break;
                }
                j -= 1;
            }
        }
        let mut ai = i;
        while ai < ar.len() && is_digit(ar[ai]) {
            an = an
                .wrapping_mul(10)
                .wrapping_add((ar[ai] - '0' as i32) as i64);
            ai += 1;
        }
        let mut bi = i;
        while bi < br.len() && is_digit(br[bi]) {
            bn = bn
                .wrapping_mul(10)
                .wrapping_add((br[bi] - '0' as i32) as i64);
            bi += 1;
        }
        if an != bn {
            return an < bn;
        }
        if ai != bi {
            return ai < bi;
        }
        return ar[i] < br[i];
    }
    ar.len() < br.len()
}

// ---------------------------------------------------------------------------
// yamlh.go (emitter half)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventType {
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    Scalar,
    SequenceStart,
    SequenceEnd,
    MappingStart,
    MappingEnd,
}

/// The event's `style` (scalar, sequence or mapping style).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Style {
    Any,
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
    /// `yaml_BLOCK_SEQUENCE_STYLE` / `yaml_BLOCK_MAPPING_STYLE`.
    Block,
}

/// Go: `yaml_event_t` (the fields the emitter reads; no anchors, no directives).
#[derive(Clone, Debug)]
struct Event {
    typ: EventType,
    tag: Vec<u8>,
    value: Vec<u8>,
    implicit: bool,
    quoted_implicit: bool,
    style: Style,
}

impl Event {
    fn new(typ: EventType) -> Event {
        Event {
            typ,
            tag: Vec::new(),
            value: Vec::new(),
            implicit: false,
            quoted_implicit: false,
            style: Style::Any,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    StreamStart,
    FirstDocumentStart,
    DocumentStart,
    DocumentContent,
    DocumentEnd,
    FlowSequenceFirstItem,
    FlowSequenceItem,
    FlowMappingFirstKey,
    FlowMappingKey,
    FlowMappingSimpleValue,
    FlowMappingValue,
    BlockSequenceFirstItem,
    BlockSequenceItem,
    BlockMappingFirstKey,
    BlockMappingKey,
    BlockMappingSimpleValue,
    BlockMappingValue,
    End,
}

struct TagDirective {
    handle: &'static [u8],
    prefix: &'static [u8],
}

// Go: yamlh.go:default_tag_directives
const DEFAULT_TAG_DIRECTIVES: [TagDirective; 2] = [
    TagDirective {
        handle: b"!",
        prefix: b"!",
    },
    TagDirective {
        handle: b"!!",
        prefix: b"tag:yaml.org,2002:",
    },
];

#[derive(Default)]
struct ScalarData {
    value: Vec<u8>,
    multiline: bool,
    flow_plain_allowed: bool,
    block_plain_allowed: bool,
    single_quoted_allowed: bool,
    block_allowed: bool,
    style: Option<Style>,
}

/// Go: `yaml_emitter_t` (string output).
struct Emitter {
    problem: String,
    out: Vec<u8>,
    canonical: bool,
    best_indent: i64,
    best_width: i64,
    unicode: bool,
    states: Vec<State>,
    state: State,
    events: Vec<Event>,
    events_head: usize,
    indents: Vec<i64>,
    tag_directives: Vec<&'static TagDirective>,
    indent: i64,
    flow_level: i64,
    root_context: bool,
    sequence_context: bool,
    mapping_context: bool,
    simple_key_context: bool,
    line: i64,
    column: i64,
    whitespace: bool,
    indention: bool,
    open_ended: bool,
    tag_handle: Vec<u8>,
    tag_suffix: Vec<u8>,
    scalar_data: ScalarData,
}

impl Default for Emitter {
    // Go: apic.go:yaml_emitter_initialize
    fn default() -> Self {
        Emitter {
            problem: String::new(),
            out: Vec::new(),
            canonical: false,
            best_indent: 0,
            best_width: 0,
            unicode: false,
            states: Vec::new(),
            state: State::StreamStart,
            events: Vec::new(),
            events_head: 0,
            indents: Vec::new(),
            tag_directives: Vec::new(),
            indent: 0,
            flow_level: 0,
            root_context: false,
            sequence_context: false,
            mapping_context: false,
            simple_key_context: false,
            line: 0,
            column: 0,
            whitespace: false,
            indention: false,
            open_ended: false,
            tag_handle: Vec::new(),
            tag_suffix: Vec::new(),
            scalar_data: ScalarData::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// emitterc.go

impl Emitter {
    /// Put a character to the output buffer.
    // Go: emitterc.go:put
    fn put(&mut self, value: u8) -> bool {
        self.out.push(value);
        self.column += 1;
        true
    }

    /// Put a line break to the output buffer (`yaml_LN_BREAK`).
    // Go: emitterc.go:put_break
    fn put_break(&mut self) -> bool {
        self.out.push(b'\n');
        self.column = 0;
        self.line += 1;
        true
    }

    /// Copy a character from a string into buffer.
    // Go: emitterc.go:write
    fn write(&mut self, s: &[u8], i: &mut usize) -> bool {
        let w = width(s[*i]);
        match w {
            1..=4 => self.out.extend_from_slice(&s[*i..*i + w]),
            _ => panic!("unknown character width"),
        }
        self.column += 1;
        *i += w;
        true
    }

    /// Write a whole string into buffer.
    // Go: emitterc.go:write_all
    fn write_all(&mut self, s: &[u8]) -> bool {
        let mut i = 0;
        while i < s.len() {
            if !self.write(s, &mut i) {
                return false;
            }
        }
        true
    }

    /// Copy a line break character from a string into buffer.
    // Go: emitterc.go:write_break
    fn write_break(&mut self, s: &[u8], i: &mut usize) -> bool {
        if s[*i] == b'\n' {
            if !self.put_break() {
                return false;
            }
            *i += 1;
        } else {
            if !self.write(s, i) {
                return false;
            }
            self.column = 0;
            self.line += 1;
        }
        true
    }

    /// Set an emitter error and return false.
    // Go: emitterc.go:yaml_emitter_set_emitter_error
    fn set_emitter_error(&mut self, problem: &str) -> bool {
        self.problem = problem.to_string();
        false
    }

    /// Emit an event.
    // Go: emitterc.go:yaml_emitter_emit
    fn emit(&mut self, event: Event) -> bool {
        self.events.push(event);
        while !self.need_more_events() {
            let event = self.events[self.events_head].clone();
            if !self.analyze_event(&event) {
                return false;
            }
            if !self.state_machine(&event) {
                return false;
            }
            self.events_head += 1;
        }
        true
    }

    /// Check if we need to accumulate more events before emitting.
    // Go: emitterc.go:yaml_emitter_need_more_events
    fn need_more_events(&self) -> bool {
        if self.events_head == self.events.len() {
            return true;
        }
        let accumulate = match self.events[self.events_head].typ {
            EventType::DocumentStart => 1,
            EventType::SequenceStart => 2,
            EventType::MappingStart => 3,
            _ => return false,
        };
        if self.events.len() - self.events_head > accumulate {
            return false;
        }
        let mut level = 0;
        for ev in &self.events[self.events_head..] {
            match ev.typ {
                EventType::StreamStart
                | EventType::DocumentStart
                | EventType::SequenceStart
                | EventType::MappingStart => level += 1,
                EventType::StreamEnd
                | EventType::DocumentEnd
                | EventType::SequenceEnd
                | EventType::MappingEnd => level -= 1,
                _ => {}
            }
            if level == 0 {
                return false;
            }
        }
        true
    }

    /// Append a directive to the directives stack.
    // Go: emitterc.go:yaml_emitter_append_tag_directive
    fn append_tag_directive(
        &mut self,
        value: &'static TagDirective,
        allow_duplicates: bool,
    ) -> bool {
        for td in &self.tag_directives {
            if value.handle == td.handle {
                if allow_duplicates {
                    return true;
                }
                return self.set_emitter_error("duplicate %TAG directive");
            }
        }
        self.tag_directives.push(value);
        true
    }

    /// Increase the indentation level.
    // Go: emitterc.go:yaml_emitter_increase_indent
    fn increase_indent(&mut self, flow: bool, indentless: bool) -> bool {
        self.indents.push(self.indent);
        if self.indent < 0 {
            if flow {
                self.indent = self.best_indent;
            } else {
                self.indent = 0;
            }
        } else if !indentless {
            self.indent += self.best_indent;
        }
        true
    }

    fn pop_indent(&mut self) {
        self.indent = self.indents.pop().expect("indents");
    }

    fn pop_state(&mut self) {
        self.state = self.states.pop().expect("states");
    }

    /// State dispatcher.
    // Go: emitterc.go:yaml_emitter_state_machine
    fn state_machine(&mut self, event: &Event) -> bool {
        match self.state {
            State::StreamStart => self.emit_stream_start(event),
            State::FirstDocumentStart => self.emit_document_start(event, true),
            State::DocumentStart => self.emit_document_start(event, false),
            State::DocumentContent => self.emit_document_content(event),
            State::DocumentEnd => self.emit_document_end(event),
            State::FlowSequenceFirstItem => self.emit_flow_sequence_item(event, true),
            State::FlowSequenceItem => self.emit_flow_sequence_item(event, false),
            State::FlowMappingFirstKey => self.emit_flow_mapping_key(event, true),
            State::FlowMappingKey => self.emit_flow_mapping_key(event, false),
            State::FlowMappingSimpleValue => self.emit_flow_mapping_value(event, true),
            State::FlowMappingValue => self.emit_flow_mapping_value(event, false),
            State::BlockSequenceFirstItem => self.emit_block_sequence_item(event, true),
            State::BlockSequenceItem => self.emit_block_sequence_item(event, false),
            State::BlockMappingFirstKey => self.emit_block_mapping_key(event, true),
            State::BlockMappingKey => self.emit_block_mapping_key(event, false),
            State::BlockMappingSimpleValue => self.emit_block_mapping_value(event, true),
            State::BlockMappingValue => self.emit_block_mapping_value(event, false),
            State::End => self.set_emitter_error("expected nothing after STREAM-END"),
        }
    }

    /// Expect STREAM-START.
    // Go: emitterc.go:yaml_emitter_emit_stream_start
    fn emit_stream_start(&mut self, event: &Event) -> bool {
        if event.typ != EventType::StreamStart {
            return self.set_emitter_error("expected STREAM-START");
        }
        // (encoding: UTF-8; no BOM is written)
        if self.best_indent < 2 || self.best_indent > 9 {
            self.best_indent = 2;
        }
        if self.best_width >= 0 && self.best_width <= self.best_indent * 2 {
            self.best_width = 80;
        }
        if self.best_width < 0 {
            self.best_width = (1 << 31) - 1;
        }

        self.indent = -1;
        self.line = 0;
        self.column = 0;
        self.whitespace = true;
        self.indention = true;

        self.state = State::FirstDocumentStart;
        true
    }

    /// Expect DOCUMENT-START or STREAM-END.
    // Go: emitterc.go:yaml_emitter_emit_document_start
    fn emit_document_start(&mut self, event: &Event, first: bool) -> bool {
        if event.typ == EventType::DocumentStart {
            // (no version or tag directives)
            for td in &DEFAULT_TAG_DIRECTIVES {
                if !self.append_tag_directive(td, true) {
                    return false;
                }
            }

            let mut implicit = event.implicit;
            if !first || self.canonical {
                implicit = false;
            }

            // (yaml_emitter_check_empty_document is always false)
            if !implicit {
                if !self.write_indent() {
                    return false;
                }
                if !self.write_indicator(b"---", true, false, false) {
                    return false;
                }
                if self.canonical && !self.write_indent() {
                    return false;
                }
            }

            self.state = State::DocumentContent;
            return true;
        }

        if event.typ == EventType::StreamEnd {
            if self.open_ended {
                if !self.write_indicator(b"...", true, false, false) {
                    return false;
                }
                if !self.write_indent() {
                    return false;
                }
            }
            self.state = State::End;
            return true;
        }

        self.set_emitter_error("expected DOCUMENT-START or STREAM-END")
    }

    /// Expect the root node.
    // Go: emitterc.go:yaml_emitter_emit_document_content
    fn emit_document_content(&mut self, event: &Event) -> bool {
        self.states.push(State::DocumentEnd);
        self.emit_node(event, true, false, false, false)
    }

    /// Expect DOCUMENT-END.
    // Go: emitterc.go:yaml_emitter_emit_document_end
    fn emit_document_end(&mut self, event: &Event) -> bool {
        if event.typ != EventType::DocumentEnd {
            return self.set_emitter_error("expected DOCUMENT-END");
        }
        if !self.write_indent() {
            return false;
        }
        if !event.implicit {
            // [Go] Allocate the slice elsewhere.
            if !self.write_indicator(b"...", true, false, false) {
                return false;
            }
            if !self.write_indent() {
                return false;
            }
        }
        self.state = State::DocumentStart;
        self.tag_directives.clear();
        true
    }

    /// Expect a flow item node.
    // Go: emitterc.go:yaml_emitter_emit_flow_sequence_item
    fn emit_flow_sequence_item(&mut self, event: &Event, first: bool) -> bool {
        if first {
            if !self.write_indicator(b"[", true, true, false) {
                return false;
            }
            if !self.increase_indent(true, false) {
                return false;
            }
            self.flow_level += 1;
        }

        if event.typ == EventType::SequenceEnd {
            self.flow_level -= 1;
            self.pop_indent();
            if self.canonical && !first {
                if !self.write_indicator(b",", false, false, false) {
                    return false;
                }
                if !self.write_indent() {
                    return false;
                }
            }
            if !self.write_indicator(b"]", false, false, false) {
                return false;
            }
            self.pop_state();
            return true;
        }

        if !first && !self.write_indicator(b",", false, false, false) {
            return false;
        }

        if (self.canonical || self.column > self.best_width) && !self.write_indent() {
            return false;
        }
        self.states.push(State::FlowSequenceItem);
        self.emit_node(event, false, true, false, false)
    }

    /// Expect a flow key node.
    // Go: emitterc.go:yaml_emitter_emit_flow_mapping_key
    fn emit_flow_mapping_key(&mut self, event: &Event, first: bool) -> bool {
        if first {
            if !self.write_indicator(b"{", true, true, false) {
                return false;
            }
            if !self.increase_indent(true, false) {
                return false;
            }
            self.flow_level += 1;
        }

        if event.typ == EventType::MappingEnd {
            self.flow_level -= 1;
            self.pop_indent();
            if self.canonical && !first {
                if !self.write_indicator(b",", false, false, false) {
                    return false;
                }
                if !self.write_indent() {
                    return false;
                }
            }
            if !self.write_indicator(b"}", false, false, false) {
                return false;
            }
            self.pop_state();
            return true;
        }

        if !first && !self.write_indicator(b",", false, false, false) {
            return false;
        }
        if (self.canonical || self.column > self.best_width) && !self.write_indent() {
            return false;
        }

        if !self.canonical && self.check_simple_key() {
            self.states.push(State::FlowMappingSimpleValue);
            return self.emit_node(event, false, false, true, true);
        }
        if !self.write_indicator(b"?", true, false, false) {
            return false;
        }
        self.states.push(State::FlowMappingValue);
        self.emit_node(event, false, false, true, false)
    }

    /// Expect a flow value node.
    // Go: emitterc.go:yaml_emitter_emit_flow_mapping_value
    fn emit_flow_mapping_value(&mut self, event: &Event, simple: bool) -> bool {
        if simple {
            if !self.write_indicator(b":", false, false, false) {
                return false;
            }
        } else {
            if (self.canonical || self.column > self.best_width) && !self.write_indent() {
                return false;
            }
            if !self.write_indicator(b":", true, false, false) {
                return false;
            }
        }
        self.states.push(State::FlowMappingKey);
        self.emit_node(event, false, false, true, false)
    }

    /// Expect a block item node.
    // Go: emitterc.go:yaml_emitter_emit_block_sequence_item
    fn emit_block_sequence_item(&mut self, event: &Event, first: bool) -> bool {
        if first && !self.increase_indent(false, self.mapping_context && !self.indention) {
            return false;
        }
        if event.typ == EventType::SequenceEnd {
            self.pop_indent();
            self.pop_state();
            return true;
        }
        if !self.write_indent() {
            return false;
        }
        if !self.write_indicator(b"-", true, false, true) {
            return false;
        }
        self.states.push(State::BlockSequenceItem);
        self.emit_node(event, false, true, false, false)
    }

    /// Expect a block key node.
    // Go: emitterc.go:yaml_emitter_emit_block_mapping_key
    fn emit_block_mapping_key(&mut self, event: &Event, first: bool) -> bool {
        if first && !self.increase_indent(false, false) {
            return false;
        }
        if event.typ == EventType::MappingEnd {
            self.pop_indent();
            self.pop_state();
            return true;
        }
        if !self.write_indent() {
            return false;
        }
        if self.check_simple_key() {
            self.states.push(State::BlockMappingSimpleValue);
            return self.emit_node(event, false, false, true, true);
        }
        if !self.write_indicator(b"?", true, false, true) {
            return false;
        }
        self.states.push(State::BlockMappingValue);
        self.emit_node(event, false, false, true, false)
    }

    /// Expect a block value node.
    // Go: emitterc.go:yaml_emitter_emit_block_mapping_value
    fn emit_block_mapping_value(&mut self, event: &Event, simple: bool) -> bool {
        if simple {
            if !self.write_indicator(b":", false, false, false) {
                return false;
            }
        } else {
            if !self.write_indent() {
                return false;
            }
            if !self.write_indicator(b":", true, false, true) {
                return false;
            }
        }
        self.states.push(State::BlockMappingKey);
        self.emit_node(event, false, false, true, false)
    }

    /// Expect a node.
    // Go: emitterc.go:yaml_emitter_emit_node
    fn emit_node(
        &mut self,
        event: &Event,
        root: bool,
        sequence: bool,
        mapping: bool,
        simple_key: bool,
    ) -> bool {
        self.root_context = root;
        self.sequence_context = sequence;
        self.mapping_context = mapping;
        self.simple_key_context = simple_key;

        match event.typ {
            EventType::Scalar => self.emit_scalar(event),
            EventType::SequenceStart => self.emit_sequence_start(event),
            EventType::MappingStart => self.emit_mapping_start(event),
            other => self.set_emitter_error(&format!(
                "expected SCALAR, SEQUENCE-START, MAPPING-START, or ALIAS, but got {}",
                event_type_string(other)
            )),
        }
    }

    /// Expect SCALAR.
    // Go: emitterc.go:yaml_emitter_emit_scalar
    fn emit_scalar(&mut self, event: &Event) -> bool {
        if !self.select_scalar_style(event) {
            return false;
        }
        // (no anchor)
        if !self.process_tag() {
            return false;
        }
        if !self.increase_indent(true, false) {
            return false;
        }
        if !self.process_scalar() {
            return false;
        }
        self.pop_indent();
        self.pop_state();
        true
    }

    /// Expect SEQUENCE-START.
    // Go: emitterc.go:yaml_emitter_emit_sequence_start
    fn emit_sequence_start(&mut self, event: &Event) -> bool {
        if !self.process_tag() {
            return false;
        }
        // (the sequence style is always block: e.flow is never set)
        let _ = event;
        if self.flow_level > 0 || self.canonical || self.check_empty_sequence() {
            self.state = State::FlowSequenceFirstItem;
        } else {
            self.state = State::BlockSequenceFirstItem;
        }
        true
    }

    /// Expect MAPPING-START.
    // Go: emitterc.go:yaml_emitter_emit_mapping_start
    fn emit_mapping_start(&mut self, _event: &Event) -> bool {
        if !self.process_tag() {
            return false;
        }
        // (the mapping style is always block: e.flow is never set)
        if self.flow_level > 0 || self.canonical || self.check_empty_mapping() {
            self.state = State::FlowMappingFirstKey;
        } else {
            self.state = State::BlockMappingFirstKey;
        }
        true
    }

    /// Check if the next events represent an empty sequence.
    // Go: emitterc.go:yaml_emitter_check_empty_sequence
    fn check_empty_sequence(&self) -> bool {
        if self.events.len() - self.events_head < 2 {
            return false;
        }
        self.events[self.events_head].typ == EventType::SequenceStart
            && self.events[self.events_head + 1].typ == EventType::SequenceEnd
    }

    /// Check if the next events represent an empty mapping.
    // Go: emitterc.go:yaml_emitter_check_empty_mapping
    fn check_empty_mapping(&self) -> bool {
        if self.events.len() - self.events_head < 2 {
            return false;
        }
        self.events[self.events_head].typ == EventType::MappingStart
            && self.events[self.events_head + 1].typ == EventType::MappingEnd
    }

    /// Check if the next node can be expressed as a simple key.
    // Go: emitterc.go:yaml_emitter_check_simple_key
    fn check_simple_key(&self) -> bool {
        let mut length = 0;
        match self.events[self.events_head].typ {
            EventType::Scalar => {
                if self.scalar_data.multiline {
                    return false;
                }
                length +=
                    self.tag_handle.len() + self.tag_suffix.len() + self.scalar_data.value.len();
            }
            EventType::SequenceStart => {
                if !self.check_empty_sequence() {
                    return false;
                }
                length += self.tag_handle.len() + self.tag_suffix.len();
            }
            EventType::MappingStart => {
                if !self.check_empty_mapping() {
                    return false;
                }
                length += self.tag_handle.len() + self.tag_suffix.len();
            }
            _ => return false,
        }
        length <= 128
    }

    /// Determine an acceptable scalar style.
    // Go: emitterc.go:yaml_emitter_select_scalar_style
    fn select_scalar_style(&mut self, event: &Event) -> bool {
        let no_tag = self.tag_handle.is_empty() && self.tag_suffix.is_empty();
        if no_tag && !event.implicit && !event.quoted_implicit {
            return self.set_emitter_error("neither tag nor implicit flags are specified");
        }

        let mut style = event.style;
        if style == Style::Any {
            style = Style::Plain;
        }
        if self.canonical {
            style = Style::DoubleQuoted;
        }
        if self.simple_key_context && self.scalar_data.multiline {
            style = Style::DoubleQuoted;
        }

        if style == Style::Plain {
            if self.flow_level > 0 && !self.scalar_data.flow_plain_allowed
                || self.flow_level == 0 && !self.scalar_data.block_plain_allowed
            {
                style = Style::SingleQuoted;
            }
            if self.scalar_data.value.is_empty() && (self.flow_level > 0 || self.simple_key_context)
            {
                style = Style::SingleQuoted;
            }
            if no_tag && !event.implicit {
                style = Style::SingleQuoted;
            }
        }
        if style == Style::SingleQuoted && !self.scalar_data.single_quoted_allowed {
            style = Style::DoubleQuoted;
        }
        if (style == Style::Literal || style == Style::Folded)
            && (!self.scalar_data.block_allowed || self.flow_level > 0 || self.simple_key_context)
        {
            style = Style::DoubleQuoted;
        }

        if no_tag && !event.quoted_implicit && style != Style::Plain {
            self.tag_handle = b"!".to_vec();
        }
        self.scalar_data.style = Some(style);
        true
    }

    /// Write a tag.
    // Go: emitterc.go:yaml_emitter_process_tag
    fn process_tag(&mut self) -> bool {
        if self.tag_handle.is_empty() && self.tag_suffix.is_empty() {
            return true;
        }
        let handle = self.tag_handle.clone();
        let suffix = self.tag_suffix.clone();
        if !handle.is_empty() {
            if !self.write_tag_handle(&handle) {
                return false;
            }
            if !suffix.is_empty() && !self.write_tag_content(&suffix, false) {
                return false;
            }
        } else {
            // [Go] Allocate these slices elsewhere.
            if !self.write_indicator(b"!<", true, false, false) {
                return false;
            }
            if !self.write_tag_content(&suffix, false) {
                return false;
            }
            if !self.write_indicator(b">", false, false, false) {
                return false;
            }
        }
        true
    }

    /// Write a scalar.
    // Go: emitterc.go:yaml_emitter_process_scalar
    fn process_scalar(&mut self) -> bool {
        let value = self.scalar_data.value.clone();
        let allow_breaks = !self.simple_key_context;
        match self.scalar_data.style.expect("scalar style") {
            Style::Plain => self.write_plain_scalar(&value, allow_breaks),
            Style::SingleQuoted => self.write_single_quoted_scalar(&value, allow_breaks),
            Style::DoubleQuoted => self.write_double_quoted_scalar(&value, allow_breaks),
            Style::Literal => self.write_literal_scalar(&value),
            Style::Folded => self.write_folded_scalar(&value),
            _ => panic!("unknown scalar style"),
        }
    }

    /// Check if a tag is valid.
    // Go: emitterc.go:yaml_emitter_analyze_tag
    fn analyze_tag(&mut self, tag: &[u8]) -> bool {
        if tag.is_empty() {
            return self.set_emitter_error("tag value must not be empty");
        }
        for td in &self.tag_directives {
            if tag.starts_with(td.prefix) {
                self.tag_handle = td.handle.to_vec();
                self.tag_suffix = tag[td.prefix.len()..].to_vec();
                return true;
            }
        }
        self.tag_suffix = tag.to_vec();
        true
    }

    /// Check if a scalar is valid.
    // Go: emitterc.go:yaml_emitter_analyze_scalar
    fn analyze_scalar(&mut self, value: &[u8]) -> bool {
        let mut block_indicators = false;
        let mut flow_indicators = false;
        let mut line_breaks = false;
        let mut special_characters = false;

        let mut leading_space = false;
        let mut leading_break = false;
        let mut trailing_space = false;
        let mut trailing_break = false;
        let mut break_space = false;
        let mut space_break = false;

        let mut previous_space = false;
        let mut previous_break = false;

        self.scalar_data.value = value.to_vec();

        if value.is_empty() {
            self.scalar_data.multiline = false;
            self.scalar_data.flow_plain_allowed = false;
            self.scalar_data.block_plain_allowed = true;
            self.scalar_data.single_quoted_allowed = true;
            self.scalar_data.block_allowed = false;
            return true;
        }

        if value.len() >= 3
            && ((value[0] == b'-' && value[1] == b'-' && value[2] == b'-')
                || (value[0] == b'.' && value[1] == b'.' && value[2] == b'.'))
        {
            block_indicators = true;
            flow_indicators = true;
        }

        let mut preceded_by_whitespace = true;
        let mut i = 0;
        while i < value.len() {
            let w = width(value[i]);
            let followed_by_whitespace = i + w >= value.len() || is_blank(value, i + w);

            if i == 0 {
                match value[i] {
                    b'#' | b',' | b'[' | b']' | b'{' | b'}' | b'&' | b'*' | b'!' | b'|' | b'>'
                    | b'\'' | b'"' | b'%' | b'@' | b'`' => {
                        flow_indicators = true;
                        block_indicators = true;
                    }
                    b'?' | b':' => {
                        flow_indicators = true;
                        if followed_by_whitespace {
                            block_indicators = true;
                        }
                    }
                    b'-' => {
                        if followed_by_whitespace {
                            flow_indicators = true;
                            block_indicators = true;
                        }
                    }
                    _ => {}
                }
            } else {
                match value[i] {
                    b',' | b'?' | b'[' | b']' | b'{' | b'}' => {
                        flow_indicators = true;
                    }
                    b':' => {
                        flow_indicators = true;
                        if followed_by_whitespace {
                            block_indicators = true;
                        }
                    }
                    b'#' => {
                        if preceded_by_whitespace {
                            flow_indicators = true;
                            block_indicators = true;
                        }
                    }
                    _ => {}
                }
            }

            if !is_printable(value, i) || !is_ascii(value, i) && !self.unicode {
                special_characters = true;
            }
            if is_space(value, i) {
                if i == 0 {
                    leading_space = true;
                }
                if i + width(value[i]) == value.len() {
                    trailing_space = true;
                }
                if previous_break {
                    break_space = true;
                }
                previous_space = true;
                previous_break = false;
            } else if is_break(value, i) {
                line_breaks = true;
                if i == 0 {
                    leading_break = true;
                }
                if i + width(value[i]) == value.len() {
                    trailing_break = true;
                }
                if previous_space {
                    space_break = true;
                }
                previous_space = false;
                previous_break = true;
            } else {
                previous_space = false;
                previous_break = false;
            }

            // [Go]: Why 'z'? Couldn't be the end of the string as that's the loop condition.
            preceded_by_whitespace = is_blankz(value, i);
            i += w;
        }

        self.scalar_data.multiline = line_breaks;
        self.scalar_data.flow_plain_allowed = true;
        self.scalar_data.block_plain_allowed = true;
        self.scalar_data.single_quoted_allowed = true;
        self.scalar_data.block_allowed = true;

        if leading_space || leading_break || trailing_space || trailing_break {
            self.scalar_data.flow_plain_allowed = false;
            self.scalar_data.block_plain_allowed = false;
        }
        if trailing_space {
            self.scalar_data.block_allowed = false;
        }
        if break_space {
            self.scalar_data.flow_plain_allowed = false;
            self.scalar_data.block_plain_allowed = false;
            self.scalar_data.single_quoted_allowed = false;
        }
        if space_break || special_characters {
            self.scalar_data.flow_plain_allowed = false;
            self.scalar_data.block_plain_allowed = false;
            self.scalar_data.single_quoted_allowed = false;
            self.scalar_data.block_allowed = false;
        }
        if line_breaks {
            self.scalar_data.flow_plain_allowed = false;
            self.scalar_data.block_plain_allowed = false;
        }
        if flow_indicators {
            self.scalar_data.flow_plain_allowed = false;
        }
        if block_indicators {
            self.scalar_data.block_plain_allowed = false;
        }
        true
    }

    /// Check if the event data is valid.
    // Go: emitterc.go:yaml_emitter_analyze_event
    fn analyze_event(&mut self, event: &Event) -> bool {
        self.tag_handle.clear();
        self.tag_suffix.clear();
        self.scalar_data.value.clear();

        match event.typ {
            EventType::Scalar => {
                if !event.tag.is_empty()
                    && (self.canonical || (!event.implicit && !event.quoted_implicit))
                    && !self.analyze_tag(&event.tag)
                {
                    return false;
                }
                if !self.analyze_scalar(&event.value) {
                    return false;
                }
            }
            EventType::SequenceStart | EventType::MappingStart => {
                if !event.tag.is_empty()
                    && (self.canonical || !event.implicit)
                    && !self.analyze_tag(&event.tag)
                {
                    return false;
                }
            }
            _ => {}
        }
        true
    }

    // Go: emitterc.go:yaml_emitter_write_indent
    fn write_indent(&mut self) -> bool {
        let indent = self.indent.max(0);
        if (!self.indention || self.column > indent || (self.column == indent && !self.whitespace))
            && !self.put_break()
        {
            return false;
        }
        while self.column < indent {
            if !self.put(b' ') {
                return false;
            }
        }
        self.whitespace = true;
        self.indention = true;
        true
    }

    // Go: emitterc.go:yaml_emitter_write_indicator
    fn write_indicator(
        &mut self,
        indicator: &[u8],
        need_whitespace: bool,
        is_whitespace: bool,
        is_indention: bool,
    ) -> bool {
        if need_whitespace && !self.whitespace && !self.put(b' ') {
            return false;
        }
        if !self.write_all(indicator) {
            return false;
        }
        self.whitespace = is_whitespace;
        self.indention = self.indention && is_indention;
        self.open_ended = false;
        true
    }

    // Go: emitterc.go:yaml_emitter_write_tag_handle
    fn write_tag_handle(&mut self, value: &[u8]) -> bool {
        if !self.whitespace && !self.put(b' ') {
            return false;
        }
        if !self.write_all(value) {
            return false;
        }
        self.whitespace = false;
        self.indention = false;
        true
    }

    // Go: emitterc.go:yaml_emitter_write_tag_content
    fn write_tag_content(&mut self, value: &[u8], need_whitespace: bool) -> bool {
        if need_whitespace && !self.whitespace && !self.put(b' ') {
            return false;
        }
        let mut i = 0;
        while i < value.len() {
            let must_write = match value[i] {
                b';' | b'/' | b'?' | b':' | b'@' | b'&' | b'=' | b'+' | b'$' | b',' | b'_'
                | b'.' | b'~' | b'*' | b'\'' | b'(' | b')' | b'[' | b']' => true,
                _ => is_alpha(value, i),
            };
            if must_write {
                if !self.write(value, &mut i) {
                    return false;
                }
            } else {
                let w = width(value[i]);
                for _ in 0..w {
                    let octet = value[i];
                    i += 1;
                    if !self.put(b'%') {
                        return false;
                    }

                    let mut c = octet >> 4;
                    if c < 10 {
                        c += b'0';
                    } else {
                        c += b'A' - 10;
                    }
                    if !self.put(c) {
                        return false;
                    }

                    c = octet & 0x0f;
                    if c < 10 {
                        c += b'0';
                    } else {
                        c += b'A' - 10;
                    }
                    if !self.put(c) {
                        return false;
                    }
                }
            }
        }
        self.whitespace = false;
        self.indention = false;
        true
    }

    // Go: emitterc.go:yaml_emitter_write_plain_scalar
    fn write_plain_scalar(&mut self, value: &[u8], allow_breaks: bool) -> bool {
        if !self.whitespace && !self.put(b' ') {
            return false;
        }

        let mut spaces = false;
        let mut breaks = false;
        let mut i = 0;
        while i < value.len() {
            if is_space(value, i) {
                if allow_breaks
                    && !spaces
                    && self.column > self.best_width
                    && !is_space(value, i + 1)
                {
                    if !self.write_indent() {
                        return false;
                    }
                    i += width(value[i]);
                } else if !self.write(value, &mut i) {
                    return false;
                }
                spaces = true;
            } else if is_break(value, i) {
                if !breaks && value[i] == b'\n' && !self.put_break() {
                    return false;
                }
                if !self.write_break(value, &mut i) {
                    return false;
                }
                self.indention = true;
                breaks = true;
            } else {
                if breaks && !self.write_indent() {
                    return false;
                }
                if !self.write(value, &mut i) {
                    return false;
                }
                self.indention = false;
                spaces = false;
                breaks = false;
            }
        }

        self.whitespace = false;
        self.indention = false;
        if self.root_context {
            self.open_ended = true;
        }

        true
    }

    // Go: emitterc.go:yaml_emitter_write_single_quoted_scalar
    fn write_single_quoted_scalar(&mut self, value: &[u8], allow_breaks: bool) -> bool {
        if !self.write_indicator(b"'", true, false, false) {
            return false;
        }

        let mut spaces = false;
        let mut breaks = false;
        let mut i = 0;
        while i < value.len() {
            if is_space(value, i) {
                if allow_breaks
                    && !spaces
                    && self.column > self.best_width
                    && i > 0
                    && i < value.len() - 1
                    && !is_space(value, i + 1)
                {
                    if !self.write_indent() {
                        return false;
                    }
                    i += width(value[i]);
                } else if !self.write(value, &mut i) {
                    return false;
                }
                spaces = true;
            } else if is_break(value, i) {
                if !breaks && value[i] == b'\n' && !self.put_break() {
                    return false;
                }
                if !self.write_break(value, &mut i) {
                    return false;
                }
                self.indention = true;
                breaks = true;
            } else {
                if breaks && !self.write_indent() {
                    return false;
                }
                if value[i] == b'\'' && !self.put(b'\'') {
                    return false;
                }
                if !self.write(value, &mut i) {
                    return false;
                }
                self.indention = false;
                spaces = false;
                breaks = false;
            }
        }
        if !self.write_indicator(b"'", false, false, false) {
            return false;
        }
        self.whitespace = false;
        self.indention = false;
        true
    }

    // Go: emitterc.go:yaml_emitter_write_double_quoted_scalar
    fn write_double_quoted_scalar(&mut self, value: &[u8], allow_breaks: bool) -> bool {
        let mut spaces = false;
        if !self.write_indicator(b"\"", true, false, false) {
            return false;
        }

        let mut i = 0;
        while i < value.len() {
            if !is_printable(value, i)
                || (!self.unicode && !is_ascii(value, i))
                || is_bom(value, i)
                || is_break(value, i)
                || value[i] == b'"'
                || value[i] == b'\\'
            {
                let mut octet = value[i];

                let (mut w, mut v): (usize, i32) = if octet & 0x80 == 0x00 {
                    (1, (octet & 0x7F) as i32)
                } else if octet & 0xE0 == 0xC0 {
                    (2, (octet & 0x1F) as i32)
                } else if octet & 0xF0 == 0xE0 {
                    (3, (octet & 0x0F) as i32)
                } else if octet & 0xF8 == 0xF0 {
                    (4, (octet & 0x07) as i32)
                } else {
                    (0, 0)
                };
                for k in 1..w {
                    octet = value[i + k];
                    v = (v << 6) + ((octet as i32) & 0x3F);
                }
                i += w;

                if !self.put(b'\\') {
                    return false;
                }

                let ok = match v {
                    0x00 => self.put(b'0'),
                    0x07 => self.put(b'a'),
                    0x08 => self.put(b'b'),
                    0x09 => self.put(b't'),
                    0x0A => self.put(b'n'),
                    0x0b => self.put(b'v'),
                    0x0c => self.put(b'f'),
                    0x0d => self.put(b'r'),
                    0x1b => self.put(b'e'),
                    0x22 => self.put(b'"'),
                    0x5c => self.put(b'\\'),
                    0x85 => self.put(b'N'),
                    0xA0 => self.put(b'_'),
                    0x2028 => self.put(b'L'),
                    0x2029 => self.put(b'P'),
                    _ => {
                        let ok;
                        if v <= 0xFF {
                            ok = self.put(b'x');
                            w = 2;
                        } else if v <= 0xFFFF {
                            ok = self.put(b'u');
                            w = 4;
                        } else {
                            ok = self.put(b'U');
                            w = 8;
                        }
                        let mut ok = ok;
                        let mut k = (w as i32 - 1) * 4;
                        while ok && k >= 0 {
                            let digit = ((v >> k) & 0x0F) as u8;
                            if digit < 10 {
                                ok = self.put(digit + b'0');
                            } else {
                                ok = self.put(digit + b'A' - 10);
                            }
                            k -= 4;
                        }
                        ok
                    }
                };
                if !ok {
                    return false;
                }
                spaces = false;
            } else if is_space(value, i) {
                if allow_breaks
                    && !spaces
                    && self.column > self.best_width
                    && i > 0
                    && i < value.len() - 1
                {
                    if !self.write_indent() {
                        return false;
                    }
                    if is_space(value, i + 1) && !self.put(b'\\') {
                        return false;
                    }
                    i += width(value[i]);
                } else if !self.write(value, &mut i) {
                    return false;
                }
                spaces = true;
            } else {
                if !self.write(value, &mut i) {
                    return false;
                }
                spaces = false;
            }
        }
        if !self.write_indicator(b"\"", false, false, false) {
            return false;
        }
        self.whitespace = false;
        self.indention = false;
        true
    }

    // Go: emitterc.go:yaml_emitter_write_block_scalar_hints
    fn write_block_scalar_hints(&mut self, value: &[u8]) -> bool {
        if is_space(value, 0) || is_break(value, 0) {
            let indent_hint = [b'0' + self.best_indent as u8];
            if !self.write_indicator(&indent_hint, false, false, false) {
                return false;
            }
        }

        self.open_ended = false;

        let mut chomp_hint = 0u8;
        if value.is_empty() {
            chomp_hint = b'-';
        } else {
            let mut i = value.len() - 1;
            while value[i] & 0xC0 == 0x80 {
                i -= 1;
            }
            if !is_break(value, i) {
                chomp_hint = b'-';
            } else if i == 0 {
                chomp_hint = b'+';
                self.open_ended = true;
            } else {
                i -= 1;
                while value[i] & 0xC0 == 0x80 {
                    i -= 1;
                }
                if is_break(value, i) {
                    chomp_hint = b'+';
                    self.open_ended = true;
                }
            }
        }
        if chomp_hint != 0 && !self.write_indicator(&[chomp_hint], false, false, false) {
            return false;
        }
        true
    }

    // Go: emitterc.go:yaml_emitter_write_literal_scalar
    fn write_literal_scalar(&mut self, value: &[u8]) -> bool {
        if !self.write_indicator(b"|", true, false, false) {
            return false;
        }
        if !self.write_block_scalar_hints(value) {
            return false;
        }
        if !self.put_break() {
            return false;
        }
        self.indention = true;
        self.whitespace = true;
        let mut breaks = true;
        let mut i = 0;
        while i < value.len() {
            if is_break(value, i) {
                if !self.write_break(value, &mut i) {
                    return false;
                }
                self.indention = true;
                breaks = true;
            } else {
                if breaks && !self.write_indent() {
                    return false;
                }
                if !self.write(value, &mut i) {
                    return false;
                }
                self.indention = false;
                breaks = false;
            }
        }

        true
    }

    // Go: emitterc.go:yaml_emitter_write_folded_scalar
    fn write_folded_scalar(&mut self, value: &[u8]) -> bool {
        if !self.write_indicator(b">", true, false, false) {
            return false;
        }
        if !self.write_block_scalar_hints(value) {
            return false;
        }

        if !self.put_break() {
            return false;
        }
        self.indention = true;
        self.whitespace = true;

        let mut breaks = true;
        let mut leading_spaces = true;
        let mut i = 0;
        while i < value.len() {
            if is_break(value, i) {
                if !breaks && !leading_spaces && value[i] == b'\n' {
                    let mut k = 0;
                    while is_break(value, k) {
                        k += width(value[k]);
                    }
                    if !is_blankz(value, k) && !self.put_break() {
                        return false;
                    }
                }
                if !self.write_break(value, &mut i) {
                    return false;
                }
                self.indention = true;
                breaks = true;
            } else {
                if breaks {
                    if !self.write_indent() {
                        return false;
                    }
                    leading_spaces = is_blank(value, i);
                }
                if !breaks
                    && is_space(value, i)
                    && !is_space(value, i + 1)
                    && self.column > self.best_width
                {
                    if !self.write_indent() {
                        return false;
                    }
                    i += width(value[i]);
                } else if !self.write(value, &mut i) {
                    return false;
                }
                self.indention = false;
                breaks = false;
            }
        }
        true
    }
}

/// Go: `yaml_event_type_t.String()` (`%v` of an event type).
fn event_type_string(t: EventType) -> &'static str {
    match t {
        EventType::StreamStart => "stream start",
        EventType::StreamEnd => "stream end",
        EventType::DocumentStart => "document start",
        EventType::DocumentEnd => "document end",
        EventType::Scalar => "scalar",
        EventType::SequenceStart => "sequence start",
        EventType::SequenceEnd => "sequence end",
        EventType::MappingStart => "mapping start",
        EventType::MappingEnd => "mapping end",
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (gopkg.in/yaml.v2@v2.4.0)
// OK yaml.go: Marshal (handleErr/failf: the error is returned)
// OK encode.go: newEncoder, init, finish, emit, must, marshalDoc, marshal (the kinds of Node),
//    mapv, slicev, mappingv (block style), isBase60Float, base60float, stringv, boolv, intv,
//    uintv, timev, floatv, nilv, emitScalar
// NOT PORTED (unreachable from a Node): itemsv, structv, the flow style, Marshaler,
//    TextMarshaler and jsonNumber dispatch (the caller converts), durations
// OK sorter.go: keyList.Less for string keys (keyFloat/numLess: string keys only)
// OK resolve.go: encodeBase64
// OK emitterc.go: flush/put/put_break/write/write_all/write_break (string output),
//    set_emitter_error, emit, need_more_events, append_tag_directive, increase_indent,
//    state_machine, emit_stream_start, emit_document_start, emit_document_content,
//    emit_document_end, emit_flow_sequence_item, emit_flow_mapping_key,
//    emit_flow_mapping_value, emit_block_sequence_item, emit_block_mapping_key,
//    emit_block_mapping_value, emit_node, emit_scalar, emit_sequence_start,
//    emit_mapping_start, check_empty_document, check_empty_sequence, check_empty_mapping,
//    check_simple_key, select_scalar_style, process_tag, process_scalar, analyze_tag,
//    analyze_scalar, analyze_event, write_indent, write_indicator, write_tag_handle,
//    write_tag_content, write_plain_scalar, write_single_quoted_scalar,
//    write_double_quoted_scalar, write_block_scalar_hints, write_literal_scalar,
//    write_folded_scalar
// NOT PORTED (no anchors, aliases or directives are emitted): emit_alias, process_anchor,
//    analyze_version_directive, analyze_tag_directive, analyze_anchor, write_anchor,
//    write_bom
// ---------------------------------------------------------------------------
