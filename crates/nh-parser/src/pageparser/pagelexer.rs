//! Port of `parser/pageparser/pagelexer.go`.
//!
//! Owner: Wave B task T03 (parser-langs).

use go_unicode::Rune;

use super::item::{Item, ItemType, Items};
use super::pagelexer_shortcode::{
    LEFT_DELIM_SC, LEFT_DELIM_SC_WITH_MARKUP, LexerShortcodeState, lex_shortcode_left_delim,
};

/// Go: `eof = -1`.
pub(crate) const EOF: Rune = -1;

/// Go: `pageparser.Config`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Config {
    pub no_front_matter: bool,
    pub no_summary_divider: bool,
}

/// Go: `stateFunc func(*pageLexer) stateFunc`. The Go closures that capture arguments
/// (`lexShortcodeParam(l, escaped)`, `lexShortcodeQuotedParamVal(l, …)`, the section handlers'
/// `origin`, always `lexMainSection`) are called directly, as Go does.
#[derive(Clone, Copy)]
pub(crate) struct StateFn(pub(crate) fn(&mut PageLexer) -> Option<StateFn>);

// Page syntax
/// Go: `byteOrderMark`.
pub(crate) const BYTE_ORDER_MARK: Rune = 0xFEFF;
/// Go: `summaryDivider`.
pub(crate) const SUMMARY_DIVIDER: &[u8] = b"<!--more-->";
/// Go: `summaryDividerOrg`.
pub(crate) const SUMMARY_DIVIDER_ORG: &[u8] = b"# more";
/// Go: `delimTOML`.
pub(crate) const DELIM_TOML: &[u8] = b"+++";
/// Go: `delimYAML`.
pub(crate) const DELIM_YAML: &[u8] = b"---";
/// Go: `delimOrg`.
pub(crate) const DELIM_ORG: &[u8] = b"#+";

/// Go: `crLf`.
const CR_LF: [Rune; 2] = ['\r' as Rune, '\n' as Rune];

/// Go: `pageLexer` — the front matter / summary divider / shortcode lexer (a Rob Pike style
/// state machine, ported state by state).
pub struct PageLexer {
    pub(crate) input: Vec<u8>,
    pub(crate) state_start: StateFn,
    /// input position
    pub(crate) pos: usize,
    /// item start position
    pub(crate) start: usize,
    /// width of last element
    pub(crate) width: usize,
    /// Contains lexers for shortcodes and other main section elements.
    pub(crate) section_handlers: SectionHandlers,
    pub(crate) cfg: Config,
    /// The summary divider to look for.
    pub(crate) summary_divider: &'static [u8],
    /// Set when we have parsed any summary divider
    pub(crate) summary_divider_checked: bool,
    pub(crate) shortcode: LexerShortcodeState,
    /// items delivered to client
    pub(crate) items: Items,
    /// error delivered to the client (Go never sets it).
    pub(crate) err: Option<String>,
}

