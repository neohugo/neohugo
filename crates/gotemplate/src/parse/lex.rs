//! The template lexer.
//!
//! Go: tpl/internal/go_templates/texttemplate/parse/lex.go (go1.24.0)
//!
//! Go's state functions (`stateFn`) become an enum dispatched in
//! [`Lexer::next_item`]. The input is bytes; runes are decoded with Go's
//! UTF-8 rules (`go_unicode::utf8`).

use go_unicode::Rune;
use go_unicode::utf8;
use go_value::Value;

use super::node::Pos;

/// Go: `item` — a token or text string returned from the scanner.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// The type of this item.
    pub typ: ItemType,
    /// The starting position, in bytes, of this item in the input string.
    pub pos: Pos,
    /// The value of this item.
    pub val: Vec<u8>,
    /// The line number at the start of this item.
    pub line: usize,
}

impl Default for Item {
    fn default() -> Self {
        Item {
            typ: ItemType::Error,
            pos: 0,
            val: Vec::new(),
            line: 0,
        }
    }
}

impl Item {
    // Go: lex.go:item.String
    pub fn string(&self) -> String {
        if self.typ == ItemType::Eof {
            return "EOF".to_string();
        }
        if self.typ == ItemType::Error {
            return String::from_utf8_lossy(&self.val).into_owned();
        }
        if self.typ.is_keyword() {
            return format!("<{}>", String::from_utf8_lossy(&self.val));
        }
        if self.val.len() > 10 {
            let s = go_fmt::sprintf("%.10q...", &[Value::string(self.val.as_slice())]);
            return String::from_utf8_lossy(&s).into_owned();
        }
        let s = go_fmt::sprintf("%q", &[Value::string(self.val.as_slice())]);
        String::from_utf8_lossy(&s).into_owned()
    }

    pub fn val_str(&self) -> &str {
        std::str::from_utf8(&self.val).unwrap_or("")
    }
}

/// Go: `itemType` — identifies the type of lex items.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub enum ItemType {
    /// error occurred; value is text of error
    Error,
    /// boolean constant
    Bool,
    /// printable ASCII character; grab bag for comma etc.
    Char,
    /// character constant
    CharConstant,
    /// comment text
    Comment,
    /// complex constant (1+2i); imaginary is just a number
    Complex,
    /// equals ('=') introducing an assignment
    Assign,
    /// colon-equals (':=') introducing a declaration
    Declare,
    Eof,
    /// alphanumeric identifier starting with '.'
    Field,
    /// alphanumeric identifier not starting with '.'
    Identifier,
    /// left action delimiter
    LeftDelim,
    /// '(' inside action
    LeftParen,
    /// simple number, including imaginary
    Number,
    /// pipe symbol
    Pipe,
    /// raw quoted string (includes quotes)
    RawString,
    /// right action delimiter
    RightDelim,
    /// ')' inside action
    RightParen,
    /// run of spaces separating arguments
    Space,
    /// quoted string (includes quotes)
    String,
    /// plain text
    Text,
    /// variable starting with '$', such as '$' or  '$1' or '$hello'
    Variable,
    // Keywords appear after all the rest.
    /// used only to delimit the keywords
    Keyword,
    /// block keyword
    Block,
    /// break keyword
    Break,
    /// continue keyword
    Continue,
    /// the cursor, spelled '.'
    Dot,
    /// define keyword
    Define,
    /// else keyword
    Else,
    /// end keyword
    End,
    /// if keyword
    If,
    /// the untyped nil constant, easiest to treat as a keyword
    Nil,
    /// range keyword
    Range,
    /// template keyword
    Template,
    /// with keyword
    With,
}

impl ItemType {
    fn is_keyword(self) -> bool {
        self > ItemType::Keyword
    }
}

// Go: lex.go:key
fn key(word: &[u8]) -> Option<ItemType> {
    Some(match word {
        b"." => ItemType::Dot,
        b"block" => ItemType::Block,
        b"break" => ItemType::Break,
        b"continue" => ItemType::Continue,
        b"define" => ItemType::Define,
        b"else" => ItemType::Else,
        b"end" => ItemType::End,
        b"if" => ItemType::If,
        b"range" => ItemType::Range,
        b"nil" => ItemType::Nil,
        b"template" => ItemType::Template,
        b"with" => ItemType::With,
        _ => return None,
    })
}

const EOF: Rune = -1;

// Trimming spaces.
// If the action begins "{{- " rather than "{{", then all space/tab/newlines
// preceding the action are trimmed; conversely if it ends " -}}" the
// leading spaces are trimmed. This is done entirely in the lexer; the
// parser never sees it happen. We require an ASCII space (' ', \t, \r, \n)
// to be present to avoid ambiguity with things like "{{-3}}". It reads
// better with the space present anyway. For simplicity, only ASCII
// does the job.
/// These are the space characters defined by Go itself.
const SPACE_CHARS: &[u8] = b" \t\r\n";
/// Attached to left/right delimiter, trims trailing spaces from preceding/following text.
const TRIM_MARKER: u8 = b'-';
/// marker plus space before or after
const TRIM_MARKER_LEN: Pos = 1 + 1;

/// Go: `stateFn`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Text,
    LeftDelim,
    Comment,
    RightDelim,
    InsideAction,
    Space,
    Identifier,
    Field,
    Variable,
    Char,
    Number,
    Quote,
    RawQuote,
}

/// Go: `lexOptions` — control behavior of the lexer. All default to false.
#[derive(Clone, Copy, Default, Debug)]
pub struct LexOptions {
    /// emit itemComment tokens.
    pub emit_comment: bool,
    /// break keyword allowed
    pub break_ok: bool,
    /// continue keyword allowed
    pub continue_ok: bool,
}

/// Go: `lexer` — holds the state of the scanner.
pub struct Lexer<'a> {
    /// the name of the input; used only for error reports
    #[allow(dead_code)]
    name: String,
    /// the string being scanned
    input: &'a [u8],
    /// start of action marker
    left_delim: Vec<u8>,
    /// end of action marker
    right_delim: Vec<u8>,
    /// current position in the input
    pos: Pos,
    /// start position of this item
    start: Pos,
    /// we have hit the end of input and returned eof
    at_eof: bool,
    /// nesting depth of ( ) exprs
    paren_depth: i64,
    /// 1+number of newlines seen
    line: usize,
    /// start line of this item
    start_line: usize,
    /// item to return to parser
    item: Item,
    /// are we inside an action?
    inside_action: bool,
    pub options: LexOptions,
}

/// Result of a state function: the next state, or `None` (Go `nil`) when
/// an item is ready in `l.item`.
type StateResult = Option<State>;

const LEFT_DELIM: &[u8] = b"{{";
const RIGHT_DELIM: &[u8] = b"}}";
const LEFT_COMMENT: &[u8] = b"/*";
const RIGHT_COMMENT: &[u8] = b"*/";