impl PageLexer {
    /// Go: `Iterator()`.
    // Go: parser/pageparser/pagelexer.go:Iterator
    pub fn iterator(&self) -> super::pageparser::Iterator<'_> {
        super::pageparser::new_iterator(&self.items)
    }

    // Go: parser/pageparser/pagelexer.go:Input
    pub fn input(&self) -> &[u8] {
        &self.input
    }

    /// The lexed items.
    pub fn items(&self) -> &Items {
        &self.items
    }

    /// Moves the lexed items out.
    pub fn into_items(self) -> Items {
        self.items
    }

    /// note: the input position here is normally 0 (start), but
    /// can be set if position of first shortcode is known
    // Go: parser/pageparser/pagelexer.go:newPageLexer
    pub(crate) fn new(input: Vec<u8>, state_start: StateFn, cfg: Config) -> Self {
        let mut lexer = PageLexer {
            input,
            state_start,
            pos: 0,
            start: 0,
            width: 0,
            section_handlers: SectionHandlers::default(),
            cfg,
            summary_divider: SUMMARY_DIVIDER,
            summary_divider_checked: false,
            shortcode: LexerShortcodeState::new(),
            items: Vec::with_capacity(5),
            err: None,
        };

        lexer.section_handlers = create_section_handlers(&lexer);

        lexer
    }

    /// main loop
    // Go: parser/pageparser/pagelexer.go:run
    pub(crate) fn run(&mut self) {
        let mut state = Some(self.state_start);
        while let Some(s) = state {
            state = (s.0)(self);
        }
    }

    // Go: parser/pageparser/pagelexer.go:next
    pub(crate) fn next(&mut self) -> Rune {
        if self.pos >= self.input.len() {
            self.width = 0;
            return EOF;
        }

        let (rune_value, rune_width) = go_unicode::utf8::decode_rune(&self.input[self.pos..]);
        self.width = rune_width;
        self.pos += self.width;

        rune_value
    }

    /// peek, but no consume
    // Go: parser/pageparser/pagelexer.go:peek
    pub(crate) fn peek(&mut self) -> Rune {
        let r = self.next();
        self.backup();
        r
    }

    /// steps back one
    // Go: parser/pageparser/pagelexer.go:backup
    pub(crate) fn backup(&mut self) {
        self.pos -= self.width;
    }

    // Go: parser/pageparser/pagelexer.go:append
    pub(crate) fn append(&mut self, mut item: Item) {
        if item.pos() < self.input.len() {
            item.first_byte = self.input[item.pos()];
        }
        self.items.push(item);
    }

    /// sends an item back to the client.
    // Go: parser/pageparser/pagelexer.go:emit
    pub(crate) fn emit(&mut self, t: ItemType) {
        self.emit_inner(t);
        // Go: defer func() { l.start = l.pos }()
        self.start = self.pos;
    }

    fn emit_inner(&mut self, t: ItemType) {
        if t == ItemType::Text {
            // Identify any trailing whitespace/intendation.
            // We currently only care about the last one.
            let mut i = self.pos as isize - 1;
            while i >= self.start as isize {
                let iu = i as usize;
                let b = self.input[iu];
                if b != b' ' && b != b'\t' && b != b'\r' && b != b'\n' {
                    break;
                }
                if iu == self.start && b != b'\n' {
                    self.append(Item::new(ItemType::Indentation, self.start, self.pos));
                    return;
                } else if b == b'\n' && iu < self.pos - 1 {
                    self.append(Item::new(t, self.start, iu + 1));
                    self.append(Item::new(ItemType::Indentation, iu + 1, self.pos));
                    return;
                } else if b == b'\n' && iu == self.pos - 1 {
                    break;
                }
                i -= 1;
            }
        }

        self.append(Item::new(t, self.start, self.pos));
    }

    /// sends a string item back to the client.
    // Go: parser/pageparser/pagelexer.go:emitString
    pub(crate) fn emit_string(&mut self, t: ItemType) {
        let mut item = Item::new(t, self.start, self.pos);
        item.is_string = true;
        self.append(item);
        self.start = self.pos;
    }

    // Go: parser/pageparser/pagelexer.go:isEOF
    pub(crate) fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// special case, do not send '\\' back to client
    ///
    /// Note: Go's `isString` argument is not stored in the item (the literal only sets `Type`
    /// and `segments`); kept as is.
    // Go: parser/pageparser/pagelexer.go:ignoreEscapesAndEmit
    pub(crate) fn ignore_escapes_and_emit(&mut self, t: ItemType, _is_string: bool) {
        let mut i = self.start;
        let mut k = i;

        let mut segments: Vec<(usize, usize)> = Vec::new();

        while i < self.pos {
            let (r, w) = go_unicode::utf8::decode_rune(&self.input[i..self.pos]);
            if r == '\\' as Rune {
                if i > k {
                    segments.push((k, i));
                }
                // See issue #10236.
                // We don't send the backslash back to the client,
                // which makes the end parsing simpler.
                // This means that we cannot render the AST back to be
                // exactly the same as the input,
                // but that was also the situation before we introduced the issue in #10236.
                k = i + w;
            }
            i += w;
        }

        if k < self.pos {
            segments.push((k, self.pos));
        }

        if !segments.is_empty() {
            let mut item = Item::new(t, 0, 0);
            item.segments = segments;
            self.append(item);
        }

        self.start = self.pos;
    }

    /// gets the current value (for debugging and error handling)
    // Go: parser/pageparser/pagelexer.go:current
    pub(crate) fn current(&self) -> &[u8] {
        &self.input[self.start..self.pos]
    }

    /// ignore current element
    // Go: parser/pageparser/pagelexer.go:ignore
    pub(crate) fn ignore(&mut self) {
        self.start = self.pos;
    }

    /// nil terminates the parser
    ///
    /// Go formats with `fmt.Errorf(format, args...)`; callers pass the formatted message bytes.
    // Go: parser/pageparser/pagelexer.go:errorf
    pub(crate) fn errorf(&mut self, msg: Vec<u8>) -> Option<StateFn> {
        let mut item = Item::new(ItemType::Error, self.start, self.pos);
        item.err = Some(String::from_utf8_lossy(&msg).into_owned());
        item.err_bytes = Some(msg);
        self.append(item);
        None
    }

    // Go: parser/pageparser/pagelexer.go:consumeCRLF
    pub(crate) fn consume_crlf(&mut self) -> bool {
        let mut consumed = false;
        for r in CR_LF {
            if self.next() != r {
                self.backup();
            } else {
                consumed = true;
            }
        }
        consumed
    }

    // Go: parser/pageparser/pagelexer.go:consumeToSpace
    pub(crate) fn consume_to_space(&mut self) {
        loop {
            let r = self.next();
            if r == EOF || go_unicode::is_space(r) {
                self.backup();
                return;
            }
        }
    }

    // Go: parser/pageparser/pagelexer.go:consumeSpace
    pub(crate) fn consume_space(&mut self) {
        loop {
            let r = self.next();
            if r == EOF || !go_unicode::is_space(r) {
                self.backup();
                return;
            }
        }
    }

    // state helpers

    // Go: parser/pageparser/pagelexer.go:index
    pub(crate) fn index(&self, sep: &[u8]) -> isize {
        go_unicode::bytes::index(&self.input[self.pos..], sep)
    }

    // Go: parser/pageparser/pagelexer.go:hasPrefix
    pub(crate) fn has_prefix(&self, prefix: &[u8]) -> bool {
        self.input[self.pos..].starts_with(prefix)
    }

    /// Go: `printCurrentInput` (debugging).
    // Go: parser/pageparser/pagelexer.go:printCurrentInput
    #[allow(dead_code)]
    pub(crate) fn print_current_input(&self) {
        print!(
            "input[{}:]: {}",
            self.pos,
            go_strconv::quote(&self.input[self.pos..])
        );
    }
}