impl<'a> Lexer<'a> {
    // Go: lex.go:lex
    /// Creates a new scanner for the input string.
    pub fn new(name: &str, input: &'a [u8], left: &str, right: &str) -> Lexer<'a> {
        let left = if left.is_empty() {
            LEFT_DELIM
        } else {
            left.as_bytes()
        };
        let right = if right.is_empty() {
            RIGHT_DELIM
        } else {
            right.as_bytes()
        };
        Lexer {
            name: name.to_string(),
            input,
            left_delim: left.to_vec(),
            right_delim: right.to_vec(),
            pos: 0,
            start: 0,
            at_eof: false,
            paren_depth: 0,
            line: 1,
            start_line: 1,
            item: Item::default(),
            inside_action: false,
            options: LexOptions::default(),
        }
    }

    // Go: lex.go:(*lexer).next
    /// Returns the next rune in the input.
    fn next(&mut self) -> Rune {
        if self.pos >= self.input.len() {
            self.at_eof = true;
            return EOF;
        }
        let (r, w) = utf8::decode_rune_in_string(&self.input[self.pos..]);
        self.pos += w;
        if r == '\n' as Rune {
            self.line += 1;
        }
        r
    }

    // Go: lex.go:(*lexer).peek
    /// Returns but does not consume the next rune in the input.
    fn peek(&mut self) -> Rune {
        let r = self.next();
        self.backup();
        r
    }

    // Go: lex.go:(*lexer).backup
    /// Steps back one rune.
    fn backup(&mut self) {
        if !self.at_eof && self.pos > 0 {
            let (r, w) = utf8::decode_last_rune(&self.input[..self.pos]);
            self.pos -= w;
            // Correct newline count.
            if r == '\n' as Rune {
                self.line -= 1;
            }
        }
    }

    // Go: lex.go:(*lexer).thisItem
    /// Returns the item at the current input point with the specified type
    /// and advances the input.
    fn this_item(&mut self, t: ItemType) -> Item {
        let i = Item {
            typ: t,
            pos: self.start,
            val: self.input[self.start..self.pos].to_vec(),
            line: self.start_line,
        };
        self.start = self.pos;
        self.start_line = self.line;
        i
    }

    // Go: lex.go:(*lexer).emit
    /// Passes the trailing text as an item back to the parser.
    fn emit(&mut self, t: ItemType) -> StateResult {
        let i = self.this_item(t);
        self.emit_item(i)
    }

    // Go: lex.go:(*lexer).emitItem
    /// Passes the specified item to the parser.
    fn emit_item(&mut self, i: Item) -> StateResult {
        self.item = i;
        None
    }

    // Go: lex.go:(*lexer).ignore
    /// Skips over the pending input before this point.
    /// It tracks newlines in the ignored text, so use it only
    /// for text that is skipped without calling l.next.
    fn ignore(&mut self) {
        self.line += count_newlines(&self.input[self.start..self.pos]);
        self.start = self.pos;
        self.start_line = self.line;
    }

    // Go: lex.go:(*lexer).accept
    /// Consumes the next rune if it's from the valid set.
    fn accept(&mut self, valid: &str) -> bool {
        let r = self.next();
        if contains_rune(valid, r) {
            return true;
        }
        self.backup();
        false
    }

    // Go: lex.go:(*lexer).acceptRun
    /// Consumes a run of runes from the valid set.
    fn accept_run(&mut self, valid: &str) {
        loop {
            let r = self.next();
            if !contains_rune(valid, r) {
                break;
            }
        }
        self.backup();
    }

    // Go: lex.go:(*lexer).errorf
    /// Returns an error token and terminates the scan by passing
    /// back a nil pointer that will be the next state, terminating l.nextItem.
    fn errorf(&mut self, msg: String) -> StateResult {
        self.item = Item {
            typ: ItemType::Error,
            pos: self.start,
            val: msg.into_bytes(),
            line: self.start_line,
        };
        self.start = 0;
        self.pos = 0;
        self.input = &self.input[..0];
        None
    }

    // Go: lex.go:(*lexer).nextItem
    /// Returns the next item from the input.
    pub fn next_item(&mut self) -> Item {
        self.item = Item {
            typ: ItemType::Eof,
            pos: self.pos,
            val: b"EOF".to_vec(),
            line: self.start_line,
        };
        let mut state = if self.inside_action {
            State::InsideAction
        } else {
            State::Text
        };
        loop {
            let next = match state {
                State::Text => self.lex_text(),
                State::LeftDelim => self.lex_left_delim(),
                State::Comment => self.lex_comment(),
                State::RightDelim => self.lex_right_delim(),
                State::InsideAction => self.lex_inside_action(),
                State::Space => self.lex_space(),
                State::Identifier => self.lex_identifier(),
                State::Field => self.lex_field(),
                State::Variable => self.lex_variable(),
                State::Char => self.lex_char(),
                State::Number => self.lex_number(),
                State::Quote => self.lex_quote(),
                State::RawQuote => self.lex_raw_quote(),
            };
            match next {
                None => return std::mem::take(&mut self.item),
                Some(s) => state = s,
            }
        }
    }

    // state functions

    // Go: lex.go:lexText
    /// Scans until an opening action delimiter, "{{".
    fn lex_text(&mut self) -> StateResult {
        if let Some(x) = find(&self.input[self.pos..], &self.left_delim) {
            if x > 0 {
                self.pos += x;
                // Do we trim any trailing space?
                let mut trim_length: Pos = 0;
                let delim_end = self.pos + self.left_delim.len();
                if has_left_trim_marker(&self.input[delim_end..]) {
                    trim_length = right_trim_length(&self.input[self.start..self.pos]);
                }
                self.pos -= trim_length;
                self.line += count_newlines(&self.input[self.start..self.pos]);
                let i = self.this_item(ItemType::Text);
                self.pos += trim_length;
                self.ignore();
                if !i.val.is_empty() {
                    return self.emit_item(i);
                }
            }
            return Some(State::LeftDelim);
        }
        self.pos = self.input.len();
        // Correctly reached EOF.
        if self.pos > self.start {
            self.line += count_newlines(&self.input[self.start..self.pos]);
            return self.emit(ItemType::Text);
        }
        self.emit(ItemType::Eof)
    }

    // Go: lex.go:(*lexer).atRightDelim
    /// Reports whether the lexer is at a right delimiter, possibly preceded
    /// by a trim marker.
    fn at_right_delim(&self) -> (bool, bool) {
        let rest = &self.input[self.pos..];
        if has_right_trim_marker(rest) && rest[TRIM_MARKER_LEN..].starts_with(&self.right_delim) {
            // With trim marker.
            return (true, true);
        }
        if rest.starts_with(&self.right_delim) {
            // Without trim marker.
            return (true, false);
        }
        (false, false)
    }

    // Go: lex.go:lexLeftDelim
    /// Scans the left delimiter, which is known to be present, possibly with
    /// a trim marker. (The text to be trimmed has already been emitted.)
    fn lex_left_delim(&mut self) -> StateResult {
        self.pos += self.left_delim.len();
        let trim_space = has_left_trim_marker(&self.input[self.pos..]);
        let after_marker = if trim_space { TRIM_MARKER_LEN } else { 0 };
        if self.input[self.pos + after_marker..].starts_with(LEFT_COMMENT) {
            self.pos += after_marker;
            self.ignore();
            return Some(State::Comment);
        }
        let i = self.this_item(ItemType::LeftDelim);
        self.inside_action = true;
        self.pos += after_marker;
        self.ignore();
        self.paren_depth = 0;
        self.emit_item(i)
    }

    // Go: lex.go:lexComment
    /// Scans a comment. The left comment marker is known to be present.
    fn lex_comment(&mut self) -> StateResult {
        self.pos += LEFT_COMMENT.len();
        let Some(x) = find(&self.input[self.pos..], RIGHT_COMMENT) else {
            return self.errorf("unclosed comment".to_string());
        };
        self.pos += x + RIGHT_COMMENT.len();
        let (delim, trim_space) = self.at_right_delim();
        if !delim {
            return self.errorf("comment ends before closing delimiter".to_string());
        }
        self.line += count_newlines(&self.input[self.start..self.pos]);
        let i = self.this_item(ItemType::Comment);
        if trim_space {
            self.pos += TRIM_MARKER_LEN;
        }
        self.pos += self.right_delim.len();
        if trim_space {
            self.pos += left_trim_length(&self.input[self.pos..]);
        }
        self.ignore();
        if self.options.emit_comment {
            return self.emit_item(i);
        }
        Some(State::Text)
    }

    // Go: lex.go:lexRightDelim
    /// Scans the right delimiter, which is known to be present, possibly
    /// with a trim marker.
    fn lex_right_delim(&mut self) -> StateResult {
        let (_, trim_space) = self.at_right_delim();
        if trim_space {
            self.pos += TRIM_MARKER_LEN;
            self.ignore();
        }
        self.pos += self.right_delim.len();
        let i = self.this_item(ItemType::RightDelim);
        if trim_space {
            self.pos += left_trim_length(&self.input[self.pos..]);
            self.ignore();
        }
        self.inside_action = false;
        self.emit_item(i)
    }

    // Go: lex.go:lexInsideAction
    /// Scans the elements inside action delimiters.
    fn lex_inside_action(&mut self) -> StateResult {
        // Either number, quoted string, or identifier.
        // Spaces separate arguments; runs of spaces turn into itemSpace.
        // Pipe symbols separate and are emitted.
        let (delim, _) = self.at_right_delim();
        if delim {
            if self.paren_depth == 0 {
                return Some(State::RightDelim);
            }
            return self.errorf("unclosed left paren".to_string());
        }
        let r = self.next();
        if r == EOF {
            return self.errorf("unclosed action".to_string());
        }
        if is_space(r) {
            self.backup(); // Put space back in case we have " -}}".
            return Some(State::Space);
        }
        match char_of(r) {
            '=' => return self.emit(ItemType::Assign),
            ':' => {
                if self.next() != '=' as Rune {
                    return self.errorf("expected :=".to_string());
                }
                return self.emit(ItemType::Declare);
            }
            '|' => return self.emit(ItemType::Pipe),
            '"' => return Some(State::Quote),
            '`' => return Some(State::RawQuote),
            '$' => return Some(State::Variable),
            '\'' => return Some(State::Char),
            '.' => {
                // special look-ahead for ".field" so we don't break l.backup().
                if self.pos < self.input.len() {
                    let r = self.input[self.pos];
                    if !r.is_ascii_digit() {
                        return Some(State::Field);
                    }
                }
                // fallthrough: '.' can start a number.
                self.backup();
                return Some(State::Number);
            }
            _ => {}
        }
        if r == '+' as Rune || r == '-' as Rune || ('0' as Rune..='9' as Rune).contains(&r) {
            self.backup();
            return Some(State::Number);
        }
        if is_alpha_numeric(r) {
            self.backup();
            return Some(State::Identifier);
        }
        if r == '(' as Rune {
            self.paren_depth += 1;
            return self.emit(ItemType::LeftParen);
        }
        if r == ')' as Rune {
            self.paren_depth -= 1;
            if self.paren_depth < 0 {
                return self.errorf("unexpected right paren".to_string());
            }
            return self.emit(ItemType::RightParen);
        }
        if r <= go_unicode::MAX_ASCII && go_unicode::is_print(r) {
            return self.emit(ItemType::Char);
        }
        let msg = sharp_u(r);
        self.errorf(format!("unrecognized character in action: {msg}"))
    }

    // Go: lex.go:lexSpace
    /// Scans a run of space characters.
    /// We have not consumed the first space, which is known to be present.
    /// Take care if there is a trim-marked right delimiter, which starts with a space.
    fn lex_space(&mut self) -> StateResult {
        let mut num_spaces = 0;
        loop {
            let r = self.peek();
            if !is_space(r) {
                break;
            }
            self.next();
            num_spaces += 1;
        }
        // Be careful about a trim-marked closing delimiter, which has a minus
        // after a space. We know there is a space, so check for the '-' that might follow.
        if has_right_trim_marker(&self.input[self.pos - 1..])
            && self.input[self.pos - 1 + TRIM_MARKER_LEN..].starts_with(&self.right_delim)
        {
            self.backup(); // Before the space.
            if num_spaces == 1 {
                return Some(State::RightDelim); // On the delim, so go right to that.
            }
        }
        self.emit(ItemType::Space)
    }

    // Go: lex.go:lexIdentifier
    /// Scans an alphanumeric.
    fn lex_identifier(&mut self) -> StateResult {
        loop {
            let r = self.next();
            if is_alpha_numeric(r) {
                // absorb.
                continue;
            }
            self.backup();
            let word = &self.input[self.start..self.pos];
            if !self.at_terminator() {
                let msg = sharp_u(r);
                return self.errorf(format!("bad character {msg}"));
            }
            let k = key(word);
            if let Some(item) = k.filter(|k| k.is_keyword()) {
                if item == ItemType::Break && !self.options.break_ok
                    || item == ItemType::Continue && !self.options.continue_ok
                {
                    return self.emit(ItemType::Identifier);
                }
                return self.emit(item);
            }
            if word[0] == b'.' {
                return self.emit(ItemType::Field);
            }
            if word == b"true" || word == b"false" {
                return self.emit(ItemType::Bool);
            }
            return self.emit(ItemType::Identifier);
        }
    }

    // Go: lex.go:lexField
    /// Scans a field: .Alphanumeric. The . has been scanned.
    fn lex_field(&mut self) -> StateResult {
        self.lex_field_or_variable(ItemType::Field)
    }

    // Go: lex.go:lexVariable
    /// Scans a Variable: $Alphanumeric. The $ has been scanned.
    fn lex_variable(&mut self) -> StateResult {
        if self.at_terminator() {
            // Nothing interesting follows -> "$".
            return self.emit(ItemType::Variable);
        }
        self.lex_field_or_variable(ItemType::Variable)
    }

    // Go: lex.go:lexFieldOrVariable
    /// Scans a field or variable: [.$]Alphanumeric. The . or $ has been scanned.
    fn lex_field_or_variable(&mut self, typ: ItemType) -> StateResult {
        if self.at_terminator() {
            // Nothing interesting follows -> "." or "$".
            if typ == ItemType::Variable {
                return self.emit(ItemType::Variable);
            }
            return self.emit(ItemType::Dot);
        }
        let mut r;
        loop {
            r = self.next();
            if !is_alpha_numeric(r) {
                self.backup();
                break;
            }
        }
        if !self.at_terminator() {
            let msg = sharp_u(r);
            return self.errorf(format!("bad character {msg}"));
        }
        self.emit(typ)
    }

    // Go: lex.go:(*lexer).atTerminator
    /// Reports whether the input is at valid termination character to
    /// appear after an identifier. Breaks .X.Y into two pieces. Also catches cases
    /// like "$x+2" not being acceptable without a space, in case we decide one
    /// day to implement arithmetic.
    fn at_terminator(&mut self) -> bool {
        let r = self.peek();
        if is_space(r) {
            return true;
        }
        if r == EOF {
            return true;
        }
        if matches!(char_of(r), '.' | ',' | '|' | ':' | ')' | '(') {
            return true;
        }
        self.input[self.pos..].starts_with(&self.right_delim)
    }

    // Go: lex.go:lexChar
    /// Scans a character constant. The initial quote is already
    /// scanned. Syntax checking is done by the parser.
    fn lex_char(&mut self) -> StateResult {
        loop {
            let r = self.next();
            if r == '\\' as Rune {
                let r = self.next();
                if r != EOF && r != '\n' as Rune {
                    continue;
                }
                return self.errorf("unterminated character constant".to_string());
            }
            if r == EOF || r == '\n' as Rune {
                return self.errorf("unterminated character constant".to_string());
            }
            if r == '\'' as Rune {
                break;
            }
        }
        self.emit(ItemType::CharConstant)
    }

    // Go: lex.go:lexNumber
    /// Scans a number: decimal, octal, hex, float, or imaginary. This
    /// isn't a perfect number scanner - for instance it accepts "." and "0x0.2"
    /// and "089" - but when it's wrong the input is invalid and the parser (via
    /// strconv) will notice.
    fn lex_number(&mut self) -> StateResult {
        if !self.scan_number() {
            let q = go_strconv::quote(&self.input[self.start..self.pos]);
            return self.errorf(format!("bad number syntax: {q}"));
        }
        let sign = self.peek();
        if sign == '+' as Rune || sign == '-' as Rune {
            // Complex: 1+2i. No spaces, must end in 'i'.
            if !self.scan_number() || self.input[self.pos - 1] != b'i' {
                let q = go_strconv::quote(&self.input[self.start..self.pos]);
                return self.errorf(format!("bad number syntax: {q}"));
            }
            return self.emit(ItemType::Complex);
        }
        self.emit(ItemType::Number)
    }

    // Go: lex.go:(*lexer).scanNumber
    fn scan_number(&mut self) -> bool {
        // Optional leading sign.
        self.accept("+-");
        // Is it hex?
        let mut digits = "0123456789_";
        if self.accept("0") {
            // Note: Leading 0 does not mean octal in floats.
            if self.accept("xX") {
                digits = "0123456789abcdefABCDEF_";
            } else if self.accept("oO") {
                digits = "01234567_";
            } else if self.accept("bB") {
                digits = "01_";
            }
        }
        self.accept_run(digits);
        if self.accept(".") {
            self.accept_run(digits);
        }
        if digits.len() == 10 + 1 && self.accept("eE") {
            self.accept("+-");
            self.accept_run("0123456789_");
        }
        if digits.len() == 16 + 6 + 1 && self.accept("pP") {
            self.accept("+-");
            self.accept_run("0123456789_");
        }
        // Is it imaginary?
        self.accept("i");
        // Next thing mustn't be alphanumeric.
        if is_alpha_numeric(self.peek()) {
            self.next();
            return false;
        }
        true
    }

    // Go: lex.go:lexQuote
    /// Scans a quoted string.
    fn lex_quote(&mut self) -> StateResult {
        loop {
            let r = self.next();
            if r == '\\' as Rune {
                let r = self.next();
                if r != EOF && r != '\n' as Rune {
                    continue;
                }
                return self.errorf("unterminated quoted string".to_string());
            }
            if r == EOF || r == '\n' as Rune {
                return self.errorf("unterminated quoted string".to_string());
            }
            if r == '"' as Rune {
                break;
            }
        }
        self.emit(ItemType::String)
    }

    // Go: lex.go:lexRawQuote
    /// Scans a raw quoted string.
    fn lex_raw_quote(&mut self) -> StateResult {
        loop {
            let r = self.next();
            if r == EOF {
                return self.errorf("unterminated raw quoted string".to_string());
            }
            if r == '`' as Rune {
                break;
            }
        }
        self.emit(ItemType::RawString)
    }
}

/// `r` as a char for matching; invalid runes map to U+FFFD (never matched).
fn char_of(r: Rune) -> char {
    char::from_u32(r as u32).unwrap_or('\u{FFFD}')
}

/// Go: `fmt.Sprintf("%#U", r)`.
fn sharp_u(r: Rune) -> String {
    let b = go_fmt::sprintf("%#U", &[Value::Int(r as i64, go_value::IntKind::Int32)]);
    String::from_utf8_lossy(&b).into_owned()
}

/// Go: `strings.ContainsRune(valid, r)` for an ASCII `valid` set.
fn contains_rune(valid: &str, r: Rune) -> bool {
    r >= 0 && valid.bytes().any(|b| b as Rune == r)
}

/// Go: `strings.Index` returning `None` for -1.
pub(crate) fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let i = go_unicode::strings::index(haystack, needle);
    if i < 0 { None } else { Some(i as usize) }
}