/// Go: `sectionHandler` kinds (Go stores closures; the two handlers are an enum here).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SectionKind {
    Shortcode,
    SummaryDivider,
}

/// Go: `sectionHandler`.
#[derive(Clone, Debug)]
pub(crate) struct SectionHandler {
    /// No more sections of this type.
    pub(crate) skip_all: bool,
    pub(crate) kind: SectionKind,
}

/// Go: `sectionHandlers`.
#[derive(Clone, Debug, Default)]
pub(crate) struct SectionHandlers {
    /// Set when none of the sections are found so we
    /// can safely stop looking and skip to the end.
    pub(crate) skip_all: bool,
    pub(crate) handlers: Vec<SectionHandler>,
    pub(crate) skip_indexes: Vec<isize>,
}

// Go: parser/pageparser/pagelexer.go:skip
fn section_handlers_skip(l: &mut PageLexer) -> isize {
    if l.section_handlers.skip_all {
        return -1;
    }

    l.section_handlers.skip_indexes.clear();
    let mut should_skip = false;
    for i in 0..l.section_handlers.handlers.len() {
        let idx = section_handler_skip(l, i);
        if idx != -1 {
            should_skip = true;
            l.section_handlers.skip_indexes.push(idx);
        }
    }

    if !should_skip {
        l.section_handlers.skip_all = true;
        return -1;
    }

    min_index(&l.section_handlers.skip_indexes)
}

// Go: parser/pageparser/pagelexer.go:createSectionHandlers
fn create_section_handlers(l: &PageLexer) -> SectionHandlers {
    let mut handlers = Vec::with_capacity(2);

    handlers.push(SectionHandler {
        skip_all: false,
        kind: SectionKind::Shortcode,
    });

    if !l.cfg.no_summary_divider {
        handlers.push(SectionHandler {
            skip_all: false,
            kind: SectionKind::SummaryDivider,
        });
    }

    let n = handlers.len();
    SectionHandlers {
        skip_all: false,
        handlers,
        skip_indexes: Vec::with_capacity(n),
    }
}