fn count_newlines(s: &[u8]) -> usize {
    s.iter().filter(|&&b| b == b'\n').count()
}

// Go: lex.go:rightTrimLength
/// Returns the length of the spaces at the end of the string.
fn right_trim_length(s: &[u8]) -> Pos {
    s.len() - go_unicode::strings::trim_right(s, SPACE_CHARS).len()
}

// Go: lex.go:leftTrimLength
/// Returns the length of the spaces at the beginning of the string.
fn left_trim_length(s: &[u8]) -> Pos {
    s.len() - go_unicode::strings::trim_left(s, SPACE_CHARS).len()
}

// Go: lex.go:isSpace
/// Reports whether r is a space character.
fn is_space(r: Rune) -> bool {
    r == ' ' as Rune || r == '\t' as Rune || r == '\r' as Rune || r == '\n' as Rune
}

// Go: lex.go:isAlphaNumeric
/// Reports whether r is an alphabetic, digit, or underscore.
fn is_alpha_numeric(r: Rune) -> bool {
    r == '_' as Rune || go_unicode::is_letter(r) || go_unicode::is_digit(r)
}

// Go: lex.go:hasLeftTrimMarker
fn has_left_trim_marker(s: &[u8]) -> bool {
    s.len() >= 2 && s[0] == TRIM_MARKER && is_space(s[1] as Rune)
}

// Go: lex.go:hasRightTrimMarker
fn has_right_trim_marker(s: &[u8]) -> bool {
    s.len() >= 2 && is_space(s[0] as Rune) && s[1] == TRIM_MARKER
}