/// The `skipFunc` of each handler.
fn skip_func(l: &PageLexer, kind: SectionKind) -> isize {
    match kind {
        SectionKind::Shortcode => l.index(LEFT_DELIM_SC),
        SectionKind::SummaryDivider => {
            if l.summary_divider_checked {
                return -1;
            }
            l.index(l.summary_divider)
        }
    }
}

/// The `lexFunc` of each handler (`origin` is always `lexMainSection`). Returns
/// `(next, handled)`; Go's `(nil, true)` is `(None, true)`.
fn lex_func(l: &mut PageLexer, kind: SectionKind, origin: StateFn) -> (Option<StateFn>, bool) {
    match kind {
        SectionKind::Shortcode => {
            if !l.is_short_code_start() {
                return (Some(origin), false);
            }

            if l.shortcode.is_inline {
                // If we're inside an inline shortcode, the only valid shortcode markup is
                // the markup which closes it.
                let mut b: &[u8] = &l.input[l.pos + 3..];
                let end = index_non_white_space(b, '/' as Rune);
                if end != l.input.len() as isize - 1 {
                    b = go_unicode::bytes::trim_space(&b[(end + 1) as usize..]);
                    let mut prefix = l.shortcode.curr_shortcode_name.clone();
                    prefix.push(b' ');
                    if end == -1 || !b.starts_with(&prefix) {
                        return (
                            l.errorf(b"inline shortcodes do not support nesting".to_vec()),
                            true,
                        );
                    }
                }
            }

            if l.has_prefix(LEFT_DELIM_SC_WITH_MARKUP) {
                l.shortcode.curr_left_delim_item = ItemType::LeftDelimScWithMarkup;
                l.shortcode.curr_right_delim_item = ItemType::RightDelimScWithMarkup;
            } else {
                l.shortcode.curr_left_delim_item = ItemType::LeftDelimScNoMarkup;
                l.shortcode.curr_right_delim_item = ItemType::RightDelimScNoMarkup;
            }

            (Some(StateFn(lex_shortcode_left_delim)), true)
        }
        SectionKind::SummaryDivider => {
            if !l.has_prefix(l.summary_divider) {
                return (Some(origin), false);
            }

            l.summary_divider_checked = true;
            l.pos += l.summary_divider.len();
            // This makes it a little easier to reason about later.
            l.consume_space();
            l.emit(ItemType::LeadSummaryDivider);

            (Some(origin), true)
        }
    }
}

// Go: parser/pageparser/pagelexer.go:lex
fn section_handlers_lex(l: &mut PageLexer, origin: StateFn) -> Option<StateFn> {
    if l.section_handlers.skip_all {
        return None;
    }

    if l.pos > l.start {
        l.emit(ItemType::Text);
    }

    for i in 0..l.section_handlers.handlers.len() {
        let handler = &l.section_handlers.handlers[i];
        if handler.skip_all {
            continue;
        }
        let kind = handler.kind;

        let (next, handled) = lex_func(l, kind, origin);
        if next.is_none() || handled {
            return next;
        }
    }

    // Not handled by the above.
    l.pos += 1;

    Some(origin)
}

// Go: parser/pageparser/pagelexer.go:(*sectionHandler).skip
fn section_handler_skip(l: &mut PageLexer, i: usize) -> isize {
    if l.section_handlers.handlers[i].skip_all {
        return -1;
    }

    let idx = skip_func(l, l.section_handlers.handlers[i].kind);
    if idx == -1 {
        l.section_handlers.handlers[i].skip_all = true;
    }
    idx
}

// Go: parser/pageparser/pagelexer.go:lexMainSection
pub(crate) fn lex_main_section(l: &mut PageLexer) -> Option<StateFn> {
    if l.is_eof() {
        return Some(StateFn(lex_done));
    }

    // Fast forward as far as possible.
    let skip = section_handlers_skip(l);

    if skip == -1 {
        l.pos = l.input.len();
        return Some(StateFn(lex_done));
    } else if skip > 0 {
        l.pos += skip as usize;
    }

    let next = section_handlers_lex(l, StateFn(lex_main_section));
    if next.is_some() {
        return next;
    }

    l.pos = l.input.len();
    Some(StateFn(lex_done))
}

// Go: parser/pageparser/pagelexer.go:lexDone
pub(crate) fn lex_done(l: &mut PageLexer) -> Option<StateFn> {
    // Done!
    if l.pos > l.start {
        l.emit(ItemType::Text);
    }
    l.emit(ItemType::Eof);
    None
}

// helper functions

/// returns the min index >= 0
// Go: parser/pageparser/pagelexer.go:minIndex
pub(crate) fn min_index(indices: &[isize]) -> isize {
    let mut min = -1;

    for &j in indices {
        if j < 0 {
            continue;
        }
        if min == -1 || j < min {
            min = j;
        }
    }
    min
}

// Go: parser/pageparser/pagelexer.go:indexNonWhiteSpace
pub(crate) fn index_non_white_space(s: &[u8], in_: Rune) -> isize {
    let idx = go_unicode::bytes::index_func(s, |r| !go_unicode::is_space(r));

    if idx == -1 {
        return -1;
    }

    let (r, _) = go_unicode::utf8::decode_rune(&s[idx as usize..]);
    if r == in_ {
        return idx;
    }
    -1
}

// Go: parser/pageparser/pagelexer.go:isSpace
pub(crate) fn is_space(r: Rune) -> bool {
    r == ' ' as Rune || r == '\t' as Rune
}

// Go: parser/pageparser/pagelexer.go:isAlphaNumericOrHyphen
pub(crate) fn is_alpha_numeric_or_hyphen(r: Rune) -> bool {
    // let unquoted YouTube ids as positional params slip through (they contain hyphens)
    is_alpha_numeric(r) || r == '-' as Rune
}

// Go: parser/pageparser/pagelexer.go:isEndOfLine
pub(crate) fn is_end_of_line(r: Rune) -> bool {
    r == '\r' as Rune || r == '\n' as Rune
}

// Go: parser/pageparser/pagelexer.go:isAlphaNumeric
pub(crate) fn is_alpha_numeric(r: Rune) -> bool {
    r == '_' as Rune || go_unicode::is_letter(r) || go_unicode::is_digit(r)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/pageparser/pagelexer.go (530 lines; 23/33 funcs executed)
//   types: stateFunc, pageLexer, Config, sectionHandlers, sectionHandler
// OK L57-59: (l *pageLexer) Iterator() *Iterator
// OK L61-63: (l *pageLexer) Input() []byte
// OK L72-89: newPageLexer(input []byte, stateStart stateFunc, cfg Config) *pageLexer
// OK L92-97: (l *pageLexer) run() *pageLexer
// OK L109-120: (l *pageLexer) next() rune
// OK L123-127: (l *pageLexer) peek() rune
// OK L130-132: (l *pageLexer) backup()
// OK L134-139: (l *pageLexer) append(item Item)
// OK L142-170: (l *pageLexer) emit(t ItemType)
// OK L173-176: (l *pageLexer) emitString(t ItemType)
// OK L178-180: (l *pageLexer) isEOF() bool
// OK L183-215: (l *pageLexer) ignoreEscapesAndEmit(t ItemType, isString bool)
// OK L218-220: (l *pageLexer) current() []byte
// OK L223-225: (l *pageLexer) ignore()
// OK L230-233: (l *pageLexer) errorf(format string, args ...any) stateFunc
// OK L235-245: (l *pageLexer) consumeCRLF() bool
// OK L247-255: (l *pageLexer) consumeToSpace()
// OK L257-265: (l *pageLexer) consumeSpace()
// OK L278-299: (s *sectionHandlers) skip() int
// OK L301-374: createSectionHandlers(l *pageLexer) *sectionHandlers
// OK L376-400: (s *sectionHandlers) lex(origin stateFunc) stateFunc
// OK L418-428: (s *sectionHandler) skip() int
// OK L430-452: lexMainSection(l *pageLexer) stateFunc
// OK L454-461: lexDone(l *pageLexer) stateFunc
// OK L464-466: (l *pageLexer) printCurrentInput()
// OK L470-472: (l *pageLexer) index(sep []byte) int
// OK L474-476: (l *pageLexer) hasPrefix(prefix []byte) bool
// OK L481-495: minIndex(indices ...int) int
// OK L497-511: indexNonWhiteSpace(s []byte, in rune) int
// OK L513-515: isSpace(r rune) bool
// OK L517-520: isAlphaNumericOrHyphen(r rune) bool
// OK L524-526: isEndOfLine(r rune) bool
// OK L528-530: isAlphaNumeric(r rune) bool
// ---------------------------------------------------------------------------
